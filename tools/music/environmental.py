#!/usr/bin/env python3
"""Synthesise the ambience beds: gentle environmental music in the spirit of
1980s Japanese kankyō ongaku (soft FM bells, glassy mallets, slow warm pads and
water), composed here from scratch with no samples.

    uv run --with numpy python tools/music/environmental.py

Writes assets/audio/ambience/day_garden.ogg and night_garden.ogg. Each is
rendered as a true loop: every note, echo and reverb tail that runs past the
end wraps around to the start, so the bed repeats with no seam. Both stay in
one pentatonic colour (D major pentatonic by day, its relative B minor at
night) so they sit under the lo-fi music without fighting it.

Needs numpy and ffmpeg (for loudness measurement and OGG Vorbis encoding).
"""

import json
import os
import subprocess
import tempfile
import wave

import numpy as np

REPO = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
OUT_DIR = os.path.join(REPO, "assets", "audio", "ambience")
SR = 32000
LUFS = -27.0        # the ambience bus sits under the music (see build_music.py)
TRUE_PEAK = -1.5    # dBFS ceiling before encoding


def hz(midi):
    return 440.0 * 2 ** ((midi - 69) / 12)


class Loop:
    """A circular stereo buffer: anything written past the end wraps around."""

    def __init__(self, secs):
        self.n = int(secs * SR)
        self.buf = np.zeros((2, self.n), np.float64)

    def add(self, start_s, mono, pan=0.0, gain=1.0):
        """Mixes a mono signal in at `start_s` seconds, panned -1 (left)..1 (right)."""
        i0 = int(start_s * SR) % self.n
        idx = (np.arange(len(mono)) + i0) % self.n
        l = np.cos((pan + 1) * np.pi / 4) * gain
        r = np.sin((pan + 1) * np.pi / 4) * gain
        np.add.at(self.buf[0], idx, mono * l)
        np.add.at(self.buf[1], idx, mono * r)


def t_axis(secs):
    return np.arange(int(secs * SR)) / SR


def fm_bell(f, secs, ratio=3.5, index=2.2, decay=2.2, bright=1.0):
    """Two-operator FM bell: the modulator's index fades faster than the tone,
    so the strike is glassy and the tail a pure sine."""
    t = t_axis(secs)
    idx = index * bright * np.exp(-t / (decay * 0.35))
    env = np.exp(-t / decay) * (1 - np.exp(-t / 0.004))
    return np.sin(2 * np.pi * f * t + idx * np.sin(2 * np.pi * f * ratio * t)) * env


def mallet(f, secs=1.6):
    """A soft marimba-like mallet: FM ratio 4 with a quick-fading bite."""
    t = t_axis(secs)
    env = np.exp(-t / 0.45) * (1 - np.exp(-t / 0.002))
    idx = 1.6 * np.exp(-t / 0.05)
    return (np.sin(2 * np.pi * f * t + idx * np.sin(2 * np.pi * 4.0 * f * t)) * env
            + 0.25 * np.sin(2 * np.pi * 2 * f * t) * np.exp(-t / 0.2))


def pad_voice(f, secs, rng):
    """A warm detuned pad tone: a few soft harmonics on three slightly detuned
    oscillators, so it breathes like a chorus."""
    t = t_axis(secs)
    out = np.zeros_like(t)
    for det in (-0.06, 0.0, 0.07):
        ff = f * 2 ** (det / 12)
        ph = rng.random() * 2 * np.pi
        for h, a in ((1, 1.0), (2, 0.32), (3, 0.12), (4, 0.05)):
            out += a * np.sin(2 * np.pi * ff * h * t + ph * h)
    trem = 1 + 0.08 * np.sin(2 * np.pi * 0.11 * t + rng.random() * 6)
    return out * trem / 3


def raised_cosine(n, fade):
    """Window that fades in and out over `fade` samples."""
    w = np.ones(n)
    k = np.arange(fade)
    ramp = 0.5 - 0.5 * np.cos(np.pi * k / fade)
    w[:fade] = ramp
    w[-fade:] = ramp[::-1]
    return w


def water(secs, rng, level):
    """A slow stream: band-limited noise with a lazy swell, made periodic."""
    n = int(secs * SR)
    spec = np.fft.rfft(rng.standard_normal(n))
    f = np.fft.rfftfreq(n, 1 / SR)
    shape = np.exp(-((np.log(f + 1) - np.log(900)) ** 2) / 0.9)
    noise = np.fft.irfft(spec * shape, n)
    noise /= np.abs(noise).max() + 1e-9
    t = np.arange(n) / SR
    k = round(secs / 9)
    swell = 0.65 + 0.35 * np.sin(2 * np.pi * k * t / secs)
    return noise * swell * level


def droplet(rng):
    """A water plink: a short sine sweeping upward, quickly gone."""
    secs = 0.18
    t = t_axis(secs)
    f0 = rng.uniform(900, 1800)
    f = f0 * (1 + 1.8 * t / secs)
    ph = 2 * np.pi * np.cumsum(f) / SR
    return np.sin(ph) * np.exp(-t / 0.035) * (1 - np.exp(-t / 0.002))


def cricket(rng):
    """One cricket phrase: a few fast pulses of a high, soft tone."""
    f = rng.uniform(4200, 4800)
    pulse = int(0.012 * SR)
    gap = int(0.018 * SR)
    count = rng.integers(3, 6)
    out = np.zeros((pulse + gap) * count)
    tt = np.arange(pulse) / SR
    blip = np.sin(2 * np.pi * f * tt) * np.hanning(pulse)
    for k in range(count):
        out[k * (pulse + gap):k * (pulse + gap) + pulse] = blip
    return out


def reverb(buf, secs, rng, decay=2.6, mix=0.32):
    """Circular convolution with a decaying-noise impulse: a soft hall whose
    tail wraps around the loop like everything else."""
    n = buf.shape[1]
    t = np.arange(int(secs * SR)) / SR
    out = buf.copy()
    for ch in range(2):
        ir = rng.standard_normal(len(t)) * np.exp(-t * 6.9 / decay)
        # Darker tail: high frequencies die sooner.
        ir = lowpass_fast(ir, 3500)
        ir /= np.sqrt(np.sum(ir ** 2))
        pad = np.zeros(n)
        pad[: len(ir)] = ir
        wet = np.fft.irfft(np.fft.rfft(buf[ch]) * np.fft.rfft(pad), n)
        out[ch] = buf[ch] * (1 - mix) + wet * mix * 1.6
    return out


def lowpass_fast(x, cutoff):
    spec = np.fft.rfft(x)
    f = np.fft.rfftfreq(len(x), 1 / SR)
    return np.fft.irfft(spec / (1 + (f / cutoff) ** 2), len(x))


def echo(loop, delay, feedback, mix):
    """Ping-pong echo, circular: left feeds right and back."""
    d = int(delay * SR)
    src = loop.buf.copy()
    l, r = src[0].copy(), src[1].copy()
    acc_l, acc_r = np.zeros_like(l), np.zeros_like(r)
    g = mix
    for k in range(1, 7):
        if k % 2:
            acc_r += np.roll(l, d * k) * g
            acc_l += np.roll(r, d * k) * g
        else:
            acc_l += np.roll(l, d * k) * g
            acc_r += np.roll(r, d * k) * g
        g *= feedback
    loop.buf[0] += acc_l
    loop.buf[1] += acc_r


def compose(night):
    rng = np.random.default_rng(7 if not night else 11)
    secs = 64.0 if not night else 72.0
    loop = Loop(secs)
    bar = secs / 4  # four chords per loop

    # ---- pad: four slow chords, each fading into the next ----
    if not night:
        # D major pentatonic colour: Dmaj9, Bm11, Gmaj9 (no 4th), A add9
        chords = [[50, 57, 64, 66, 73], [47, 54, 61, 64, 69], [43, 50, 57, 62, 66], [45, 52, 59, 64, 69]]
        pad_gain = 0.055
    else:
        # B minor colour, lower and slower: Bm9, Gmaj7, Em9, F#m7(11)
        chords = [[47, 54, 61, 62, 66], [43, 50, 54, 59, 62], [40, 47, 54, 57, 62], [42, 49, 54, 57, 59]]
        pad_gain = 0.05
    fade = int(bar * 0.45 * SR)
    for k, ch in enumerate(chords):
        span = bar + bar * 0.45
        for m in ch:
            v = pad_voice(hz(m), span, rng)
            v *= raised_cosine(len(v), fade)
            pan = rng.uniform(-0.5, 0.5)
            loop.add(k * bar - bar * 0.225, v, pan, pad_gain)

    # ---- bells: sparse pentatonic strikes high up, echoing across ----
    scale = [62, 64, 66, 69, 71] if not night else [59, 62, 64, 66, 69]
    octaves = [12, 24] if not night else [12, 12, 24]
    t = 0.0
    step = (1.3, 3.2) if not night else (2.4, 5.0)
    while t < secs:
        note = scale[rng.integers(len(scale))] + octaves[rng.integers(len(octaves))]
        b = fm_bell(hz(note), 5.0, ratio=rng.choice([3.5, 2.0, 1.4]), decay=rng.uniform(1.6, 2.8),
                    bright=0.8 if night else 1.0)
        loop.add(t, b, rng.uniform(-0.7, 0.7), 0.07 if not night else 0.06)
        t += rng.uniform(*step)

    # ---- mallet phrases: a short rising-falling figure every few bars ----
    figures = [[0, 2, 4, 3], [4, 3, 1, 2], [1, 2, 4, 2, 0]]
    every = 8.0 if not night else 12.0
    for k in range(int(secs / every)):
        if night and k % 2:
            continue
        fig = figures[k % len(figures)]
        base = k * every + rng.uniform(0.5, 2.0)
        for j, deg in enumerate(fig):
            note = scale[deg] + (12 if not night else 0)
            loop.add(base + j * 0.21 + rng.uniform(0, 0.02), mallet(hz(note)), 0.3 * np.sin(j), 0.06)

    echo(loop, 0.375 if not night else 0.5, 0.45, 0.33)

    # ---- nature ----
    stream = water(secs, rng, 0.012 if not night else 0.008)
    loop.buf[0] += stream
    loop.buf[1] += np.roll(stream, int(0.37 * SR))
    if not night:
        t = rng.uniform(0, 3)
        while t < secs:
            loop.add(t, droplet(rng), rng.uniform(-0.9, 0.9), 0.025)
            t += rng.exponential(2.2)
    else:
        t = rng.uniform(0, 2)
        while t < secs:
            loop.add(t, cricket(rng), rng.uniform(-0.9, 0.9), 0.012)
            t += rng.exponential(1.1)

    out = reverb(loop.buf, 4.0, rng, decay=3.2 if not night else 4.0, mix=0.35 if not night else 0.42)
    # Gentle top-end roll-off for a soft, warm finish.
    for ch in range(2):
        out[ch] = lowpass_fast(out[ch], 9000)
    return out


def write_wav(path, stereo):
    pcm = (np.clip(stereo.T, -1, 1) * 32767).astype("<i2")
    with wave.open(path, "wb") as w:
        w.setnchannels(2)
        w.setsampwidth(2)
        w.setframerate(SR)
        w.writeframes(pcm.tobytes())


def loudness(path):
    r = subprocess.run(["ffmpeg", "-hide_banner", "-nostats", "-i", path, "-af",
                        "loudnorm=print_format=json", "-f", "null", "-"], capture_output=True, text=True)
    j = r.stderr[r.stderr.rindex("{"):r.stderr.rindex("}") + 1]
    return float(json.loads(j)["input_i"])


def master(stereo, out_ogg):
    with tempfile.TemporaryDirectory() as d:
        wav = os.path.join(d, "x.wav")
        stereo = stereo / (np.abs(stereo).max() + 1e-9) * 0.5
        write_wav(wav, stereo)
        gain = 10 ** ((LUFS - loudness(wav)) / 20)
        stereo = stereo * gain
        ceiling = 10 ** (TRUE_PEAK / 20)
        peak = np.abs(stereo).max()
        if peak > ceiling:
            stereo *= ceiling / peak
        write_wav(wav, stereo)
        subprocess.run(["ffmpeg", "-y", "-hide_banner", "-loglevel", "error", "-i", wav,
                        "-c:a", "libvorbis", "-q:a", "5", out_ogg], check=True)
        return loudness(out_ogg)


def main():
    os.makedirs(OUT_DIR, exist_ok=True)
    for name, night in (("day_garden", False), ("night_garden", True)):
        out = os.path.join(OUT_DIR, f"{name}.ogg")
        lu = master(compose(night), out)
        print(f"[ambience] {name:13s} {lu:+.1f} LUFS  -> {os.path.relpath(out, REPO)}")


if __name__ == "__main__":
    main()

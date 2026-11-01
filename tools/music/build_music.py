#!/usr/bin/env python3
"""Build mastered lo-fi music, ambience and foley from the Mondo Loops packs.

Offline build tool (stdlib + numpy).  It reads the owner's licensed sample packs
from a gitignored folder and writes ORIGINAL arrangements to `assets/audio/` and a
generated `src/audio_list.rs`.  It never copies a raw sample into the repository.

Run:

    uv run --with numpy python tools/music/build_music.py --packs <packs-dir>
    uv run --with numpy python tools/music/build_music.py --check

`--check` only re-measures the committed outputs under `assets/audio/` and exits
non-zero if any loudness / true-peak / memory target is missed.

Licence: the packs are the owner's personal, non-transferable licence from
https://mondoloops.com/pages/free-lofi-samples -- never commit them, and never ship
a raw sample as-is (see tools/music/README.md).
"""

from __future__ import annotations

import argparse
import json
import math
import os
import re
import subprocess
import sys
import tempfile

import numpy as np

SR = 32000  # everything is decoded / rendered at 32 kHz stereo

REPO = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
MUSIC_DIR = os.path.join(REPO, "assets", "audio", "music")
AMBIENCE_DIR = os.path.join(REPO, "assets", "audio", "ambience")
SFX_DIR = os.path.join(REPO, "assets", "audio", "sfx")
AUDIO_LIST_RS = os.path.join(REPO, "src", "audio_list.rs")
PREVIEW_OGG = "/tmp/claude-1000/sound-preview.ogg"

# Loudness targets.
MUSIC_LUFS = -16.0
AMBIENCE_LUFS = -28.0
TRUE_PEAK = -1.0          # dBTP
SFX_PEAK_DBFS = -3.0      # dBFS
MEMORY_BUDGET_BYTES = 160 * 1024 * 1024  # 160 MB of decoded f32 audio


# ---------------------------------------------------------------------------
# ffmpeg / ffprobe plumbing
# ---------------------------------------------------------------------------

def run_checked(cmd, **kw):
    p = subprocess.run(cmd, capture_output=True, **kw)
    if p.returncode != 0:
        raise RuntimeError(
            f"command failed ({p.returncode}): {' '.join(cmd)}\n"
            + p.stderr.decode(errors="replace")[-4000:]
        )
    return p


def decode(path, sr=SR):
    """Decode an audio file to float32 stereo at `sr` (shape (n, 2))."""
    p = subprocess.run(
        ["ffmpeg", "-v", "error", "-i", path, "-f", "f32le", "-ac", "2", "-ar", str(sr), "-"],
        capture_output=True,
    )
    if p.returncode != 0:
        raise RuntimeError(f"ffmpeg decode failed for {path}:\n" + p.stderr.decode(errors="replace"))
    x = np.frombuffer(p.stdout, dtype=np.float32).reshape(-1, 2)
    return x


def decode_stretched(path, tempo=None, pitch=None, sr=SR):
    """Decode with optional rubberband time-stretch (ratio) and pitch (semitones)."""
    af = []
    if tempo is not None or pitch is not None:
        parts = []
        if pitch is not None:
            # rubberband `pitch` is a frequency ratio; semitones -> 2**(n/12).
            parts.append(f"pitch={2 ** (pitch / 12.0):.9f}")
        if tempo is not None:
            parts.append(f"tempo={tempo:.9f}")
        af.append("rubberband=" + ":".join(parts))
    cmd = ["ffmpeg", "-v", "error", "-i", path]
    if af:
        cmd += ["-af", ",".join(af)]
    cmd += ["-f", "f32le", "-ac", "2", "-ar", str(sr), "-"]
    p = subprocess.run(cmd, capture_output=True)
    if p.returncode != 0:
        raise RuntimeError(
            f"ffmpeg rubberband decode failed for {path}:\n" + p.stderr.decode(errors="replace")
        )
    return np.frombuffer(p.stdout, dtype=np.float32).reshape(-1, 2)


def encode_ogg(x, out_ogg, sr=SR):
    """Encode a float32 stereo array to OGG Vorbis -q:a 5 (plain, no loudnorm)."""
    os.makedirs(os.path.dirname(out_ogg), exist_ok=True)
    run_checked(
        ["ffmpeg", "-v", "error", "-f", "f32le", "-ar", str(sr), "-ac", "2", "-i", "-",
         "-c:a", "libvorbis", "-q:a", "5", out_ogg, "-y"],
        input=np.ascontiguousarray(x, dtype=np.float32).tobytes(),
    )


def extract_json(text):
    a = text.find("{")
    b = text.rfind("}")
    if a < 0 or b < a:
        raise RuntimeError("no loudnorm JSON in ffmpeg output:\n" + text[-4000:])
    return json.loads(text[a:b + 1])


def loudnorm_normalize(x, target, tp=TRUE_PEAK, lra=11.0, sr=SR):
    """Two-pass EBU R128 loudnorm to `target` LUFS; returns the level-corrected float."""
    raw = np.ascontiguousarray(x, dtype=np.float32).tobytes()
    fd, tmp = tempfile.mkstemp(suffix=".f32")
    fd2, wav = tempfile.mkstemp(suffix=".wav")
    os.close(fd2)
    try:
        with os.fdopen(fd, "wb") as f:
            f.write(raw)
        base = ["ffmpeg", "-v", "info", "-f", "f32le", "-ar", str(sr), "-ac", "2", "-i", tmp]
        p1 = subprocess.run(
            base + ["-af", f"loudnorm=I={target}:TP={tp}:LRA={lra}:print_format=json",
                    "-f", "null", "-"],
            capture_output=True,
        )
        js = extract_json(p1.stderr.decode(errors="replace"))
        af = (f"loudnorm=I={target}:TP={tp}:LRA={lra}:"
              f"measured_I={js['input_i']}:measured_TP={js['input_tp']}:"
              f"measured_LRA={js['input_lra']}:measured_thresh={js['input_thresh']}:"
              f"offset={js['target_offset']}:linear=true")
        run_checked(
            ["ffmpeg", "-v", "error", "-f", "f32le", "-ar", str(sr), "-ac", "2", "-i", tmp,
             "-af", af, "-ar", str(sr), "-c:a", "pcm_s24le", wav, "-y"],
        )
        return decode(wav)
    finally:
        os.unlink(tmp)
        if os.path.exists(wav):
            os.unlink(wav)


def loudnorm_encode(x, out_ogg, target, tp=TRUE_PEAK, lra=11.0, sr=SR):
    """Two-pass EBU R128 loudnorm to `target` LUFS, then OGG Vorbis -q:a 5."""
    x = loudnorm_normalize(x, target, tp, lra, sr)
    encode_ogg(x, out_ogg, sr)


def measure_loudnorm(path):
    p = subprocess.run(
        ["ffmpeg", "-v", "info", "-i", path,
         "-af", f"loudnorm=I={MUSIC_LUFS}:TP={TRUE_PEAK}:LRA=11:print_format=json",
         "-f", "null", "-"],
        capture_output=True,
    )
    return extract_json(p.stderr.decode(errors="replace"))


def measure_peak_dbfs(path):
    p = subprocess.run(
        ["ffmpeg", "-v", "info", "-i", path, "-af", "volumedetect", "-f", "null", "-"],
        capture_output=True,
    )
    s = p.stderr.decode(errors="replace")
    m = re.search(r"max_volume:\s*(-?[\d.]+)\s*dB", s)
    if not m:
        raise RuntimeError(f"volumedetect failed for {path}:\n{s[-2000:]}")
    return float(m.group(1))


def probe_meta(path):
    p = subprocess.run(
        ["ffprobe", "-v", "error", "-show_entries", "stream=sample_rate,channels",
         "-show_entries", "format=duration", "-of", "json", path],
        capture_output=True,
    )
    d = json.loads(p.stdout.decode(errors="replace"))
    st = d["streams"][0]
    return (
        float(d["format"]["duration"]),
        int(st.get("sample_rate") or 0),
        int(st.get("channels") or 0),
    )


# ---------------------------------------------------------------------------
# DSP helpers (stdlib numpy only)
# ---------------------------------------------------------------------------

def lowpass_fir(fc, sr, ntaps):
    n = np.arange(ntaps)
    center = (ntaps - 1) / 2.0
    h = np.sinc(2.0 * fc / sr * (n - center)) * (2.0 * fc / sr)
    h = h * np.hanning(ntaps)
    h /= h.sum()
    return h


def fft_convolve(x, kernel, block=1 << 16):
    """Linear convolution via overlap-add FFT; returns x aligned in time."""
    m = len(kernel)
    n = x.shape[0]
    out = np.zeros((n + m - 1, x.shape[1]), np.float64)
    K = np.fft.rfft(kernel, block)
    L = block - m + 1
    for i in range(0, n, L):
        seg = x[i:i + L]
        if len(seg) < L:
            seg = np.pad(seg, ((0, L - len(seg)), (0, 0)))
        S = np.fft.rfft(seg, block, axis=0)
        chunk = np.fft.irfft(S * K[:, None], block, axis=0)
        j0, j1 = i, min(i + block, n + m - 1)
        out[j0:j1] += chunk[:j1 - j0]
    d = (m - 1) // 2
    return out[d:d + n]


def high_shelf(x, fc=12000.0, gain_db=-3.0, ntaps=1023):
    """Light high-shelf: `g*x + (1-g)*lowpass(x)` == shelf cut of `gain_db` above fc."""
    g = 10 ** (gain_db / 20.0)
    lp = lowpass_fir(fc, SR, ntaps)
    return g * x + (1.0 - g) * fft_convolve(x, lp)


def lowpass(x, fc):
    return fft_convolve(x, lowpass_fir(fc, SR, 255))


def saturate(x, drive=1.5):
    return np.tanh(x * drive) / drive


def true_peak_limiter(x, ceiling_db=-2.2, factor=4):
    """Cap inter-sample (true) peaks at `ceiling_db` dBTP via 4x oversampling.

    Applied after loudnorm to leave headroom for Vorbis' lossy overshoot, so the
    committed OGG measures <= -1.0 dBTP while integrated loudness barely moves.
    """
    ceiling = 10 ** (ceiling_db / 20.0)
    n = x.shape[0]
    X = np.fft.rfft(x, axis=0)
    up = np.fft.irfft(X, n * factor, axis=0) * factor
    mag = np.abs(up)
    g = np.minimum(1.0, ceiling / np.maximum(mag, 1e-9))
    up *= g
    return up[::factor][:n].astype(np.float32)


def tape_wobble(x, sr=SR, depth_samples=30.0, rate_hz=0.5):
    """Subtle pitch/tape wobble: +/-0.3% speed at `rate_hz` (periodic over the clip)."""
    n = x.shape[0]
    t = np.arange(n, dtype=np.float64)
    if n > sr:
        period = n / sr
        k = max(1, int(round(rate_hz * period)))
        rate = k / period
    else:
        rate = rate_hz
    idx = t + depth_samples * np.sin(2 * np.pi * rate * t / sr)
    out = np.empty_like(x, dtype=np.float64)
    for c in range(x.shape[1]):
        out[:, c] = np.interp(idx, t, x[:, c].astype(np.float64))
    return out.astype(np.float32)


def lofi_bus(x):
    x = high_shelf(x)
    x = saturate(x)
    x = tape_wobble(x)
    return x.astype(np.float32)


def make_seamless(x, xf):
    """Crossfade the tail into the head so the clip loops seamlessly."""
    xf = int(xf)
    n = len(x)
    if xf <= 0 or n <= xf:
        return x
    y = x.copy()
    w = np.linspace(0.0, 1.0, xf)
    w_in = np.sin(0.5 * np.pi * w)[:, None]
    w_out = np.cos(0.5 * np.pi * w)[:, None]
    y[-xf:] = x[-xf:] * w_out + x[:xf] * w_in
    return y


def resample_len(x, n):
    if len(x) == n:
        return x
    old = np.arange(len(x), dtype=np.float64)
    new = np.linspace(0, len(x) - 1, n)
    return np.stack(
        [np.interp(new, old, x[:, c]) for c in range(x.shape[1])], axis=1
    ).astype(np.float32)


def prepare_loop(path, bars, bpm_src, bpm_dst, semitones=None):
    """Decode (+rubberband), then snap to exactly `bars` bars at `bpm_dst` so it tiles."""
    tempo = (bpm_dst / bpm_src) if bpm_src != bpm_dst else None
    x = decode_stretched(path, tempo=tempo, pitch=semitones)
    bar_samples = int(round(SR * 240.0 / bpm_dst))
    target = int(round(bars * bar_samples))
    return resample_len(x, target)


def layer_env(boundaries, gains, bar_samples, total_bars, fade_edge=2.0, fade_mid=1.0):
    """Gain envelope: fade in/out at the edges, ramps between arrangement sections."""
    s = len(gains)
    key = [(0.0, 0.0)]
    for i in range(s):
        b0 = boundaries[i]
        b1 = boundaries[i + 1]
        gi = gains[i]
        prev = gains[i - 1] if i > 0 else 0.0
        nxt = gains[i + 1] if i + 1 < s else 0.0
        fe = fade_edge if i == 0 else fade_mid
        fo = fade_edge if i == s - 1 else fade_mid
        rin = b0 + fe
        rout = b1 - fo
        key.append((b0, prev))
        key.append((rin, gi))
        if rout > rin:
            key.append((rout, gi))
        key.append((b1, nxt))
    key.append((float(total_bars), 0.0))
    key.sort(key=lambda k: k[0])
    bars = np.array([k[0] for k in key], np.float64)
    vals = np.array([k[1] for k in key], np.float64)
    idx = np.arange(total_bars * bar_samples) / float(bar_samples)
    return np.interp(idx, bars, vals).astype(np.float32)


# ---------------------------------------------------------------------------
# One-shots, drums, tracks
# ---------------------------------------------------------------------------

def load_oneshot(path):
    x = decode(path)
    x = x - x.mean(axis=0, keepdims=True)  # DC removal
    n = len(x)
    fi = int(0.002 * SR)
    fo = int(0.03 * SR)
    if n > fi + fo:
        x[:fi] *= np.linspace(0.0, 1.0, fi)[:, None]
        x[-fo:] *= np.linspace(1.0, 0.0, fo)[:, None]
    return x


def standard_pattern():
    ev = []
    for i in range(16):
        ev.append((i * 0.5, "hat", 0.18))
    for b in (0.0, 2.0, 4.0, 6.0):
        ev.append((b, "kick", 0.9))
    ev.append((2.5, "kick", 0.7))
    for b in (1.0, 3.0, 5.0, 7.0):
        ev.append((b, "snare", 0.8))
    ev.append((6.5, "snare", 0.5))
    return ev


def build_drums(oneshots, events, bar_samples, total_bars):
    n = total_bars * bar_samples
    out = np.zeros((n, 2), np.float64)
    beat = bar_samples / 4.0
    for base in range(0, total_bars, 2):
        for (b, kind, g) in events:
            off = int(round((base * 4 + b) * beat))
            if off >= n:
                continue
            os_ = oneshots[kind]
            e = min(len(os_), n - off)
            out[off:off + e] += os_[:e] * g
    return out


def render_track(track, lookup):
    bpm = track["bpm"]
    total_bars = track["total_bars"]
    bar_samples = int(round(SR * 240.0 / bpm))
    n = total_bars * bar_samples
    mix = np.zeros((n, 2), np.float64)
    boundaries = [0] + track["sections"] + [total_bars]

    for layer in track["layers"]:
        src = lookup[layer["src"]]
        if layer.get("pad"):
            loop = make_seamless(decode(src), int(2.0 * SR))
        else:
            loop = prepare_loop(src, layer["bars"], layer["src_bpm"], bpm, layer.get("semitones"))
        lb = len(loop)
        reps = (n + lb - 1) // lb
        big = np.tile(loop, (reps, 1))[:n]
        env = layer_env(boundaries, layer["gains"], bar_samples, total_bars)
        mix += big * env[:, None]

    if "drums" in track:
        oneshots = {k: load_oneshot(lookup[v]) for k, v in track["drums"]["samples"].items()}
        dl = build_drums(oneshots, track["drums"]["events"], bar_samples, total_bars)
        env = layer_env(boundaries, track["drums"]["gains"], bar_samples, total_bars)
        mix += dl * env[:, None]

    peak = float(np.max(np.abs(mix)))
    if peak > 1e-4:
        mix *= 0.9 / peak

    x = lofi_bus(mix.astype(np.float32))
    x = make_seamless(x, int(round(2.0 * bar_samples)))
    return x, n / SR


# ---------------------------------------------------------------------------
# SFX + ambience
# ---------------------------------------------------------------------------

def trim_transient(x, thresh=0.08, pre_ms=3.0, post_ms=15.0):
    peak = float(np.max(np.abs(x)))
    if peak < 1e-6:
        return x
    t = np.max(np.abs(x), axis=1)
    above = np.where(t > thresh * peak)[0]
    if len(above) == 0:
        return x
    i0 = max(0, int(above[0]) - int(pre_ms * SR / 1000.0))
    i1 = min(len(x), int(above[-1]) + int(post_ms * SR / 1000.0))
    return x[i0:i1]


def apply_fades(x, fade_in, fade_out):
    n = len(x)
    fi = min(n, int(fade_in * SR))
    fo = min(n, int(fade_out * SR))
    x = x.copy()
    if fi > 0:
        x[:fi] *= np.linspace(0.0, 1.0, fi)[:, None]
    if fo > 0:
        x[-fo:] *= np.linspace(1.0, 0.0, fo)[:, None]
    return x


def add_layers(x, a, gain=1.0):
    if x is None:
        return a * gain
    L = max(len(x), len(a))
    xx = np.zeros((L, 2), np.float32)
    xx[:len(x)] += x
    xx[:len(a)] += a * gain
    return xx


def render_sfx(entry, lookup, out_ogg):
    x = None
    for s in entry["srcs"]:
        a = decode(lookup[s])
        a = a - a.mean(axis=0, keepdims=True)
        a = trim_transient(a)
        if entry.get("lowpass"):
            a = lowpass(a, entry["lowpass"])
        x = add_layers(x, a)
    for s in entry.get("pad_srcs", []):
        a = decode(lookup[s])
        a = a - a.mean(axis=0, keepdims=True)
        x = add_layers(x, a, gain=0.45)
    x = apply_fades(x, 0.005, 0.04)
    if entry.get("max_len"):
        x = x[:int(entry["max_len"] * SR)]
    peak = float(np.max(np.abs(x)))
    if peak > 1e-8:
        x = x * (10 ** (SFX_PEAK_DBFS / 20.0) / peak)
    encode_ogg(x, out_ogg)


def render_ambience(src, out_ogg, target=AMBIENCE_LUFS):
    x = decode(src)
    x = x - x.mean(axis=0, keepdims=True)
    x = make_seamless(x, int(SR * 1.0))
    peak = float(np.max(np.abs(x)))
    if peak > 1e-4:
        x = x * (0.8 / peak)
    loudnorm_encode(x, out_ogg, target, tp=TRUE_PEAK, lra=7.0)


# ---------------------------------------------------------------------------
# Compositions
# ---------------------------------------------------------------------------

TRACKS = [
    dict(
        name="day_meadow", mood="Day", bpm=70, total_bars=32,
        sections=[6, 14, 18, 26],
        layers=[
            dict(name="piano", src="Floaty Lofi Piano Loop (70 BPM) D Maj.wav",
                 bars=8, src_bpm=70, gains=[0.85, 0.85, 0.75, 0.85, 0.85]),
            dict(name="shaker", src="ML Lofi Kitchen Shaker Loop 70 BPM.wav",
                 bars=2, src_bpm=70, gains=[0.0, 0.35, 0.0, 0.35, 0.0]),
            dict(name="hats", src="High Loop 75 BPM.wav",
                 bars=1, src_bpm=75, gains=[0.0, 0.22, 0.0, 0.22, 0.0]),
        ],
        drums=dict(
            samples={"kick": "ML Dusty Kick.wav", "snare": "ML Wood Snare 2.wav",
                     "hat": "ML Lofi Brush 1.wav"},
            events=standard_pattern(),
            gains=[0.0, 0.5, 0.0, 0.5, 0.0],
        ),
    ),
    dict(
        name="day_workshop", mood="Day", bpm=70, total_bars=32,
        sections=[6, 14, 18, 26],
        layers=[
            dict(name="keys", src="70 BPM Wobbly Piano [Bb Major].wav",
                 bars=8, src_bpm=70, gains=[0.85, 0.85, 0.75, 0.85, 0.85]),
            dict(name="guitar", src="Electric Layers Lofi Guitar C Maj.wav",
                 bars=16, src_bpm=68, semitones=-2, gains=[0.0, 0.5, 0.3, 0.5, 0.0]),
            dict(name="texture", src="ML Lofi Stereo Perc Texture Loop 3 - 75 BPM.wav",
                 bars=2, src_bpm=75, gains=[0.0, 0.3, 0.0, 0.3, 0.0]),
        ],
        drums=dict(
            samples={"kick": "ML Dusty Kick 2.wav", "snare": "ML Lofi Hip Hop Snare Muted.wav",
                     "hat": "ML Lofi Brush 1.wav"},
            events=standard_pattern(),
            gains=[0.0, 0.5, 0.0, 0.5, 0.0],
        ),
    ),
    dict(
        name="dusk_lanterns", mood="Dusk", bpm=60, total_bars=30,
        sections=[6, 13, 18, 25],
        layers=[
            dict(name="guitar", src="Dreamy Lofi Guitar (60BPM) C#m.wav",
                 bars=8, src_bpm=60, gains=[0.8, 0.8, 0.7, 0.8, 0.8]),
            dict(name="electric", src="Dreamy Electric 1 (60 BPM) E Maj.wav",
                 bars=8, src_bpm=60, gains=[0.0, 0.45, 0.25, 0.45, 0.0]),
            dict(name="brushes", src="60 BPM Crunchy Brush Texture.wav",
                 bars=4, src_bpm=60, gains=[0.0, 0.3, 0.0, 0.3, 0.0]),
            dict(name="drone", src="Vocal Drone A1.wav", pad=True,
                 gains=[0.4, 0.4, 0.4, 0.4, 0.4]),
        ],
    ),
    dict(
        name="dusk_embers", mood="Dusk", bpm=58, total_bars=28,
        sections=[5, 12, 16, 23],
        layers=[
            dict(name="piano", src="58 BPM Living Room Piano [F Minor] .wav",
                 bars=8, src_bpm=58, gains=[0.85, 0.85, 0.75, 0.85, 0.85]),
            dict(name="vox", src="Choppy Vox [F Min] .wav",
                 bars=2, src_bpm=60, gains=[0.0, 0.35, 0.15, 0.35, 0.0]),
            dict(name="texture", src="60 BPM Dark Textures 1.wav",
                 bars=4, src_bpm=60, gains=[0.0, 0.3, 0.1, 0.3, 0.0]),
        ],
        drums=dict(
            samples={"kick": "ML Lofi Kick 3 .wav", "snare": "MK Wood Snare 7 .wav",
                     "hat": "ML Hat 1.wav"},
            events=standard_pattern(),
            gains=[0.0, 0.5, 0.0, 0.5, 0.0],
        ),
    ),
]

AMBIENCE = [
    dict(name="day_birds", src="BirdSong.wav", night=False),
    dict(name="night_fire", src="Studying By The Fire Ambiance .wav", night=True),
]

SFX_SPEC = [
    dict(name="place_0", srcs=["ML Lofi brick Tap.wav"], lowpass=2000),
    dict(name="place_1", srcs=["ML Wood Snare 2.wav"], lowpass=2000),
    dict(name="place_2", srcs=["ML Wood Snare 3.wav"], lowpass=2000),
    dict(name="pickup_0", srcs=["ML Shell Foley 3.wav"]),
    dict(name="pickup_1", srcs=["ML Shell Foley 4.wav"]),
    dict(name="craft_0", srcs=["ML Glass Perc 1.wav"]),
    dict(name="craft_1", srcs=["ML Glass Perc 2.wav"]),
    dict(name="craft_2", srcs=["ML Glass Perc 3.wav"]),
    dict(name="research_0", srcs=["ML Glass Perc 3.wav"], pad_srcs=["Bubble swell.wav"], max_len=1.6),
    dict(name="research_1", srcs=["ML Glass Perc 2.wav"], pad_srcs=["Breathy Swell.wav"], max_len=1.6),
    dict(name="click_0", srcs=["ML Stick Foley 1 .wav"]),
    dict(name="click_1", srcs=["ML Stick Foley 2 .wav"]),
    dict(name="shot_0", srcs=["ML Lofi Crunch Perc 1.wav"]),
    dict(name="shot_1", srcs=["ML Lofi RIM 1.wav"]),
    dict(name="shot_2", srcs=["ML Lofi Rim 2.wav"]),
]


def scan_packs(packs_dir):
    lookup = {}
    for root, dirs, files in os.walk(packs_dir):
        if "__MACOSX" in root:
            continue
        for fn in files:
            if not fn.lower().endswith(".wav"):
                continue
            full = os.path.join(root, fn)
            if fn in lookup and os.path.abspath(lookup[fn]) != os.path.abspath(full):
                raise SystemExit(f"duplicate basename in packs: {fn}")
            lookup[fn] = full
    return lookup


# ---------------------------------------------------------------------------
# audio_list.rs generation
# ---------------------------------------------------------------------------

def rust_float(v):
    return f"{v:.6f}".rstrip("0").rstrip(".") if v != int(v) else f"{int(v)}.0"


def write_audio_list(music_meta, ambience_meta, sfx_files):
    def track_entries():
        out = []
        for m in music_meta:
            out.append(
                f'    Track {{ name: "{m["name"]}", mood: Mood::{m["mood"]}, '
                f'secs: {rust_float(m["secs"])}f32, '
                f'bytes: include_bytes!("../assets/audio/music/{m["name"]}.ogg") }},'
            )
        return "\n".join(out)

    def sfx_entries():
        # group by base name, preserving SFX_SPEC order
        groups = {}
        order = []
        for name, _ in sfx_files:
            base = name.rsplit("_", 1)[0]
            if base not in groups:
                groups[base] = []
                order.append(base)
            groups[base].append(name)
        lines = []
        for base in order:
            variants = ", ".join(
                f'include_bytes!("../assets/audio/sfx/{v}.ogg")' for v in groups[base]
            )
            lines.append(f'    ("{base}", &[{variants}]),')
        return "\n".join(lines)

    header = (
        "// @generated by `uv run --with numpy python tools/music/build_music.py --packs <dir>`. "
        "Do not edit.\n"
    )
    body = f"""\
{header}
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Mood {{ Day, Dusk }}

pub struct Track {{
    pub name: &'static str,
    pub mood: Mood,
    pub secs: f32,
    pub bytes: &'static [u8],
}}

pub const MUSIC: &[Track] = &[
{track_entries()}
];

pub const AMBIENCE_DAY: &[u8] = include_bytes!("../assets/audio/ambience/{ambience_meta[0]["name"]}.ogg");
pub const AMBIENCE_NIGHT: &[u8] = include_bytes!("../assets/audio/ambience/{ambience_meta[1]["name"]}.ogg");

pub const SFX: &[(&str, &[&[u8]])] = &[
{sfx_entries()}
];
"""
    with open(AUDIO_LIST_RS, "w") as f:
        f.write(body)


# ---------------------------------------------------------------------------
# Check mode
# ---------------------------------------------------------------------------

def collect_audio_files():
    files = []
    for d in (MUSIC_DIR, AMBIENCE_DIR, SFX_DIR):
        if os.path.isdir(d):
            for fn in sorted(os.listdir(d)):
                if fn.lower().endswith(".ogg"):
                    files.append(os.path.join(d, fn))
    return files


def check():
    ok = True
    print("== audio check ==\n")
    print(f"{'file':42s} {'type':9s} {'integrated':>10s} {'true peak':>10s} {'peak':>8s}")
    print("-" * 88)

    total_bytes = 0
    rows = []
    for path in collect_audio_files():
        rel = os.path.relpath(path, REPO)
        dur, sr, ch = probe_meta(path)
        total_bytes += dur * sr * ch * 4
        if "/music/" in path:
            js = measure_loudnorm(path)
            lu = float(js["input_i"])
            tp = float(js["input_tp"])
            pk = "-"
            good = abs(lu - MUSIC_LUFS) <= 1.0 and tp <= TRUE_PEAK
            rows.append((rel, "music", f"{lu:+.2f} LUFS", f"{tp:+.2f} dBTP", pk, good,
                         abs(lu - MUSIC_LUFS) <= 1.0, tp <= TRUE_PEAK))
        elif "/ambience/" in path:
            js = measure_loudnorm(path)
            lu = float(js["input_i"])
            tp = float(js["input_tp"])
            pk = "-"
            good = abs(lu - AMBIENCE_LUFS) <= 2.0
            rows.append((rel, "ambience", f"{lu:+.2f} LUFS", f"{tp:+.2f} dBTP", pk, good,
                         abs(lu - AMBIENCE_LUFS) <= 2.0, True))
        else:
            pk = measure_peak_dbfs(path)
            lu = "-"
            tp = "-"
            good = -4.0 <= pk <= -2.0
            rows.append((rel, "sfx", lu, tp, f"{pk:+.2f} dBFS", good, True, True))

    for rel, typ, lu, tp, pk, good, *_ in rows:
        print(f"{rel:42s} {typ:9s} {lu:>10s} {tp:>10s} {pk:>8s}  {'OK' if good else 'FAIL'}")

    mem_mb = total_bytes / (1024 * 1024)
    print("-" * 88)
    print(f"decoded f32 audio total: {total_bytes} bytes = {mem_mb:.1f} MB "
          f"(budget {MEMORY_BUDGET_BYTES / (1024 * 1024):.0f} MB)  "
          f"{'OK' if total_bytes <= MEMORY_BUDGET_BYTES else 'FAIL'}")

    if total_bytes > MEMORY_BUDGET_BYTES:
        ok = False
    for r in rows:
        if not r[5]:
            ok = False

    if not ok:
        print("\nCHECK FAILED")
        sys.exit(1)
    print("\nCHECK PASSED")
    return rows, total_bytes


# ---------------------------------------------------------------------------
# Build
# ---------------------------------------------------------------------------

def build(packs_dir):
    if not os.path.isdir(packs_dir):
        raise SystemExit(f"--packs directory not found: {packs_dir}")
    lookup = scan_packs(packs_dir)

    # music
    music_meta = []
    for track in TRACKS:
        x, _ = render_track(track, lookup)
        x = loudnorm_normalize(x, MUSIC_LUFS)
        x = true_peak_limiter(x, -2.2)  # headroom for Vorbis lossy overshoot
        out = os.path.join(MUSIC_DIR, f"{track['name']}.ogg")
        encode_ogg(x, out)
        dur, _, _ = probe_meta(out)
        music_meta.append(dict(name=track["name"], mood=track["mood"], secs=dur))
        layers = ", ".join(l["name"] for l in track["layers"])
        print(f"[music] {track['name']:14s} {track['mood']:4s} {dur:6.2f}s  "
              f"layers: {layers}" + (" + drums" if "drums" in track else ""))

    # ambience
    ambience_meta = []
    for a in AMBIENCE:
        out = os.path.join(AMBIENCE_DIR, f"{a['name']}.ogg")
        render_ambience(lookup[a["src"]], out)
        dur, _, _ = probe_meta(out)
        ambience_meta.append(dict(name=a["name"], secs=dur))
        print(f"[ambience] {a['name']:14s} {dur:6.2f}s  src: {a['src']}")

    # sfx
    sfx_files = []
    for entry in SFX_SPEC:
        out = os.path.join(SFX_DIR, f"{entry['name']}.ogg")
        render_sfx(entry, lookup, out)
        sfx_files.append((entry["name"], out))
    print(f"[sfx] {len(sfx_files)} variants")

    write_audio_list(music_meta, ambience_meta, sfx_files)
    print(f"[gen] wrote {os.path.relpath(AUDIO_LIST_RS, REPO)}")

    build_preview(music_meta, ambience_meta, sfx_files)
    print(f"[preview] {PREVIEW_OGG}")

    print()
    check()


def build_preview(music_meta, ambience_meta, sfx_files):
    segs = []
    for m in music_meta:
        segs.append(decode(os.path.join(MUSIC_DIR, f"{m['name']}.ogg"))[:int(20.0 * SR)])
        segs.append(np.zeros((int(0.5 * SR), 2), np.float32))
    for a in ambience_meta:
        segs.append(decode(os.path.join(AMBIENCE_DIR, f"{a['name']}.ogg"))[:int(10.0 * SR)])
        segs.append(np.zeros((int(0.5 * SR), 2), np.float32))
    for name, path in sfx_files:
        segs.append(decode(path))
        segs.append(np.zeros((int(0.5 * SR), 2), np.float32))
    preview = np.concatenate(segs)
    os.makedirs(os.path.dirname(PREVIEW_OGG), exist_ok=True)
    encode_ogg(preview, PREVIEW_OGG)


# ---------------------------------------------------------------------------

def main():
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--packs", help="folder containing the extracted Mondo Loops packs (read-only)")
    ap.add_argument("--check", action="store_true", help="re-measure committed outputs and exit")
    args = ap.parse_args()

    if args.check:
        check()
        return

    if not args.packs:
        raise SystemExit("--packs is required to build (or use --check)")
    build(args.packs)


if __name__ == "__main__":
    main()

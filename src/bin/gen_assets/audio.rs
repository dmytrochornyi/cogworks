//! Synthesizes the soundtrack and sound effects.

use std::f32::consts::TAU;
use std::fs::File;
use std::io::Write;

pub const SR: f32 = 24_000.0;

pub fn write_wav(path: &str, samples: &[f32]) {
    let data: Vec<u8> = samples
        .iter()
        .flat_map(|s| ((s.clamp(-1.0, 1.0) * 32767.0) as i16).to_le_bytes())
        .collect();
    let mut f = File::create(path).expect("create wav");
    let sr = SR as u32;
    f.write_all(b"RIFF").unwrap();
    f.write_all(&(36 + data.len() as u32).to_le_bytes()).unwrap();
    f.write_all(b"WAVEfmt ").unwrap();
    f.write_all(&16u32.to_le_bytes()).unwrap();
    f.write_all(&1u16.to_le_bytes()).unwrap(); // PCM
    f.write_all(&1u16.to_le_bytes()).unwrap(); // mono
    f.write_all(&sr.to_le_bytes()).unwrap();
    f.write_all(&(sr * 2).to_le_bytes()).unwrap();
    f.write_all(&2u16.to_le_bytes()).unwrap();
    f.write_all(&16u16.to_le_bytes()).unwrap();
    f.write_all(b"data").unwrap();
    f.write_all(&(data.len() as u32).to_le_bytes()).unwrap();
    f.write_all(&data).unwrap();
}

struct Rng(u64);

impl Rng {
    fn next(&mut self) -> f32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        (self.0 >> 40) as f32 / (1u64 << 24) as f32
    }

    fn noise(&mut self) -> f32 {
        self.next() * 2.0 - 1.0
    }
}

fn midi(n: f32) -> f32 {
    440.0 * 2f32.powf((n - 69.0) / 12.0)
}

/// Adds `voice(t)` for `dur` seconds starting at `start` into `buf`.
fn render(buf: &mut [f32], start: f32, dur: f32, gain: f32, voice: impl Fn(f32) -> f32) {
    let s0 = (start * SR) as usize;
    let n = (dur * SR) as usize;
    for i in 0..n {
        if let Some(b) = buf.get_mut(s0 + i) {
            *b += voice(i as f32 / SR) * gain;
        }
    }
}

fn release(t: f32, len: f32) -> f32 {
    if t > len { (1.0 - (t - len) / 0.25).max(0.0) } else { 1.0 }
}

/// Warm electric piano: sine body, a decaying octave and a little bell "tine".
fn epiano(f: f32, len: f32) -> impl Fn(f32) -> f32 {
    move |t| {
        let env = (1.0 - (-t / 0.004).exp()) * (-t * 1.2).exp() * release(t, len);
        let tine = (TAU * f * 4.0 * t).sin() * 0.12 * (-t * 9.0).exp();
        let body = (TAU * f * t).sin() + (TAU * f * 2.0 * t).sin() * 0.22 * (-t * 3.0).exp();
        (body + tine) * env
    }
}

fn bass(f: f32, len: f32) -> impl Fn(f32) -> f32 {
    move |t| {
        let env = (1.0 - (-t / 0.008).exp()) * (-t * 1.4).exp() * release(t, len);
        let s = (TAU * f * t).sin() + 0.25 * (TAU * 2.0 * f * t).sin();
        (s * 1.4).tanh() * env
    }
}

/// Slow-swelling pad: a few detuned sines per note, so chords bloom and breathe.
fn pad(f: f32, len: f32) -> impl Fn(f32) -> f32 {
    move |t| {
        let attack = (t / 0.9).min(1.0);
        let rel = if t > len { (1.0 - (t - len) / 1.2).max(0.0) } else { 1.0 };
        let s = (TAU * f * t).sin()
            + (TAU * f * 1.003 * t).sin() * 0.7
            + (TAU * f * 0.997 * t).sin() * 0.7
            + (TAU * f * 2.0 * t).sin() * 0.12;
        s * attack * attack * rel
    }
}

/// Soft breathy lead: sine with delayed vibrato and a slow attack.
fn flute(f: f32, len: f32) -> impl Fn(f32) -> f32 {
    move |t| {
        let vib = 1.0 + 0.006 * (TAU * 5.2 * t).sin() * ((t - 0.15) * 2.5).clamp(0.0, 1.0);
        let ph = TAU * f * vib * t;
        let env = (t / 0.06).min(1.0) * (-t * 0.7).exp() * release(t, len);
        (ph.sin() + 0.08 * (2.0 * ph).sin()) * env
    }
}

pub fn music() -> Vec<f32> {
    const BPM: f32 = 80.0;
    let beat = 60.0 / BPM;
    let bars = 32;
    let len = bars as f32 * 4.0 * beat;
    let tail = 5.0;
    let total = ((len + tail) * SR) as usize;
    let mut keys = vec![0.0; total];
    let mut pads = vec![0.0; total];
    let mut low = vec![0.0; total];
    let mut mel = vec![0.0; total];
    let mut drums = vec![0.0; total];
    let mut kicks_at: Vec<f32> = Vec::new();
    let mut rng = Rng(0x5eed_1234_abcd);

    // A lazy descending progression in C: Fmaj9 Em9 Dm9 Cmaj9 / Fmaj9 Em9 Dm9 G13.
    let chords: [(f32, [f32; 4]); 8] = [
        (41.0, [57.0, 60.0, 64.0, 67.0]),
        (40.0, [55.0, 59.0, 62.0, 66.0]),
        (38.0, [53.0, 57.0, 60.0, 64.0]),
        (36.0, [52.0, 55.0, 59.0, 62.0]),
        (41.0, [57.0, 60.0, 64.0, 67.0]),
        (40.0, [55.0, 59.0, 62.0, 66.0]),
        (38.0, [53.0, 57.0, 60.0, 64.0]),
        (43.0, [53.0, 59.0, 64.0, 69.0]),
    ];
    // A written hook: (bar within the 8-bar cycle, beat, midi note, length in beats).
    let hook: &[(usize, f32, f32, f32)] = &[
        (0, 0.5, 76.0, 0.5), (0, 1.0, 79.0, 0.5), (0, 1.5, 76.0, 1.0), (0, 3.0, 74.0, 0.5), (0, 3.5, 72.0, 1.5),
        (1, 2.0, 74.0, 0.5), (1, 2.5, 76.0, 2.0),
        (2, 0.0, 81.0, 1.0), (2, 1.5, 79.0, 0.5), (2, 2.0, 76.0, 1.0), (2, 3.5, 74.0, 0.5),
        (3, 0.0, 72.0, 1.0), (3, 1.0, 74.0, 0.5), (3, 1.5, 67.0, 2.5),
        (4, 0.5, 76.0, 0.5), (4, 1.0, 79.0, 0.5), (4, 1.5, 76.0, 1.0), (4, 3.0, 79.0, 0.5), (4, 3.5, 81.0, 1.5),
        (5, 2.0, 79.0, 0.5), (5, 2.5, 78.0, 2.0),
        (6, 0.0, 77.0, 1.0), (6, 1.0, 76.0, 0.5), (6, 1.5, 74.0, 0.5), (6, 2.0, 72.0, 1.5),
        (7, 0.0, 74.0, 1.0), (7, 1.5, 71.0, 0.5), (7, 2.0, 74.0, 2.0),
    ];
    let swing = |b: f32| if (b * 2.0).fract().abs() > 0.4 { b + 0.08 } else { b };

    for bar in 0..bars {
        let t0 = bar as f32 * 4.0 * beat;
        let cyc = bar % 8;
        let (root, voicing) = chords[cyc];
        // Intro (0-4) pad + keys, groove (4-12), hook (12-20), breakdown (20-24), full (24-32).
        let intro = bar < 4;
        let breakdown = (20..24).contains(&bar);
        let hook_on = (12..20).contains(&bar) || bar >= 24;

        // Pad holds the whole bar and overlaps into the next.
        for &n in &voicing {
            render(&mut pads, t0, 4.0 * beat + 1.4, 0.035, pad(midi(n), 4.0 * beat));
        }

        // Rhodes comping: lazy, slightly rolled chords.
        let hits: &[(f32, f32, f32)] = if intro || breakdown {
            &[(0.0, 3.6, 0.8)]
        } else {
            &[(0.0, 1.4, 0.9), (1.5, 0.9, 0.55), (2.75, 1.1, 0.7)]
        };
        for &(hit, dur, vel) in hits {
            let jitter = rng.next() * 0.012;
            for (k, &n) in voicing.iter().enumerate() {
                let start = t0 + swing(hit) * beat + k as f32 * 0.018 + jitter;
                render(&mut keys, start, dur * beat + 1.4, 0.085 * vel, epiano(midi(n), dur * beat));
            }
        }

        // Round sub bass.
        if !intro {
            if breakdown {
                render(&mut low, t0, 3.8 * beat + 0.3, 0.3, bass(midi(root), 3.6 * beat));
            } else {
                render(&mut low, t0, 1.8 * beat + 0.3, 0.34, bass(midi(root), 1.7 * beat));
                let walk = if cyc % 2 == 0 { root + 7.0 } else { root + 12.0 };
                render(&mut low, t0 + swing(2.5) * beat, beat + 0.3, 0.24, bass(midi(walk), 0.9 * beat));
                render(&mut low, t0 + swing(3.5) * beat, 0.5 * beat + 0.3, 0.18, bass(midi(root + 10.0), 0.4 * beat));
            }
        }

        // The hook, on a breathy lead; the breakdown hears it an octave down on keys.
        if hook_on || breakdown {
            for &(_, at, n, l) in hook.iter().filter(|h| h.0 == cyc) {
                let start = t0 + swing(at) * beat + rng.next() * 0.01;
                if breakdown {
                    render(&mut mel, start, l * beat + 1.5, 0.11, epiano(midi(n - 12.0), l * beat));
                } else {
                    render(&mut mel, start, l * beat + 1.0, 0.1, flute(midi(n), l * beat));
                }
            }
        }

        // Soft boom-bap drums, absent in the intro and breakdown (hats only there).
        let hats_only = intro || breakdown;
        if !hats_only {
            let kicks: &[f32] = if cyc % 4 == 3 { &[0.0, 1.75, 2.5] } else { &[0.0, 2.5] };
            for &k in kicks {
                let at = t0 + swing(k) * beat;
                kicks_at.push(at);
                render(&mut drums, at, 0.6, 0.7, |t| {
                    let ph = TAU * (48.0 * t + 60.0 * (1.0 - (-t * 25.0).exp()) / 25.0);
                    ph.sin() * (-t * 6.0).exp()
                });
            }
            // A soft, low snare/rim on 2 and 4.
            for s in [1.0, 3.0] {
                let seed = rng.0;
                render(&mut drums, t0 + s * beat + 0.015, 0.35, 0.22, move |t| {
                    let mut r = Rng(seed ^ ((t * SR) as u64 * 2_654_435_761));
                    r.noise() * 0.6 * (-t * 20.0).exp() + (TAU * 170.0 * t).sin() * 0.4 * (-t * 25.0).exp()
                });
            }
        }
        let hat_gain = if hats_only { 0.035 } else { 0.06 };
        let mut e = 0.0;
        while e < 4.0 {
            let accent = if e % 1.0 == 0.0 { 1.0 } else { 0.6 };
            let vel = hat_gain * accent * (0.85 + rng.next() * 0.3);
            let seed = rng.0;
            render(&mut drums, t0 + swing(e) * beat, 0.06, vel, move |t| {
                let mut r = Rng(seed ^ ((t * SR) as u64).wrapping_mul(0x9E37_79B9));
                r.noise() * (-t * 70.0).exp()
            });
            e += 0.5;
        }
    }

    // Dreamy dotted-eighth echo on the melody.
    let delay = (0.75 * beat * SR) as usize;
    for i in delay..total {
        mel[i] += mel[i - delay] * 0.42;
    }
    // A short slapback blur on the keys for space.
    let slap = (0.11 * SR) as usize;
    for i in slap..total {
        keys[i] += keys[i - slap] * 0.25;
    }

    // Sidechain "pump": pad and keys duck under each kick and swell back.
    let mut duck = vec![1.0f32; total];
    for &k in &kicks_at {
        let s0 = (k * SR) as usize;
        for j in 0..(0.45 * SR) as usize {
            if let Some(d) = duck.get_mut(s0 + j) {
                let t = j as f32 / SR;
                *d = d.min(1.0 - 0.4 * (-t / 0.14).exp());
            }
        }
    }

    // Mix: Rhodes tremolo, tape wobble, a gentle lowpass and a whisper of vinyl.
    let mut out = vec![0.0; total];
    let mut lp = 0.0;
    let mut hiss_lp = 0.0;
    let mut crackle = 0.0f32;
    for i in 0..total {
        let t = i as f32 / SR;
        let wobble = 1.0 + 0.0025 * (TAU * 0.27 * t).sin() + 0.001 * (TAU * 0.71 * t).sin();
        let ki = ((i as f32) * wobble) as usize;
        let k = keys.get(ki).copied().unwrap_or(0.0) * (1.0 + 0.18 * (TAU * 4.2 * t).sin());
        let p = pads.get(ki).copied().unwrap_or(0.0);
        let mut s = (k + p) * duck[i] + mel[i] + low[i] + drums[i];
        lp += (s - lp) * 0.6;
        s = lp;
        hiss_lp += (rng.noise() - hiss_lp) * 0.15;
        if rng.next() < 3.0 / SR {
            crackle = (rng.next() * 0.025 + 0.005) * if rng.next() < 0.5 { 1.0 } else { -1.0 };
        }
        s += hiss_lp * 0.004 + crackle;
        crackle *= 0.5;
        out[i] = (s * 1.1).tanh();
    }

    // Fold the tail onto the start so the loop is seamless.
    let n = (len * SR) as usize;
    let (head, rest) = out.split_at_mut(n);
    for (h, t) in head.iter_mut().zip(rest.iter()) {
        *h += *t;
    }
    let mut song = out[..n].to_vec();
    let peak = song.iter().fold(0.0f32, |m, s| m.max(s.abs()));
    for s in &mut song {
        *s = *s / peak * 0.8;
    }
    song
}

pub fn sfx(name: &str) -> Vec<f32> {
    let dur = match name {
        "research" => 1.6,
        "craft" => 0.9,
        _ => 0.25,
    };
    let mut buf = vec![0.0; (dur * SR) as usize];
    let mut rng = Rng(0xabc ^ name.len() as u64);
    match name {
        "place" => {
            render(&mut buf, 0.0, 0.2, 0.7, |t| {
                let f = 110.0 + 90.0 * (-t * 40.0).exp();
                (TAU * f * t).sin() * (-t * 28.0).exp()
            });
            let noise: Vec<f32> = (0..200).map(|_| rng.noise()).collect();
            render(&mut buf, 0.0, 0.008, 0.25, move |t| noise[(t * SR) as usize % 200] * (-t * 400.0).exp());
        }
        "pickup" => render(&mut buf, 0.0, 0.2, 0.45, |t| {
            let ph = TAU * (420.0 * t + 1800.0 * t * t);
            ph.sin() * (-t * 22.0).exp()
        }),
        "craft" => {
            for (k, n) in [88.0, 95.0].into_iter().enumerate() {
                let f = midi(n);
                render(&mut buf, k as f32 * 0.08, 0.8, 0.3, move |t| {
                    ((TAU * f * t).sin() + 0.3 * (TAU * f * 2.76 * t).sin() * (-t * 6.0).exp()) * (-t * 5.0).exp()
                });
            }
        }
        "research" => {
            for (k, n) in [72.0, 76.0, 79.0, 84.0, 88.0].into_iter().enumerate() {
                let f = midi(n);
                render(&mut buf, k as f32 * 0.1, 1.4, 0.24, move |t| {
                    ((TAU * f * t).sin() + 0.25 * (TAU * f * 3.01 * t).sin() * (-t * 5.0).exp()) * (-t * 2.8).exp()
                });
            }
        }
        "shot" => {
            let noise: Vec<f32> = (0..4000).map(|_| rng.noise()).collect();
            render(&mut buf, 0.0, 0.2, 0.5, move |t| {
                let n = noise[(t * SR) as usize % 4000];
                n * (-t * 35.0).exp() * 0.5 + (TAU * 85.0 * t).sin() * (-t * 25.0).exp()
            });
            let mut lp = 0.0;
            for s in &mut buf {
                lp += (*s - lp) * 0.25;
                *s = lp;
            }
        }
        _ => render(&mut buf, 0.0, 0.05, 0.4, |t| (TAU * 1900.0 * t).sin() * (-t * 120.0).exp()),
    }
    // Level each effect to a fixed peak so one-shots sit consistently under the music.
    let target = match name {
        "place" => 0.55,
        "pickup" => 0.42,
        "craft" => 0.5,
        "research" => 0.45,
        "shot" => 0.35,
        _ => 0.3,
    };
    let peak = buf.iter().fold(0.0f32, |m, s| m.max(s.abs()));
    if peak > 0.0 {
        for s in &mut buf {
            *s = *s / peak * target;
        }
    }
    buf
}

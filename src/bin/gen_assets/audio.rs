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

fn lead(f: f32, len: f32) -> impl Fn(f32) -> f32 {
    move |t| {
        let vib = 1.0 + 0.004 * (TAU * 5.0 * t).sin() * (t * 2.0).min(1.0);
        let ph = TAU * f * vib * t;
        let env = (1.0 - (-t / 0.02).exp()) * (-t * 1.6).exp() * release(t, len);
        (ph.sin() + 0.15 * (2.0 * ph).sin() + 0.05 * (3.0 * ph).sin()) * env
    }
}

pub fn music() -> Vec<f32> {
    const BPM: f32 = 74.0;
    let beat = 60.0 / BPM;
    let bars = 32;
    let len = bars as f32 * 4.0 * beat;
    let tail = 4.0;
    let total = ((len + tail) * SR) as usize;
    let mut keys = vec![0.0; total];
    let mut low = vec![0.0; total];
    let mut mel = vec![0.0; total];
    let mut counter = vec![0.0; total];
    let mut drums = vec![0.0; total];
    let mut rng = Rng(0x5eed_1234_abcd);

    // (bass root, chord voicing) per bar.
    let chords: [(f32, [f32; 4]); 8] = [
        (41.0, [57.0, 60.0, 64.0, 67.0]),
        (40.0, [55.0, 59.0, 62.0, 64.0]),
        (38.0, [53.0, 57.0, 60.0, 64.0]),
        (43.0, [53.0, 57.0, 60.0, 65.0]),
        (36.0, [52.0, 55.0, 59.0, 62.0]),
        (45.0, [55.0, 59.0, 60.0, 64.0]),
        (38.0, [53.0, 57.0, 60.0, 62.0]),
        (40.0, [56.0, 59.0, 62.0, 65.0]),
    ];
    let pentatonic = [72.0, 74.0, 76.0, 79.0, 81.0, 84.0];
    let high = [84.0, 86.0, 88.0, 91.0, 93.0];
    let swing = |b: f32| if (b * 2.0).fract().abs() > 0.4 { b + 0.12 } else { b };
    let hat = |drums: &mut [f32], at: f32, vel: f32, seed: u64| {
        render(drums, at, 0.08, vel, move |t| {
            let mut r = Rng(seed ^ ((t * SR) as u64).wrapping_mul(0x9E37_79B9));
            r.noise() * (-t * 55.0).exp()
        });
    };
    let snare = |drums: &mut [f32], at: f32, vel: f32, seed: u64, tone: f32| {
        render(drums, at, 0.4, vel, move |t| {
            let mut r = Rng(seed ^ ((t * SR) as u64 * 2_654_435_761));
            let n = r.noise();
            (n * 0.7 * (-t * 16.0).exp()) + (TAU * tone * t).sin() * 0.35 * (-t * 22.0).exp()
        });
    };

    for bar in 0..bars {
        let t0 = bar as f32 * 4.0 * beat;
        let (root, voicing) = chords[bar % 8];
        // A (0-8) sparse, B (8-16) full, C (16-24) breakdown then build, D (24-32) full.
        let breakdown = (16..20).contains(&bar);
        let build = (20..24).contains(&bar);
        let full = (8..16).contains(&bar) || bar >= 24;
        let last = bar == bars - 1;

        // Keys: the main voice, still present through the breakdown.
        let hits: &[(f32, f32, f32)] = if breakdown {
            &[(0.0, 3.4, 0.95)]
        } else if build {
            &[(0.0, 1.8, 0.8), (3.0, 1.0, 0.5)]
        } else if full {
            &[(0.0, 1.8, 1.0), (2.5, 1.3, 0.65), (3.5, 1.0, 0.35)]
        } else {
            &[(0.0, 1.8, 1.0), (2.5, 1.3, 0.6)]
        };
        for &(hit, dur, vel) in hits {
            for (k, &n) in voicing.iter().enumerate() {
                let start = t0 + swing(hit) * beat + k as f32 * 0.014;
                render(&mut keys, start, dur * beat + 1.2, 0.11 * vel, epiano(midi(n), dur * beat));
            }
        }

        // Bass. The breakdown holds a long root so the drop-out leaves air.
        if breakdown {
            render(&mut low, t0, 3.6 * beat + 0.3, 0.32, bass(midi(root), 3.4 * beat));
        } else {
            render(&mut low, t0, 1.6 * beat + 0.3, 0.34, bass(midi(root), 1.5 * beat));
            if !build {
                let second = if bar % 2 == 0 { root + 7.0 } else { root + 12.0 };
                render(&mut low, t0 + swing(2.5) * beat, beat + 0.3, 0.26, bass(midi(second), 0.9 * beat));
            }
        }

        // Melody: absent in the breakdown, denser in the full sections.
        if !breakdown && bar >= 2 {
            let notes = if full { 3 } else { 2 };
            for _ in 0..notes {
                let pos = (rng.next() * 8.0).floor() / 2.0;
                if rng.next() < 0.8 {
                    let n = pentatonic[(rng.next() * pentatonic.len() as f32) as usize];
                    render(&mut mel, t0 + swing(pos) * beat, 1.6, 0.13, lead(midi(n), 0.6));
                }
            }
        }
        // Countermelody only in the last section, an octave up and very quiet.
        if bar >= 24 && bar % 4 == 2 {
            for k in 0..2 {
                let n = high[(rng.next() * high.len() as f32) as usize];
                let at = t0 + (1.5 + k as f32 * 1.0) * beat;
                render(&mut counter, at, 1.4, 0.055, lead(midi(n), 0.5));
            }
        }

        // Drums drop out for the breakdown and return half-time in the build.
        if !breakdown {
            let kicks: &[f32] = if build {
                &[0.0]
            } else if bar % 4 == 3 {
                &[0.0, 1.75, 2.5]
            } else {
                &[0.0, 2.5]
            };
            for &k in kicks {
                render(&mut drums, t0 + swing(k) * beat, 0.5, 0.75, |t| {
                    let ph = TAU * (45.0 * t + 70.0 * (1.0 - (-t * 30.0).exp()) / 30.0);
                    ph.sin() * (-t * 8.0).exp()
                });
            }
            let snare_beats: &[f32] = if build { &[3.0] } else { &[1.0, 3.0] };
            for &s in snare_beats {
                snare(&mut drums, t0 + s * beat + 0.01, 0.32, rng.0, 185.0);
            }
            let step = if build { 1.0 } else { 0.5 };
            let mut e = 0.0;
            while e < 4.0 {
                let vel = if e % 1.0 == 0.0 { 0.09 } else { 0.05 };
                hat(&mut drums, t0 + swing(e) * beat, vel, rng.0 + e as u64);
                e += step;
            }
            // A fill at the end of every eight bars, plus a soft ride on the first beat.
            if bar % 8 == 7 {
                for (k, at) in [2.5, 2.75, 3.0, 3.25].into_iter().enumerate() {
                    snare(&mut drums, t0 + at * beat, 0.22 + k as f32 * 0.04, rng.0 + k as u64, 150.0 + k as f32 * 18.0);
                }
            }
            if bar % 8 == 0 {
                let seed = rng.0;
                render(&mut drums, t0, 1.6, 0.12, move |t| {
                    let mut r = Rng(seed ^ ((t * SR) as u64 * 0x9E37_79B9));
                    r.noise() * (-t * 2.2).exp()
                });
            }
        } else if last {
            // The loop still needs a downbeat under the held key.
            render(&mut low, t0, beat, 0.25, bass(midi(root), beat));
        }
    }

    // Melody echoes, and a quieter echo on the counter line.
    let delay = (0.75 * beat * SR) as usize;
    for i in delay..total {
        mel[i] += mel[i - delay] * 0.38;
    }
    let far = (1.5 * beat * SR) as usize;
    for i in far..total {
        counter[i] += counter[i - far] * 0.3;
    }

    // Mix, then warm it up: lowpass, tape wobble and vinyl.
    let mut out = vec![0.0; total];
    let mut lp = 0.0;
    let mut hiss_lp = 0.0;
    let mut crackle = 0.0f32;
    for i in 0..total {
        let t = i as f32 / SR;
        // Two slow detune LFOs make the keys and melody drift like worn tape.
        let wobble = 1.0 + 0.0035 * (TAU * 0.31 * t).sin() + 0.0015 * (TAU * 0.87 * t).sin();
        let ki = ((i as f32) * wobble) as usize;
        let mi = ((i as f32) * (1.0 + (wobble - 1.0) * 0.6)) as usize;
        let k = keys.get(ki).copied().unwrap_or(0.0);
        let m = mel.get(mi).copied().unwrap_or(0.0);
        let mut s = k + m + low[i] + counter[i] + drums[i];
        lp += (s - lp) * 0.45;
        s = lp;
        // Continuous vinyl hiss with occasional louder crackles and pops.
        hiss_lp += (rng.noise() - hiss_lp) * 0.2;
        if rng.next() < 12.0 / SR {
            crackle = (rng.next() * 0.18 + 0.02) * if rng.next() < 0.5 { 1.0 } else { -1.0 };
        }
        s += hiss_lp * 0.008 + crackle;
        crackle *= 0.55;
        out[i] = (s * 1.2).tanh();
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

//! Day/dusk music director, ambience beds and foley effects.
//!
//! Owns every sound in the game. The [`Director`] picks the playlist and its
//! crossfades; [`Audio`] applies those gains to the actual macroquad sounds and
//! drives the day/night ambience beds. All the interesting decisions live in
//! pure, testable pieces ([`equal_power`], [`next_mood`], [`Director`] and
//! [`VariantPicker`]) so they can be exercised without a window or a device.

use std::collections::HashMap;

use macroquad::audio::{
    PlaySoundParams, Sound, load_sound_from_bytes, play_sound, set_sound_volume, stop_sound,
};

use crate::audio_list::{AMBIENCE_DAY, AMBIENCE_NIGHT, MUSIC, SFX, Mood};

/// How long a music crossfade lasts.
const CROSSFADE: f32 = 6.0;

/// Master levels for the three buses. Applied on top of the per-sound gains.
#[derive(Clone, Copy, Debug)]
pub struct Levels {
    pub music: f32,
    pub ambience: f32,
    pub sfx: f32,
}

impl Default for Levels {
    fn default() -> Self {
        Levels { music: 0.55, ambience: 0.35, sfx: 0.8 }
    }
}

/// Equal-power crossfade gains for progress `t` in `0..=1`.
/// Returns `(out_gain, in_gain)` with `out² + in² = 1`, so the loudness stays
/// constant as one track hands over to the next.
pub fn equal_power(t: f32) -> (f32, f32) {
    let t = t.clamp(0.0, 1.0);
    ((1.0 - t).sqrt(), t.sqrt())
}

/// The day/dusk mood for `night` (0 = noon, 1 = midnight), with hysteresis:
/// it flips to dusk only at `night >= 0.45` and back to day only at
/// `night <= 0.30`, so the boundary doesn't flicker back and forth.
pub fn next_mood(night: f32, current: Mood) -> Mood {
    if night >= 0.45 {
        Mood::Dusk
    } else if night <= 0.30 {
        Mood::Day
    } else {
        current
    }
}

/// The pure music gain mapping: muted music is silent; otherwise the director
/// gain is scaled by the music level.
pub fn music_gain(director_gain: f32, levels: &Levels, muted: bool) -> f32 {
    if muted {
        0.0
    } else {
        director_gain * levels.music
    }
}

fn mood_index(m: Mood) -> usize {
    match m {
        Mood::Day => 0,
        Mood::Dusk => 1,
    }
}

/// Picks which track plays and at what gain, pure and deterministic apart from
/// the incoming `night`.
pub struct Director {
    mood: Mood,
    /// Index of the primary (most recently started) track.
    current: usize,
    /// Index of the track fading out during a crossfade.
    previous: Option<usize>,
    /// Seconds elapsed since the current crossfade began.
    fade: f32,
    /// Seconds since the current track began playing.
    elapsed: f32,
    /// Round-robin counters per mood (0 = Day, 1 = Dusk).
    next: [usize; 2],
}

impl Director {
    pub fn new(start: Mood) -> Self {
        Director {
            mood: start,
            current: usize::MAX,
            previous: None,
            fade: 0.0,
            elapsed: 0.0,
            next: [0, 0],
        }
    }

    /// The next track of `mood` in rotation, as an index into `tracks`.
    fn pick(&mut self, mood: Mood, tracks: &[(Mood, f32)]) -> usize {
        let sub: Vec<usize> = tracks
            .iter()
            .enumerate()
            .filter(|(_, (m, _))| *m == mood)
            .map(|(i, _)| i)
            .collect();
        if sub.is_empty() {
            // Real playlists always have tracks of each mood; this only guards
            // against a degenerate, all-one-mood list.
            return 0;
        }
        let k = self.next[mood_index(mood)] % sub.len();
        self.next[mood_index(mood)] += 1;
        sub[k]
    }

    /// Advances the director by `dt` and returns the `(track index, gain)`
    /// pairs currently audible, with gains in `0..=1`.
    pub fn step(&mut self, dt: f32, night: f32, tracks: &[(Mood, f32)]) -> Vec<(usize, f32)> {
        if tracks.is_empty() {
            self.current = usize::MAX;
            self.previous = None;
            return Vec::new();
        }

        let target = next_mood(night, self.mood);
        if self.current == usize::MAX {
            // First step: pick the initial track for the night-derived mood.
            self.mood = target;
            self.current = self.pick(target, tracks);
            self.elapsed = 0.0;
            self.previous = None;
            self.fade = 0.0;
        } else if target != self.mood {
            // Mood flipped: crossfade immediately into a track of the new mood.
            let old = self.current;
            let new = self.pick(target, tracks);
            self.mood = target;
            self.previous = Some(old);
            self.current = new;
            self.fade = 0.0;
            self.elapsed = 0.0;
        }

        self.elapsed += dt;
        if self.previous.is_some() {
            self.fade += dt;
        }

        // Finish the crossfade once its 6 s are up.
        if self.previous.is_some() && self.fade >= CROSSFADE {
            self.previous = None;
            self.fade = 0.0;
        }

        // Start the next track 6 s before the current one ends, so the two
        // overlap for one seamless crossfade rather than going silent.
        if self.previous.is_none() {
            let secs = tracks[self.current].1;
            if self.elapsed >= secs - CROSSFADE {
                let old = self.current;
                let new = self.pick(self.mood, tracks);
                self.previous = Some(old);
                self.current = new;
                self.fade = 0.0;
                self.elapsed = 0.0;
            }
        }

        let mut out = Vec::new();
        match self.previous {
            Some(prev) => {
                let t = (self.fade / CROSSFADE).clamp(0.0, 1.0);
                let (out_gain, in_gain) = equal_power(t);
                if out_gain > 0.0 {
                    out.push((prev, out_gain));
                }
                if in_gain > 0.0 {
                    out.push((self.current, in_gain));
                }
            }
            None => out.push((self.current, 1.0)),
        }
        out
    }
}

/// Picks a variant index per effect name, never repeating the same variant
/// twice in a row.
pub struct VariantPicker {
    last: HashMap<String, usize>,
    state: u64,
}

impl VariantPicker {
    pub fn new() -> Self {
        VariantPicker { last: HashMap::new(), state: 0x9E37_79B9_7F4A_7C15 }
    }

    /// The next variant index in `0..count`. For `count == 1` this is always 0;
    /// otherwise it never returns the same index two calls in a row.
    pub fn pick(&mut self, name: &str, count: usize) -> usize {
        if count <= 1 {
            return 0;
        }
        // xorshift64.
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.state = x;
        let r = (x % count as u64) as usize;
        let idx = if self.last.get(name) == Some(&r) { (r + 1) % count } else { r };
        self.last.insert(name.to_owned(), idx);
        idx
    }
}

/// All in-game audio: the loaded sounds plus the state that drives them.
pub struct Audio {
    music: Vec<Sound>,
    /// `(mood, secs)` per track, mirroring `music` for the director.
    track_meta: Vec<(Mood, f32)>,
    /// Which music track currently has a live voice.
    playing: Vec<bool>,
    ambience_day: Sound,
    ambience_night: Sound,
    /// Effect variants by name.
    sfx: HashMap<&'static str, Vec<Sound>>,
    director: Director,
    levels: Levels,
    muted: bool,
    picker: VariantPicker,
    /// Smoothed day/night ambience gains (0..1, before the ambience level).
    amb_day: f32,
    amb_night: f32,
}

impl Audio {
    pub async fn load() -> Audio {
        let mut music = Vec::with_capacity(MUSIC.len());
        let mut track_meta = Vec::with_capacity(MUSIC.len());
        for track in MUSIC {
            let sound = load_sound_from_bytes(track.bytes)
                .await
                .unwrap_or_else(|e| panic!("failed to decode music {}: {e}", track.name));
            music.push(sound);
            track_meta.push((track.mood, track.secs));
        }

        let ambience_day = load_sound_from_bytes(AMBIENCE_DAY)
            .await
            .unwrap_or_else(|e| panic!("failed to decode day ambience: {e}"));
        let ambience_night = load_sound_from_bytes(AMBIENCE_NIGHT)
            .await
            .unwrap_or_else(|e| panic!("failed to decode night ambience: {e}"));

        let mut sfx = HashMap::new();
        for (name, variants) in SFX {
            let mut sounds = Vec::with_capacity(variants.len());
            for bytes in *variants {
                let sound = load_sound_from_bytes(bytes)
                    .await
                    .unwrap_or_else(|e| panic!("failed to decode sfx {name}: {e}"));
                sounds.push(sound);
            }
            sfx.insert(*name, sounds);
        }

        let audio = Audio {
            playing: vec![false; music.len()],
            music,
            track_meta,
            ambience_day,
            ambience_night,
            sfx,
            director: Director::new(Mood::Day),
            levels: Levels::default(),
            muted: false,
            picker: VariantPicker::new(),
            amb_day: 1.0,
            amb_night: 0.0,
        };

        // The ambience beds loop forever; their gains follow the day/night mix.
        play_sound(&audio.ambience_day, PlaySoundParams { looped: true, volume: 0.0 });
        play_sound(&audio.ambience_night, PlaySoundParams { looped: true, volume: 0.0 });

        audio
    }

    /// Drives the director, applies crossfade gains and mixes the ambience.
    pub fn update(&mut self, dt: f32, night: f32) {
        let night = night.clamp(0.0, 1.0);
        let audible = self.director.step(dt, night, &self.track_meta);

        // Match live voices to the director's audible set.
        let mut wanted = vec![false; self.music.len()];
        for &(idx, gain) in &audible {
            wanted[idx] = true;
            if !self.playing[idx] {
                play_sound(&self.music[idx], PlaySoundParams { looped: false, volume: 0.0 });
                self.playing[idx] = true;
            }
            set_sound_volume(&self.music[idx], music_gain(gain, &self.levels, self.muted));
        }
        for (idx, is_playing) in self.playing.iter_mut().enumerate() {
            if *is_playing && !wanted[idx] {
                stop_sound(&self.music[idx]);
                *is_playing = false;
            }
        }

        // Day bed = 1 - night, night bed = night, both smoothed.
        let k = 1.0 - (-dt * 2.0).exp();
        self.amb_day += ((1.0 - night) - self.amb_day) * k;
        self.amb_night += (night - self.amb_night) * k;
        set_sound_volume(&self.ambience_day, self.amb_day * self.levels.ambience);
        set_sound_volume(&self.ambience_night, self.amb_night * self.levels.ambience);
    }

    /// Plays one variant of the named effect at `volume` (scaled by the sfx level).
    pub fn sfx(&mut self, name: &str, volume: f32) {
        let Some(variants) = self.sfx.get(name) else {
            return;
        };
        if variants.is_empty() {
            return;
        }
        let i = self.picker.pick(name, variants.len());
        play_sound(&variants[i], PlaySoundParams {
            looped: false,
            volume: volume * self.levels.sfx,
        });
    }

    /// Toggles music and returns the new on-state, for the radio widget.
    /// Sets the bus levels (from the settings); takes effect on the next update.
    pub fn set_levels(&mut self, levels: Levels) {
        self.levels = levels;
    }

    pub fn toggle_music(&mut self) -> bool {
        self.muted = !self.muted;
        !self.muted
    }

    pub fn music_on(&self) -> bool {
        !self.muted
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audio_list::Mood;
    use std::f32::consts::TAU;

    #[test]
    fn equal_power_is_constant_power() {
        assert_eq!(equal_power(0.0), (1.0, 0.0));
        assert_eq!(equal_power(1.0), (0.0, 1.0));
        let mut t = 0.0f32;
        while t <= 1.0 {
            let (out, inn) = equal_power(t);
            let sum = out * out + inn * inn;
            assert!((sum - 1.0).abs() < 1e-4, "t={t}: sum {sum}");
            t += 0.05;
        }
    }

    #[test]
    fn mood_has_hysteresis() {
        assert_eq!(next_mood(0.40, Mood::Day), Mood::Day);
        assert_eq!(next_mood(0.45, Mood::Day), Mood::Dusk);
        assert_eq!(next_mood(0.35, Mood::Dusk), Mood::Dusk);
        assert_eq!(next_mood(0.30, Mood::Dusk), Mood::Day);
    }

    #[test]
    fn director_never_plays_more_than_two_tracks() {
        let tracks = [(Mood::Day, 30.0), (Mood::Day, 30.0), (Mood::Dusk, 30.0), (Mood::Dusk, 30.0)];
        let mut d = Director::new(Mood::Day);
        let mut t = 0.0f32;
        for _ in 0..6000 {
            let night = 0.5 + 0.5 * (TAU * t / 120.0).sin();
            let aud = d.step(0.1, night, &tracks);
            let audible: Vec<&(usize, f32)> = aud.iter().filter(|(_, g)| *g > 0.0).collect();
            assert!(audible.len() <= 2, "more than two tracks at t={t}");
            for (_, g) in &aud {
                assert!(*g >= 0.0 && *g <= 1.0, "gain {g} out of range at t={t}");
            }
            if audible.len() == 2 {
                let sum: f32 = audible.iter().map(|(_, g)| g * g).sum();
                assert!((sum - 1.0).abs() < 0.02, "equal-power sum {sum} at t={t}");
            }
            t += 0.1;
        }
    }

    #[test]
    fn director_never_goes_silent() {
        let tracks = [(Mood::Day, 30.0), (Mood::Day, 30.0), (Mood::Dusk, 30.0), (Mood::Dusk, 30.0)];
        let mut d = Director::new(Mood::Day);
        let mut t = 0.0f32;
        for _ in 0..6000 {
            let night = 0.5 + 0.5 * (TAU * t / 120.0).sin();
            let aud = d.step(0.1, night, &tracks);
            let audible: Vec<&(usize, f32)> = aud.iter().filter(|(_, g)| *g > 0.0).collect();
            if audible.len() == 1 {
                assert!(audible[0].1 >= 0.7, "silence at t={t}: {aud:?}");
            } else {
                assert_eq!(audible.len(), 2, "unexpected audible count at t={t}");
                let sum: f32 = audible.iter().map(|(_, g)| g * g).sum();
                assert!((sum - 1.0).abs() < 0.02, "equal-power sum {sum} at t={t}");
            }
            t += 0.1;
        }
    }

    #[test]
    fn mute_scales_all_music_gains() {
        let levels = Levels { music: 0.55, ambience: 0.35, sfx: 0.8 };
        assert_eq!(music_gain(0.0, &levels, true), 0.0);
        assert_eq!(music_gain(0.5, &levels, true), 0.0);
        assert_eq!(music_gain(1.0, &levels, true), 0.0);
        assert_eq!(music_gain(0.0, &levels, false), 0.0);
        assert_eq!(music_gain(0.5, &levels, false), 0.5 * 0.55);
        assert_eq!(music_gain(1.0, &levels, false), 0.55);
    }

    #[test]
    fn sfx_variants_rotate_without_immediate_repeat() {
        let mut p = VariantPicker::new();
        let mut prev = None;
        let mut seen = std::collections::HashSet::new();
        for _ in 0..300 {
            let i = p.pick("place", 3);
            if let Some(pp) = prev {
                assert_ne!(i, pp, "immediate repeat");
            }
            prev = Some(i);
            seen.insert(i);
        }
        assert_eq!(seen.len(), 3, "all variants should appear: {seen:?}");

        let mut single = VariantPicker::new();
        for _ in 0..10 {
            assert_eq!(single.pick("one", 1), 0);
        }
    }
}

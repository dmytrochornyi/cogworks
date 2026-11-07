//! Screen-space post-processing: the last pass that turns the lit world into
//! a painted frame in the spirit of hand-painted animated films. Lush but soft
//! colour, cool airy shadows by day and warm ones at golden hour, a gentle
//! halation around bright things, slanting sun shafts, drifting cloud shadows
//! and a light aerial haze toward the top of the screen; plus camera motion
//! blur. There is deliberately no grain or paper noise: the image stays clean.
//! `Look` picks the preset.
//!
//! Cost is kept low: the bloom is thresholded and blurred at quarter
//! resolution (so the full-screen pass reads it with one tap), motion blur only
//! takes extra taps while the camera moves, and the Clean look skips the post
//! pass and both bloom passes entirely.

use macroquad::prelude::*;
use serde::{Deserialize, Serialize};

/// A preset for the post-processing pass, cycled with the V key.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, Serialize, Deserialize)]
pub enum Look {
    /// Painted storybook: soft bloom, lush colour, sun shafts, cloud shadows.
    #[default]
    Storybook,
    /// No post effects at all (and no post cost).
    Clean,
}

/// How strongly each effect applies, 0..1.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct LookParams {
    pub bloom: f32,
    /// Lifts the saturation of muted colours (greens, sky), leaving vivid ones.
    pub vibrance: f32,
    /// Soft drifting cloud shadows over the land by day.
    pub clouds: f32,
    /// Camera motion blur, as a fraction of a frame's travel.
    pub motion: f32,
    /// Colour grade, sun shafts and aerial haze (scaled by the time of day).
    pub mood: f32,
}

impl Look {
    pub const ALL: [Look; 2] = [Look::Storybook, Look::Clean];

    pub fn next(self) -> Look {
        let i = Self::ALL.iter().position(|&l| l == self).unwrap_or(0);
        Self::ALL[(i + 1) % Self::ALL.len()]
    }

    pub fn name(self) -> &'static str {
        match self {
            Look::Storybook => "Storybook",
            Look::Clean => "Clean",
        }
    }

    /// Parses a look name, as given to `COGWORKS_LOOK`.
    pub fn parse(s: &str) -> Option<Look> {
        Self::ALL.into_iter().find(|l| l.name().eq_ignore_ascii_case(s.trim()))
    }

    pub fn params(self) -> LookParams {
        match self {
            Look::Storybook => LookParams { bloom: 0.7, vibrance: 1.0, clouds: 1.0, motion: 1.0, mood: 1.0 },
            Look::Clean => LookParams { bloom: 0.0, vibrance: 0.0, clouds: 0.0, motion: 0.0, mood: 0.0 },
        }
    }

    /// Whether this look needs the post pass at all.
    pub fn has_post(self) -> bool {
        let p = self.params();
        p.bloom + p.vibrance + p.clouds + p.motion + p.mood > 0.0
    }
}

/// Longest motion-blur streak, in screen pixels, however fast the camera moves.
pub const MAX_BLUR_PX: f32 = 36.0;

/// Motion-blur streak for a frame in which the view slid by `delta` screen
/// pixels. A huge jump (loading a save, respawning) is a cut, not motion, so it
/// gets no blur at all; otherwise the streak follows the travel and is capped.
pub fn blur_streak(delta: Vec2, strength: f32) -> Vec2 {
    let len = delta.length();
    if !(0.3..=200.0).contains(&len) {
        return Vec2::ZERO;
    }
    let s = delta * strength * 1.4;
    s.clamp_length_max(MAX_BLUR_PX)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn looks_cycle_through_every_preset() {
        let mut l = Look::default();
        let mut seen = vec![l];
        for _ in 0..Look::ALL.len() - 1 {
            l = l.next();
            seen.push(l);
        }
        assert_eq!(l.next(), Look::default());
        for look in Look::ALL {
            assert!(seen.contains(&look));
        }
    }

    #[test]
    fn clean_look_turns_everything_off_and_skips_the_pass() {
        let p = Look::Clean.params();
        assert_eq!([p.bloom, p.vibrance, p.clouds, p.motion, p.mood], [0.0; 5]);
        assert!(!Look::Clean.has_post());
        assert!(Look::Storybook.has_post());
    }

    #[test]
    fn looks_parse_by_name() {
        assert_eq!(Look::parse("clean"), Some(Look::Clean));
        assert_eq!(Look::parse(" Storybook "), Some(Look::Storybook));
        assert_eq!(Look::parse("comic"), None);
    }

    #[test]
    fn blur_follows_motion_but_ignores_cuts() {
        assert_eq!(blur_streak(Vec2::ZERO, 1.0), Vec2::ZERO);
        let s = blur_streak(vec2(6.0, 0.0), 1.0);
        assert!(s.x > 6.0 && s.y == 0.0, "a pan streaks along its direction: {s}");
        assert!(blur_streak(vec2(150.0, 0.0), 1.0).length() <= MAX_BLUR_PX + 1e-3);
        assert_eq!(blur_streak(vec2(900.0, 0.0), 1.0), Vec2::ZERO, "a teleport is a cut");
        assert_eq!(blur_streak(vec2(6.0, 0.0), 0.0), Vec2::ZERO);
    }
}

//! Screen-space post-processing: the last pass that turns the lit world into
//! a painted frame. Soft bloom and gentle ink give it a hand-painted
//! storybook feel; ink outlines, halftone shading and slightly misregistered
//! colour plates give it a printed-comic one; camera motion blur ties it
//! together. There is deliberately no grain or paper noise: the image stays
//! clean. `Look` picks how strongly each effect applies.

use macroquad::prelude::*;

/// A preset for the post-processing pass, cycled with the V key.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Look {
    /// Painted picture-book: soft bloom and gentle ink.
    #[default]
    Storybook,
    /// Printed comic: bold ink lines, halftone shadows, offset colour plates.
    Comic,
    /// No post effects at all.
    Clean,
}

/// How strongly each effect applies, 0..1.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct LookParams {
    pub ink: f32,
    pub halftone: f32,
    /// Colour-plate misregistration (red and blue nudged apart).
    pub chroma: f32,
    pub bloom: f32,
    /// Flattens colours toward a few printed tones.
    pub posterize: f32,
    /// Camera motion blur, as a fraction of a frame's travel.
    pub motion: f32,
}

impl Look {
    pub const ALL: [Look; 3] = [Look::Storybook, Look::Comic, Look::Clean];

    pub fn next(self) -> Look {
        let i = Self::ALL.iter().position(|&l| l == self).unwrap_or(0);
        Self::ALL[(i + 1) % Self::ALL.len()]
    }

    pub fn name(self) -> &'static str {
        match self {
            Look::Storybook => "Storybook",
            Look::Comic => "Comic",
            Look::Clean => "Clean",
        }
    }

    /// Parses a look name, as given to `COGWORKS_LOOK`.
    pub fn parse(s: &str) -> Option<Look> {
        Self::ALL.into_iter().find(|l| l.name().eq_ignore_ascii_case(s.trim()))
    }

    pub fn params(self) -> LookParams {
        match self {
            Look::Storybook => LookParams {
                ink: 0.45,
                halftone: 0.22,
                chroma: 0.25,
                bloom: 0.6,
                posterize: 0.0,
                motion: 1.0,
            },
            Look::Comic => LookParams {
                ink: 1.0,
                halftone: 0.9,
                chroma: 1.0,
                bloom: 0.4,
                posterize: 0.3,
                motion: 1.0,
            },
            Look::Clean => LookParams {
                ink: 0.0,
                halftone: 0.0,
                chroma: 0.0,
                bloom: 0.0,
                posterize: 0.0,
                motion: 0.0,
            },
        }
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
    fn clean_look_turns_everything_off() {
        let p = Look::Clean.params();
        assert_eq!(
            [p.ink, p.halftone, p.chroma, p.bloom, p.posterize, p.motion],
            [0.0; 6]
        );
    }

    #[test]
    fn looks_parse_by_name() {
        assert_eq!(Look::parse("comic"), Some(Look::Comic));
        assert_eq!(Look::parse(" Storybook "), Some(Look::Storybook));
        assert_eq!(Look::parse("vhs"), None);
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

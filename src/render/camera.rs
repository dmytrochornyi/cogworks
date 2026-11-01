use macroquad::prelude::*;

use crate::world::{MAP_H, MAP_W};

pub struct Cam {
    /// World position (in tiles) at the centre of the screen.
    pub pos: Vec2,
    /// Pixels per tile.
    pub zoom: f32,
}

impl Cam {
    pub(super) fn half_screen() -> Vec2 {
        vec2(screen_width(), screen_height()) * 0.5
    }

    pub fn w2s(&self, p: Vec2) -> Vec2 {
        (p - self.pos) * self.zoom + Self::half_screen()
    }

    pub fn s2w(&self, s: Vec2) -> Vec2 {
        (s - Self::half_screen()) / self.zoom + self.pos
    }

    /// Inclusive range of tiles on screen, clamped to the map.
    pub(super) fn visible(&self) -> (IVec2, IVec2) {
        let lo = self.s2w(Vec2::ZERO).floor().as_ivec2().max(IVec2::ZERO);
        let hi = self
            .s2w(vec2(screen_width(), screen_height()))
            .floor()
            .as_ivec2()
            .min(ivec2(MAP_W - 1, MAP_H - 1));
        (lo, hi)
    }

    pub(super) fn on_screen(&self, p: Vec2, margin: f32) -> bool {
        let s = self.w2s(p);
        let m = margin * self.zoom;
        s.x > -m && s.y > -m && s.x < screen_width() + m && s.y < screen_height() + m
    }
}

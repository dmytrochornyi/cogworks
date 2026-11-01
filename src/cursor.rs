//! In-world cursor drawing: the placement ghost and the mining / pickup progress ring.
//! The cozy custom cursor (Task 9) will live here too.

use macroquad::prelude::{IVec2, Rect, draw_circle, mouse_position};

use crate::game::Game;
use crate::items::Item;
use crate::render::draw_ghost;
use crate::theme;
use crate::ui;
use crate::world::dims;

impl Game {
    pub fn draw_cursor(&self) {
        if self.mouse_over_ui() || !self.world.player.alive() {
            return;
        }
        let z = self.cam.zoom;
        match self.tool.and_then(Item::place_kind) {
            Some(kind) => {
                let pos = self.ghost_pos(kind);
                let center = pos + (dims(kind, self.rot) - IVec2::ONE) / 2;
                let ok = self.world.player.inv.get(kind.item().unwrap_or(Item::Belt)) > 0
                    && self.world.player.in_reach(center)
                    && self.world.check_place(kind, pos, self.rot).is_ok();
                draw_ghost(&self.assets, &self.cam, kind, pos, self.rot, ok, self.time);
            }
            None => {
                let tile = self.mouse_tile();
                let (pos, size) = match self.world.entity_ref_at(tile) {
                    Some(e) => (e.pos, e.dims()),
                    None => (tile, IVec2::ONE),
                };
                let s = self.cam.w2s(pos.as_vec2());
                let w = size.as_vec2() * z;
                let col = if self.world.player.in_reach(tile) {
                    theme::alpha(theme::CREAM, 0.75)
                } else {
                    theme::alpha(theme::TERRA_400, 0.6)
                };
                ui::rrect_lines(Rect::new(s.x, s.y, w.x, w.y), z * 0.12, 2.0, col);
            }
        }
        // Progress ring for mining / picking up.
        let progress = self
            .deconstruct
            .map(|(_, p)| p)
            .or(self.world.player.mining.map(|(_, p)| p));
        if let Some(p) = progress {
            let (mx, my) = mouse_position();
            draw_circle(mx + 20.0, my + 20.0, 12.0, theme::alpha(theme::BG, 0.85));
            let steps = (p * 24.0) as i32;
            for k in 0..steps {
                let a = -std::f32::consts::FRAC_PI_2 + k as f32 / 24.0 * std::f32::consts::TAU;
                draw_circle(mx + 20.0 + a.cos() * 8.0, my + 20.0 + a.sin() * 8.0, 2.2, theme::color(theme::KEY));
            }
        }
    }
}

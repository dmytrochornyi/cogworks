//! In-world cursor drawing (the placement ghost and the mining ring) and the
//! cozy custom mouse cursor drawn last in UI space (Task 9).

use macroquad::prelude::{
    IVec2, MouseButton, Rect, draw_circle, is_mouse_button_down, mouse_position, vec2,
};

use crate::assets::SpriteParams;
use crate::game::Game;
use crate::input::Gesture;
use crate::items::Item;
use crate::render::draw_ghost;
use crate::theme;
use crate::ui;
use crate::world::{Kind, dims};

/// Which of the four cozy cursor sprites to show under the mouse.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum CursorKind {
    Point,
    Hand,
    Build,
    Mine,
}

/// Native pixel size of each cursor sprite.
const CURSOR_SIZE: f32 = 48.0;
/// On-screen size in logical UI pixels (scales with `ui_scale`).
const CURSOR_DRAW: f32 = 26.0;

/// Where each sprite anchors to the mouse position, in native pixels.
pub const POINT_HOTSPOT: [f32; 2] = [7.0, 7.0];
pub const HAND_HOTSPOT: [f32; 2] = [9.0, 9.0];
pub const BUILD_HOTSPOT: [f32; 2] = [7.0, 7.0];
pub const MINE_HOTSPOT: [f32; 2] = [7.0, 7.0];

/// Picks the cursor sprite. Priorities: Mine > Build > Hand > Point, with one
/// wrinkle — Build only applies when the mouse is *not* over clickable UI, so
/// hovering a slot while holding a tool still shows the Hand.
pub fn cursor_kind(over_clickable_ui: bool, tool: bool, right_held_on_minable: bool) -> CursorKind {
    if right_held_on_minable {
        CursorKind::Mine
    } else if tool && !over_clickable_ui {
        CursorKind::Build
    } else if over_clickable_ui {
        CursorKind::Hand
    } else {
        CursorKind::Point
    }
}

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

    /// The cozy custom mouse cursor, drawn last so it sits above tooltips.
    pub fn draw_ui_cursor(&self) {
        // Plain screenshots (COGWORKS_SCREENSHOT without COGWORKS_MOUSE) stay
        // cursor-free, so their pixel diffs don't change.
        if ui::screenshot_mode() && ui::mouse_override().is_none() {
            return;
        }
        let m = ui::ui_mouse();
        let over_clickable = self.clickables.iter().any(|r| r.contains(m));
        let kind = cursor_kind(over_clickable, self.tool.is_some(), self.right_held_on_minable());
        let (name, hotspot) = match kind {
            CursorKind::Point => ("cursor_point", POINT_HOTSPOT),
            CursorKind::Hand => ("cursor_hand", HAND_HOTSPOT),
            CursorKind::Build => ("cursor_build", BUILD_HOTSPOT),
            CursorKind::Mine => ("cursor_mine", MINE_HOTSPOT),
        };
        // A gentle 0.9→1.0 press squash while the left button is held.
        let squash = if left_down_for_squash() { 0.9 } else { 1.0 };
        let w = CURSOR_DRAW;
        let h = CURSOR_DRAW * squash;
        let pos = m - vec2(hotspot[0] / CURSOR_SIZE * w, hotspot[1] / CURSOR_SIZE * h);
        self.assets.draw(name, pos, vec2(w, h), SpriteParams::default());
    }

    /// Whether the right button is held over something the world can mine or
    /// pick up — the `Mine` cursor state.
    fn right_held_on_minable(&self) -> bool {
        if !is_mouse_button_down(MouseButton::Right) || self.gesture_right != Gesture::World {
            return false;
        }
        let tile = self.mouse_tile();
        if !self.world.player.in_reach(tile) {
            return false;
        }
        self.world.ore_at(tile).is_some()
            || self.world.has_tree(tile)
            || self.world.entity_ref_at(tile).is_some_and(|e| e.kind != Kind::Hive)
    }
}

/// Whether the left button is held, for the cursor press squash only. In
/// screenshot mode `COGWORKS_MOUSE_DOWN=1` fakes the press so a reviewer can
/// capture it; it is deliberately never fed into gameplay input.
fn left_down_for_squash() -> bool {
    if is_mouse_button_down(MouseButton::Left) {
        return true;
    }
    ui::screenshot_mode() && std::env::var("COGWORKS_MOUSE_DOWN").map(|v| v == "1").unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cursor_kind_priorities() {
        // Mine beats everything.
        assert_eq!(cursor_kind(true, true, true), CursorKind::Mine);
        assert_eq!(cursor_kind(false, false, true), CursorKind::Mine);
        // Build beats Hand and Point, but only outside clickable UI.
        assert_eq!(cursor_kind(false, true, false), CursorKind::Build);
        // Over clickable UI with a tool, show Hand (Build does not apply over UI).
        assert_eq!(cursor_kind(true, true, false), CursorKind::Hand);
        // Hand beats Point.
        assert_eq!(cursor_kind(true, false, false), CursorKind::Hand);
        // Point is the fallback.
        assert_eq!(cursor_kind(false, false, false), CursorKind::Point);
    }
}

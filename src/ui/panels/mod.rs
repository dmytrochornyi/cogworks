//! Window panels: inventory, research and building, plus the shared helpers
//! they use for slide-in animation, centring and grid layout.

use macroquad::prelude::{Rect, Vec2};

use crate::Game;

use super::metrics::{ui_height, ui_width};
use super::primitives::ease_out;
use super::style::GAP;
use crate::theme::space;

pub(super) mod building;
pub(super) mod inventory;
pub(super) mod research;

/// Pixel offset of the open modal panel during its drop-in animation.
fn slide_px(g: &Game) -> f32 {
    (1.0 - ease_out(g.panel_slide)) * 28.0
}

fn centered(w: f32, h: f32) -> Rect {
    let (sw, sh) = (ui_width(), ui_height());
    let w = w.min(sw - 2.0 * space::L);
    let h = h.min(sh - 2.0 * space::L);
    Rect::new(((sw - w) / 2.0).max(space::L), ((sh - h) / 2.0).max(space::L), w, h)
}

fn grid_rect(origin: Vec2, k: usize, cols: usize, size: f32) -> Rect {
    let (col, row) = (k % cols, k / cols);
    Rect::new(origin.x + col as f32 * (size + GAP), origin.y + row as f32 * (size + GAP), size, size)
}

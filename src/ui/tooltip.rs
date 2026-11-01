//! The floating tooltip and the recipe text it shows.

use macroquad::prelude::*;

use crate::assets::Assets;
use crate::items::*;
use crate::theme::{space, TextSize};
use crate::world::*;

use super::metrics::{ui_height, ui_mouse, ui_width};
use super::primitives::{Elevation, surface};
use super::style::{LINE, TEXT, TITLE};
use super::text::{text, text_w};

/// Tooltip text for a recipe, with what the player has of each ingredient.
pub(super) fn recipe_lines(world: &World, r: usize) -> Vec<String> {
    let rec = &RECIPES[r];
    let mut lines = vec![rec.name.to_string()];
    if let Some((f, n)) = rec.fluid_in {
        lines.push(format!("{n} {}", f.name()));
    }
    for &(i, n) in rec.inputs {
        lines.push(format!("{n} {} · Have {}", i.name(), world.player.inv.get(i)));
    }
    let outs: Vec<String> = rec
        .outputs
        .iter()
        .map(|(i, n)| format!("{n} {}", i.name()))
        .chain(rec.fluid_out.map(|(f, n)| format!("{n} {}", f.name())))
        .collect();
    lines.push(format!("Makes {} in {}s", outs.join(" + "), rec.time));
    lines
}

pub(super) fn draw_tooltip(a: &Assets, lines: &[String]) {
    if lines.is_empty() {
        return;
    }
    let width = lines.iter().map(|l| text_w(a, l, TextSize::M)).fold(0.0, f32::max);
    let m = ui_mouse();
    let h = lines.len() as f32 * LINE + 16.0;
    let x = (m.x + 18.0).min(ui_width() - width - 34.0).max(space::S);
    let y = (m.y + 18.0).min(ui_height() - h - space::S).max(space::S);
    let r = Rect::new(x, y, width + 28.0, h);
    surface(r, Elevation::Tooltip);
    for (k, l) in lines.iter().enumerate() {
        let col = if k == 0 { TITLE } else { TEXT };
        text(a, l, x + 14.0, y + 8.0 + k as f32 * LINE, TextSize::M, col);
    }
}

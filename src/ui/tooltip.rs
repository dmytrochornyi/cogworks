//! The floating tooltip and the recipe card it shows.

use macroquad::prelude::*;

use crate::assets::Assets;
use crate::items::*;
use crate::render::draw_item;
use crate::theme::{self, line, radius, space, TextSize};
use crate::world::*;

use super::metrics::{ui_height, ui_mouse, ui_width};
use super::primitives::{Elevation, circle, circle_lines, rrect, rrect_lines, surface};
use super::style::{BG, EDGE, KEY, LINE, PEACH, TEXT, TEXT_DIM, TITLE};
use super::text::{text, text_in, text_w};

/// A hover tooltip: plain text lines, or a structured recipe card.
pub enum Tip {
    Lines(Vec<String>),
    Recipe(usize),
}

/// How an ingredient's requirement stands against the backpack and the workbench.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum InputState {
    /// Already have enough.
    Enough,
    /// Short, but the workbench can craft the missing parts by hand.
    AutoCraft,
    /// Short, and no hand recipe can fill the gap.
    Missing,
}

/// One item or fluid output, for the "Makes" row.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum OutputKind {
    Item(Item),
    Fluid(Fluid),
}

/// One ingredient row of the card.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct CardRow {
    pub item: Option<Item>,
    pub fluid: Option<Fluid>,
    pub need: u32,
    pub have: u32,
    pub state: InputState,
}

/// The recipe card's data: one row per input, the outputs, and the craft time.
/// Pure: it only reads the world, so tests can call it without a window.
pub fn recipe_card(world: &World, recipe: usize) -> (Vec<CardRow>, Vec<(OutputKind, u32)>, f32) {
    let rec = &RECIPES[recipe];
    let mut rows = Vec::new();
    if let Some((f, n)) = rec.fluid_in {
        // Hand recipes don't use fluids; machine recipes shown elsewhere just
        // list the amount, which the workbench can't top up.
        rows.push(CardRow {
            item: None,
            fluid: Some(f),
            need: n as u32,
            have: 0,
            state: InputState::Enough,
        });
    }
    for &(i, n) in rec.inputs {
        let have = world.player.inv.get(i);
        let state = if have >= n {
            InputState::Enough
        } else if world.can_cover(i, n) {
            InputState::AutoCraft
        } else {
            InputState::Missing
        };
        rows.push(CardRow { item: Some(i), fluid: None, need: n, have, state });
    }
    let mut outs = Vec::new();
    for &(i, n) in rec.outputs {
        outs.push((OutputKind::Item(i), n));
    }
    if let Some((f, n)) = rec.fluid_out {
        outs.push((OutputKind::Fluid(f), n as u32));
    }
    (rows, outs, rec.time)
}

pub(super) fn draw_tooltip(a: &Assets, world: &World, tip: &Option<Tip>) {
    match tip {
        None => {}
        Some(Tip::Lines(lines)) => draw_lines_tooltip(a, lines),
        Some(Tip::Recipe(r)) => draw_recipe_card(a, world, *r),
    }
}

fn draw_lines_tooltip(a: &Assets, lines: &[String]) {
    if lines.is_empty() {
        return;
    }
    let width = lines.iter().map(|l| text_w(a, l, TextSize::M)).fold(0.0, f32::max);
    let m = ui_mouse();
    let h = lines.len() as f32 * LINE + space::L;
    let x = (m.x + 18.0).min(ui_width() - width - 2.0 * space::L - space::S).max(space::S);
    let y = (m.y + 18.0).min(ui_height() - h - space::S).max(space::S);
    let r = Rect::new(x, y, width + 2.0 * space::L, h);
    surface(r, Elevation::Tooltip);
    for (k, l) in lines.iter().enumerate() {
        let col = if k == 0 { TITLE } else { TEXT };
        text(a, l, x + space::L, y + space::S + k as f32 * LINE, TextSize::M, col);
    }
}

fn draw_recipe_card(a: &Assets, world: &World, recipe: usize) {
    let (rows, outs, time) = recipe_card(world, recipe);
    let rec = &RECIPES[recipe];

    // Layout constants, snapped to the theme's spacing steps.
    let w = 288.0;
    let pad = space::L; // 16
    let header_h = 34.0;
    let row_h = 26.0;
    let out_h = 24.0;
    let foot_h = 22.0;
    let sec = space::S; // 8
    let h = space::M + header_h
        + rows.len() as f32 * row_h
        + sec + line::HAIR + sec
        + out_h + sec + foot_h
        + space::M;

    // Clamp to the screen like the text tooltip.
    let m = ui_mouse();
    let x = (m.x + 18.0).min(ui_width() - w - space::S).max(space::S);
    let y = (m.y + 18.0).min(ui_height() - h - space::S).max(space::S);
    let r = Rect::new(x, y, w, h);
    surface(r, Elevation::Tooltip);

    let left = r.x + pad;
    let right = r.right() - pad;
    let mut cy = r.y + space::M;

    // Header: the output's icon and the recipe name.
    if let Some((kind, _)) = outs.first() {
        draw_output_icon(a, vec2(left + 11.0, cy + header_h / 2.0), 22.0, *kind);
    }
    text(a, rec.name, left + 26.0, cy + (header_h - LINE) / 2.0, TextSize::M, TITLE);
    cy += header_h;

    // One row per input.
    for row in &rows {
        let icon = vec2(left + 11.0, cy + row_h / 2.0);
        match (row.item, row.fluid) {
            (Some(i), _) => draw_item(a, icon, 20.0, i),
            (None, Some(f)) => draw_fluid_icon(icon, 20.0, f),
            (None, None) => {}
        }
        let name = match (row.item, row.fluid) {
            (Some(i), _) => i.name(),
            (_, Some(f)) => f.name(),
            (None, None) => "",
        };
        text(a, name, left + 26.0, cy + 4.0, TextSize::M, TEXT);
        // Fluids have no backpack count, so show the amount; items show have/need.
        let right_text = if row.fluid.is_some() {
            row.need.to_string()
        } else {
            format!("{} / {}", row.have, row.need)
        };
        let (col, auto) = match row.state {
            InputState::Enough => (TEXT, false),
            InputState::AutoCraft => (KEY, true),
            InputState::Missing => (PEACH, false),
        };
        let tw = text_w(a, &right_text, TextSize::M);
        let tx = right - tw;
        text(a, &right_text, tx, cy + 4.0, TextSize::M, col);
        if auto {
            let s = "auto";
            let cw = text_w(a, s, TextSize::Xs) + 10.0;
            let ch = 16.0;
            let cr = Rect::new(tx - cw - space::XS, cy + (row_h - ch) / 2.0, cw, ch);
            rrect(cr, radius::SM, BG);
            rrect_lines(cr, radius::SM, line::HAIR, EDGE);
            text_in(a, s, cr, TextSize::Xs, KEY);
        }
        cy += row_h;
    }

    // Thin divider between the ingredients and the output.
    cy += sec;
    draw_line(left, cy, right, cy, line::HAIR, theme::color(theme::WOOD_TRIM));
    cy += line::HAIR + sec;

    // Output row: "Makes" with icons and counts, and the craft time.
    text(a, "Makes", left, cy + 4.0, TextSize::M, TEXT);
    let mut ox = left + text_w(a, "Makes", TextSize::M) + space::S;
    for (kind, n) in &outs {
        draw_output_icon(a, vec2(ox + 10.0, cy + out_h / 2.0), 20.0, *kind);
        let cnt = format!("x{n}");
        text(a, &cnt, ox + 21.0, cy + 4.0, TextSize::S, TEXT_DIM);
        ox += 21.0 + text_w(a, &cnt, TextSize::S) + space::M;
    }
    let time_s = if time.fract() == 0.0 {
        format!("{time:.0}s")
    } else {
        format!("{time:.1}s")
    };
    let clock_c = vec2(right - 10.0, cy + out_h / 2.0);
    text(
        a,
        &time_s,
        clock_c.x - 16.0 - text_w(a, &time_s, TextSize::S),
        cy + 4.0,
        TextSize::S,
        TEXT_DIM,
    );
    draw_clock(clock_c, 8.0, TEXT);
    cy += out_h;

    // Footer: key chips for the click gestures.
    cy += sec;
    let mut cx = left;
    for label in ["Click · craft 1", "Shift+Click · craft 5"] {
        let cw = text_w(a, label, TextSize::Xs) + 14.0;
        let ch = 20.0;
        let cr = Rect::new(cx, cy, cw, ch);
        rrect(cr, radius::SM, BG);
        rrect_lines(cr, radius::SM, line::HAIR, EDGE);
        text_in(a, label, cr, TextSize::Xs, KEY);
        cx += cw + space::XS;
    }
}

fn draw_output_icon(a: &Assets, c: Vec2, size: f32, kind: OutputKind) {
    match kind {
        OutputKind::Item(i) => draw_item(a, c, size, i),
        OutputKind::Fluid(f) => draw_fluid_icon(c, size, f),
    }
}

fn draw_fluid_icon(c: Vec2, size: f32, f: Fluid) {
    let r = size * 0.5;
    circle(c, r, f.color());
    circle_lines(c, r, line::STRONG, TEXT);
}

/// A tiny clock face drawn with primitives: a circle and two hands.
fn draw_clock(c: Vec2, r: f32, col: Color) {
    circle_lines(c, r, line::HAIR, col);
    let hour = c + vec2(0.0, -r * 0.5);
    let minute = c + vec2(r * 0.65, 0.0);
    draw_line(c.x, c.y, hour.x, hour.y, line::HAIR, col);
    draw_line(c.x, c.y, minute.x, minute.y, line::HAIR, col);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::items::recipe_id;

    fn world_with(items: &[(Item, u32)]) -> World {
        let mut w = World::new(7);
        w.player.inv.clear();
        for &(i, n) in items {
            w.player.inv.add(i, n);
        }
        w
    }

    fn state_of(rows: &[CardRow], item: Item) -> InputState {
        rows.iter().find(|r| r.item == Some(item)).unwrap().state
    }

    #[test]
    fn recipe_card_marks_each_input() {
        // Motor: Steel 1, Gear 2, Wire 4. Steel is Enough; Gear is craftable
        // from the Iron Plates in the backpack (AutoCraft); Wire has no copper
        // to be drawn from, so it is Missing.
        let w = world_with(&[(Item::Steel, 10), (Item::IronPlate, 4)]);
        let (rows, _, _) = recipe_card(&w, recipe_id("Motor"));
        assert_eq!(state_of(&rows, Item::Steel), InputState::Enough);
        assert_eq!(state_of(&rows, Item::Gear), InputState::AutoCraft);
        assert_eq!(state_of(&rows, Item::Wire), InputState::Missing);
    }

    #[test]
    fn recipe_card_lists_fluids_and_outputs() {
        // Polymer: Coal and Naphtha in, Polymer out; fluid amounts are whole.
        let w = world_with(&[]);
        let (rows, outs, time) = recipe_card(&w, recipe_id("Polymer"));
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].fluid, Some(Fluid::Naphtha));
        assert_eq!(rows[0].need, 20);
        assert_eq!(rows[0].state, InputState::Enough);
        assert_eq!(rows[1].item, Some(Item::Coal));
        assert_eq!(rows[1].need, 1);
        assert_eq!(outs, vec![(OutputKind::Item(Item::Polymer), 2)]);
        assert_eq!(time, 2.0);

        // Naphtha: a pure fluid recipe, in and out.
        let (rows, outs, time) = recipe_card(&w, recipe_id("Naphtha"));
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].fluid, Some(Fluid::Tar));
        assert_eq!(rows[0].need, 10);
        assert_eq!(outs, vec![(OutputKind::Fluid(Fluid::Naphtha), 8)]);
        assert_eq!(time, 4.0);
    }
}

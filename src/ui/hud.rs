//! The HUD: health/research header, quickbar, craft queue, radio, toasts,
//! help note and end-of-frame overlays.

use macroquad::prelude::*;

use crate::assets::Assets;
use crate::items::*;
use crate::render::*;
use crate::theme::{self, TextSize, line, space};
use crate::world::*;
use crate::Game;

use super::metrics::{ui_height, ui_width};
use super::primitives::{Elevation, clicked, hovered, rrect, rrect_lines, surface, surface_alpha};
use super::style::{GAP, HONEY, KEY, LINE, PEACH, QUICK_SLOT, SLOT, TERRA, TEAL, TEXT, TEXT_DIM, TITLE};
use super::text::{text, text_keys, text_w};
use super::tooltip::Tip;
use super::widgets::{bar, slot};

pub(super) fn hud(g: &mut Game) {
    let a = g.assets.clone();
    let t = g.time;
    let w = &g.world;
    let r = Rect::new(space::L, space::L, 306.0, 112.0);
    surface(r, Elevation::Panel);
    a.draw_centered("icon_heart", vec2(r.x + 26.0, r.y + 26.0), Vec2::splat(24.0), 0.0, WHITE);
    let hp_frac = w.player.hp.max(0.0) / PLAYER_HP;
    bar(Rect::new(r.x + 44.0, r.y + 18.0, 244.0, 14.0), hp_frac, TERRA);
    if hp_frac > 0.0 && hp_frac < 0.3 {
        let p = 0.5 + 0.5 * (t * 5.0).sin();
        rrect_lines(
            Rect::new(r.x + 42.0, r.y + 16.0, 248.0, 18.0),
            9.0,
            line::STRONG,
            theme::alpha(theme::TERRA_400, 0.3 + 0.5 * p),
        );
    }
    a.draw_centered("icon_flask", vec2(r.x + 26.0, r.y + 58.0), Vec2::splat(24.0), 0.0, WHITE);
    match w.research.current {
        Some(t) => {
            let tech = &TECHS[t];
            text(&a, tech.name, r.x + 44.0, r.y + 46.0, TextSize::M, TEXT);
            bar(Rect::new(r.x + 44.0, r.y + 66.0, 244.0, 8.0), w.research.progress as f32 / tech.units as f32, TEAL);
        }
        None => text(&a, "Pick a research · press T", r.x + 44.0, r.y + 51.0, TextSize::M, KEY),
    }
    let sky = sky(w.time);
    let threat = if w.peaceful { "Peaceful".to_string() } else { format!("Hives {:.0}%", w.evolution * 100.0) };
    text(&a, &format!("{} · {threat}", sky.label), r.x + space::L, r.y + 88.0, TextSize::S, TEXT_DIM);
    g.ui_rects.push(r);
}

fn quickbar_rects() -> Vec<Rect> {
    let total = 10.0 * QUICK_SLOT + 9.0 * GAP;
    let x0 = (ui_width() - total) / 2.0;
    let y = ui_height() - QUICK_SLOT - space::L;
    (0..10).map(|k| Rect::new(x0 + k as f32 * (QUICK_SLOT + GAP), y, QUICK_SLOT, QUICK_SLOT)).collect()
}

pub(super) fn quickbar(g: &mut Game) {
    let a = g.assets.clone();
    let rects = quickbar_rects();
    let back = Rect::new(
        rects[0].x - space::S,
        rects[0].y - space::S,
        rects[9].right() - rects[0].x + 2.0 * space::S,
        QUICK_SLOT + 2.0 * space::S,
    );
    surface(back, Elevation::Panel);
    g.ui_rects.push(back);
    for (k, r) in rects.into_iter().enumerate() {
        let item = g.world.player.quickbar[k];
        let count = item.map(|i| g.world.player.inv.get(i));
        let selected = item.is_some() && g.tool == item;
        slot(&a, r, item, count, selected, count == Some(0), &mut g.clickables);
        text(&a, &((k + 1) % 10).to_string(), r.x + 5.0, r.y + 3.0, TextSize::Xs, TEXT_DIM);
        if hovered(r)
            && let Some(i) = item
        {
            g.tooltip = Some(Tip::item(i, &[&format!("Key {}", (k + 1) % 10)]));
        }
        if clicked(r) {
            g.select_slot(k);
        }
    }
}

pub(super) fn craft_queue(g: &mut Game) {
    if g.world.player.queue.is_empty() {
        return;
    }
    let a = g.assets.clone();
    let y = ui_height() - QUICK_SLOT - space::L - SLOT - 28.0;
    let total = 10.0 * QUICK_SLOT + 9.0 * GAP;
    let x0 = (ui_width() - total) / 2.0;
    text(&a, "Crafting · click to cancel", x0, y - 20.0, TextSize::S, TEXT_DIM);
    let mut cancel = None;
    for (k, c) in g.world.player.queue.iter().take(12).enumerate() {
        let r = Rect::new(x0 + k as f32 * (SLOT + GAP), y, SLOT, SLOT);
        let rec = &RECIPES[c.recipe];
        slot(&a, r, rec.main_output(), None, k == 0, false, &mut g.clickables);
        if k == 0 {
            bar(Rect::new(r.x + 5.0, r.bottom() - 8.0, r.w - 10.0, 4.0), c.progress / rec.time, HONEY);
        }
        if hovered(r) {
            g.tooltip = Some(match rec.main_output() {
                Some(i) => Tip::item(i, &["Click to cancel"]),
                None => Tip::Card { icon: None, rows: vec![(None, rec.name.to_string())] },
            });
        }
        if clicked(r) {
            cancel = Some(k);
        }
        g.ui_rects.push(r);
    }
    if g.world.player.queue.len() > 12 {
        let more = format!("+{}", g.world.player.queue.len() - 12);
        text(&a, &more, x0 + 12.0 * (SLOT + GAP), y + 13.0, TextSize::L, TEXT);
    }
    if let Some(k) = cancel {
        g.world.cancel_craft(k);
    }
}

pub(super) fn radio(g: &Game, a: &Assets) {
    let (prefix, suffix) = if g.music_on { ("Lo-fi radio · ", " to mute") } else { ("Radio off · ", " to play") };
    let w = text_w(a, prefix, TextSize::S) + text_w(a, "M", TextSize::S) + text_w(a, suffix, TextSize::S);
    let x = ui_width() - w - 46.0;
    let total = 10.0 * QUICK_SLOT + 9.0 * GAP;
    let qx = (ui_width() - total) / 2.0;
    // On narrow screens the quickbar reaches the bottom-right corner; lift the
    // radio above it instead of letting the two panels overlap.
    let overlap = x + w > qx && x < qx + total;
    let y = if overlap { ui_height() - QUICK_SLOT - space::L - 26.0 } else { ui_height() - 30.0 };
    // A small paper pill keeps the label readable over the grass.
    surface(Rect::new(x - 36.0, y - 20.0, w + 50.0, 44.0), Elevation::Toast);
    let bob = if g.music_on { (g.time * 2.4).sin() * 2.0 } else { 0.0 };
    let tint = if g.music_on { WHITE } else { theme::alpha(theme::WHITE, 0.4) };
    a.draw_centered("icon_note", vec2(x - 14.0, y - 5.0 + bob), Vec2::splat(20.0), 0.0, tint);
    text(a, prefix, x, y, TextSize::S, TEXT_DIM);
    let mx = x + text_w(a, prefix, TextSize::S);
    text(a, "M", mx, y, TextSize::S, KEY);
    text(a, suffix, mx + text_w(a, "M", TextSize::S), y, TextSize::S, TEXT_DIM);
}

pub(super) fn toasts(g: &Game, a: &Assets) {
    for (k, (msg, t)) in g.toasts.iter().enumerate() {
        let alpha = t.min(1.0);
        let age = 3.0 - t;
        let slide = if age < 0.25 { (1.0 - age / 0.25) * -22.0 } else { 0.0 };
        let w = text_w(a, msg, TextSize::L);
        let x = (ui_width() - w) / 2.0;
        let y = 140.0 + k as f32 * 42.0 + slide;
        let r = Rect::new(x - 18.0, y - 6.0, w + 36.0, 38.0);
        surface_alpha(r, Elevation::Toast, alpha);
        rrect(
            Rect::new(r.x, r.y + 9.0, 4.0, r.h - 18.0),
            2.0,
            theme::alpha(theme::EDGE, alpha),
        );
        text(a, msg, x, y + 4.0, TextSize::L, theme::alpha(theme::TEXT, alpha));
    }
}

/// The help note's text rows: (text, size, colour).
const HELP_LINES: &[(&str, TextSize, Color)] = &[
    ("Welcome to Cogworks", TextSize::Xl, TITLE),
    ("", TextSize::M, TEXT),
    ("WASD · walk          Wheel · zoom          E · backpack & workbench          T · research", TextSize::M, TEXT),
    ("1–0 · quickbar      R · rotate      Q · copy building or empty hands      Esc · close", TextSize::M, TEXT),
    ("Left click · build (drag belts, pipes, walls) or open a building", TextSize::M, TEXT),
    ("Right hold · mine ore by hand, or pick a building back up", TextSize::M, TEXT),
    ("F5 · save   F9 · load   O · pollution   P · peaceful   M · music   Space · pause", TextSize::M, TEXT),
    ("", TextSize::M, TEXT),
    ("Arms carry things from behind them to the front. Drills drop ore at their chute.", TextSize::M, TEXT),
    ("Belts have two lanes; feeding from the side fills the near one.", TextSize::M, TEXT),
    ("Machines need power: burn coal in a Generator, spread it with lamp poles.", TextSize::M, TEXT),
    ("Smoke drifts out to the hives at the map's edge, and they come to visit.", TextSize::M, TEXT),
    ("Fill Labs with data, research upward, and wake the Grand Engine.", TextSize::M, TEXT),
    ("", TextSize::M, TEXT),
    ("H · hide this note", TextSize::S, TEXT_DIM),
];

/// Widest help line measured at 17px M PLUS (≈650 px), plus rounding headroom
/// so the note's width is stable at every UI scale and never clips the text.
const HELP_MAX_TEXT_W: f32 = 700.0;

/// The help note's on-screen bounds, clamped so it never runs off the right or
/// bottom edge. The text is drawn at `x + space::XL`, `y + space::L + k * LINE`,
/// which this rect covers by construction.
fn help_rect(ui_w: f32, ui_h: f32) -> Rect {
    let w = (HELP_MAX_TEXT_W + 2.0 * space::XL).min(ui_w - 2.0 * space::L).max(0.0);
    let h = (HELP_LINES.len() as f32 * LINE + 2.0 * space::L).min(ui_h - 142.0).max(0.0);
    Rect::new(space::L, 142.0, w, h)
}

pub(super) fn help(g: &mut Game, a: &Assets) {
    let r = help_rect(ui_width(), ui_height());
    surface(r, Elevation::Panel);
    for (k, (l, size, col)) in HELP_LINES.iter().enumerate() {
        text_keys(a, l, r.x + space::XL, r.y + space::L + k as f32 * LINE, *size, *col);
    }
    // Clicks that start on the note must not build, mine or open buildings.
    g.ui_rects.push(r);
}

pub(super) fn overlays(g: &Game, a: &Assets) {
    let banner = |msg: &str, col: Color, y: f32| {
        let w = text_w(a, msg, TextSize::Xxl);
        let x = (ui_width() - w) / 2.0;
        let r = Rect::new(x - 28.0, y, w + 56.0, 54.0);
        surface(r, Elevation::Banner);
        text(a, msg, x, y + 12.0, TextSize::Xxl, col);
    };
    if !g.world.player.alive() {
        let msg = format!("Cog took a tumble · back in {:.0}…", g.world.player.respawn.ceil());
        banner(&msg, PEACH, ui_height() / 2.0 - 40.0);
    }
    if g.world.won {
        banner("The Grand Engine hums awake · you did it", TITLE, 92.0);
    }
    if g.paused {
        banner("Paused", TEXT, ui_height() / 2.0 - 110.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn help_note_blocks_clicks() {
        let r = help_rect(1440.0, 900.0);
        // The note is non-empty.
        assert!(r.w > 0.0 && r.h > 0.0, "help rect is empty: {r:?}");
        // It covers every line of text, drawn at y = r.y + space::L + k * LINE.
        for k in 0..HELP_LINES.len() {
            let top = r.y + space::L + k as f32 * LINE;
            assert!(top >= r.y, "line {k} starts above the rect");
            assert!(top + LINE <= r.y + r.h, "line {k} falls below the rect");
        }
        // The text is drawn at x = r.x + space::XL, so the rect must leave room
        // for the widest line to the right of that point.
        assert!(r.w - space::XL >= HELP_MAX_TEXT_W, "rect too narrow for the help text");
        // On a normal window the whole note stays on screen.
        assert!(r.x + r.w <= 1440.0 && r.y + r.h <= 900.0, "note runs off screen: {r:?}");
    }
}

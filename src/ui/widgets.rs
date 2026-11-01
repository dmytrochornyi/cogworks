//! Reusable widgets: bars, buttons, slots, the window `Panel`, recipe icons
//! and the shared stock-line listing.

use macroquad::prelude::*;

use crate::assets::Assets;
use crate::items::*;
use crate::render::*;
use crate::theme::{self, TextSize, line, radius, space};
use crate::Game;

use super::primitives::{Elevation, circle, circle_lines, clicked, hovered, rrect, rrect_lines, surface};
use super::style::{BG, BG2, EDGE, HEADER, KEY, SLOT_WELL, TEXT, TEXT_DIM, TITLE};
use super::text::{text, text_in, text_w};

pub(super) fn bar(r: Rect, frac: f32, col: Color) {
    rrect(r, r.h / 2.0, SLOT_WELL);
    if frac > 0.0 {
        let w = (r.w * frac.clamp(0.0, 1.0)).max(r.h);
        rrect(Rect::new(r.x, r.y, w, r.h), r.h / 2.0, col);
    }
    rrect_lines(r, r.h / 2.0, line::HAIR, theme::alpha(theme::TEXT, 0.22));
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum ButtonKind {
    Primary,
    Secondary,
}

fn button_kind(a: &Assets, r: Rect, label: &str, enabled: bool, kind: ButtonKind) -> bool {
    let hot = enabled && hovered(r);
    let down = hot && is_mouse_button_down(MouseButton::Left);
    let lacquer = theme::color(theme::LACQUER_500);
    let lacquer_hot = theme::color(theme::LACQUER_400);
    let (fill, border, text_col) = match (kind, enabled, hot) {
        (ButtonKind::Primary, true, false) => (lacquer, lacquer, BG),
        (ButtonKind::Primary, true, true) => (lacquer_hot, lacquer_hot, BG),
        (ButtonKind::Primary, false, _) => (theme::alpha(theme::SLOT_WELL, 0.5), theme::alpha(theme::EDGE, 0.35), theme::alpha(theme::TEXT_DIM, 0.5)),
        (ButtonKind::Secondary, true, false) => (BG, EDGE, TEXT),
        (ButtonKind::Secondary, true, true) => (BG2, EDGE, TEXT),
        (ButtonKind::Secondary, false, _) => (theme::alpha(theme::BG2, 0.5), theme::alpha(theme::EDGE, 0.35), theme::alpha(theme::TEXT_DIM, 0.45)),
    };
    let off = if down { 1.0 } else { 0.0 };
    rrect(Rect::new(r.x, r.y + off, r.w, r.h), radius::MD, fill);
    rrect_lines(Rect::new(r.x, r.y + off, r.w, r.h), radius::MD, line::BASE, border);
    text_in(a, label, Rect::new(r.x, r.y + off, r.w, r.h), TextSize::M, text_col);
    enabled && clicked(r)
}

pub(super) fn button(a: &Assets, r: Rect, label: &str, enabled: bool) -> bool {
    button_kind(a, r, label, enabled, ButtonKind::Secondary)
}

pub(super) fn primary_button(a: &Assets, r: Rect, label: &str, enabled: bool) -> bool {
    button_kind(a, r, label, enabled, ButtonKind::Primary)
}

pub(super) fn slot(a: &Assets, r: Rect, item: Option<Item>, count: Option<u32>, selected: bool, unavailable: bool) -> bool {
    let hot = hovered(r);
    // A warm paper "well" under the icon; hovering lightens it to the panel.
    let fill = if hot && !selected { BG } else { SLOT_WELL };
    rrect(r, radius::MD, fill);
    // Selection is the only lacquer-ringed state; hover keeps the normal wood edge.
    let (thick, ring) = if selected {
        (line::STRONG, theme::color(theme::LACQUER_500))
    } else {
        (line::BASE, EDGE)
    };
    rrect_lines(r, radius::MD, thick, ring);
    if let Some(item) = item {
        let tint = if unavailable { theme::alpha(theme::WHITE, 0.4) } else { WHITE };
        draw_item_tint(a, r.center(), r.w * 0.7, item, tint);
    }
    if let Some(n) = count {
        let s = n.to_string();
        // A small paper badge at the slot's bottom-right so the count stays
        // readable over the icon instead of dark ink floating on art.
        let bw = text_w(a, &s, TextSize::S) + 8.0;
        let bh = 18.0;
        let br = Rect::new(r.right() - bw - 2.0, r.bottom() - bh - 2.0, bw, bh);
        rrect(br, radius::SM, BG);
        rrect_lines(br, radius::SM, line::HAIR, EDGE);
        text_in(a, &s, br, TextSize::S, if unavailable { TEXT_DIM } else { TEXT });
    }
    if selected {
        // Small corner tick that survives even when the icon is bright.
        let p = vec2(r.right() - 3.0, r.y + 3.0);
        draw_triangle(p, p + vec2(-7.0, 0.0), p + vec2(0.0, 7.0), KEY);
    }
    clicked(r)
}

pub(super) fn recipe_icon(a: &Assets, r: Rect, recipe: usize, selected: bool, unavailable: bool) {
    let rec = &RECIPES[recipe];
    slot(a, r, rec.main_output(), None, selected, unavailable);
    if rec.main_output().is_none()
        && let Some((f, _)) = rec.fluid_out
    {
        let tint = if unavailable { theme::alpha(theme::WHITE, 0.4) } else { f.color() };
        circle(r.center(), r.w * 0.26, tint);
        circle_lines(r.center(), r.w * 0.26, line::STRONG, TEXT);
    }
}

pub(super) fn stock_lines(a: &Assets, stock: &Stock, x: f32, y: &mut f32) {
    for (i, n) in stock.iter() {
        draw_item(a, vec2(x + 10.0, *y + 2.0), 20.0, i);
        text(a, &format!("{} x{n}", i.name()), x + 26.0, *y, TextSize::S, TEXT_DIM);
        *y += 24.0;
    }
}

/// A window with a title, a close button and an Esc hint.
pub(super) struct Panel<'a> {
    r: Rect,
    title: Option<&'a str>,
    subtitle: Option<&'a str>,
    closable: bool,
}

impl<'a> Panel<'a> {
    pub fn new(r: Rect) -> Self {
        Panel { r, title: None, subtitle: None, closable: false }
    }

    pub fn title(mut self, t: &'a str) -> Self {
        self.title = Some(t);
        self
    }

    pub fn subtitle(mut self, t: &'a str) -> Self {
        self.subtitle = Some(t);
        self
    }

    pub fn closable(mut self) -> Self {
        self.closable = true;
        self
    }

    /// Draws the window and returns its content area plus whether close was clicked.
    pub fn show(self, g: &mut Game) -> (Rect, bool) {
        surface(self.r, Elevation::Panel);
        g.ui_rects.push(self.r);
        let a = g.assets.clone();
        let mut closed = false;
        if let Some(t) = self.title {
            text(&a, t, self.r.x + space::XL, self.r.y + space::L, TextSize::Xl, TITLE);
        }
        if let Some(s) = self.subtitle {
            let w = self.title.map(|t| text_w(&a, t, TextSize::Xl)).unwrap_or(0.0);
            text(&a, s, self.r.x + space::XL + w + space::L, self.r.y + space::L + 8.0, TextSize::S, TEXT_DIM);
        }
        if self.closable {
            let cb = Rect::new(self.r.right() - space::L - 28.0, self.r.y + space::M, 28.0, 28.0);
            let hint = text_w(&a, "Esc", TextSize::Xs);
            text(&a, "Esc", cb.x - hint - space::S, self.r.y + space::L - 2.0, TextSize::Xs, KEY);
            if button_kind(&a, cb, "x", true, ButtonKind::Secondary) {
                closed = true;
            }
            g.ui_rects.push(cb);
        }
        let top = if self.title.is_some() { self.r.y + HEADER } else { self.r.y + space::L };
        let content = Rect::new(
            self.r.x + space::XL,
            top,
            self.r.w - 2.0 * space::XL,
            (self.r.bottom() - space::L - top).max(0.0),
        );
        (content, closed)
    }
}

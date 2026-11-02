//! Drawing primitives: rounded rects, circles, hit-testing and surfaces.

use std::f32::consts::{FRAC_PI_2, PI};

use macroquad::prelude::*;

use crate::theme::{self, line, radius};

use super::metrics::ui_mouse;
use super::style::{BG2, EDGE, SHADE};

// Logo plaque colours for windows, tooltips, toasts and banners.
const PLAQUE_INK: u32 = 0x3a2418;
const PLAQUE_COPPER: u32 = 0xc4742f;
const PLAQUE_COPPER_HI: u32 = 0xf2b56a;
const PLAQUE_COPPER_LO: u32 = 0x7e4320;
/// Width of the copper band around a plaque's cream face.
const PLAQUE_BAND: f32 = 7.0;

const CORNERS: [f32; 4] = [PI, -FRAC_PI_2, 0.0, FRAC_PI_2];

fn corner_centers(r: Rect, rad: f32) -> [Vec2; 4] {
    [
        vec2(r.x + rad, r.y + rad),
        vec2(r.right() - rad, r.y + rad),
        vec2(r.right() - rad, r.bottom() - rad),
        vec2(r.x + rad, r.bottom() - rad),
    ]
}

/// Filled rounded rectangle with no overlapping parts, so translucent colours stay even.
pub fn rrect(r: Rect, rad: f32, col: Color) {
    let rad = rad.min(r.w / 2.0).min(r.h / 2.0).max(0.0);
    draw_rectangle(r.x, r.y + rad, r.w, r.h - 2.0 * rad, col);
    draw_rectangle(r.x + rad, r.y, r.w - 2.0 * rad, rad, col);
    draw_rectangle(r.x + rad, r.bottom() - rad, r.w - 2.0 * rad, rad, col);
    for (c, start) in corner_centers(r, rad).into_iter().zip(CORNERS) {
        for k in 0..12 {
            let a0 = start + k as f32 / 12.0 * FRAC_PI_2;
            let a1 = start + (k + 1) as f32 / 12.0 * FRAC_PI_2;
            draw_triangle(c, c + Vec2::from_angle(a0) * rad, c + Vec2::from_angle(a1) * rad, col);
        }
    }
}

pub fn rrect_lines(r: Rect, rad: f32, thick: f32, col: Color) {
    let rad = rad.min(r.w / 2.0).min(r.h / 2.0);
    draw_line(r.x + rad, r.y, r.right() - rad, r.y, thick, col);
    draw_line(r.x + rad, r.bottom(), r.right() - rad, r.bottom(), thick, col);
    draw_line(r.x, r.y + rad, r.x, r.bottom() - rad, thick, col);
    draw_line(r.right(), r.y + rad, r.right(), r.bottom() - rad, thick, col);
    for (c, start) in corner_centers(r, rad).into_iter().zip(CORNERS) {
        for k in 0..12 {
            let p0 = c + Vec2::from_angle(start + k as f32 / 12.0 * FRAC_PI_2) * rad;
            let p1 = c + Vec2::from_angle(start + (k + 1) as f32 / 12.0 * FRAC_PI_2) * rad;
            draw_line(p0.x, p0.y, p1.x, p1.y, thick, col);
        }
    }
}

/// Dashed rounded outline, used by the placement preview.
pub fn rrect_dashed_lines(r: Rect, rad: f32, thick: f32, col: Color) {
    let rad = rad.min(r.w / 2.0).min(r.h / 2.0).max(0.0);
    let mut path: Vec<Vec2> = Vec::new();
    let edge = |pts: &mut Vec<Vec2>, a: Vec2, b: Vec2| {
        let n = ((b - a).length() / 4.0).ceil().max(1.0) as usize;
        for k in 0..=n {
            pts.push(a.lerp(b, k as f32 / n as f32));
        }
    };
    edge(&mut path, vec2(r.x + rad, r.y), vec2(r.right() - rad, r.y));
    for k in 0..=12 {
        let a = -FRAC_PI_2 + k as f32 / 12.0 * FRAC_PI_2;
        path.push(vec2(r.right() - rad, r.y + rad) + Vec2::from_angle(a) * rad);
    }
    edge(&mut path, vec2(r.right(), r.y + rad), vec2(r.right(), r.bottom() - rad));
    for k in 0..=12 {
        let a = k as f32 / 12.0 * FRAC_PI_2;
        path.push(vec2(r.right() - rad, r.bottom() - rad) + Vec2::from_angle(a) * rad);
    }
    edge(&mut path, vec2(r.right() - rad, r.bottom()), vec2(r.x + rad, r.bottom()));
    for k in 0..=12 {
        let a = FRAC_PI_2 + k as f32 / 12.0 * FRAC_PI_2;
        path.push(vec2(r.x + rad, r.bottom() - rad) + Vec2::from_angle(a) * rad);
    }
    edge(&mut path, vec2(r.x, r.bottom() - rad), vec2(r.x, r.y + rad));
    for k in 0..=12 {
        let a = PI + k as f32 / 12.0 * FRAC_PI_2;
        path.push(vec2(r.x + rad, r.y + rad) + Vec2::from_angle(a) * rad);
    }
    let mut dist = 0.0;
    for w in path.windows(2) {
        let seg = (w[1] - w[0]).length();
        if (dist % 14.0) < 7.0 {
            draw_line(w[0].x, w[0].y, w[1].x, w[1].y, thick, col);
        }
        dist += seg;
    }
}

/// Smoother circles than macroquad's default 20-gon, for badges and rings.
pub fn circle(c: Vec2, r: f32, col: Color) {
    draw_poly(c.x, c.y, 48, r, 0.0, col);
}

pub fn circle_lines(c: Vec2, r: f32, thick: f32, col: Color) {
    draw_poly_lines(c.x, c.y, 48, r, 0.0, thick, col);
}

pub(super) fn hovered(r: Rect) -> bool {
    r.contains(ui_mouse())
}

pub(super) fn clicked(r: Rect) -> bool {
    is_mouse_button_pressed(MouseButton::Left) && hovered(r)
}

pub(super) fn ease_out(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    1.0 - (1.0 - t).powi(3)
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Elevation {
    /// Full window with a soft two-layer shadow.
    Panel,
    /// Inset card (research, recipe rows).
    Card,
    /// Floating tooltip.
    Tooltip,
    /// Toast pill.
    Toast,
    /// Centre-screen banner.
    Banner,
}

pub(super) fn surface(r: Rect, elev: Elevation) {
    surface_alpha(r, elev, 1.0);
}

pub(super) fn surface_alpha(r: Rect, elev: Elevation, alpha: f32) {
    let (rad, fill, border, border_w, shadow) = match elev {
        Elevation::Panel => (radius::LG, theme::alpha(theme::BG, 0.96), EDGE, line::BASE, true),
        Elevation::Card => (radius::MD, BG2, theme::alpha(theme::EDGE, 0.7), line::BASE, false),
        Elevation::Tooltip => (radius::MD, theme::alpha(theme::BG, 0.98), EDGE, line::BASE, true),
        Elevation::Toast => (radius::MD, theme::alpha(theme::BG, 0.94), EDGE, line::BASE, true),
        Elevation::Banner => (radius::LG, theme::alpha(theme::BG, 0.92), EDGE, line::BASE, true),
    };
    let mix_a = |c: Color, a: f32| theme::with_alpha(c, c.a * a * alpha);
    if shadow {
        // Soft drop shadow, kept subtle under the cream surface.
        rrect(Rect::new(r.x + 4.0, r.y + 6.0, r.w, r.h), rad, mix_a(SHADE, 0.22));
        rrect(Rect::new(r.x + 8.0, r.y + 10.0, r.w, r.h), rad, mix_a(SHADE, 0.14));
        // Hard wood under-edge, offset straight down so it reads as paper thickness.
        rrect(Rect::new(r.x, r.y + 3.0, r.w, r.h), rad, mix_a(theme::color(theme::TRIM_SHADOW), 1.0));
    }
    if elev != Elevation::Card {
        // Riveted copper plaque, matching the logo: dark ink outline, a copper
        // band lit along its top edge, then the cream face with corner rivets.
        let ink = theme::color(PLAQUE_INK);
        let band = PLAQUE_BAND;
        rrect(r, rad, mix_a(ink, 1.0));
        let b = Rect::new(r.x + 2.0, r.y + 2.0, r.w - 4.0, r.h - 4.0);
        rrect(b, (rad - 2.0).max(0.0), mix_a(theme::color(PLAQUE_COPPER_LO), 1.0));
        rrect(Rect::new(b.x, b.y, b.w, b.h - 2.0), (rad - 2.0).max(0.0), mix_a(theme::color(PLAQUE_COPPER), 1.0));
        rrect(Rect::new(b.x + 3.0, b.y + 1.0, b.w - 6.0, 2.0), 1.0, mix_a(theme::color(PLAQUE_COPPER_HI), 0.8));
        let face = Rect::new(r.x + band, r.y + band, r.w - 2.0 * band, r.h - 2.0 * band);
        rrect(Rect::new(face.x - 1.5, face.y - 1.5, face.w + 3.0, face.h + 3.0), (rad - band + 1.5).max(0.0), mix_a(ink, 0.85));
        rrect(face, (rad - band).max(0.0), mix_a(fill, 1.0));
        if r.w >= 120.0 && r.h >= 48.0 {
            let k = band * 0.5 + 1.0;
            for c in [
                vec2(r.x + k + 2.0, r.y + k + 2.0),
                vec2(r.x + r.w - k - 2.0, r.y + k + 2.0),
                vec2(r.x + k + 2.0, r.y + r.h - k - 2.0),
                vec2(r.x + r.w - k - 2.0, r.y + r.h - k - 2.0),
            ] {
                circle(c, 2.6, mix_a(ink, 0.9));
                circle(c - vec2(0.6, 0.6), 1.5, mix_a(theme::color(PLAQUE_COPPER_HI), 1.0));
            }
        }
        let _ = (border, border_w);
        return;
    }
    // Border as two filled rounded rects: an outer one in the border colour and
    // an inner one in the fill colour, inset by the border width. This gives a
    // clean smooth edge band instead of an anti-aliased outline line.
    rrect(r, rad, mix_a(border, 1.0));
    rrect(
        Rect::new(r.x + border_w, r.y + border_w, r.w - 2.0 * border_w, r.h - 2.0 * border_w),
        (rad - border_w).max(0.0),
        mix_a(fill, 1.0),
    );
}

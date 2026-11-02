use macroquad::prelude::*;

use crate::assets::Assets;
use crate::theme::{self, line};
use crate::ui::{rrect, rrect_dashed_lines, rrect_lines};
use crate::world::*;

use super::camera::Cam;
use super::entities::draw_kind;
use super::{PX, WIND, hash, rgba};

/// Where wires meet a pole: the post, level with its insulator crossbar.
const WIRE_ANCHOR: Vec2 = Vec2::new(24.0 / PX, (22.0 - 48.0) / PX);

pub(super) fn draw_wires(world: &World, cam: &Cam) {
    let col = theme::alpha(theme::INK, 0.85);
    for &(p, q) in &world.power.wires {
        let (a, b) = (p - Vec2::splat(0.5) + WIRE_ANCHOR, q - Vec2::splat(0.5) + WIRE_ANCHOR);
        if !cam.on_screen(a, 10.0) && !cam.on_screen(b, 10.0) {
            continue;
        }
        let sag = vec2(0.0, 0.18 + a.distance(b) * 0.04);
        let mut prev = cam.w2s(a);
        for i in 1..=10 {
            let u = i as f32 / 10.0;
            let pt = a.lerp(b, u) + sag * 4.0 * u * (1.0 - u);
            let s = cam.w2s(pt);
            draw_line(prev.x, prev.y, s.x, s.y, (cam.zoom * 0.03).max(1.0), col);
            prev = s;
        }
    }
}

pub(super) fn draw_clouds(a: &Assets, cam: &Cam, time: f32, day: f32) {
    if day < 0.05 {
        return;
    }
    const CELL: f32 = 26.0;
    let drift = WIND * time;
    let lo = ((cam.s2w(Vec2::ZERO) - drift) / CELL).floor().as_ivec2() - IVec2::ONE;
    let hi = ((cam.s2w(vec2(screen_width(), screen_height())) - drift) / CELL).floor().as_ivec2() + IVec2::ONE;
    for cy in lo.y..=hi.y {
        for cx in lo.x..=hi.x {
            let h = hash(cx * 13 + 5, cy * 29 - 3);
            if h.is_multiple_of(3) {
                continue;
            }
            let jitter = vec2((h % 100) as f32, ((h >> 8) % 100) as f32) / 100.0 * CELL;
            let c = vec2(cx as f32, cy as f32) * CELL + jitter + drift;
            let size = (10.0 + ((h >> 16) % 8) as f32) * cam.zoom;
            a.draw_centered("cloud", cam.w2s(c), Vec2::splat(size), (h % 7) as f32, rgba(WHITE, 0.16 * day));
        }
    }
}

pub(super) fn draw_pollution(world: &World, cam: &Cam) {
    let (lo, hi) = cam.visible();
    let chunk = 8;
    for cy in lo.y / chunk..=hi.y / chunk {
        for cx in lo.x / chunk..=hi.x / chunk {
            let p = vec2((cx * chunk) as f32, (cy * chunk) as f32);
            let v = world.pollution_at(p + Vec2::splat(1.0));
            if v > 0.5 {
                let s = cam.w2s(p);
                let w = chunk as f32 * cam.zoom;
                draw_rectangle(s.x, s.y, w, w, theme::alpha(theme::POLLUTION, (v / 200.0).min(0.5)));
            }
        }
    }
}

/// Placement preview for a building.
pub fn draw_ghost(a: &Assets, cam: &Cam, kind: Kind, pos: IVec2, dir: Dir, ok: bool, t: f32) {
    let z = cam.zoom;
    let s0 = cam.w2s(pos.as_vec2());
    let size = dims(kind, dir).as_vec2() * z;
    let pulse = 0.55 + 0.1 * (t * 4.0).sin();
    let tint = if ok {
        theme::rgba(0.85, 1.0, 0.85, pulse)
    } else {
        theme::rgba(1.0, 0.55, 0.5, pulse)
    };
    draw_kind(a, kind, dir, s0, z, tint);
    let edge = if ok {
        theme::alpha(theme::SAGE_300, 0.95)
    } else {
        theme::alpha(theme::TERRA_400, 0.95)
    };
    let r = Rect::new(s0.x, s0.y, size.x, size.y);
    rrect(r, radius_of(size), theme::with_alpha(edge, 0.10));
    rrect_dashed_lines(r, radius_of(size), line::STRONG, edge);
    if kind.directional() && !matches!(kind, Kind::Belt | Kind::Drill | Kind::Tunnel | Kind::Splitter) {
        let c = s0 + size * 0.5;
        let f = dir.vec().as_vec2();
        let side = vec2(-f.y, f.x);
        let tip = c + f * size.x.min(size.y) * 0.45;
        let base = tip - f * z * 0.3;
        draw_triangle(tip, base + side * z * 0.18, base - side * z * 0.18, theme::alpha(theme::CREAM, 0.9));
    }
    if kind == Kind::Pole {
        let r = POLE_SUPPLY as f32;
        let p = cam.w2s(pos.as_vec2() - Vec2::splat(r));
        let w = (2.0 * r + 1.0) * z;
        let range = Rect::new(p.x, p.y, w, w);
        rrect(range, z * 0.5, theme::alpha(theme::HONEY, 0.10));
        rrect_lines(range, z * 0.5, line::BASE, theme::alpha(theme::HONEY, 0.45));
    }
    if kind == Kind::Tunnel {
        for d in 1..=TUNNEL_MAX {
            let p = cam.w2s((pos + dir.vec() * d).as_vec2() + Vec2::splat(0.5));
            draw_circle(p.x, p.y, z * 0.06, theme::alpha(theme::CREAM, 0.35));
        }
    }
}

fn radius_of(size: Vec2) -> f32 {
    (size.min_element() * 0.12).clamp(4.0, 12.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preview_corners_stay_small() {
        assert_eq!(radius_of(Vec2::splat(20.0)), 4.0);
        assert_eq!(radius_of(Vec2::splat(200.0)), 12.0);
    }
}

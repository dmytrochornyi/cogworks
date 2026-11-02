use std::f32::consts::FRAC_PI_2;

use macroquad::prelude::*;

use crate::assets::Assets;
use crate::theme;
use crate::world::*;

use super::camera::Cam;
use super::{PX, lantern_seed, rgba, sway};

/// Lantern hook attachment on a pole, in tiles from its top-left tile. This is
/// the swing pivot; the lantern body hangs straight down from it.
const LANTERN_HOOK: Vec2 = Vec2::new(46.0 / PX, (21.0 - 48.0) / PX);
/// How far the lantern body centre hangs below the hook, in tiles.
const LANTERN_DROP: f32 = 14.0 / PX;

pub(super) fn light(a: &Assets, cam: &Cam, pos: Vec2, radius: f32, col: Color) {
    if !cam.on_screen(pos, radius) {
        return;
    }
    let size = Vec2::splat(radius * 2.0 * cam.zoom);
    a.draw_centered("glow", cam.w2s(pos), size, 0.0, theme::rgba(col.r * 0.5, col.g * 0.5, col.b * 0.5, col.a));
}

pub(super) fn draw_lights(a: &Assets, world: &World, cam: &Cam, t: f32, glow: f32) {
    let warm = theme::color(theme::LAMP);
    for e in world.entities.iter().flatten() {
        let p = e.pos.as_vec2();
        // Lights breathe slowly (0.85..1.0) with a very small flicker on top.
        let breathe = 0.925 + 0.075 * (t * 1.1 + p.x * 2.3).sin();
        let flick = breathe + 0.02 * (t * 19.0 + p.y * 5.0).sin();
        let px = |x: f32, y: f32| p + vec2(x, y) / PX;
        match e.kind {
            Kind::Pole => {
                // The lantern's light follows its swing about the hook.
                let sw = sway(t, lantern_seed(p), 0.06, 3.5);
                let center = p + LANTERN_HOOK + Vec2::from_angle(FRAC_PI_2 + sw) * LANTERN_DROP;
                // A generous, clearly visible warm peach pool from golden hour
                // on: a 0.3 floor once glow begins, rising to full at night.
                let strength = if glow > 0.0 { (0.3 + glow * 0.7).min(1.0) * flick } else { 0.0 };
                light(a, cam, center, 4.2, rgba(warm, strength));
            }
            Kind::Smelter if e.active => light(a, cam, px(64.0, 104.0), 2.6, theme::rgba(1.0, 0.5, 0.2, (0.3 + glow * 0.7) * flick)),
            Kind::Generator if e.active => light(a, cam, px(42.0, 108.0), 2.0, theme::rgba(1.0, 0.7, 0.35, 0.15 + glow * 0.5)),
            Kind::Lab if e.active => light(a, cam, px(95.0, 92.0), 3.0, theme::rgba(0.5, 0.95, 0.9, 0.25 + glow * 0.6)),
            Kind::Assembler if e.active => light(a, cam, px(97.0, 68.0), 1.8, theme::rgba(0.9, 0.8, 0.5, glow * 0.5)),
            Kind::Sentry if e.active => light(a, cam, px(64.0, 46.0), 1.6, theme::rgba(1.0, 0.85, 0.5, 0.5 * flick)),
            Kind::GrandEngine => light(a, cam, px(128.0, 97.0), 5.0, theme::rgba(0.75, 0.5, 1.0, 0.3 + glow * 0.6)),
            Kind::Hive => light(a, cam, px(96.0, 96.0), 3.0, theme::rgba(0.95, 0.4, 0.7, 0.15 + glow * 0.45)),
            _ => {}
        }
    }
    if world.player.alive() {
        light(a, cam, world.player.pos, 4.5, rgba(warm, glow * 0.75));
    }
    for s in &world.shots {
        light(a, cam, s.from, 1.4, theme::rgba(1.0, 0.85, 0.5, 0.6));
    }
}

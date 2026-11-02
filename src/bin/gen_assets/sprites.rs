//! Every sprite in the game, painted from code.

use std::f32::consts::{PI, TAU};

use macroquad::math::{Vec2, vec2};

use super::paint::*;
use crate::theme;

/// Outline width used by the large machine sprites, in native pixels.
const OW: f32 = 2.5;
/// Target outline thickness on screen at the default zoom (52 px per tile).
const OUTLINE_PX: f32 = 2.2;
const SHADOW: Vec2 = Vec2::new(5.0, 7.0);

/// Fixed lighting band for machine top faces and front walls.
const TOP_HI: f32 = 1.06;
const TOP_LO: f32 = 0.94;
const SIDE_DARK: f32 = 0.68;

/// Converts the on-screen target outline into native sprite pixels for a sprite
/// that is `native` pixels wide but drawn at `drawn` pixels.
const fn ow(native: f32, drawn: f32) -> f32 {
    let w = OUTLINE_PX * native / drawn;
    if w < 1.0 {
        1.0
    } else if w > 5.5 {
        5.5
    } else {
        w
    }
}

// Default on-screen sizes at zoom 52 (see render.rs), so the small sprites get
// the same visual outline weight as the machines.
const OW_ITEM: f32 = ow(48.0, 52.0 * 0.4);
const OW_ICON: f32 = ow(32.0, 22.0);
const OW_HAND: f32 = ow(24.0, 24.0 * 52.0 / 64.0);
const OW_PORT: f32 = ow(24.0, 52.0 * 0.42);
const OW_PIPE: f32 = ow(32.0, 32.0 * 52.0 / 64.0);
const OW_OVERLAY: f32 = ow(96.0, 96.0 * 52.0 / 64.0);
const OW_GEAR: f32 = ow(96.0, 80.0 * 52.0 / 64.0);
const OW_LIMB: f32 = ow(64.0, 52.0);
const OW_CHUTE: f32 = ow(32.0, 52.0 * 0.5);

fn v(x: f32, y: f32) -> Vec2 {
    vec2(x, y)
}

/// Subtle paper-like grain over a region.
fn grain(cv: &mut Canvas, sdf: impl Fn(Vec2) -> f32 + Copy, seed: u32, amt: f32) {
    cv.fill(sdf, move |p| {
        let n = fbm(p, 64.0, seed);
        if n > 0.5 { with_alpha(WHITE, (n - 0.5) * amt) } else { with_alpha(INK, (0.5 - n) * amt) }
    });
}

/// A machine block seen in 3/4 view: a darker front wall under a lit top face.
/// Returns the top face as (x, y, w, h).
///
/// Material rules:
/// - `top` is the plain material colour. The top face is always lit inside
///   `[TOP_LO, TOP_HI]` (0.94..1.06), so no machine is brighter than its neighbours.
/// - `side` is the front wall and should normally be `lit(material, SIDE_DARK)` (0.68).
/// - Metal uses STEEL/STEEL_D; wood WOOD/WOOD_DARK; brick TERRA/RUST;
///   stone WALL_STONE/REFINERY_STONE; painted bodies use a palette accent with
///   a side about 32% darker.
#[allow(clippy::too_many_arguments)]
fn block(cv: &mut Canvas, x: f32, y: f32, w: f32, h: f32, depth: f32, r: f32, top: C, side: C) -> (f32, f32, f32, f32) {
    cv.shadow(move |p| rbox(p, x, y, w, h, r), SHADOW, 7.0, 0.35);
    cv.solid(move |p| rbox(p, x, y, w, h, r), side, OW);
    let th = h - depth;
    cv.shape(
        move |p| rbox(p, x, y, w, th, r),
        move |p| mix(lit(top, TOP_HI), lit(top, TOP_LO), (p.y - y) / th),
        OW,
    );
    grain(cv, move |p| rbox(p, x + 3.0, y + 3.0, w - 6.0, th - 6.0, r), 11, 0.08);
    cv.fill(move |p| rbox(p, x + OW + 3.0, y + OW + 1.5, w - 2.0 * OW - 6.0, 2.5, 1.2), |_| with_alpha(WHITE, 0.3));
    (x, y, w, th)
}

fn side_of(top: C) -> C {
    lit(top, SIDE_DARK)
}

fn rivet(cv: &mut Canvas, c: Vec2, r: f32) {
    cv.solid(move |p| circle(p, c, r), STEEL, 1.2);
    cv.fill(move |p| circle(p, c - v(r * 0.3, r * 0.3), r * 0.35), |_| with_alpha(WHITE, 0.6));
}

fn ground_shadow(cv: &mut Canvas, c: Vec2, r: Vec2, a: f32) {
    cv.shadow(move |p| ellipse(p, c, r), Vec2::ZERO, 5.0, a);
}

// ---------------------------------------------------------------- terrain

/// One 256x256 meadow sheet whose noise and blades are periodic with period
/// 256. `draw_ground` slices it into sixteen `grass_{i}_{j}` tiles; because the
/// sheet wraps, neighbouring tiles always continue each other (no visible grid).
pub fn ground_sheet() -> Canvas {
    let mut cv = Canvas::new(256, 256);
    let a = GRASS_A;
    let b = GRASS_B;
    let warm = GRASS_WARM;
    // Soft pastel meadow greens with gentle large-scale variation (period 256).
    cv.fill(|_| -1.0, move |p| {
        let n = fbm(p, 256.0, 100);
        let m = fbm(p + v(61.0, 29.0), 256.0, 200);
        mix(mix(b, a, n), warm, (m - 0.55).max(0.0) * 1.6)
    });
    // Little blades, wrapped across the whole sheet so they tile seamlessly.
    for i in 0..288u32 {
        let base = v(rand(i, 7) * 256.0, rand(i + 50, 7) * 256.0);
        let lean = (rand(i + 99, 7) - 0.5) * 4.0;
        let len = 3.0 + rand(i + 7, 7) * 4.0;
        let col = if i % 3 == 0 { BLADE_LIGHT } else { BLADE_DARK };
        for ox in [-256.0, 0.0, 256.0] {
            for oy in [-256.0, 0.0, 256.0] {
                let b0 = base + v(ox, oy);
                let tip = b0 + v(lean, -len);
                cv.fill(move |p| capsule(p, b0, tip, 0.8), move |_| with_alpha(col, 0.7));
            }
        }
    }
    cv
}

pub fn decor(kind: u32) -> Canvas {
    let mut cv = Canvas::new(32, 32);
    let c = v(16.0, 16.0);
    let flower = |cv: &mut Canvas, petal: C, center: C, n: u32, r: f32| {
        cv.shadow(move |p| circle(p, c, r * 1.8), v(1.5, 2.0), 2.0, 0.25);
        for k in 0..n {
            let a = k as f32 / n as f32 * TAU + 0.3;
            let pc = c + Vec2::from_angle(a) * r;
            cv.solid(move |p| circle(p, pc, r * 0.85), petal, 1.2);
        }
        cv.solid(move |p| circle(p, c, r * 0.6), center, 1.0);
    };
    match kind {
        0 => flower(&mut cv, FLOWER_PINK, FLOWER_BUTTER, 5, 4.0),
        1 => flower(&mut cv, FLOWER_BUTTER, HONEY, 5, 3.6),
        2 => {
            for (k, d) in [v(-5.0, 2.0), v(4.0, -3.0), v(3.0, 6.0)].into_iter().enumerate() {
                let pc = c + d;
                let col = if k == 1 { FLOWER_BUTTER } else { FLOWER_LILAC };
                cv.solid(move |p| circle(p, pc, 2.8), col, 1.0);
                cv.fill(move |p| circle(p, pc, 1.0), |_| HONEY);
            }
        }
        3 => {
            for (d, r) in [(v(-4.0, 2.0), v(5.0, 3.5)), (v(5.0, -1.0), v(3.5, 2.8)), (v(1.0, 6.0), v(2.5, 2.0))] {
                let pc = c + d;
                cv.shadow(move |p| ellipse(p, pc, r), v(1.0, 1.5), 1.5, 0.3);
                cv.shape(move |p| ellipse(p, pc, r), move |p| mix(PEBBLE_LIGHT, PEBBLE_DARK, (p.y - pc.y + r.y) / (2.0 * r.y)), 1.2);
            }
        }
        4 => {
            for k in 0..5 {
                let x = 9.0 + k as f32 * 3.5;
                let tip = v(x + (k as f32 - 2.0) * 2.0, 6.0 + (k % 2) as f32 * 4.0);
                let col = if k % 2 == 0 { BLADE_DARK } else { STEM_LIGHT };
                cv.fill(move |p| capsule(p, v(x, 26.0), tip, 1.4), move |_| col);
            }
        }
        5 => {
            cv.shadow(|p| circle(p, v(16.0, 18.0), 7.0), v(1.5, 2.0), 2.0, 0.3);
            cv.solid(|p| rbox(p, 13.5, 15.0, 5.0, 10.0, 2.0), CREAM, 1.2);
            cv.shape(|p| ellipse(p, v(16.0, 14.0), v(9.0, 6.0)), |p| mix(PEACH, TERRA, (p.y - 8.0) / 12.0), 1.2);
            for d in [v(-3.0, -2.0), v(3.0, -1.0), v(0.0, 2.0)] {
                cv.fill(move |p| circle(p, v(16.0, 14.0) + d, 1.3), |_| CREAM);
            }
        }
        6 => {
            for k in 0..3 {
                let a = k as f32 * TAU / 3.0 - PI / 2.0;
                let pc = c + Vec2::from_angle(a) * 3.5;
                cv.solid(move |p| circle(p, pc, 3.5), MOSS, 1.0);
            }
            cv.fill(|p| capsule(p, c, v(18.0, 26.0), 0.8), |_| MOSS_DARK);
        }
        _ => {
            let leaf = move |p: Vec2| {
                let q = (p - c).rotate(Vec2::from_angle(-0.6));
                ellipse(q, Vec2::ZERO, v(8.0, 4.0))
            };
            cv.shadow(leaf, v(1.5, 2.0), 2.0, 0.25);
            cv.solid(leaf, MUSTARD, 1.2);
            cv.fill(|p| capsule(p, c - Vec2::from_angle(0.6) * 6.0, c + Vec2::from_angle(0.6) * 6.0, 0.6), |_| with_alpha(RUST, 0.8));
        }
    }
    cv
}

fn rock(cv: &mut Canvas, c: Vec2, r: Vec2, body: C, facet: C, ow: f32) {
    cv.shadow(move |p| ellipse(p, c, r), v(2.5, 3.5), 3.0, 0.35);
    cv.shape(move |p| ellipse(p, c, r), move |p| mix(lit(body, 1.1), lit(body, 0.8), (p.y - c.y + r.y) / (2.0 * r.y)), ow);
    let f = c - r * 0.25;
    let fr = r * 0.45;
    cv.fill(move |p| ellipse(p, f, fr), move |_| with_alpha(facet, 0.9));
    cv.fill(move |p| circle(p, f - fr * 0.3, fr.min_element() * 0.35), |_| with_alpha(WHITE, 0.45));
}

/// A single angular, faceted rock chunk: a convex polygon shaded from a lit
/// top to a dark base, with one lighter planar face across its two top vertices.
fn angular_chunk(cv: &mut Canvas, pts: &[Vec2], body: C, facet: C) {
    let (mut miny, mut maxy) = (f32::MAX, f32::MIN);
    for p in pts {
        miny = miny.min(p.y);
        maxy = maxy.max(p.y);
    }
    let span = (maxy - miny).max(1.0);
    let sdf = move |p: Vec2| poly(p, pts);
    cv.shadow(sdf, v(2.5, 3.0), 3.0, 0.20);
    cv.shape(sdf, move |p| mix(lit(body, 1.12), lit(body, 0.88), ((p.y - miny) / span).clamp(0.0, 1.0)), OW);
    let a = pts[0];
    let b = pts[1];
    let c = (a + b) * 0.5 + v(0.0, 2.0);
    cv.fill(move |p| poly(p, &[a, b, c]), |_| with_alpha(facet, 0.85));
}

/// Blue-grey angular chunks with a silver glint on each top facet.
fn iron_ore(cv: &mut Canvas, seed: u32) {
    let mut glints: Vec<Vec2> = Vec::new();
    for k in 0..3u32 {
        let c = v(16.0 + rand(k, seed) * 32.0, 16.0 + rand(k + 9, seed) * 32.0);
        let n = 5 + (rand(k + 20, seed) * 2.999) as u32;
        let r = 7.0 + rand(k + 30, seed) * 5.0;
        let pts: Vec<Vec2> = (0..n)
            .map(|m| {
                let a = m as f32 / n as f32 * TAU + rand(k * 7 + m + 1, seed) * 0.6;
                let rr = r * (0.68 + 0.55 * rand(k * 13 + m + 3, seed));
                c + Vec2::from_angle(a) * rr
            })
            .collect();
        let top = pts.iter().fold(pts[0], |acc, p| if p.y < acc.y { *p } else { acc });
        angular_chunk(cv, &pts, ORE_IRON, ORE_IRON_FACET);
        glints.push(top);
    }
    for (k, g) in glints.iter().take(2).enumerate() {
        let off = v(rand(k as u32 * 3, seed) * 2.0 - 1.0, -rand(k as u32 * 5 + 1, seed) * 1.5);
        cv.fill(move |p| circle(p, *g + off, 1.2), |_| with_alpha(ORE_IRON_GLINT, 0.95));
    }
}

/// Warm orange rounded nuggets with teal patina spots.
fn copper_ore(cv: &mut Canvas, seed: u32) {
    let spots: Vec<(Vec2, Vec2)> = (0..4u32)
        .map(|i| {
            let c = v(14.0 + rand(i, seed) * 36.0, 14.0 + rand(i + 9, seed) * 36.0);
            let s = 6.0 + rand(i + 20, seed) * 5.0;
            (c, v(s * 1.15, s))
        })
        .collect();
    for (c, r) in &spots {
        rock(cv, *c, *r, ORE_COPPER, ORE_COPPER_FACET, OW);
    }
    for (i, (c, r)) in spots.iter().enumerate() {
        let pc = *c + v((rand(i as u32 + 40, seed) - 0.5) * r.x * 0.9, (rand(i as u32 + 50, seed) - 0.5) * r.y * 0.6);
        cv.fill(move |p| circle(p, pc, 1.4 + rand(i as u32 + 60, seed) * 1.0), |_| with_alpha(ORE_COPPER_PATINA, 0.75));
    }
}

/// Glossy black lumps with a faint purple sheen highlight.
fn coal_ore(cv: &mut Canvas, seed: u32) {
    for k in 0..3u32 {
        let c = v(16.0 + rand(k, seed) * 32.0, 18.0 + rand(k + 9, seed) * 28.0);
        let r = 7.0 + rand(k + 20, seed) * 4.0;
        let a = c + v(-r * 0.5, 0.0);
        let b = c + v(r * 0.5, -r * 0.15);
        let d = c + v(0.0, r * 0.5);
        let lump = move |p: Vec2| circle(p, a, r * 0.7).min(circle(p, b, r * 0.65)).min(circle(p, d, r * 0.6));
        cv.shadow(lump, v(2.0, 3.0), 3.0, 0.35);
        cv.shape(lump, move |p| mix(lit(ORE_COAL, 1.25), ORE_COAL, ((p.y - (c.y - r)) / (2.0 * r)).clamp(0.0, 1.0)), OW);
        let s0 = c + v(-r * 0.4, -r * 0.45);
        let s1 = c + v(r * 0.3, -r * 0.2);
        cv.fill(move |p| capsule(p, s0, s1, 1.6), |_| with_alpha(ORE_COAL_SHEEN, 0.55));
    }
}

/// Pale sandy smooth rounded boulders.
fn stone_ore(cv: &mut Canvas, seed: u32) {
    for k in 0..3u32 {
        let c = v(16.0 + rand(k, seed) * 32.0, 18.0 + rand(k + 9, seed) * 28.0);
        let r = 9.0 + rand(k + 20, seed) * 4.0;
        rock(cv, c, v(r, r * 0.9), ORE_STONE, ORE_STONE_FACET, OW);
    }
}

/// Lilac crystal spikes, refined with a thin bright ridge on the centre shard.
fn quartz_ore(cv: &mut Canvas, seed: u32) {
    let spots: Vec<(Vec2, Vec2)> = (0..4u32)
        .map(|i| {
            let c = v(14.0 + rand(i, seed) * 36.0, 14.0 + rand(i + 9, seed) * 36.0);
            let s = 6.0 + rand(i + 20, seed) * 6.0;
            (c, v(s * 1.2, s))
        })
        .collect();
    for (c, r) in spots {
        for k in 0..3 {
            let a = -PI / 2.0 + (k as f32 - 1.0) * 0.45;
            let len = r.y * (1.6 - 0.25 * (k as f32 - 1.0).abs());
            let dir = Vec2::from_angle(a);
            let side = v(-dir.y, dir.x) * r.x * 0.3;
            let base = c + v(0.0, r.y * 0.5);
            let pts = [base - side, base - side + dir * len * 0.8, base + dir * len, base + side + dir * len * 0.8, base + side];
            cv.shadow(move |p| poly(p, &pts), v(2.0, 3.0), 2.0, 0.3);
            cv.shape(move |p| poly(p, &pts), move |p| mix(QUARTZ_LIGHT, QUARTZ_SHADE, (p.x - base.x + 6.0) / 12.0), 1.8);
            if k == 1 {
                cv.fill(move |p| capsule(p, base + dir * len * 0.2, base + dir * len * 0.9, 1.0), |_| with_alpha(QUARTZ_LIGHT, 0.7));
            }
        }
    }
}

pub fn ore(name: &str, variant: u32) -> Canvas {
    let mut cv = Canvas::new(64, 64);
    let seed = variant * 31 + name.len() as u32;
    match name {
        "iron" => iron_ore(&mut cv, seed),
        "copper" => copper_ore(&mut cv, seed),
        "coal" => coal_ore(&mut cv, seed),
        "stone" => stone_ore(&mut cv, seed),
        _ => quartz_ore(&mut cv, seed),
    }
    cv
}

pub fn tar(variant: u32) -> Canvas {
    let mut cv = Canvas::new(64, 64);
    let blobs: Vec<(Vec2, f32)> = (0..4)
        .map(|i| (v(20.0 + rand(i, 70 + variant) * 24.0, 20.0 + rand(i + 4, 70 + variant) * 24.0), 10.0 + rand(i + 8, variant) * 8.0))
        .collect();
    let b2 = &blobs;
    let sdf = move |p: Vec2| b2.iter().map(|(c, r)| circle(p, *c, *r)).fold(f32::MAX, f32::min);
    cv.shape(sdf, |p| mix(TAR_LIGHT, TAR_DARK, p.y / 64.0), 2.0);
    let (c, r) = blobs[0];
    cv.fill(move |p| ellipse(p, c - v(r * 0.3, r * 0.4), v(r * 0.5, r * 0.18)), |_| with_alpha(TAR_SHEEN, 0.5));
    let bubble = blobs[1].0;
    cv.fill(move |p| ring(p, bubble, 3.0, 1.2), |_| with_alpha(TAR_BUBBLE, 0.6));
    cv
}

// ---------------------------------------------------------------- logistics

const BAND: C = BELT_BAND;
const RIDGE: C = BELT_RIDGE;

fn rails(cv: &mut Canvas, sdf_rail: impl Fn(Vec2) -> f32 + Copy) {
    cv.shape(sdf_rail, |p| mix(BELT_RAIL_HI, BELT_RAIL_LO, (p.y % 64.0) / 64.0), 2.6);
}

pub fn belt(frame: u32) -> Canvas {
    let mut cv = Canvas::new(64, 64);
    cv.solid(|p| rbox(p, -4.0, 9.0, 72.0, 46.0, 0.0), BAND, 0.0);
    let shift = frame as f32 * 2.0;
    cv.fill(|p| rbox(p, -4.0, 11.0, 72.0, 42.0, 0.0), move |p| {
        let s = ((p.x - shift) / 16.0).rem_euclid(1.0);
        if s < 0.2 { RIDGE } else { with_alpha(RIDGE, 0.0) }
    });
    cv.fill(|p| rbox(p, -4.0, 31.0, 72.0, 2.0, 0.0), |_| with_alpha(INK, 0.35));
    for y in [4.0, 53.0] {
        rails(&mut cv, move |p| rbox(p, -4.0, y, 72.0, 7.0, 2.0));
        for x in [8.0, 40.0] {
            let c = v(x, y + 3.5);
            cv.fill(move |p| circle(p, c, 2.2), |_| with_alpha(INK, 0.85));
            cv.fill(move |p| circle(p, c - v(0.5, 0.5), 1.1), |_| with_alpha(CREAM, 0.9));
        }
    }
    cv
}

/// A belt curving in from the bottom edge and out of the right edge.
pub fn belt_curve(frame: u32) -> Canvas {
    let mut cv = Canvas::new(64, 64);
    let corner = v(64.0, 64.0);
    let shift = frame as f32 * 2.0;
    cv.fill(move |p| { let d = p.distance(corner); (d - 55.0).max(9.0 - d) }, |_| BAND);
    cv.fill(move |p| { let d = p.distance(corner); (d - 53.0).max(11.0 - d) }, move |p| {
        let q = p - corner;
        let a = q.y.atan2(q.x).rem_euclid(TAU);
        let s = ((a - PI) * 32.0 - shift) / 16.0;
        if s.rem_euclid(1.0) < 0.2 { RIDGE } else { with_alpha(RIDGE, 0.0) }
    });
    for (r0, r1) in [(53.0, 60.0), (4.0, 11.0)] {
        rails(&mut cv, move |p| { let d = p.distance(corner); (d - r1).max(r0 - d) });
    }
    cv
}

/// Tunnel entrance facing east: belt runs in from the left into a stone arch.
pub fn tunnel() -> Canvas {
    let mut cv = belt(0);
    let stone = TUNNEL_STONE;
    block(&mut cv, 26.0, 2.0, 36.0, 60.0, 8.0, 8.0, stone, side_of(stone));
    cv.solid(|p| rbox(p, 24.0, 14.0, 18.0, 36.0, 9.0), TUNNEL_MOUTH, OW);
    for y in [10.0, 30.0, 50.0] {
        cv.fill(move |p| rbox(p, 46.0, y - 1.0, 14.0, 2.0, 1.0), |_| with_alpha(INK, 0.4));
    }
    cv.fill(|p| circle(p, v(52.0, 32.0), 3.0), |_| HONEY);
    cv
}

/// Splitter facing north, two tiles wide.
pub fn splitter() -> Canvas {
    let mut cv = Canvas::new(128, 64);
    for x in [9.0, 73.0] {
        cv.solid(move |p| rbox(p, x, -4.0, 46.0, 72.0, 0.0), BAND, 0.0);
    }
    let (x, y, w, h) = block(&mut cv, 4.0, 16.0, 120.0, 34.0, 8.0, 8.0, MUSTARD, RUST);
    cv.solid(move |p| rbox(p, x + 54.0, y + 4.0, 12.0, h - 8.0, 4.0), BROWN_D, 1.5);
    for cx in [30.0, 98.0] {
        cv.solid(move |p| circle(p, v(cx, y + h / 2.0), 5.0), CREAM, 1.5);
        cv.fill(move |p| circle(p, v(cx, y + h / 2.0), 2.5), |_| TERRA);
    }
    let _ = w;
    cv
}

pub fn arm_base() -> Canvas {
    let mut cv = Canvas::new(64, 64);
    let c = v(32.0, 32.0);
    cv.shadow(move |p| circle(p, c, 18.0), v(3.0, 4.0), 5.0, 0.35);
    cv.shape(move |p| circle(p, c, 18.0), move |p| mix(lit(WOOD_DARK, 1.15), lit(WOOD_DARK, 0.7), (p.y - 14.0) / 36.0), OW);
    cv.solid(move |p| circle(p, c, 9.0), BRASS, 1.5);
    for k in 0..6 {
        let rc = c + Vec2::from_angle(k as f32 * TAU / 6.0) * 13.5;
        cv.solid(move |p| circle(p, rc, 1.9), BRASS, 1.0);
        cv.fill(move |p| circle(p, rc - v(0.5, 0.5), 0.8), |_| with_alpha(WHITE, 0.6));
    }
    cv
}

pub fn arm_limb() -> Canvas {
    let mut cv = Canvas::new(64, 16);
    cv.shape(|p| capsule(p, v(8.0, 8.0), v(56.0, 8.0), 5.0), |p| mix(lit(BRASS, 1.1), lit(BRASS, 0.65), (p.y - 3.0) / 10.0), OW_LIMB);
    cv.solid(|p| circle(p, v(8.0, 8.0), 3.0), WOOD_DARK, OW_LIMB * 0.5);
    cv
}

pub fn arm_hand() -> Canvas {
    let mut cv = Canvas::new(24, 24);
    for s in [-1.0, 1.0] {
        cv.solid(move |p| capsule(p, v(8.0, 12.0 + s * 3.0), v(19.0, 12.0 + s * 6.0), 2.4), BRASS, OW_HAND * 0.7);
    }
    cv.solid(|p| circle(p, v(9.0, 12.0), 5.0), WOOD_DARK, OW_HAND);
    cv
}

pub fn chest() -> Canvas {
    let mut cv = Canvas::new(64, 64);
    let wood = WOOD_DARK;
    let (x, y, w, h) = block(&mut cv, 7.0, 8.0, 50.0, 50.0, 11.0, 5.0, wood, side_of(wood));
    for k in 1..4 {
        let ly = y + h * k as f32 / 4.0;
        cv.fill(move |p| rbox(p, x + 4.0, ly - 0.8, w - 8.0, 1.6, 0.8), |_| with_alpha(INK, 0.45));
    }
    // Brass corner brackets.
    for (cx, cy) in [(x + 5.0, y + 5.0), (x + w - 5.0, y + 5.0), (x + 5.0, y + h - 4.0), (x + w - 5.0, y + h - 4.0)] {
        cv.solid(move |p| rbox(p, cx - 3.0, cy - 3.0, 7.0, 7.0, 2.0), BRASS, 1.2);
        cv.fill(move |p| circle(p, v(cx, cy), 1.4), |_| with_alpha(WHITE, 0.6));
    }
    cv.solid(|p| rbox(p, 28.0, 46.0, 8.0, 8.0, 2.0), BRASS, 1.5);
    cv
}

pub fn pipe_hub() -> Canvas {
    let mut cv = Canvas::new(64, 64);
    let c = v(32.0, 32.0);
    cv.shadow(move |p| circle(p, c, 13.0), v(2.0, 3.0), 3.0, 0.3);
    cv.shape(move |p| circle(p, c, 13.0), move |p| mix(lit(COPPER, 1.15), COPPER_DARK, (p.y - 19.0) / 26.0), OW);
    // Brass flange ring.
    cv.solid(move |p| ring(p, c, 10.0, 2.0), BRASS, 1.0);
    for k in 0..4 {
        let rc = c + Vec2::from_angle(k as f32 * TAU / 4.0 + PI / 4.0) * 8.5;
        cv.solid(move |p| circle(p, rc, 1.7), BRASS, 1.0);
        cv.fill(move |p| circle(p, rc - v(0.5, 0.5), 0.7), |_| with_alpha(WHITE, 0.6));
    }
    cv
}

/// Half a pipe, from the hub centre towards the east edge.
pub fn pipe_seg() -> Canvas {
    let mut cv = Canvas::new(32, 24);
    cv.fill(|p| rbox(p, -2.0, 3.0, 36.0, 18.0, 0.0), |p| mix(lit(COPPER, 1.2), COPPER_DARK, (p.y - 3.0) / 18.0));
    for y in [3.0, 19.5] {
        cv.fill(move |p| rbox(p, -2.0, y, 36.0, OW_PIPE, 0.0), |_| BRASS);
    }
    cv.fill(|p| rbox(p, 22.0, 1.0, 6.0, 22.0, 2.0), |_| BRASS);
    cv.fill(|p| rbox(p, 22.0, 1.0, 6.0, 22.0, 2.0).max(-rbox(p, 23.5, 2.5, 3.0, 19.0, 1.0)), |_| TRIM_DARK);
    cv
}

pub fn port(input: bool) -> Canvas {
    let mut cv = Canvas::new(24, 24);
    let col = if input { BRASS } else { TRIM };
    cv.solid(|p| circle(p, v(12.0, 12.0), 8.0), col, OW_PORT);
    cv.solid(|p| rbox(p, 7.0, 10.5, 10.0, 3.0, 1.5), CREAM, OW_PORT * 0.5);
    cv
}

// ---------------------------------------------------------------- production

/// Signed distance to a curved pagoda roof: a straight ridge along the top,
/// sides that taper in toward the ridge, and an eave whose corners curl up.
fn roof_sdf(p: Vec2, x: f32, y: f32, w: f32, th: f32, r: f32) -> f32 {
    // Ridge is narrower than the eave (r on each side), so the roof flares.
    let x0 = x + r;
    let x1 = x + w - r;
    let y0 = y + r;
    // Eave line: corners lift up by `r`, the middle sags gently downward.
    let cu = ((p.x - (x + w / 2.0)) / (w / 2.0)).clamp(-1.0, 1.0);
    let eave = y + th - r * (cu * cu);
    // Signed distance to the two half-plane strips the roof is carved from.
    let sx = (x0 - p.x).max(p.x - x1);
    let sy = (y0 - p.y).max(p.y - eave);
    sx.max(sy)
}

/// Subtle horizontal scallop rows across the roof, like overlapping shingles.
fn shingles(cv: &mut Canvas, x: f32, y: f32, w: f32, th: f32) {
    let n = ((th / 11.0).round() as i32).clamp(3, 6);
    for k in 1..n {
        let ry = y + th * k as f32 / n as f32;
        let off = if k % 2 == 0 { 3.0 } else { 0.0 };
        // Little downward arcs, staggered so the rows read as shingle courses.
        cv.fill(
            move |p| {
                let seg = ((p.x - x + off) / 6.5).floor();
                let cx = x + off + seg * 6.5 + 3.2;
                circle(p, v(cx, ry), 3.2).max(ry - p.y)
            },
            move |_| with_alpha(ROOF_SHADE, 0.30),
        );
        cv.fill(move |p| capsule(p, v(x + 4.0, ry), v(x + w - 4.0, ry), 0.6), move |_| with_alpha(ROOF_SHADE, 0.2));
    }
}

/// A curved market-stall roof painted over a region `(x, y, w, th)`: soft
/// lacquer rose with a verdigris ridge cap, upturned eave corners, scalloped
/// shingle rows and a soft eave shadow cast onto whatever sits below it.
fn roof(cv: &mut Canvas, x: f32, y: f32, w: f32, th: f32, r: f32) {
    // Soft shadow cast by the eaves onto the surface below (the front wall).
    cv.fill(
        move |p| rbox(p, x, y + th, w, 9.0, r * 0.5),
        move |p| with_alpha(INK, 0.30 * (1.0 - ((p.y - (y + th)) / 9.0).clamp(0.0, 1.0))),
    );
    // Roof body: top-left lit, bottom-right shaded, slight curve at the eaves.
    cv.shape(
        move |p| roof_sdf(p, x, y, w, th, r),
        move |p| {
            let t = ((p.y - y) / th).clamp(0.0, 1.0);
            let s = ((p.x - x) / w).clamp(0.0, 1.0);
            let base = mix(ROOF_LIGHT, ROOF, t * 0.9);
            mix(base, ROOF_SHADE, (s * 0.2 + t * 0.16).min(0.32))
        },
        OW,
    );
    shingles(cv, x, y, w, th);
    // Verdigris ridge cap along the top, with small upturned finials.
    cv.solid(move |p| rbox(p, x + r - 2.0, y + 1.5, w - 2.0 * r + 4.0, 4.0, 2.0), TRIM, 1.2);
    for fx in [x + r - 1.0, x + w - r + 1.0] {
        cv.solid(move |p| circle(p, v(fx, y + 3.5), 2.6), TRIM, 1.0);
    }
    // Verdigris tips on the two upturned eave corners.
    for cx in [x + 1.5, x + w - 1.5] {
        cv.fill(move |p| circle(p, v(cx, y + th - r), 3.0), move |_| TRIM_DARK);
    }
}

/// A tiny lantern-market stall: dark-wood walls under a small curved rose roof.
fn pavilion(cv: &mut Canvas, x: f32, y: f32, w: f32, h: f32) {
    cv.shadow(move |p| rbox(p, x, y, w, h, 4.0), SHADOW, 4.0, 0.3);
    // Dark-wood front wall under the roof.
    cv.solid(move |p| rbox(p, x, y + h * 0.5, w, h * 0.5, 3.0), side_of(WOOD_DARK), OW);
    window_glow(cv, v(x + w / 2.0, y + h * 0.74), v(w * 0.22, h * 0.1));
    roof(cv, x, y, w, h * 0.58, 4.0);
}

/// Warm brick courses over a furnace body, with light mortar lines.
fn brick(cv: &mut Canvas, x: f32, y: f32, w: f32, h: f32, r: f32) {
    cv.fill(
        move |p| rbox(p, x, y, w, h, r),
        move |p| {
            let row = ((p.y - y) / 11.0).floor();
            let fy = ((p.y - y) / 11.0).rem_euclid(1.0);
            let fx = ((p.x + (row % 2.0) * 10.0) / 21.0).rem_euclid(1.0);
            let base = mix(BRICK_LIGHT, BRICK, fy * 0.55);
            if fy < 0.12 || fx < 0.08 { mix(base, BRICK_MORTAR, 0.55) } else { base }
        },
    );
}

/// Warm wood planks over a front wall band.
fn planks(cv: &mut Canvas, x: f32, y: f32, w: f32, h: f32, r: f32) {
    cv.fill(
        move |p| rbox(p, x, y, w, h, r),
        move |p| {
            let row = ((p.y - y) / 8.0).floor();
            let fy = ((p.y - y) / 8.0).rem_euclid(1.0);
            let fx = ((p.x + row * 13.0) / 19.0).rem_euclid(1.0);
            let base = mix(WOOD, WOOD_DARK, fy * 0.6);
            if fy < 0.12 || fx < 0.06 { mix(base, BROWN_D, 0.45) } else { base }
        },
    );
}

/// A small brass steam vent: an open dark mouth at `c` (the steam origin) with
/// a brass collar below it. `c` matches a `VENTS` entry in `src/render.rs`.
fn vent(cv: &mut Canvas, c: Vec2) {
    cv.solid(move |p| ellipse(p, c + v(0.0, 2.0), v(5.0, 2.0)), TRIM_DARK, 1.0);
    cv.solid(move |p| rbox(p, c.x - 3.5, c.y, 7.0, 6.0, 2.0), BRASS, 1.2);
    cv.fill(move |p| ellipse(p, c, v(3.5, 1.6)), |_| with_alpha(INK, 0.85));
}

/// A warm glowing window (or porthole) framed in brass, at `c`.
fn window_glow(cv: &mut Canvas, c: Vec2, r: Vec2) {
    cv.solid(move |p| ellipse(p, c, r), BRASS, 1.2);
    cv.shape(
        move |p| ellipse(p, c, r * 0.82),
        move |p| {
            let t = ((p.y - c.y + r.y) / (2.0 * r.y)).clamp(0.0, 1.0);
            mix(LAMP_INNER, LAMP, t)
        },
        1.0,
    );
    cv.fill(move |p| rbox(p, c.x - r.x * 0.8, c.y - 0.6, r.x * 1.6, 1.2, 0.6), |_| with_alpha(BROWN_D, 0.6));
    cv.fill(move |p| rbox(p, c.x - 0.6, c.y - r.y * 0.8, 1.2, r.y * 1.6, 0.6), |_| with_alpha(BROWN_D, 0.6));
}

/// A brass pipe run with flanged joints.
fn brass_pipe(cv: &mut Canvas, a: Vec2, b: Vec2, r: f32) {
    cv.shape(
        move |p| capsule(p, a, b, r),
        move |p| mix(lit(BRASS, 1.12), lit(BRASS, 0.7), (p.y - a.y) / (b.y - a.y).abs().max(1.0)),
        1.2,
    );
    for t in [0.25, 0.5, 0.75] {
        let c = a.lerp(b, t);
        cv.solid(move |p| circle(p, c, r * 1.3), TRIM_DARK, 1.0);
    }
}

/// A copper pipe run with riveted flanges.
fn copper_pipe(cv: &mut Canvas, a: Vec2, b: Vec2, r: f32) {
    cv.shape(
        move |p| capsule(p, a, b, r),
        move |p| mix(lit(COPPER, 1.1), lit(COPPER, 0.65), (p.y - a.y) / (b.y - a.y).abs().max(1.0)),
        1.2,
    );
    for t in [0.3, 0.7] {
        let c = a.lerp(b, t);
        cv.solid(move |p| circle(p, c, r * 1.3), TRIM_DARK, 1.0);
    }
}

/// A verdigris pipe run with flanged joints.
fn verdigris_pipe(cv: &mut Canvas, a: Vec2, b: Vec2, r: f32) {
    cv.shape(
        move |p| capsule(p, a, b, r),
        move |p| mix(lit(TRIM, 1.12), lit(TRIM, 0.66), (p.y - a.y) / (b.y - a.y).abs().max(1.0)),
        1.2,
    );
    for t in [0.3, 0.7] {
        let c = a.lerp(b, t);
        cv.solid(move |p| circle(p, c, r * 1.3), TRIM_DARK, 1.0);
    }
}

pub fn drill() -> Canvas {
    let mut cv = Canvas::new(128, 128);
    let (x, y, w, th) = block(&mut cv, 6.0, 8.0, 116.0, 112.0, 16.0, 14.0, STONE, side_of(STONE_DARK));
    // Stone pad: faint joint lines so it reads as dressed stone.
    cv.fill(
        move |p| rbox(p, x, y, w, th, 14.0),
        move |p| {
            if (p.x - x - 8.0).rem_euclid(34.0) < 1.4 || (p.y - y - 8.0).rem_euclid(30.0) < 1.4 {
                with_alpha(STONE_DARK, 0.35)
            } else {
                with_alpha(INK, 0.0)
            }
        },
    );
    // Brass collar and dark bore around the drill bit (bit overlay at 64,56).
    cv.solid(move |p| circle(p, v(64.0, 56.0), 34.0), BRASS, OW);
    cv.solid(move |p| circle(p, v(64.0, 56.0), 28.0), DRILL_HOLE, OW);
    cv.solid(move |p| circle(p, v(64.0, 56.0), 21.0), PIT, 1.5);
    // Wooden A-frame derrick rising over the bit.
    let apex = v(64.0, 14.0);
    let foot_l = v(22.0, 108.0);
    let foot_r = v(106.0, 108.0);
    cv.shadow(move |p| capsule(p, foot_l, apex, 5.5).min(capsule(p, foot_r, apex, 5.5)), v(3.0, 4.0), 4.0, 0.3);
    for (a, b) in [(foot_l, apex), (foot_r, apex)] {
        cv.shape(move |p| capsule(p, a, b, 5.5), move |p| mix(lit(BROWN_D, 1.3), BROWN_D, (p.y - 14.0) / 94.0), OW);
    }
    // Cross-braces between the legs.
    for by in [50.0, 72.0, 94.0] {
        let u = (by - 14.0) / 94.0;
        let lx = 22.0 + (64.0 - 22.0) * u;
        let rx = 106.0 + (64.0 - 106.0) * u;
        cv.solid(move |p| capsule(p, v(lx, by), v(rx, by), 3.0), BROWN_D, 1.2);
    }
    // Little winch hut on the top-right corner, with a curved rose roof.
    pavilion(&mut cv, 80.0, 8.0, 42.0, 40.0);
    vent(&mut cv, v(96.0, 22.0));
    cv
}

pub fn drill_bit() -> Canvas {
    let mut cv = Canvas::new(64, 64);
    let c = v(32.0, 32.0);
    for k in 0..3 {
        let a = k as f32 * TAU / 3.0;
        let tip = c + Vec2::from_angle(a) * 24.0;
        let mid = c + Vec2::from_angle(a + 0.35) * 14.0;
        cv.shape(move |p| capsule(p, c, mid, 5.0).min(capsule(p, mid, tip, 3.5)), |p| mix(STEEL_LIGHT, STEEL_D, p.y / 64.0), OW_OVERLAY);
    }
    cv.solid(move |p| circle(p, c, 8.0), BRASS, OW_OVERLAY * 0.75);
    cv
}

pub fn chute() -> Canvas {
    let mut cv = Canvas::new(32, 32);
    let pts = [v(4.0, 6.0), v(28.0, 10.0), v(28.0, 22.0), v(4.0, 26.0)];
    cv.shape(move |p| poly(p, &pts), |p| mix(COPPER_LIGHT, COPPER_DARK, p.y / 32.0), OW_CHUTE);
    cv
}

pub fn smelter() -> Canvas {
    let mut cv = Canvas::new(128, 128);
    let (x, y, w, th) = block(&mut cv, 6.0, 22.0, 116.0, 100.0, 22.0, 12.0, BRICK, side_of(BRICK_DARK));
    brick(&mut cv, x, y, w, th, 12.0);
    brick(&mut cv, x, y + th + 2.0, w, 17.0, 6.0);
    // A small curved rose canopy over the back half only, so the brick furnace
    // body stays the dominant material.
    roof(&mut cv, x, y, w, th * 0.52, 12.0);
    // Arched furnace mouth with a glowing core at the light coordinate (64,104).
    cv.solid(move |p| rbox(p, 44.0, 92.0, 40.0, 28.0, 14.0), BRASS, OW);
    cv.fill(move |p| rbox(p, 49.0, 96.0, 30.0, 22.0, 11.0), |_| FIREBOX);
    cv.shape(move |p| ellipse(p, v(64.0, 104.0), v(11.5, 7.5)), move |p| {
        let t = ((p.x - 64.0) / 11.5).abs().min(1.0);
        mix(LAMP_INNER, LAMP, t)
    }, 1.2);
    for k in 0..3 {
        let bx = 55.0 + k as f32 * 9.0;
        cv.fill(move |p| rbox(p, bx, 90.0, 3.0, 7.0, 1.0), |_| BRICK_DARK);
    }
    // Brick chimney rising from the roof, with a verdigris cap and steam vent.
    cv.shadow(|p| rbox(p, 86.0, 4.0, 20.0, 36.0, 4.0), v(4.0, 4.0), 4.0, 0.3);
    cv.shape(|p| rbox(p, 86.0, 6.0, 20.0, 32.0, 4.0), |p| mix(BRICK_LIGHT, BRICK_DARK, (p.x - 86.0) / 20.0), OW);
    cv.solid(|p| rbox(p, 86.0, 4.0, 20.0, 5.0, 2.5), TRIM, 1.2);
    vent(&mut cv, v(97.0, 6.0));
    cv
}

pub fn assembler() -> Canvas {
    let mut cv = Canvas::new(192, 192);
    let (x, y, w, th) = block(&mut cv, 8.0, 10.0, 176.0, 174.0, 22.0, 16.0, WOOD_DARK, side_of(BROWN_D));
    // Dark warm wood workshop: plank top and a framed front wall.
    planks(&mut cv, x, y, w, th, 16.0);
    planks(&mut cv, x + 4.0, y + th + 1.0, w - 8.0, 19.0, 7.0);
    // Visible framing beams around the deck.
    for (bx, by, bw2, bh) in [(x + 5.0, y + 8.0, 3.5, th - 12.0), (x + w - 8.5, y + 8.0, 3.5, th - 12.0), (x + 5.0, y + th - 6.0, w - 10.0, 3.5)] {
        cv.solid(move |p| rbox(p, bx, by, bw2, bh, 1.5), BROWN_D, 1.0);
    }
    // Curved rose roof as a band along the top edge, with a verdigris ridge cap.
    roof(&mut cv, x, y, w, 30.0, 16.0);
    let c = v(x + w / 2.0, y + th / 2.0);
    // Glowing glass skylight at the light coordinate (96,86); the gear overlay
    // spins on top of it.
    cv.solid(move |p| rrect(p, c, v(52.0, 46.0), 14.0), BRASS, OW);
    cv.shape(move |p| rrect(p, c, v(46.0, 40.0), 11.0), move |p| {
        let t = ((p.y - c.y + 40.0) / 80.0).clamp(0.0, 1.0);
        mix(LAMP_INNER, LAMP, t)
    }, 1.4);
    cv.fill(move |p| rrect(p, c - v(24.0, 24.0), v(12.0, 3.0), 2.0), |_| with_alpha(WHITE, 0.4));
    // Brass rivets on the skylight frame.
    for (dx, dy) in [(-52.0, -46.0), (52.0, -46.0), (-52.0, 46.0), (52.0, 46.0)] {
        rivet(&mut cv, c + v(dx, dy), 3.5);
    }
    // Brass pipe run along the roof band's lower edge.
    brass_pipe(&mut cv, v(x + 20.0, y + 18.0), v(x + w - 20.0, y + 18.0), 3.5);
    // A small warm window on the front wall.
    window_glow(&mut cv, v(c.x, y + th + 10.0), v(11.0, 6.0));
    vent(&mut cv, v(x + w - 30.0, y + 14.0));
    cv
}

pub fn gear_overlay(size: usize, teeth: u32, col: C) -> Canvas {
    let mut cv = Canvas::new(size, size);
    let s = size as f32;
    let c = v(s / 2.0, s / 2.0);
    let r = s * 0.36;
    cv.shape(move |p| gear(p, c, r, teeth, s * 0.08, 0.0).max(-circle(p, c, r * 0.35)), move |p| mix(lit(col, 1.15), lit(col, 0.8), p.y / s), OW_GEAR);
    for k in 0..4 {
        let a = k as f32 * TAU / 4.0;
        let h = c + Vec2::from_angle(a) * r * 0.62;
        cv.fill(move |p| circle(p, h, r * 0.13), |_| with_alpha(INK, 0.55));
    }
    cv
}

pub fn lab() -> Canvas {
    let mut cv = Canvas::new(192, 192);
    let (x, y, w, th) = block(&mut cv, 8.0, 12.0, 176.0, 172.0, 22.0, 26.0, COPPER, side_of(COPPER_DARK));
    // Calm copper tile on the plinth top: hairline grout instead of the old
    // noise patches, so the base reads as smooth plaster rather than glyphs.
    cv.fill(move |p| rbox(p, x, y, w, th, 26.0), move |p| {
        let gx = ((p.x - x) / (w / 6.0)).fract();
        let gy = ((p.y - y) / (th / 4.0)).fract();
        if !(0.04..=0.96).contains(&gx) || !(0.04..=0.96).contains(&gy) {
            with_alpha(COPPER_DARK, 0.30)
        } else {
            with_alpha(INK, 0.0)
        }
    });
    let c = v(x + w / 2.0, y + th / 2.0);
    // Brass ribs framing the round greenhouse dome.
    for k in 0..8 {
        let a = k as f32 * TAU / 8.0;
        let o = c + Vec2::from_angle(a) * 62.0;
        cv.solid(move |p| capsule(p, c, o, 3.0), BRASS, 1.8);
    }
    // Glass dome with a glowing orb at the light coordinate (96,87).
    cv.solid(move |p| circle(p, c, 62.0), BRASS, OW);
    cv.shape(move |p| circle(p, c, 56.0), move |p| mix(DOME_GLASS, DOME_GLASS_D, (p.y - c.y + 56.0) / 112.0), 1.4);
    cv.shape(move |p| circle(p, c, 30.0), move |p| {
        let t = ((p.x - c.x) / 30.0).abs().min(1.0);
        mix(LAMP_INNER, LAMP, t)
    }, 1.4);
    cv.fill(move |p| ellipse(p, c - v(16.0, 20.0), v(14.0, 7.0)), |_| with_alpha(GLASS_HI, 0.6));
    // A small rose finial/cap at the dome apex.
    cv.solid(move |p| circle(p, v(c.x, y + 10.0), 7.0), ROOF, 1.4);
    cv.solid(move |p| rbox(p, c.x - 5.0, y + 14.0, 10.0, 3.0, 1.5), TRIM_DARK, 1.0);
    cv.fill(move |p| circle(p, v(c.x - 2.0, y + 8.0), 2.5), |_| with_alpha(ROOF_LIGHT, 0.75));
    // Copper pipe along the front wall and a brass vent on the plinth.
    copper_pipe(&mut cv, v(x + 24.0, y + th - 10.0), v(x + w - 24.0, y + th - 10.0), 3.5);
    vent(&mut cv, v(x + w - 34.0, y + 20.0));
    cv
}

pub fn orb(size: usize, core: C, edge: C) -> Canvas {
    let mut cv = Canvas::new(size, size);
    let s = size as f32;
    let c = v(s / 2.0, s / 2.0);
    cv.fill(move |p| circle(p, c, s * 0.48), move |p| {
        let t = p.distance(c) / (s * 0.48);
        with_alpha(mix(core, edge, t), (1.0 - t * t).max(0.0))
    });
    cv
}

pub fn generator() -> Canvas {
    let mut cv = Canvas::new(128, 128);
    let (x, y, w, th) = block(&mut cv, 6.0, 12.0, 116.0, 108.0, 18.0, 12.0, WOOD_DARK, side_of(BROWN_D));
    planks(&mut cv, x, y, w, th, 12.0);
    planks(&mut cv, x + 3.0, y + th + 1.0, w - 6.0, 16.0, 6.0);
    // Brass boiler drum lying horizontally, with verdigris bands and rivets.
    let drum_y = 22.0;
    let drum_h = 70.0;
    cv.shadow(move |p| rbox(p, x + 4.0, drum_y, w - 8.0, drum_h, 24.0), v(3.0, 4.0), 5.0, 0.3);
    cv.shape(move |p| rbox(p, x + 4.0, drum_y, w - 8.0, drum_h, 24.0), |p| mix(lit(BRASS, 1.12), lit(BRASS, 0.58), (p.y - drum_y) / drum_h), OW);
    for k in 1..4 {
        let bx = x + 4.0 + k as f32 * (w - 8.0) / 4.0;
        cv.fill(move |p| rbox(p, bx - 2.0, drum_y + 5.0, 4.0, drum_h - 10.0, 2.0), |_| with_alpha(TRIM_DARK, 0.75));
    }
    // Rivets along the drum's top and bottom rims.
    for k in 0..9 {
        let rx = x + 14.0 + k as f32 * 12.0;
        rivet(&mut cv, v(rx, drum_y + 9.0), 1.7);
        rivet(&mut cv, v(rx, drum_y + drum_h - 9.0), 1.7);
    }
    // Glowing firebox door at the wheel/light coordinate (74,61).
    cv.solid(move |p| circle(p, v(74.0, 61.0), 26.0), TRIM_DARK, OW);
    cv.solid(move |p| circle(p, v(74.0, 61.0), 21.0), FIREBOX, 1.2);
    cv.shape(move |p| circle(p, v(74.0, 61.0), 15.0), move |p| {
        let t = ((p.x - 74.0) / 15.0).abs().min(1.0);
        mix(LAMP_INNER, LAMP, t)
    }, 1.4);
    // Small curved rose roof hood over the firebox, perched on the drum's top.
    roof(&mut cv, 54.0, 7.0, 40.0, 19.0, 8.0);
    // Steam vent at the top-left of the boiler drum.
    vent(&mut cv, v(31.0, 20.0));
    cv
}

pub fn wheel() -> Canvas {
    let mut cv = Canvas::new(64, 64);
    let c = v(32.0, 32.0);
    cv.solid(move |p| ring(p, c, 25.0, 6.0), BRASS, OW_OVERLAY);
    for k in 0..6 {
        let a = k as f32 * TAU / 6.0;
        let e = c + Vec2::from_angle(a) * 23.0;
        cv.solid(move |p| capsule(p, c, e, 2.2), COPPER, OW_OVERLAY * 0.6);
    }
    cv.solid(move |p| circle(p, c, 6.0), WOOD_DARK, OW_OVERLAY);
    cv
}

/// Lamp post, 64x112: the bottom 64x64 is the tile it stands on. The hanging
/// lantern is split into its own `lantern` sprite so it can swing in the game.
pub fn pole() -> Canvas {
    let mut cv = Canvas::new(64, 112);
    ground_shadow(&mut cv, v(36.0, 98.0), v(14.0, 5.0), 0.4);
    let wood = WOOD_DARK;
    cv.shape(|p| rbox(p, 28.0, 12.0, 8.0, 88.0, 3.0), move |p| mix(lit(wood, 1.15), lit(wood, 0.8), (p.x - 28.0) / 8.0), 2.0);
    cv.shape(|p| rbox(p, 10.0, 16.0, 44.0, 7.0, 3.0), move |_| wood, 2.0);
    for x in [14.0, 50.0] {
        cv.solid(move |p| circle(p, v(x, 15.0), 3.0), CREAM, 1.2);
    }
    cv
}

/// The red paper lantern hanging from its hook, 64x48. The hook's top sits at
/// local (32, 4) — that's the swing pivot `render.rs` rotates around, so the
/// lantern keeps its place on the pole and only the body sways.
pub fn lantern() -> Canvas {
    let mut cv = Canvas::new(64, 48);
    // Hook hanging from the crossarm above.
    cv.fill(|p| capsule(p, v(32.0, 4.0), v(32.0, 9.0), 1.0), |_| INK);
    // Dark top and bottom caps.
    cv.solid(|p| rbox(p, 21.0, 9.0, 22.0, 5.0, 2.5), BROWN_D, 1.2);
    cv.solid(|p| rbox(p, 21.0, 34.0, 22.0, 5.0, 2.5), BROWN_D, 1.2);
    // Big oval lacquer body: saturated rose-red with a bright warm core that
    // fades to the paper at the rim, so the lantern visibly glows.
    cv.shape(|p| ellipse(p, v(32.0, 18.0), v(15.0, 19.0)), move |p| {
        let d = ((p - v(32.0, 18.0)) / v(15.0, 19.0)).length();
        let paper = mix(ROOF_LIGHT, ROOF, ((p.y + 1.0) / 38.0).clamp(0.0, 1.0));
        let core = 1.0 - smooth(0.15, 0.62, d);
        mix(paper, LAMP_INNER, core * 0.85)
    }, 1.6);
    // Faint vertical ribs across the body (kept off the bright core).
    for k in [-3, -1, 1, 3] {
        let rx = 32.0 + k as f32 * 2.5;
        let top = 15.0 + (k as f32 * 0.5);
        let bot = 21.0 - (k as f32 * 0.5);
        cv.fill(move |p| capsule(p, v(rx, top), v(rx, bot), 0.7), |_| with_alpha(ROOF_SHADE, 0.32));
    }
    // Little tassel under the bottom cap.
    cv.fill(|p| capsule(p, v(32.0, 39.0), v(32.0, 43.0), 0.9), |_| INK);
    cv
}

pub fn solar() -> Canvas {
    let mut cv = Canvas::new(128, 128);
    let (x, y, w, h) = block(&mut cv, 6.0, 8.0, 116.0, 112.0, 10.0, 8.0, BRASS, COPPER_DARK);
    for r in 0..3 {
        for c in 0..3 {
            let cx = x + 8.0 + c as f32 * (w - 16.0) / 3.0;
            let cy = y + 8.0 + r as f32 * (h - 16.0) / 3.0;
            let cw = (w - 16.0) / 3.0 - 4.0;
            let ch = (h - 16.0) / 3.0 - 4.0;
            cv.shape(move |p| rbox(p, cx, cy, cw, ch, 3.0), move |p| mix(SOLAR_HI, SOLAR_LO, (p.y - cy) / ch + (p.x - cx) / cw * 0.3), 1.6);
            cv.fill(move |p| rbox(p, cx + 4.0, cy + 4.0, cw * 0.4, 2.0, 1.0), |_| with_alpha(WHITE, 0.35));
        }
    }
    cv
}

pub fn wellhead() -> Canvas {
    let mut cv = Canvas::new(128, 128);
    let (x, y, w, th) = block(&mut cv, 6.0, 10.0, 116.0, 110.0, 14.0, 10.0, STONE, side_of(STONE_DARK));
    // Stone pump base with faint joint lines.
    cv.fill(
        move |p| rbox(p, x, y, w, th, 10.0),
        move |p| {
            if (p.x - x - 8.0).rem_euclid(32.0) < 1.3 || (p.y - y - 8.0).rem_euclid(28.0) < 1.3 {
                with_alpha(STONE_DARK, 0.35)
            } else {
                with_alpha(INK, 0.0)
            }
        },
    );
    // Pump well opening under the A-frame, at the beam pivot (78,52).
    cv.solid(move |p| circle(p, v(78.0, 52.0), 15.0), BRASS, OW);
    cv.solid(move |p| circle(p, v(78.0, 52.0), 9.0), WELLHEAD_DARK, OW);
    cv.solid(move |p| circle(p, v(78.0, 52.0), 5.0), PIT, 1.2);
    // Dark-wood A-frame over the beam.
    let apex = v(78.0, 16.0);
    let foot_l = v(54.0, 102.0);
    let foot_r = v(102.0, 102.0);
    cv.shadow(move |p| capsule(p, foot_l, apex, 4.5).min(capsule(p, foot_r, apex, 4.5)), v(3.0, 4.0), 4.0, 0.3);
    for (a, b) in [(foot_l, apex), (foot_r, apex)] {
        cv.shape(move |p| capsule(p, a, b, 4.5), move |p| mix(lit(BROWN_D, 1.3), BROWN_D, (p.y - 16.0) / 86.0), OW);
    }
    cv.solid(move |p| capsule(p, v(68.0, 66.0), v(88.0, 66.0), 3.0), BROWN_D, 1.2);
    // Small rose-roofed pump house on the top-left corner.
    pavilion(&mut cv, 6.0, 10.0, 46.0, 38.0);
    vent(&mut cv, v(46.0, 24.0));
    // A warm porthole on the front wall.
    window_glow(&mut cv, v(90.0, y + th + 7.0), v(9.0, 5.5));
    cv
}

pub fn beam() -> Canvas {
    let mut cv = Canvas::new(112, 40);
    cv.shape(|p| capsule(p, v(14.0, 20.0), v(96.0, 20.0), 6.0), |p| mix(lit(WOOD_DARK, 1.2), WOOD_DARK, p.y / 40.0), OW_OVERLAY);
    let head = [v(8.0, 8.0), v(22.0, 10.0), v(22.0, 32.0), v(8.0, 34.0)];
    cv.solid(move |p| poly(p, &head), BRASS, OW_OVERLAY);
    cv.solid(|p| circle(p, v(62.0, 20.0), 6.0), BRASS, OW_OVERLAY * 0.8);
    cv.solid(|p| rbox(p, 92.0, 10.0, 14.0, 20.0, 3.0), COPPER_DARK, OW_OVERLAY * 0.8);
    cv
}

fn tank(cv: &mut Canvas, c: Vec2, r: f32, height: f32, col: C) {
    cv.shadow(move |p| rbox(p, c.x - r, c.y - r * 0.5, r * 2.0, height + r, r * 0.9), SHADOW, 6.0, 0.35);
    cv.shape(move |p| rbox(p, c.x - r, c.y, r * 2.0, height, r * 0.4), move |p| mix(lit(col, 1.05), lit(col, 0.62), (p.x - c.x + r) / (2.0 * r)), OW);
    cv.shape(move |p| ellipse(p, c, v(r, r * 0.55)), move |_| lit(col, 1.15), OW);
    cv.fill(move |p| ellipse(p, c - v(r * 0.25, r * 0.1), v(r * 0.45, r * 0.18)), |_| with_alpha(WHITE, 0.35));
    for k in 1..3 {
        let by = c.y + height * k as f32 / 3.0;
        cv.fill(move |p| rbox(p, c.x - r + 2.0, by, r * 2.0 - 4.0, 2.5, 1.0), |_| with_alpha(INK, 0.4));
    }
}

pub fn refinery() -> Canvas {
    let mut cv = Canvas::new(192, 192);
    let (x, y, w, th) = block(&mut cv, 8.0, 12.0, 176.0, 172.0, 16.0, 12.0, WOOD_DARK, side_of(BROWN_D));
    planks(&mut cv, x, y, w, th, 12.0);
    planks(&mut cv, x + 4.0, y + th + 1.0, w - 8.0, 14.0, 6.0);
    // Brass and copper pipe manifold between the tanks.
    brass_pipe(&mut cv, v(58.0, 116.0), v(140.0, 116.0), 6.0);
    brass_pipe(&mut cv, v(58.0, 66.0), v(58.0, 146.0), 5.0);
    copper_pipe(&mut cv, v(136.0, 92.0), v(136.0, 148.0), 4.5);
    // Boiler-room tanks in brass, copper and verdigris on the wood deck.
    tank(&mut cv, v(58.0, 40.0), 30.0, 70.0, BRASS);
    tank(&mut cv, v(136.0, 54.0), 26.0, 56.0, COPPER);
    tank(&mut cv, v(126.0, 122.0), 20.0, 36.0, TRIM);
    // A glowing furnace mouth at the base of the front wall.
    cv.solid(move |p| rbox(p, 40.0, y + th - 4.0, 34.0, 18.0, 9.0), BRASS, OW);
    cv.fill(move |p| rbox(p, 44.0, y + th, 26.0, 12.0, 7.0), |_| FIREBOX);
    cv.shape(move |p| ellipse(p, v(57.0, y + th + 6.0), v(9.0, 5.0)), move |p| {
        let t = ((p.x - 57.0) / 9.0).abs().min(1.0);
        mix(LAMP_INNER, LAMP, t)
    }, 1.2);
    // Corner rivets.
    for (dx, dy) in [(x + 14.0, y + 14.0), (x + w - 14.0, y + 14.0), (x + 14.0, y + th - 14.0), (x + w - 14.0, y + th - 14.0)] {
        rivet(&mut cv, v(dx, dy), 3.5);
    }
    // Steam vent at the top of the first tank (58,36).
    vent(&mut cv, v(58.0, 36.0));
    cv
}

pub fn mixer() -> Canvas {
    let mut cv = Canvas::new(128, 128);
    let (x, y, w, th) = block(&mut cv, 6.0, 10.0, 116.0, 110.0, 16.0, 14.0, WOOD_DARK, side_of(BROWN_D));
    planks(&mut cv, x, y, w, th, 14.0);
    planks(&mut cv, x + 3.0, y + th + 1.0, w - 6.0, 14.0, 5.0);
    let c = v(64.0, 57.0);
    // Big round copper vat with a wooden rim, seen from above.
    cv.shadow(move |p| circle(p, c, 46.0), v(3.0, 4.0), 6.0, 0.35);
    cv.solid(move |p| ring(p, c, 46.0, 5.0), WOOD, 1.4);
    cv.shape(move |p| circle(p, c, 43.0), move |p| mix(COPPER_LIGHT, COPPER_DARK, (p.y - c.y + 43.0) / 86.0), OW);
    // Warm amber liquid with a bright gleam.
    cv.solid(move |p| circle(p, c, 26.0), VAT_LIQUID, 1.2);
    cv.fill(move |p| ellipse(p, c - v(9.0, 10.0), v(11.0, 4.5)), |_| with_alpha(VAT_HI, 0.55));
    // Glowing sight glass at the paddle coordinate (64,57).
    cv.solid(move |p| circle(p, c, 13.0), BRASS, 1.4);
    cv.shape(move |p| circle(p, c, 10.0), move |p| {
        let t = ((p.x - c.x) / 10.0).abs().min(1.0);
        mix(LAMP_INNER, LAMP, t)
    }, 1.2);
    // Rivets around the wooden rim.
    for k in 0..8 {
        rivet(&mut cv, c + Vec2::from_angle(k as f32 * TAU / 8.0) * 43.5, 2.0);
    }
    // Brass stirring gantry over the vat.
    brass_pipe(&mut cv, v(20.0, 16.0), v(108.0, 16.0), 3.5);
    brass_pipe(&mut cv, v(20.0, 16.0), v(20.0, 30.0), 3.5);
    brass_pipe(&mut cv, v(108.0, 16.0), v(108.0, 30.0), 3.5);
    // Steam vent on the rim, top-right (94,24).
    vent(&mut cv, v(94.0, 24.0));
    cv
}

pub fn paddle() -> Canvas {
    let mut cv = Canvas::new(64, 64);
    let c = v(32.0, 32.0);
    cv.solid(move |p| capsule(p, c - v(24.0, 0.0), c + v(24.0, 0.0), 4.0), BRASS, OW_OVERLAY);
    cv.solid(move |p| capsule(p, c - v(0.0, 24.0), c + v(0.0, 24.0), 4.0), BRASS, OW_OVERLAY);
    cv.solid(move |p| circle(p, c, 7.0), WOOD_DARK, OW_OVERLAY);
    cv
}

pub fn wall() -> Canvas {
    let mut cv = Canvas::new(64, 64);
    let stone = WALL_STONE;
    let (x, y, w, h) = block(&mut cv, 2.0, 2.0, 60.0, 60.0, 14.0, 6.0, stone, side_of(stone));
    cv.fill(move |p| rbox(p, x + 3.0, y + 3.0, w - 6.0, h + 10.0, 4.0), move |p| {
        let row = ((p.y - y) / 12.0).floor();
        let fy = ((p.y - y) / 12.0).rem_euclid(1.0);
        let fx = ((p.x + row * 10.0) / 20.0).rem_euclid(1.0);
        if fy < 0.12 || fx < 0.07 { with_alpha(INK, 0.28) } else { with_alpha(INK, 0.0) }
    });
    for (c, r) in [(v(14.0, 12.0), 5.0), (v(46.0, 40.0), 4.0)] {
        cv.fill(move |p| circle(p, c, r), |_| with_alpha(lit(WALL_STONE, 0.72), 0.7));
        cv.fill(move |p| circle(p, c - v(r * 0.25, r * 0.25), r * 0.5), |_| with_alpha(lit(WALL_STONE, 0.94), 0.7));
    }
    cv
}

pub fn sentry_base() -> Canvas {
    let mut cv = Canvas::new(128, 128);
    let (x, y, w, h) = block(&mut cv, 8.0, 10.0, 112.0, 110.0, 16.0, 30.0, WOOD_DARK, side_of(BROWN_D));
    let c = v(x + w / 2.0, y + h / 2.0);
    cv.solid(move |p| ring(p, c, 34.0, 6.0), BRASS, 1.8);
    for k in 0..8 {
        let rc = c + Vec2::from_angle(k as f32 * TAU / 8.0) * 34.0;
        cv.solid(move |p| circle(p, rc, 2.2), BRASS, 1.0);
        cv.fill(move |p| circle(p, rc - v(0.6, 0.6), 0.9), |_| with_alpha(WHITE, 0.6));
    }
    cv
}

/// Turret head facing east.
pub fn sentry_head() -> Canvas {
    let mut cv = Canvas::new(96, 96);
    let c = v(48.0, 48.0);
    cv.shadow(move |p| circle(p, c, 22.0), v(3.0, 4.0), 4.0, 0.35);
    // Verdigris barrel pointing east.
    cv.shape(|p| rbox(p, 50.0, 41.0, 42.0, 14.0, 5.0), |p| mix(lit(TRIM, 1.15), TRIM_DARK, (p.y - 41.0) / 14.0), OW_OVERLAY);
    cv.solid(|p| rbox(p, 84.0, 39.0, 9.0, 18.0, 3.0), BRASS, OW_OVERLAY * 0.8);
    // Brass turret housing.
    cv.shape(move |p| circle(p, c, 22.0), move |p| mix(lit(BRASS, 1.1), lit(BRASS, 0.62), (p.y - 26.0) / 44.0), OW_OVERLAY);
    cv.solid(move |p| rbox(p, 38.0, 42.0, 14.0, 6.0, 3.0), INK, 0.0);
    cv.fill(move |p| circle(p, v(46.0, 45.0), 2.2), |_| LAMP);
    cv
}

pub fn engine() -> Canvas {
    let mut cv = Canvas::new(256, 256);
    let (x, y, w, th) = block(&mut cv, 8.0, 10.0, 240.0, 236.0, 26.0, 24.0, STONE, side_of(STONE_DARK));
    // Stone platform with faint dressed-stone joints.
    cv.fill(
        move |p| rbox(p, x, y, w, th, 24.0),
        move |p| {
            if (p.x - x - 10.0).rem_euclid(44.0) < 1.6 || (p.y - y - 10.0).rem_euclid(40.0) < 1.6 {
                with_alpha(STONE_DARK, 0.32)
            } else {
                with_alpha(INK, 0.0)
            }
        },
    );
    let c = v(x + w / 2.0, y + th / 2.0);
    // Brass reactor ring; the engine_ring overlay spins on top of it.
    cv.solid(move |p| circle(p, c, 94.0), BRASS, OW);
    cv.solid(move |p| circle(p, c, 82.0), ENGINE_INNER, OW);
    // Glowing furnace core at the light coordinate (128,115).
    cv.shape(move |p| circle(p, c, 56.0), move |p| {
        let t = ((p.x - c.x) / 56.0).abs().min(1.0);
        mix(LAMP_INNER, LAMP, t)
    }, 1.4);
    for k in 0..12 {
        let a = k as f32 * TAU / 12.0;
        rivet(&mut cv, c + Vec2::from_angle(a) * 88.0, 2.8);
    }
    // Verdigris pipe runs from the reactor rim out to each corner pavilion.
    verdigris_pipe(&mut cv, v(60.0, 55.0), v(34.0, 34.0), 3.5);
    verdigris_pipe(&mut cv, v(196.0, 55.0), v(222.0, 34.0), 3.5);
    verdigris_pipe(&mut cv, v(60.0, 175.0), v(34.0, 196.0), 3.5);
    verdigris_pipe(&mut cv, v(196.0, 175.0), v(222.0, 196.0), 3.5);
    // Four little rose-roofed corner pavilions, like lantern-market stalls.
    pavilion(&mut cv, 14.0, 14.0, 46.0, 42.0);
    pavilion(&mut cv, 196.0, 14.0, 46.0, 42.0);
    pavilion(&mut cv, 14.0, 172.0, 46.0, 42.0);
    pavilion(&mut cv, 196.0, 172.0, 46.0, 42.0);
    // Steam vent at the top of the reactor.
    vent(&mut cv, v(c.x, y + 12.0));
    cv
}

pub fn engine_ring() -> Canvas {
    let mut cv = Canvas::new(160, 160);
    let c = v(80.0, 80.0);
    cv.solid(move |p| ring(p, c, 64.0, 8.0), BRASS, OW_OVERLAY);
    for k in 0..16 {
        let a = k as f32 * TAU / 16.0;
        let m = c + Vec2::from_angle(a) * 64.0;
        let col = if k % 2 == 0 { TRIM } else { COPPER };
        cv.solid(move |p| circle(p, m, 4.0), col, OW_OVERLAY * 0.7);
    }
    cv.solid(move |p| ring(p, c, 44.0, 4.0), with_alpha(COPPER_LIGHT, 0.9), OW_OVERLAY * 0.7);
    cv
}

pub fn hive() -> Canvas {
    let mut cv = Canvas::new(192, 192);
    let blobs = [(v(96.0, 104.0), 70.0), (v(62.0, 80.0), 42.0), (v(132.0, 76.0), 46.0), (v(96.0, 56.0), 40.0)];
    let sdf = move |p: Vec2| blobs.iter().map(|(c, r)| circle(p, *c, *r)).fold(f32::MAX, f32::min);
    cv.shadow(sdf, SHADOW, 9.0, 0.4);
    cv.shape(sdf, |p| mix(HIVE_SHELL, HIVE_SHELL_D, p.y / 192.0), 3.0);
    grain(&mut cv, move |p| sdf(p) + 4.0, 5, 0.18);
    for (k, (c, r)) in [(v(70.0, 84.0), 12.0), (v(124.0, 74.0), 10.0), (v(100.0, 120.0), 14.0), (v(92.0, 58.0), 8.0), (v(140.0, 120.0), 8.0)]
        .into_iter()
        .enumerate()
    {
        cv.solid(move |p| circle(p, c, r), HIVE_CELL, 2.0);
        cv.fill(move |p| circle(p, c + v(0.0, r * 0.2), r * 0.55), move |_| if k % 2 == 0 { HIVE_GLOW_A } else { HIVE_GLOW_B });
    }
    cv
}

// ---------------------------------------------------------------- creatures

/// Cog, the little worker robot, seen from above.
pub fn player(frame: u32) -> Canvas {
    let mut cv = Canvas::new(64, 64);
    let bob = frame as f32 * 1.5;
    let c = v(32.0, 32.0 - bob);
    ground_shadow(&mut cv, v(32.0, 50.0), v(15.0, 5.0), 0.35);
    // Standing feet stay on the ground (they don't bob with the body), so the
    // robot keeps its legs while breathing instead of growing them on the move.
    player_feet(&mut cv, v(32.0, 51.0), 0.0);
    player_body(&mut cv, c, bob);
    cv
}

/// The walk-cycle frames: the same body with two little feet and a tiny arm
/// swing. Contact frames (0, 2) plant one foot ahead of the other; passing
/// frames (1, 3) bring them together. The renderer supplies bob, lean and
/// squash, so the body stays centred here.
pub fn player_walk(frame: u32) -> Canvas {
    let mut cv = Canvas::new(64, 64);
    let c = v(32.0, 32.0);
    ground_shadow(&mut cv, v(32.0, 50.0), v(15.0, 5.0), 0.35);
    // +1 / -1 through the cycle: which foot is forward.
    let step = match frame % 4 {
        0 => 1.0,
        1 => 0.0,
        2 => -1.0,
        _ => 0.0,
    };
    player_feet(&mut cv, v(32.0, 51.0), step);
    // Tiny arms swinging on the sides, opposite to the same-side foot.
    for side in [-1.0f32, 1.0] {
        let shoulder = v(c.x + side * 13.0, c.y + 1.0);
        let hand = v(c.x + side * 17.0, c.y + 1.0 + side * step * 3.0);
        cv.solid(move |p| capsule(p, shoulder, hand, 1.9), INK, 0.6);
    }
    player_body(&mut cv, c, 0.0);
    cv
}

/// The two little feet: planted side by side when `step == 0`, otherwise one
/// steps ahead of the other. `anchor` is the ground point between the feet, so
/// the idle sprite can keep them planted while its body bobs.
fn player_feet(cv: &mut Canvas, anchor: Vec2, step: f32) {
    for side in [-1.0f32, 1.0] {
        let x = anchor.x + side * 8.0;
        let y = anchor.y - side * step * 3.0;
        cv.solid(move |p| capsule(p, v(x, y - 2.0), v(x, y + 4.0), 2.4), INK, 0.8);
    }
}

/// Body, scarf, visor, eyes and antenna shared by the idle and walk sprites.
fn player_body(cv: &mut Canvas, c: Vec2, bob: f32) {
    // Antenna with a little glowing bulb.
    let tip = c + v(9.0, -22.0 + bob * 0.5);
    cv.solid(move |p| capsule(p, c + v(5.0, -12.0), tip, 1.4), BROWN_D, 0.0);
    cv.solid(move |p| circle(p, tip, 3.6), LAMP, 1.5);
    cv.shape(move |p| circle(p, c, 17.0), move |p| mix(CREAM, PAPER, (p.y - c.y + 17.0) / 34.0), OW);
    // Scarf: a soft dusty-rose accent, like the machine roofs.
    cv.shape(move |p| ring(p, c + v(0.0, 3.0), 14.0, 5.0).max(c.y + 4.0 - p.y), |p| mix(ROOF_LIGHT, ROOF, (p.x - 14.0) / 36.0), 1.6);
    cv.solid(move |p| rbox(p, c.x + 9.0, c.y + 10.0, 5.0, 10.0, 2.5), ROOF_SHADE, 1.4);
    // Visor and eyes.
    cv.solid(move |p| rbox(p, c.x - 11.0, c.y - 9.0, 22.0, 11.0, 5.5), VISOR, 1.8);
    for dx in [-4.5, 4.5] {
        cv.fill(move |p| circle(p, c + v(dx, -3.5), 2.2), |_| LAMP);
    }
    cv.fill(move |p| circle(p, c + v(-8.0, -12.0), 2.5), |_| with_alpha(WHITE, 0.6));
}

pub fn mite(frame: u32, brute: bool) -> Canvas {
    let size = if brute { 72 } else { 48 };
    let mut cv = Canvas::new(size, size);
    let s = size as f32;
    let c = v(s / 2.0, s / 2.0);
    let r = v(s * 0.24, s * 0.19);
    ground_shadow(&mut cv, c + v(0.0, s * 0.12), r * 1.1, 0.35);
    let swing = if frame == 0 { 0.25 } else { -0.25 };
    for k in 0..3 {
        for side in [-1.0, 1.0] {
            let base = c + v((k as f32 - 1.0) * r.x * 0.6, 0.0);
            let a = side * (PI / 2.0) + (k as f32 - 1.0) * 0.6 * side + swing * if k % 2 == 0 { 1.0 } else { -1.0 };
            let knee = base + Vec2::from_angle(a) * s * 0.22;
            let foot = knee + Vec2::from_angle(a + side * 0.5) * s * 0.12;
            cv.solid(move |p| capsule(p, base, knee, s * 0.025).min(capsule(p, knee, foot, s * 0.02)), INK, 0.0);
        }
    }
    let (shell, dark) = if brute { (BRUTE_SHELL, BRUTE_DARK) } else { (MITE_SHELL, MITE_DARK) };
    cv.shape(move |p| ellipse(p, c, r), move |p| mix(shell, dark, (p.y - c.y + r.y) / (2.0 * r.y)), 2.2);
    cv.fill(move |p| capsule(p, c - v(r.x * 0.9, 0.0), c + v(r.x * 0.6, 0.0), 0.9), |_| with_alpha(INK, 0.7));
    let head = c + v(r.x * 1.05, 0.0);
    cv.solid(move |p| circle(p, head, s * 0.1), dark, 2.0);
    for dy in [-1.0, 1.0] {
        cv.fill(move |p| circle(p, head + v(s * 0.04, dy * s * 0.045), s * 0.025), |_| HONEY);
    }
    if brute {
        for k in 0..3 {
            let sp = c + v((k as f32 - 1.0) * r.x * 0.6, -r.y * 0.7);
            cv.solid(move |p| circle(p, sp, s * 0.035), CREAM, 1.2);
        }
    }
    cv
}

// ---------------------------------------------------------------- items

pub fn item(name: &str) -> Canvas {
    let mut cv = Canvas::new(48, 48);
    let c = v(24.0, 24.0);
    let o = OW_ITEM;
    let inner = o * 0.55;
    let sh = |cv: &mut Canvas, sdf: &dyn Fn(Vec2) -> f32| cv.shadow(sdf, v(2.0, 3.0), 2.5, 0.35);
    let ingot = |cv: &mut Canvas, col: C| {
        let pts = [v(10.0, 18.0), v(38.0, 18.0), v(43.0, 34.0), v(5.0, 34.0)];
        sh(cv, &move |p| poly(p, &pts));
        cv.shape(move |p| poly(p, &pts), move |p| mix(lit(col, 1.15), lit(col, 0.75), (p.y - 18.0) / 16.0), o);
        cv.fill(|p| rbox(p, 12.0, 20.0, 22.0, 2.0, 1.0), |_| with_alpha(WHITE, 0.5));
    };
    let chunk = |cv: &mut Canvas, body: C, facet: C| rock(cv, c, v(15.0, 12.0), body, facet, o);
    let vial = |cv: &mut Canvas, liquid: C| {
        let sdf = move |p: Vec2| circle(p, v(24.0, 30.0), 12.0).min(rbox(p, 19.0, 6.0, 10.0, 20.0, 2.0));
        sh(cv, &sdf);
        cv.solid(sdf, with_alpha(VIAL_GLASS, 0.9), o);
        cv.fill(|p| circle(p, v(24.0, 30.0), 9.5).max(25.0 - p.y), move |p| mix(lit(liquid, 1.2), lit(liquid, 0.8), (p.y - 25.0) / 15.0));
        cv.solid(|p| rbox(p, 17.0, 4.0, 14.0, 6.0, 2.0), BROWN, inner);
        cv.fill(|p| ellipse(p, v(19.0, 27.0), v(2.0, 4.0)), |_| with_alpha(WHITE, 0.6));
    };
    let crate_of = |cv: &mut Canvas, tip: C| {
        cv.solid(|p| rbox(p, 8.0, 20.0, 32.0, 20.0, 3.0), WOOD_DARK, o);
        for k in 0..4 {
            let x = 12.0 + k as f32 * 7.0;
            cv.solid(move |p| rbox(p, x, 8.0, 5.0, 14.0, 2.5), BRASS, inner);
            cv.fill(move |p| rbox(p, x, 8.0, 5.0, 5.0, 2.5), move |_| tip);
        }
        cv.solid(|p| rbox(p, 8.0, 20.0, 32.0, 20.0, 3.0), WOOD_DARK, o);
        cv.fill(|p| rbox(p, 14.0, 26.0, 20.0, 6.0, 2.0), |_| with_alpha(PAPER, 0.6));
    };
    match name {
        "iron_ore" => chunk(&mut cv, ORE_IRON, ORE_IRON_FACET),
        "copper_ore" => chunk(&mut cv, ORE_COPPER, ORE_COPPER_FACET),
        "coal" => chunk(&mut cv, ORE_COAL, ORE_COAL_FACET),
        "stone" => chunk(&mut cv, ORE_STONE, ORE_STONE_FACET),
        "quartz" => {
            let pts = [v(16.0, 40.0), v(14.0, 16.0), v(24.0, 5.0), v(34.0, 16.0), v(32.0, 40.0)];
            sh(&mut cv, &move |p| poly(p, &pts));
            cv.shape(move |p| poly(p, &pts), |p| mix(QUARTZ_LIGHT, QUARTZ_SHADE, (p.x - 14.0) / 20.0), o);
            cv.fill(|p| capsule(p, v(19.0, 16.0), v(20.0, 34.0), 1.2), |_| with_alpha(WHITE, 0.7));
        }
        "iron_plate" => ingot(&mut cv, IRON_PLATE),
        "copper_plate" => ingot(&mut cv, COPPER_PLATE),
        "steel" => {
            let sdf = |p: Vec2| rbox(p, 6.0, 14.0, 36.0, 6.0, 1.0).min(rbox(p, 6.0, 28.0, 36.0, 6.0, 1.0)).min(rbox(p, 21.0, 14.0, 6.0, 20.0, 0.0));
            sh(&mut cv, &sdf);
            cv.shape(sdf, |p| mix(STEEL_PLATE_HI, STEEL_PLATE_LO, (p.y - 14.0) / 20.0), o);
        }
        "brick" => {
            let sdf = |p: Vec2| rbox(p, 6.0, 15.0, 36.0, 20.0, 3.0);
            sh(&mut cv, &sdf);
            cv.shape(sdf, |p| mix(TERRA, RUST, (p.y - 15.0) / 20.0), o);
            cv.fill(|p| rbox(p, 6.0, 24.0, 36.0, 1.5, 0.0), |_| with_alpha(CREAM, 0.6));
        }
        "glass" => {
            let sdf = |p: Vec2| rbox(p, 8.0, 8.0, 32.0, 32.0, 4.0);
            sh(&mut cv, &sdf);
            cv.solid(sdf, with_alpha(GLASS, 0.85), o);
            cv.fill(|p| capsule(p, v(14.0, 30.0), v(28.0, 14.0), 2.0), |_| with_alpha(WHITE, 0.7));
        }
        "gear" => {
            let sdf = move |p: Vec2| gear(p, c, 13.0, 8, 4.5, 0.0).max(-circle(p, c, 5.0));
            sh(&mut cv, &sdf);
            cv.shape(sdf, |p| mix(STEEL_LIGHT, STEEL_D, p.y / 48.0), o);
        }
        "wire" => {
            for k in 0..4 {
                let cc = v(15.0 + k as f32 * 6.0, 24.0);
                cv.solid(move |p| ring(p, cc, 10.0, 3.0).max(cc.x - 3.0 - p.x), WIRE, inner);
            }
            cv.solid(|p| rbox(p, 10.0, 12.0, 28.0, 24.0, 6.0).max(-rbox(p, 13.0, 15.0, 22.0, 18.0, 4.0)), with_alpha(BROWN_D, 0.0), 0.0);
        }
        "rod" => {
            for k in 0..2 {
                let o = k as f32 * 9.0;
                let a = v(8.0 + o, 38.0);
                let b = v(32.0 + o, 10.0);
                cv.shadow(move |p| capsule(p, a, b, 3.5), v(2.0, 3.0), 2.0, 0.3);
                cv.solid(move |p| capsule(p, a, b, 3.5), ROD, inner);
            }
        }
        "circuit" => {
            let sdf = |p: Vec2| rbox(p, 7.0, 9.0, 34.0, 30.0, 4.0);
            sh(&mut cv, &sdf);
            cv.shape(sdf, |p| mix(CIRCUIT_HI, CIRCUIT_LO, p.y / 48.0), o);
            for (a, b) in [(v(12.0, 16.0), v(26.0, 16.0)), (v(26.0, 16.0), v(26.0, 30.0)), (v(14.0, 32.0), v(36.0, 32.0))] {
                cv.fill(move |p| capsule(p, a, b, 1.2), |_| HONEY);
            }
            cv.solid(|p| rbox(p, 29.0, 13.0, 9.0, 9.0, 1.5), CHAR, inner);
        }
        "motor" => {
            let sdf = |p: Vec2| rbox(p, 8.0, 12.0, 28.0, 24.0, 8.0);
            sh(&mut cv, &sdf);
            cv.shape(sdf, |p| mix(MOTOR_HI, MOTOR_LO, (p.y - 12.0) / 24.0), o);
            for k in 0..3 {
                let x = 14.0 + k as f32 * 6.0;
                cv.fill(move |p| rbox(p, x, 12.0, 2.5, 24.0, 1.0), |_| COPPER_PLATE);
            }
            cv.solid(|p| rbox(p, 34.0, 21.0, 9.0, 6.0, 2.0), STEEL, inner);
        }
        "lens" => {
            sh(&mut cv, &|p| circle(p, v(24.0, 24.0), 15.0));
            cv.solid(|p| circle(p, v(24.0, 24.0), 15.0), MUSTARD, o);
            cv.shape(|p| circle(p, v(24.0, 24.0), 11.0), |p| mix(GLASS_LIGHT, SKY, p.y / 48.0), inner);
            cv.fill(|p| ellipse(p, v(20.0, 19.0), v(4.0, 2.5)), |_| with_alpha(WHITE, 0.8));
        }
        "polymer" => {
            for (k, d) in [v(-7.0, 4.0), v(6.0, 5.0), v(0.0, -5.0), v(-1.0, 9.0)].into_iter().enumerate() {
                let cc = c + d;
                cv.shadow(move |p| circle(p, cc, 6.5), v(1.5, 2.0), 1.5, 0.25);
                let col = if k % 2 == 0 { POLYMER_A } else { POLYMER_B };
                cv.solid(move |p| circle(p, cc, 6.5), col, inner);
                cv.fill(move |p| circle(p, cc - v(2.0, 2.0), 1.6), |_| with_alpha(WHITE, 0.8));
            }
        }
        "processor" => {
            for k in 0..4 {
                let o = 13.0 + k as f32 * 7.0;
                cv.solid(move |p| rbox(p, o, 6.0, 3.0, 36.0, 1.0), HONEY, inner);
                cv.solid(move |p| rbox(p, 6.0, o, 36.0, 3.0, 1.0), HONEY, inner);
            }
            let sdf = |p: Vec2| rbox(p, 10.0, 10.0, 28.0, 28.0, 4.0);
            cv.shape(sdf, |p| mix(PROCESSOR_HI, PROCESSOR_LO, p.y / 48.0), o);
            cv.fill(|p| circle(p, v(24.0, 24.0), 6.0), |_| PROCESSOR_LED);
        }
        "fuel_cell" => {
            let sdf = |p: Vec2| rbox(p, 13.0, 8.0, 22.0, 32.0, 6.0);
            sh(&mut cv, &sdf);
            cv.shape(sdf, |p| mix(FUEL_HI, FUEL_LO, (p.x - 13.0) / 22.0), o);
            cv.solid(|p| rbox(p, 13.0, 20.0, 22.0, 6.0, 0.0), CREAM, inner);
            cv.solid(|p| rbox(p, 19.0, 4.0, 10.0, 6.0, 2.0), CHAR, inner);
        }
        "slug_clip" => crate_of(&mut cv, MUSTARD),
        "heavy_clip" => crate_of(&mut cv, TERRA),
        "aether_core" => {
            cv.glow(c, 22.0, with_alpha(AETHER_GLOW, 0.6));
            let pts = [v(24.0, 6.0), v(38.0, 20.0), v(24.0, 42.0), v(10.0, 20.0)];
            cv.shape(move |p| poly(p, &pts), |p| mix(AETHER_HI, AETHER_LO, p.y / 48.0), o);
            cv.fill(|p| poly(p, &[v(24.0, 9.0), v(30.0, 20.0), v(24.0, 22.0)]), |_| with_alpha(WHITE, 0.6));
        }
        "survey_data" => vial(&mut cv, DATA_RED),
        "mech_data" => vial(&mut cv, DATA_GREEN),
        "optic_data" => vial(&mut cv, DATA_BLUE),
        _ => vial(&mut cv, DATA_PURPLE),
    }
    cv
}

// ---------------------------------------------------------------- effects & UI

pub fn soft_glow() -> Canvas {
    let mut cv = Canvas::new(128, 128);
    let c = v(64.0, 64.0);
    cv.fill(|_| -1.0, move |p| {
        let t = (1.0 - p.distance(c) / 64.0).max(0.0);
        with_alpha(WHITE, t * t * (3.0 - 2.0 * t))
    });
    cv
}

pub fn smoke() -> Canvas {
    let mut cv = Canvas::new(64, 64);
    for (d, r) in [(v(0.0, 4.0), 18.0), (v(-10.0, -2.0), 13.0), (v(10.0, -4.0), 14.0), (v(0.0, -10.0), 12.0)] {
        let cc = v(32.0, 32.0) + d;
        cv.fill(move |p| circle(p, cc, r), move |p| with_alpha(SMOKE, 0.55 * (1.0 - p.distance(cc) / r).max(0.0).sqrt()));
    }
    cv
}

pub fn cloud() -> Canvas {
    let mut cv = Canvas::new(256, 256);
    let c = v(128.0, 128.0);
    cv.fill(|_| -1.0, move |p| {
        let n = fbm(p, 256.0, 77);
        let t = (1.0 - p.distance(c) / 120.0).max(0.0);
        let a = smooth(0.25, 0.7, t * 0.8 + n * 0.5 - 0.1);
        with_alpha(CLOUD, a)
    });
    cv
}

pub fn steam() -> Canvas {
    let mut cv = Canvas::new(64, 64);
    let c = v(32.0, 32.0);
    let blobs = [(v(0.0, 2.0), 16.0), (v(-9.0, -3.0), 11.0), (v(9.0, -4.0), 12.0), (v(0.0, -9.0), 10.0)];
    for &(d, r) in &blobs {
        let cc = c + d;
        cv.fill(move |p| circle(p, cc, r), move |p| {
            let t = (1.0 - p.distance(cc) / r).max(0.0);
            with_alpha(WHITE, 0.4 * t * t * (3.0 - 2.0 * t))
        });
    }
    cv
}

/// A butterfly seen from above: a dark body with two pairs of wings. `frame`
/// flaps the wings (0 = spread, 1 = raised). Wings are painted near-white so
/// the game's pastel tint colours each butterfly.
pub fn butterfly(frame: u32) -> Canvas {
    let mut cv = Canvas::new(32, 32);
    // Slim dark body along the vertical axis, with two antennae.
    cv.solid(move |p| capsule(p, v(16.0, 12.0), v(16.0, 20.0), 1.3), INK, 1.0);
    for s in [-1.0f32, 1.0] {
        cv.fill(move |p| capsule(p, v(16.0, 12.0), v(16.0 + s * 2.5, 9.0), 0.5), |_| INK);
    }
    let flap = if frame == 0 { 1.0 } else { 0.45 };
    for s in [-1.0f32, 1.0] {
        let dir = s;
        cv.solid(move |p| ellipse(p, v(16.0 + dir * 4.0, 14.5), v(9.0 * flap, 5.0)), WHITE, 1.0);
        cv.solid(move |p| ellipse(p, v(16.0 + dir * 3.0, 18.5), v(6.5 * flap, 3.6)), WHITE, 1.0);
    }
    cv
}

/// A single pastel-pink petal, pointed at the top.
pub fn petal() -> Canvas {
    let mut cv = Canvas::new(16, 16);
    let c = v(8.0, 8.0);
    let petal = move |p: Vec2| ellipse(p, c, v(3.6, 6.0));
    cv.shadow(petal, v(0.5, 1.0), 1.0, 0.2);
    cv.shape(petal, move |p| mix(lit(FLOWER_PINK, 1.12), FLOWER_PINK, ((p.y - 2.0) / 12.0).clamp(0.0, 1.0)), 0.8);
    cv.fill(move |p| capsule(p, v(8.0, 14.0), v(8.0, 3.0), 0.4), |_| with_alpha(WHITE, 0.35));
    cv
}

/// A V-shaped bird shadow seen from above: a short body with two wings swept
/// back. Painted as a soft dark cloud, so the game can draw it at low alpha.
pub fn bird_shadow() -> Canvas {
    let mut cv = Canvas::new(160, 64);
    let head = v(140.0, 32.0);
    let body_tail = v(70.0, 32.0);
    let strokes = [(body_tail, head, 7.0), (body_tail, v(18.0, 10.0), 6.0), (body_tail, v(18.0, 54.0), 6.0)];
    let dark = hex(crate::theme::SHADOW);
    for &(a, b, r) in &strokes {
        cv.fill(move |p| capsule(p, a, b, r), move |p| {
            let d = capsule(p, a, b, r);
            with_alpha(dark, smooth(r * 0.5, -r * 0.5, d) * 0.85)
        });
    }
    cv
}

pub fn icon(name: &str) -> Canvas {
    let mut cv = Canvas::new(32, 32);
    let o = OW_ICON;
    match name {
        "heart" => {
            let sdf = |p: Vec2| circle(p, v(11.0, 12.0), 6.5).min(circle(p, v(21.0, 12.0), 6.5)).min(poly(p, &[v(5.0, 14.0), v(27.0, 14.0), v(16.0, 27.0)]));
            cv.shape(sdf, |p| mix(HEART, TERRA, p.y / 32.0), o);
            cv.fill(|p| circle(p, v(10.0, 10.0), 2.0), |_| with_alpha(WHITE, 0.7));
        }
        "flask" => {
            let sdf = |p: Vec2| poly(p, &[v(12.0, 4.0), v(20.0, 4.0), v(20.0, 12.0), v(27.0, 27.0), v(5.0, 27.0), v(12.0, 12.0)]);
            cv.solid(sdf, GLASS_LIGHT, o);
            cv.fill(|p| poly(p, &[v(9.0, 19.0), v(23.0, 19.0), v(25.5, 25.5), v(6.5, 25.5)]), |_| TRIM);
        }
        "bolt" => {
            let pts = [v(18.0, 2.0), v(7.0, 18.0), v(15.0, 18.0), v(12.0, 30.0), v(25.0, 12.0), v(17.0, 12.0)];
            cv.shape(move |p| poly(p, &pts[..]).min(poly(p, &[pts[0], pts[1], pts[2], pts[5]])), |_| BRASS, o);
        }
        _ => {
            cv.solid(|p| circle(p, v(10.0, 24.0), 5.0).min(circle(p, v(23.0, 21.0), 5.0)), BRASS, o);
            cv.solid(|p| rbox(p, 13.0, 5.0, 3.0, 19.0, 1.0).min(rbox(p, 26.0, 3.0, 3.0, 18.0, 1.0)).min(rbox(p, 13.0, 3.0, 16.0, 5.0, 1.5)), BRASS, o);
        }
    }
    cv
}

// ---------------------------------------------------------------- cursors

/// Outline width for the cozy cursor sprites, in native pixels.
const OW_CURSOR: f32 = 3.0;

/// Dark trim outline, soft shadow and paper fill shared by the cursor sprites.
fn cursor_body(cv: &mut Canvas, sdf: impl Fn(Vec2) -> f32 + Copy, fill: impl Fn(Vec2) -> C) {
    let outline = hex(theme::TRIM_SHADOW);
    cv.shadow(sdf, v(2.0, 3.0), 3.0, 0.5);
    cv.fill(move |p| sdf(p) - OW_CURSOR, |_| outline);
    cv.fill(sdf, fill);
}

/// A rounded arrow cursor: cream-paper fill, wood-trim outline and a dusty-rose
/// tip. The arrow tip sits at the top-left hotspot (7, 7).
pub fn cursor_point() -> Canvas {
    let mut cv = Canvas::new(48, 48);
    let sdf = |p: Vec2| poly(p, &[v(8.0, 7.0), v(30.0, 21.0), v(21.0, 26.0), v(21.0, 38.0), v(13.0, 38.0), v(13.0, 25.0), v(16.0, 15.0)]);
    let fill = move |p: Vec2| mix(CREAM, PAPER, ((p.y - 7.0) / 31.0).clamp(0.0, 1.0));
    cursor_body(&mut cv, sdf, fill);
    // Dusty-rose lacquer tip.
    cv.fill(move |p| poly(p, &[v(8.0, 7.0), v(20.0, 14.0), v(13.0, 16.0)]), |_| hex(theme::LACQUER_500));
    cv
}

/// A soft mitten-like pointing hand in the same paper-and-wood style. The
/// fingertip sits at the hotspot (9, 9).
pub fn cursor_hand() -> Canvas {
    let mut cv = Canvas::new(48, 48);
    let finger = move |p: Vec2| capsule(p, v(9.0, 9.0), v(22.0, 23.0), 5.5);
    let body = move |p: Vec2| circle(p, v(28.0, 33.0), 13.0);
    let thumb = move |p: Vec2| capsule(p, v(21.0, 26.0), v(14.0, 36.0), 5.0);
    let sdf = move |p: Vec2| finger(p).min(body(p)).min(thumb(p));
    let fill = move |p: Vec2| mix(CREAM, PAPER, ((p.y - 9.0) / 30.0).clamp(0.0, 1.0));
    cursor_body(&mut cv, sdf, fill);
    cv
}

/// The point cursor plus a small brass "+" badge at the bottom-right.
pub fn cursor_build() -> Canvas {
    let mut cv = cursor_point();
    let c = v(37.0, 37.0);
    cv.shadow(move |p| circle(p, c, 8.0), v(1.0, 1.0), 1.5, 0.3);
    cv.solid(move |p| circle(p, c, 7.0), BRASS, 1.2);
    cv.solid(move |p| rbox(p, c.x - 4.0, c.y - 1.2, 8.0, 2.4, 0.6), CREAM, 0.0);
    cv.solid(move |p| rbox(p, c.x - 1.2, c.y - 4.0, 2.4, 8.0, 0.6), CREAM, 0.0);
    cv
}

/// A tiny pickaxe: a wooden handle under a brass head, for the mine cursor.
/// The pick tip sits at the hotspot (8, 14).
pub fn cursor_mine() -> Canvas {
    let mut cv = Canvas::new(48, 48);
    let outline = hex(theme::TRIM_SHADOW);
    let head = move |p: Vec2| capsule(p, v(7.0, 14.0), v(26.0, 19.0), 4.0);
    let handle = move |p: Vec2| capsule(p, v(17.0, 16.0), v(29.0, 40.0), 3.2);
    let sdf = move |p: Vec2| head(p).min(handle(p));
    cv.shadow(sdf, v(2.0, 3.0), 3.0, 0.5);
    cv.fill(move |p| sdf(p) - OW_CURSOR, |_| outline);
    cv.fill(head, move |p| mix(BRASS, COPPER, ((p.x - 7.0) / 19.0).clamp(0.0, 1.0)));
    cv.fill(handle, move |p| mix(WOOD, WOOD_DARK, ((p.y - 16.0) / 24.0).clamp(0.0, 1.0)));
    cv.solid(move |p| circle(p, v(17.0, 16.0), 3.0), BRASS, 0.0);
    cv
}

#[cfg(test)]
mod tests {
    use super::ground_sheet;
    use super::ore;
    use crate::paint::{C, Canvas};

    /// Mean absolute RGB difference between two same-length pixel strips.
    fn mean_diff(a: &[C], b: &[C]) -> f32 {
        assert_eq!(a.len(), b.len());
        let mut acc = 0.0;
        for (x, y) in a.iter().zip(b.iter()) {
            acc += (x[0] - y[0]).abs() + (x[1] - y[1]).abs() + (x[2] - y[2]).abs();
        }
        acc / (a.len() as f32 * 3.0)
    }

    /// The 16 ground tiles cut from the 256 sheet must continue into each other
    /// on every edge, including the wrap from the last column/row back to the
    /// first. This is what removes the visible tile grid.
    #[test]
    fn ground_tiles_are_seamless() {
        let sheet = ground_sheet();
        assert_eq!((sheet.w, sheet.h), (256, 256), "sheet must be 256x256");
        for j in 0..4usize {
            for i in 0..4usize {
                let right: Vec<C> = (0..64).map(|k| sheet.pixel(i * 64 + 63, j * 64 + k)).collect();
                let left: Vec<C> = (0..64).map(|k| sheet.pixel(((i + 1) % 4) * 64, j * 64 + k)).collect();
                assert!(mean_diff(&right, &left) < 0.04, "horizontal seam at tile ({i},{j}) = {:.3}", mean_diff(&right, &left));

                let bottom: Vec<C> = (0..64).map(|k| sheet.pixel(i * 64 + k, j * 64 + 63)).collect();
                let top: Vec<C> = (0..64).map(|k| sheet.pixel(i * 64 + k, ((j + 1) % 4) * 64)).collect();
                assert!(mean_diff(&bottom, &top) < 0.04, "vertical seam at tile ({i},{j}) = {:.3}", mean_diff(&bottom, &top));
            }
        }
    }

    /// Mean perceived luminance of the opaque pixels of a canvas.
    fn mean_luma(cv: &Canvas) -> f32 {
        let mut sum = 0.0;
        let mut n = 0usize;
        for y in 0..cv.h {
            for x in 0..cv.w {
                let c = cv.pixel(x, y);
                if c[3] > 0.01 {
                    sum += 0.299 * c[0] + 0.587 * c[1] + 0.114 * c[2];
                    n += 1;
                }
            }
        }
        sum / n as f32
    }

    /// Iron ore must read clearly lighter than coal by value, not just shape.
    #[test]
    fn iron_is_lighter_than_coal() {
        let iron = mean_luma(&ore("iron", 0));
        let coal = mean_luma(&ore("coal", 0));
        assert!(
            iron >= 1.8 * coal,
            "iron ore ({iron:.3}) must be clearly lighter than coal ({coal:.3})"
        );
    }
}

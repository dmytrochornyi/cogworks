//! Every sprite in the game, painted from code.

use std::f32::consts::{PI, TAU};

use macroquad::math::{Vec2, vec2};

use super::paint::*;

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
    cv.shadow(sdf, v(2.5, 3.0), 3.0, 0.35);
    cv.shape(sdf, move |p| mix(lit(body, 1.12), lit(body, 0.66), ((p.y - miny) / span).clamp(0.0, 1.0)), OW);
    let a = pts[0];
    let b = pts[1];
    let c = (a + b) * 0.5 + v(0.0, 2.0);
    cv.fill(move |p| poly(p, &[a, b, c]), |_| with_alpha(facet, 0.6));
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
    cv.shape(sdf_rail, |p| mix(BELT_RAIL_HI, BELT_RAIL_LO, (p.y % 64.0) / 64.0), 1.8);
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
            cv.fill(move |p| circle(p, c, 1.3), |_| with_alpha(CREAM, 0.8));
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
    cv.shape(move |p| circle(p, c, 18.0), move |p| mix(HONEY, RUST, (p.y - 14.0) / 36.0), OW);
    cv.solid(move |p| circle(p, c, 9.0), CHAR, 1.5);
    for k in 0..6 {
        let rc = c + Vec2::from_angle(k as f32 * TAU / 6.0) * 13.5;
        rivet(&mut cv, rc, 1.8);
    }
    cv
}

pub fn arm_limb() -> Canvas {
    let mut cv = Canvas::new(64, 16);
    cv.shape(|p| capsule(p, v(8.0, 8.0), v(56.0, 8.0), 5.0), |p| mix(CREAM, STEEL, (p.y - 3.0) / 10.0), OW_LIMB);
    cv.solid(|p| circle(p, v(8.0, 8.0), 3.0), BROWN_D, OW_LIMB * 0.5);
    cv
}

pub fn arm_hand() -> Canvas {
    let mut cv = Canvas::new(24, 24);
    for s in [-1.0, 1.0] {
        cv.solid(move |p| capsule(p, v(8.0, 12.0 + s * 3.0), v(19.0, 12.0 + s * 6.0), 2.4), STEEL_D, OW_HAND * 0.7);
    }
    cv.solid(|p| circle(p, v(9.0, 12.0), 5.0), TERRA, OW_HAND);
    cv
}

pub fn chest() -> Canvas {
    let mut cv = Canvas::new(64, 64);
    let wood = WOOD;
    let (x, y, w, h) = block(&mut cv, 7.0, 8.0, 50.0, 50.0, 11.0, 5.0, wood, side_of(wood));
    for k in 1..4 {
        let ly = y + h * k as f32 / 4.0;
        cv.fill(move |p| rbox(p, x + 4.0, ly - 0.8, w - 8.0, 1.6, 0.8), |_| with_alpha(INK, 0.45));
    }
    for (cx, cy) in [(x + 5.0, y + 5.0), (x + w - 5.0, y + 5.0), (x + 5.0, y + h - 4.0), (x + w - 5.0, y + h - 4.0)] {
        rivet(&mut cv, v(cx, cy), 2.6);
    }
    cv.solid(|p| rbox(p, 28.0, 46.0, 8.0, 8.0, 2.0), HONEY, 1.5);
    cv
}

pub fn pipe_hub() -> Canvas {
    let mut cv = Canvas::new(64, 64);
    let c = v(32.0, 32.0);
    cv.shadow(move |p| circle(p, c, 13.0), v(2.0, 3.0), 3.0, 0.3);
    cv.shape(move |p| circle(p, c, 13.0), move |p| mix(lit(STEEL, 1.15), STEEL_D, (p.y - 19.0) / 26.0), OW);
    for k in 0..4 {
        rivet(&mut cv, c + Vec2::from_angle(k as f32 * TAU / 4.0 + PI / 4.0) * 8.5, 1.6);
    }
    cv
}

/// Half a pipe, from the hub centre towards the east edge.
pub fn pipe_seg() -> Canvas {
    let mut cv = Canvas::new(32, 24);
    cv.fill(|p| rbox(p, -2.0, 3.0, 36.0, 18.0, 0.0), |p| mix(lit(STEEL, 1.2), STEEL_D, (p.y - 3.0) / 18.0));
    for y in [3.0, 19.5] {
        cv.fill(move |p| rbox(p, -2.0, y, 36.0, OW_PIPE, 0.0), |_| INK);
    }
    cv.fill(|p| rbox(p, 22.0, 1.0, 6.0, 22.0, 2.0), |_| STEEL_D);
    cv.fill(|p| rbox(p, 22.0, 1.0, 6.0, 22.0, 2.0).max(-rbox(p, 23.5, 2.5, 3.0, 19.0, 1.0)), |_| INK);
    cv
}

pub fn port(input: bool) -> Canvas {
    let mut cv = Canvas::new(24, 24);
    let col = if input { TEAL } else { PEACH };
    cv.solid(|p| circle(p, v(12.0, 12.0), 8.0), col, OW_PORT);
    cv.solid(|p| rbox(p, 7.0, 10.5, 10.0, 3.0, 1.5), CREAM, OW_PORT * 0.5);
    cv
}

// ---------------------------------------------------------------- production

pub fn drill() -> Canvas {
    let mut cv = Canvas::new(128, 128);
    let (x, y, w, h) = block(&mut cv, 6.0, 8.0, 116.0, 112.0, 16.0, 14.0, MUSTARD, side_of(MUSTARD));
    let c = v(x + w / 2.0, y + h / 2.0);
    cv.solid(move |p| circle(p, c, 34.0), DRILL_HOLE, OW);
    cv.solid(move |p| circle(p, c, 27.0), PIT, 1.5);
    for (cx, cy) in [(x + 12.0, y + 12.0), (x + w - 12.0, y + 12.0), (x + 12.0, y + h - 12.0), (x + w - 12.0, y + h - 12.0)] {
        rivet(&mut cv, v(cx, cy), 3.5);
    }
    // Hazard stripes along the front wall.
    cv.fill(move |p| rbox(p, x + 14.0, y + h + 4.0, w - 28.0, 6.0, 2.0), |p| {
        if ((p.x + p.y) / 8.0).rem_euclid(1.0) < 0.5 { CHAR } else { HONEY }
    });
    cv
}

pub fn drill_bit() -> Canvas {
    let mut cv = Canvas::new(64, 64);
    let c = v(32.0, 32.0);
    for k in 0..3 {
        let a = k as f32 * TAU / 3.0;
        let tip = c + Vec2::from_angle(a) * 24.0;
        let mid = c + Vec2::from_angle(a + 0.35) * 14.0;
        cv.shape(move |p| capsule(p, c, mid, 5.0).min(capsule(p, mid, tip, 3.5)), |p| mix(CREAM, STEEL, p.y / 64.0), OW_OVERLAY);
    }
    cv.solid(move |p| circle(p, c, 8.0), TERRA, OW_OVERLAY * 0.75);
    cv
}

pub fn chute() -> Canvas {
    let mut cv = Canvas::new(32, 32);
    let pts = [v(4.0, 6.0), v(28.0, 10.0), v(28.0, 22.0), v(4.0, 26.0)];
    cv.shape(move |p| poly(p, &pts), |p| mix(HONEY, RUST, p.y / 32.0), OW_CHUTE);
    cv
}

pub fn smelter() -> Canvas {
    let mut cv = Canvas::new(128, 128);
    let (x, y, w, h) = block(&mut cv, 6.0, 22.0, 116.0, 100.0, 22.0, 12.0, TERRA, RUST);
    // Brick courses on the top and the front wall.
    cv.fill(move |p| rbox(p, x + 4.0, y + 4.0, w - 8.0, h + 14.0, 8.0), move |p| {
        let row = ((p.y - y) / 11.0).floor();
        let fy = ((p.y - y) / 11.0).rem_euclid(1.0);
        let fx = ((p.x + row * 9.0) / 18.0).rem_euclid(1.0);
        if fy < 0.1 || fx < 0.06 { with_alpha(INK, 0.22) } else { with_alpha(INK, 0.0) }
    });
    // Arched firebox mouth on the front wall.
    cv.solid(move |p| rbox(p, 42.0, y + h - 8.0, 44.0, 30.0, 12.0), FIREBOX, OW);
    for k in 0..3 {
        let bx = 52.0 + k as f32 * 12.0;
        cv.fill(move |p| rbox(p, bx, y + h + 6.0, 3.0, 14.0, 1.0), |_| BROWN_D);
    }
    // Chimney.
    cv.shadow(|p| rbox(p, 86.0, 2.0, 22.0, 40.0, 4.0), v(4.0, 4.0), 4.0, 0.3);
    cv.shape(|p| rbox(p, 86.0, 6.0, 22.0, 36.0, 4.0), |p| mix(PEACH, RUST, (p.x - 86.0) / 22.0), OW);
    cv.solid(|p| rbox(p, 83.0, 2.0, 28.0, 8.0, 3.0), BROWN_D, OW);
    cv
}

pub fn assembler() -> Canvas {
    let mut cv = Canvas::new(192, 192);
    let (x, y, w, h) = block(&mut cv, 8.0, 10.0, 176.0, 174.0, 22.0, 16.0, TEAL, TEAL_D);
    let c = v(x + w / 2.0, y + h / 2.0);
    cv.shape(move |p| rrect(p, c, v(52.0, 46.0), 14.0), |p| mix(VENT_HI, VENT_LO, p.y / 192.0), OW);
    cv.fill(move |p| rrect(p, c - v(30.0, 30.0), v(14.0, 3.0), 3.0), |_| with_alpha(WHITE, 0.15));
    for (dx, dy) in [(-72.0, -62.0), (72.0, -62.0), (-72.0, 62.0), (72.0, 62.0)] {
        rivet(&mut cv, c + v(dx, dy), 4.0);
    }
    for k in 0..5 {
        let vx = x + 30.0 + k as f32 * 26.0;
        cv.fill(move |p| rbox(p, vx, y + h + 6.0, 16.0, 4.0, 2.0), |_| with_alpha(INK, 0.5));
    }
    cv.solid(move |p| circle(p, v(x + w - 18.0, y + h + 10.0), 4.0), HONEY, 1.5);
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
    let (x, y, w, h) = block(&mut cv, 8.0, 12.0, 176.0, 172.0, 22.0, 26.0, PAPER, BROWN);
    let c = v(x + w / 2.0, y + h / 2.0);
    for k in 0..4 {
        let a = k as f32 * TAU / 4.0 + PI / 4.0;
        let o = c + Vec2::from_angle(a) * 62.0;
        cv.solid(move |p| capsule(p, c, o, 6.0), MUSTARD, 2.0);
    }
    cv.solid(move |p| circle(p, c, 50.0), BROWN_D, OW);
    cv.shape(move |p| circle(p, c, 44.0), move |p| mix(GLASS_HI, SKY, (p.y - c.y + 44.0) / 88.0), OW);
    cv.fill(move |p| ellipse(p, c - v(16.0, 20.0), v(14.0, 7.0)), |_| with_alpha(WHITE, 0.5));
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
    let (x, y, w, h) = block(&mut cv, 6.0, 12.0, 116.0, 108.0, 18.0, 12.0, BROWN, side_of(BROWN));
    let c = v(x + w / 2.0 + 10.0, y + h / 2.0 + 4.0);
    cv.solid(move |p| circle(p, c, 32.0), PIT, OW);
    // Boiler drum along the top edge.
    cv.shape(move |p| rbox(p, x + 8.0, y + 6.0, 34.0, 70.0, 16.0), |p| mix(PEACH, TERRA, (p.x - 14.0) / 34.0), OW);
    for k in 0..3 {
        let by = y + 20.0 + k as f32 * 20.0;
        cv.fill(move |p| rbox(p, x + 8.0, by, 34.0, 3.0, 1.0), |_| with_alpha(INK, 0.6));
    }
    cv.solid(move |p| circle(p, v(x + 25.0, y + h - 14.0), 6.0), STEEL_D, 2.0);
    cv
}

pub fn wheel() -> Canvas {
    let mut cv = Canvas::new(64, 64);
    let c = v(32.0, 32.0);
    cv.solid(move |p| ring(p, c, 25.0, 6.0), MUSTARD, OW_OVERLAY);
    for k in 0..6 {
        let a = k as f32 * TAU / 6.0;
        let e = c + Vec2::from_angle(a) * 23.0;
        cv.solid(move |p| capsule(p, c, e, 2.2), HONEY, OW_OVERLAY * 0.6);
    }
    cv.solid(move |p| circle(p, c, 6.0), RUST, OW_OVERLAY);
    cv
}

/// Lamp post, 64x112: the bottom 64x64 is the tile it stands on.
pub fn pole() -> Canvas {
    let mut cv = Canvas::new(64, 112);
    ground_shadow(&mut cv, v(36.0, 98.0), v(14.0, 5.0), 0.4);
    let wood = WOOD_DARK;
    cv.shape(|p| rbox(p, 28.0, 12.0, 8.0, 88.0, 3.0), move |p| mix(lit(wood, 1.15), lit(wood, 0.8), (p.x - 28.0) / 8.0), 2.0);
    cv.shape(|p| rbox(p, 10.0, 16.0, 44.0, 7.0, 3.0), move |_| wood, 2.0);
    for x in [14.0, 50.0] {
        cv.solid(move |p| circle(p, v(x, 15.0), 3.0), CREAM, 1.2);
    }
    // Hanging lantern.
    cv.fill(|p| capsule(p, v(46.0, 23.0), v(46.0, 28.0), 0.8), |_| INK);
    cv.solid(|p| rbox(p, 40.0, 26.0, 12.0, 4.0, 1.5), BROWN_D, 1.2);
    cv.shape(|p| rbox(p, 41.0, 29.0, 10.0, 12.0, 3.0), |p| mix(LAMP_INNER, LAMP, (p.y - 29.0) / 12.0), 1.6);
    cv
}

pub fn solar() -> Canvas {
    let mut cv = Canvas::new(128, 128);
    let (x, y, w, h) = block(&mut cv, 6.0, 8.0, 116.0, 112.0, 10.0, 8.0, PAPER, BROWN);
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
    let (x, y, w, h) = block(&mut cv, 6.0, 10.0, 116.0, 110.0, 14.0, 10.0, WELLHEAD_STONE, WELLHEAD_STONE_D);
    let c = v(x + w / 2.0, y + h / 2.0);
    cv.solid(move |p| circle(p, c + v(-26.0, 16.0), 14.0), WELLHEAD_DARK, OW);
    cv.solid(move |p| circle(p, c + v(-26.0, 16.0), 7.0), BROWN_D, 1.5);
    // A-frame supporting the beam pivot.
    for s in [-1.0, 1.0] {
        let foot = c + v(14.0 + s * 18.0, 30.0);
        cv.solid(move |p| capsule(p, foot, c + v(14.0, -6.0), 4.0), RUST, 2.0);
    }
    cv
}

pub fn beam() -> Canvas {
    let mut cv = Canvas::new(112, 40);
    cv.shape(|p| capsule(p, v(14.0, 20.0), v(96.0, 20.0), 6.0), |p| mix(HONEY, MUSTARD, p.y / 40.0), OW_OVERLAY);
    let head = [v(8.0, 8.0), v(22.0, 10.0), v(22.0, 32.0), v(8.0, 34.0)];
    cv.solid(move |p| poly(p, &head), RUST, OW_OVERLAY);
    cv.solid(|p| circle(p, v(62.0, 20.0), 6.0), STEEL, OW_OVERLAY * 0.8);
    cv.solid(|p| rbox(p, 92.0, 10.0, 14.0, 20.0, 3.0), CHAR, OW_OVERLAY * 0.8);
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
    block(&mut cv, 8.0, 12.0, 176.0, 172.0, 16.0, 12.0, REFINERY_STONE, REFINERY_STONE_D);
    cv.solid(|p| capsule(p, v(58.0, 110.0), v(140.0, 110.0), 6.0), STEEL, 2.0);
    cv.solid(|p| capsule(p, v(58.0, 60.0), v(58.0, 140.0), 5.0), STEEL_D, 2.0);
    tank(&mut cv, v(58.0, 40.0), 30.0, 70.0, PEACH);
    tank(&mut cv, v(136.0, 54.0), 26.0, 56.0, TERRA);
    tank(&mut cv, v(126.0, 122.0), 20.0, 36.0, MUSTARD);
    cv
}

pub fn mixer() -> Canvas {
    let mut cv = Canvas::new(128, 128);
    let (x, y, w, h) = block(&mut cv, 6.0, 10.0, 116.0, 110.0, 16.0, 14.0, SAGE, SAGE_D);
    let c = v(x + w / 2.0, y + h / 2.0);
    cv.solid(move |p| circle(p, c, 38.0), STEEL_D, OW);
    cv.shape(move |p| circle(p, c, 31.0), move |p| mix(HONEY, MUSTARD, (p.y - c.y + 31.0) / 62.0), OW);
    cv.fill(move |p| ring(p, c, 18.0, 2.0), |_| with_alpha(WHITE, 0.3));
    cv
}

pub fn paddle() -> Canvas {
    let mut cv = Canvas::new(64, 64);
    let c = v(32.0, 32.0);
    cv.solid(move |p| capsule(p, c - v(24.0, 0.0), c + v(24.0, 0.0), 4.0), CREAM, OW_OVERLAY);
    cv.solid(move |p| capsule(p, c - v(0.0, 24.0), c + v(0.0, 24.0), 4.0), CREAM, OW_OVERLAY);
    cv.solid(move |p| circle(p, c, 7.0), TERRA, OW_OVERLAY);
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
        cv.fill(move |p| circle(p, c, r), |_| with_alpha(SAGE_D, 0.75));
    }
    cv
}

pub fn sentry_base() -> Canvas {
    let mut cv = Canvas::new(128, 128);
    let (x, y, w, h) = block(&mut cv, 8.0, 10.0, 112.0, 110.0, 16.0, 30.0, OLIVE, lit(OLIVE, 0.66));
    let c = v(x + w / 2.0, y + h / 2.0);
    cv.solid(move |p| ring(p, c, 34.0, 6.0), MUSTARD, 1.8);
    for k in 0..8 {
        rivet(&mut cv, c + Vec2::from_angle(k as f32 * TAU / 8.0) * 34.0, 2.0);
    }
    cv
}

/// Turret head facing east.
pub fn sentry_head() -> Canvas {
    let mut cv = Canvas::new(96, 96);
    let c = v(48.0, 48.0);
    cv.shadow(move |p| circle(p, c, 22.0), v(3.0, 4.0), 4.0, 0.35);
    cv.shape(|p| rbox(p, 50.0, 41.0, 42.0, 14.0, 5.0), |p| mix(STEEL, STEEL_D, (p.y - 41.0) / 14.0), OW_OVERLAY);
    cv.solid(|p| rbox(p, 84.0, 39.0, 9.0, 18.0, 3.0), CHAR, OW_OVERLAY * 0.8);
    cv.shape(move |p| circle(p, c, 22.0), move |p| mix(lit(SAGE, 1.1), OLIVE, (p.y - 26.0) / 44.0), OW_OVERLAY);
    cv.solid(move |p| rbox(p, 38.0, 42.0, 14.0, 6.0, 3.0), INK, 0.0);
    cv.fill(move |p| circle(p, v(46.0, 45.0), 2.2), |_| LAMP);
    cv
}

pub fn engine() -> Canvas {
    let mut cv = Canvas::new(256, 256);
    let (x, y, w, h) = block(&mut cv, 8.0, 10.0, 240.0, 236.0, 26.0, 24.0, PLUM, lit(PLUM, 0.62));
    let c = v(x + w / 2.0, y + h / 2.0);
    cv.solid(move |p| circle(p, c, 92.0), MUSTARD, OW);
    cv.solid(move |p| circle(p, c, 80.0), ENGINE_INNER, OW);
    for k in 0..12 {
        let a = k as f32 * TAU / 12.0;
        rivet(&mut cv, c + Vec2::from_angle(a) * 86.0, 2.8);
    }
    for (dx, dy) in [(-96.0, -92.0), (96.0, -92.0), (-96.0, 92.0), (96.0, 92.0)] {
        cv.solid(move |p| circle(p, c + v(dx, dy), 9.0), HONEY, 2.0);
    }
    cv
}

pub fn engine_ring() -> Canvas {
    let mut cv = Canvas::new(160, 160);
    let c = v(80.0, 80.0);
    cv.solid(move |p| ring(p, c, 64.0, 8.0), HONEY, OW_OVERLAY);
    for k in 0..16 {
        let a = k as f32 * TAU / 16.0;
        let m = c + Vec2::from_angle(a) * 64.0;
        let col = if k % 2 == 0 { ROSE } else { CREAM };
        cv.solid(move |p| circle(p, m, 4.0), col, OW_OVERLAY * 0.7);
    }
    cv.solid(move |p| ring(p, c, 44.0, 4.0), with_alpha(PEACH, 0.9), OW_OVERLAY * 0.7);
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
    // Antenna with a little glowing bulb.
    let tip = c + v(9.0, -22.0 + bob * 0.5);
    cv.solid(move |p| capsule(p, c + v(5.0, -12.0), tip, 1.4), CHAR, 0.0);
    cv.solid(move |p| circle(p, tip, 3.6), HONEY, 1.5);
    cv.shape(move |p| circle(p, c, 17.0), move |p| mix(CREAM, PAPER, (p.y - c.y + 17.0) / 34.0), OW);
    // Scarf.
    cv.shape(move |p| ring(p, c + v(0.0, 3.0), 14.0, 5.0).max(c.y + 4.0 - p.y), |p| mix(PEACH, TERRA, (p.x - 14.0) / 36.0), 1.6);
    cv.solid(move |p| rbox(p, c.x + 9.0, c.y + 10.0, 5.0, 10.0, 2.5), TERRA, 1.4);
    // Visor and eyes.
    cv.solid(move |p| rbox(p, c.x - 11.0, c.y - 9.0, 22.0, 11.0, 5.5), VISOR, 1.8);
    for dx in [-4.5, 4.5] {
        cv.fill(move |p| circle(p, c + v(dx, -3.5), 2.2), |_| LAMP);
    }
    cv.fill(move |p| circle(p, c + v(-8.0, -12.0), 2.5), |_| with_alpha(WHITE, 0.6));
    cv
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
        cv.solid(|p| rbox(p, 8.0, 20.0, 32.0, 20.0, 3.0), OLIVE, o);
        for k in 0..4 {
            let x = 12.0 + k as f32 * 7.0;
            cv.solid(move |p| rbox(p, x, 8.0, 5.0, 14.0, 2.5), HONEY, inner);
            cv.fill(move |p| rbox(p, x, 8.0, 5.0, 5.0, 2.5), move |_| tip);
        }
        cv.solid(|p| rbox(p, 8.0, 20.0, 32.0, 20.0, 3.0), OLIVE, o);
        cv.fill(|p| rbox(p, 14.0, 26.0, 20.0, 6.0, 2.0), |_| with_alpha(CREAM, 0.7));
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
            cv.solid(sdf, CREAM, o);
            cv.fill(|p| poly(p, &[v(9.0, 19.0), v(23.0, 19.0), v(25.5, 25.5), v(6.5, 25.5)]), |_| TEAL);
        }
        "bolt" => {
            let pts = [v(18.0, 2.0), v(7.0, 18.0), v(15.0, 18.0), v(12.0, 30.0), v(25.0, 12.0), v(17.0, 12.0)];
            cv.shape(move |p| poly(p, &pts[..]).min(poly(p, &[pts[0], pts[1], pts[2], pts[5]])), |_| HONEY, o);
        }
        _ => {
            cv.solid(|p| circle(p, v(10.0, 24.0), 5.0).min(circle(p, v(23.0, 21.0), 5.0)), CREAM, o);
            cv.solid(|p| rbox(p, 13.0, 5.0, 3.0, 19.0, 1.0).min(rbox(p, 26.0, 3.0, 3.0, 18.0, 1.0)).min(rbox(p, 13.0, 3.0, 16.0, 5.0, 1.5)), CREAM, o);
        }
    }
    cv
}

#[cfg(test)]
mod tests {
    use super::ground_sheet;
    use crate::paint::C;

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
}

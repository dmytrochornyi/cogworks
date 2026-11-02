use std::f32::consts::{FRAC_PI_2, PI};

use macroquad::prelude::*;

use crate::assets::{Assets, SpriteParams};
use crate::items::*;
use crate::theme;
use crate::ui::{circle, circle_lines};
use crate::world::*;

use super::actors::{draw_enemy, draw_player};
use super::camera::Cam;
use super::ground::draw_ground;
use super::overlays::draw_wires;
use super::{BADGE, HONEY, INK, PX, SAGE, TERRA, angle, lantern_seed, rgba, sway, world_bar};

fn item_key(item: Item) -> Option<&'static str> {
    Some(match item {
        Item::IronOre => "item_iron_ore",
        Item::CopperOre => "item_copper_ore",
        Item::Coal => "item_coal",
        Item::Stone => "item_stone",
        Item::Quartz => "item_quartz",
        Item::IronPlate => "item_iron_plate",
        Item::CopperPlate => "item_copper_plate",
        Item::Brick => "item_brick",
        Item::Steel => "item_steel",
        Item::Glass => "item_glass",
        Item::Gear => "item_gear",
        Item::Wire => "item_wire",
        Item::Rod => "item_rod",
        Item::Circuit => "item_circuit",
        Item::Motor => "item_motor",
        Item::Lens => "item_lens",
        Item::Polymer => "item_polymer",
        Item::Processor => "item_processor",
        Item::FuelCell => "item_fuel_cell",
        Item::SlugClip => "item_slug_clip",
        Item::HeavyClip => "item_heavy_clip",
        Item::AetherCore => "item_aether_core",
        Item::SurveyData => "item_survey_data",
        Item::MechData => "item_mech_data",
        Item::OpticData => "item_optic_data",
        Item::QuantumData => "item_quantum_data",
        _ => return None,
    })
}

fn body_key(kind: Kind) -> &'static str {
    match kind {
        Kind::Belt => "belt_0",
        Kind::Tunnel => "tunnel",
        Kind::Splitter => "splitter",
        Kind::Arm => "arm_base",
        Kind::Chest => "chest",
        Kind::Drill => "drill",
        Kind::Smelter => "smelter",
        Kind::Assembler => "assembler",
        Kind::Lab => "lab",
        Kind::Generator => "generator",
        Kind::Pole => "pole",
        Kind::Solar => "solar",
        Kind::Pipe => "pipe_hub",
        Kind::Wellhead => "wellhead",
        Kind::Refinery => "refinery",
        Kind::Mixer => "mixer",
        Kind::Wall => "wall",
        Kind::Sentry => "sentry_base",
        Kind::GrandEngine => "engine",
        Kind::Hive => "hive",
    }
}

/// Draws an item icon centred at `c`, `size` pixels across.
pub fn draw_item(a: &Assets, c: Vec2, size: f32, item: Item) {
    draw_item_tint(a, c, size, item, WHITE);
}

/// An item icon with a tint, used to grey out unavailable slots.
pub fn draw_item_tint(a: &Assets, c: Vec2, size: f32, item: Item, tint: Color) {
    match item_key(item) {
        Some(key) => a.draw_centered(key, c, Vec2::splat(size), 0.0, tint),
        None => {
            if let Some(kind) = item.place_kind() {
                let native = a.size(body_key(kind));
                let fit = size / native.max_element();
                let d = dims(kind, Dir::E).as_vec2();
                let z = fit * PX;
                // Centre the footprint, not the sprite (poles are taller than their tile).
                let s0 = c - d * z * 0.5 + vec2(0.0, (native.y / PX - d.y).max(0.0) * z * 0.5);
                draw_kind(a, kind, Dir::E, s0, z, tint);
            }
        }
    }
}

/// Draws a building's static look with its footprint's top-left at screen `s0`.
pub fn draw_kind(a: &Assets, kind: Kind, dir: Dir, s0: Vec2, z: f32, tint: Color) {
    let k = z / PX;
    let d = dims(kind, dir).as_vec2() * z;
    let c = s0 + d * 0.5;
    let p = SpriteParams { tint, ..Default::default() };
    match kind {
        Kind::Belt => a.draw("belt_0", s0, Vec2::splat(z), SpriteParams { rotation: angle(dir), ..p }),
        Kind::Tunnel => a.draw("tunnel", s0, Vec2::splat(z), SpriteParams { rotation: angle(dir), ..p }),
        Kind::Splitter => {
            let size = vec2(2.0 * z, z);
            a.draw("splitter", c - size * 0.5, size, SpriteParams { rotation: angle(dir) + FRAC_PI_2, ..p });
        }
        Kind::Arm => {
            a.draw("arm_base", s0, Vec2::splat(z), p);
            draw_arm_limb(a, c, arm_limb_angle(None, dir), z, tint);
        }
        Kind::Pipe => a.draw("pipe_hub", s0, Vec2::splat(z), p),
        Kind::Pole => {
            a.draw("pole", s0 - vec2(0.0, 48.0 * k), vec2(64.0, 112.0) * k, p);
            draw_lantern(a, s0, k, 0.0, tint);
        }
        _ => {
            let key = body_key(kind);
            a.draw(key, s0, a.size(key) * k, p);
            if kind == Kind::Drill {
                let cell = edge_tile(IVec2::ZERO, dims(kind, dir), dir) - dir.vec();
                let cc = s0 + (cell.as_vec2() + Vec2::splat(0.5)) * z + dir.vec().as_vec2() * z * 0.42;
                a.draw_centered("chute", cc, Vec2::splat(z * 0.5), angle(dir), tint);
            }
        }
    }
    if kind.has_fluid_in() {
        port(a, s0, dims(kind, dir), dir.opposite(), z, "port_in", tint);
    }
    if kind.has_fluid_out() {
        port(a, s0, dims(kind, dir), dir, z, "port_out", tint);
    }
}

fn port(a: &Assets, s0: Vec2, d: IVec2, dir: Dir, z: f32, key: &str, tint: Color) {
    let cell = edge_tile(IVec2::ZERO, d, dir) - dir.vec();
    let c = s0 + (cell.as_vec2() + Vec2::splat(0.5) + dir.vec().as_vec2() * 0.38) * z;
    a.draw_centered(key, c, Vec2::splat(z * 0.42), 0.0, tint);
}

fn draw_arm_limb(a: &Assets, c: Vec2, ang: f32, z: f32, tint: Color) {
    let k = z / PX;
    a.draw("arm_limb", c - vec2(8.0, 8.0) * k, vec2(64.0, 16.0) * k, SpriteParams {
        rotation: ang,
        pivot: Some(c),
        tint,
        ..Default::default()
    });
    let hand = c + Vec2::from_angle(ang) * 48.0 * k;
    a.draw_centered("arm_hand", hand, Vec2::splat(24.0 * k), ang, tint);
}

/// The resting limb angle for an arm facing `dir`: it points at the pickup side
/// (the tile behind the base), from which it swings items to the drop side.
pub fn arm_idle_angle(dir: Dir) -> f32 {
    (-dir.vec().as_vec2()).to_angle()
}

/// Limb angle shared by the placement ghost and the placed arm. The ghost has
/// no swing yet, so `None` means idle; a placed arm adds `swing * PI` so a full
/// swing reaches the drop side.
fn arm_limb_angle(swing: Option<f32>, dir: Dir) -> f32 {
    arm_idle_angle(dir) + swing.unwrap_or(0.0) * PI
}

/// World position of an item on a belt lane at progress `p`, or None if it's underground.
fn belt_item_pos(world: &World, id: usize, lane: usize, p: f32) -> Option<Vec2> {
    let e = world.entity(id);
    let State::Belt(b) = &e.state else {
        return None;
    };
    if e.kind == Kind::Tunnel && !b.exit && p > 0.5 {
        return None;
    }
    let c = e.pos.as_vec2() + Vec2::splat(0.5);
    let f = e.dir.vec().as_vec2();
    let left = e.dir.left().as_vec2();
    let sign = if lane == 0 { 1.0 } else { -1.0 };
    if let Some(side) = world.curve_side(id) {
        let s = side.as_vec2();
        let corner = c + (s + f) * 0.5;
        let d_in = -s;
        let left_in = vec2(d_in.y, -d_in.x);
        let r = 0.5 + left_in.dot(-f) * 0.2 * sign;
        let th = p.clamp(0.0, 1.0) * FRAC_PI_2;
        return Some(corner + (-f * th.cos() - s * th.sin()) * r);
    }
    Some(c + f * (p.min(1.0) - 0.5) + left * 0.2 * sign)
}

fn draw_pipe(a: &Assets, world: &World, e: &Entity, s0: Vec2, z: f32) {
    let c = s0 + Vec2::splat(z / 2.0);
    let k = z / PX;
    for d in Dir::ALL {
        let n = e.pos + d.vec();
        let linked = world.entity_ref_at(n).is_some_and(|o| o.kind == Kind::Pipe) || world.pipe_port_link(e.pos, n);
        if linked {
            a.draw("pipe_seg", c - vec2(0.0, 12.0 * k), vec2(32.0, 24.0) * k, SpriteParams {
                rotation: angle(d),
                pivot: Some(c),
                ..Default::default()
            });
        }
    }
    a.draw("pipe_hub", s0, Vec2::splat(z), SpriteParams::default());
    let fill = e.fnet.and_then(|n| world.fluids.nets.get(n)).and_then(|n| n.fluid.map(|f| (f, n.amount / n.cap.max(1.0))));
    if let Some((f, frac)) = fill {
        draw_circle(c.x, c.y, z * 0.1, rgba(f.color(), 0.5 + 0.5 * frac));
    }
}

enum Drawable<'a> {
    Entity(&'a Entity),
    Player,
    Enemy(&'a Enemy),
}

pub fn draw_world(a: &Assets, world: &World, cam: &Cam, t: f32) {
    let (lo, hi) = cam.visible();
    let z = cam.zoom;
    let k = z / PX;
    draw_ground(a, world, cam, lo, hi, t);

    let on_screen: Vec<(usize, &Entity)> = world
        .entities
        .iter()
        .enumerate()
        .filter_map(|(id, e)| e.as_ref().map(|e| (id, e)))
        .filter(|(_, e)| {
            let d = e.dims();
            e.pos.x + d.x > lo.x - 1 && e.pos.y + d.y > lo.y - 1 && e.pos.x <= hi.x + 1 && e.pos.y <= hi.y + 2
        })
        .collect();
    let flat = |kind: Kind| matches!(kind, Kind::Belt | Kind::Tunnel | Kind::Splitter | Kind::Pipe);
    let frame = ((t * BELT_SPEED / 0.25 * 8.0) as usize) % 8;

    for &(id, e) in on_screen.iter().filter(|(_, e)| flat(e.kind)) {
        let s0 = cam.w2s(e.pos.as_vec2());
        match &e.state {
            State::Belt(b) if e.kind == Kind::Tunnel => a.draw("tunnel", s0, Vec2::splat(z), SpriteParams {
                rotation: angle(e.dir),
                flip_x: b.exit,
                ..Default::default()
            }),
            State::Belt(_) => match world.curve_side(id) {
                Some(side) => a.draw(&format!("belt_curve_{frame}"), s0, Vec2::splat(z), SpriteParams {
                    rotation: angle(e.dir),
                    flip_y: side == e.dir.left(),
                    ..Default::default()
                }),
                None => a.draw(&format!("belt_{frame}"), s0, Vec2::splat(z), SpriteParams {
                    rotation: angle(e.dir),
                    ..Default::default()
                }),
            },
            State::Pipe { .. } => draw_pipe(a, world, e, s0, z),
            _ => draw_kind(a, e.kind, e.dir, s0, z, WHITE),
        }
    }
    for &(id, e) in &on_screen {
        match &e.state {
            State::Belt(b) => {
                for (lane, items) in b.lanes.iter().enumerate() {
                    for &(item, p) in items {
                        if let Some(wp) = belt_item_pos(world, id, lane, p) {
                            draw_item(a, cam.w2s(wp), z * 0.4, item);
                        }
                    }
                }
            }
            State::Splitter { lanes, .. } => {
                let c = cam.w2s(e.center());
                for (lane, items) in lanes.iter().enumerate() {
                    if let Some(&item) = items.first() {
                        let off = e.dir.left().as_vec2() * if lane == 0 { 0.2 } else { -0.2 };
                        draw_item(a, c + off * z, z * 0.4, item);
                    }
                }
            }
            _ => {}
        }
    }

    // Everything standing up is drawn back to front.
    let mut stand: Vec<(f32, Drawable)> = on_screen
        .iter()
        .filter(|(_, e)| !flat(e.kind))
        .map(|&(_, e)| ((e.pos.y + e.dims().y) as f32, Drawable::Entity(e)))
        .collect();
    stand.extend(world.enemies.iter().map(|en| (en.pos.y + 0.3, Drawable::Enemy(en))));
    if world.player.alive() {
        stand.push((world.player.pos.y + 0.3, Drawable::Player));
    }
    stand.sort_by(|x, y| x.0.total_cmp(&y.0));
    for (_, d) in &stand {
        match d {
            Drawable::Entity(e) => draw_entity(a, e, cam, t),
            Drawable::Enemy(en) => draw_enemy(a, en, cam),
            Drawable::Player => draw_player(a, &world.player, cam, t),
        }
    }

    draw_wires(world, cam);

    // Status badges: recipe, progress, power and damage.
    for &(_, e) in &on_screen {
        let s0 = cam.w2s(e.pos.as_vec2());
        let size = e.dims().as_vec2() * z;
        if let State::Crafter(c) = &e.state {
            let badge = s0 + Vec2::splat(z * 0.32);
            if let Some(out) = c.recipe.and_then(|r| RECIPES[r].main_output()) {
                circle(badge, z * 0.24, rgba(BADGE, 0.92));
                circle_lines(badge, z * 0.24, (z * 0.022).clamp(1.2, 2.0), rgba(INK, 0.95));
                draw_item(a, badge, z * 0.34, out);
            } else if let Some((f, _)) = c.recipe.and_then(|r| RECIPES[r].fluid_out) {
                circle(badge, z * 0.2, f.color());
                circle_lines(badge, z * 0.2, (z * 0.022).clamp(1.2, 2.0), rgba(INK, 0.95));
            }
            if c.crafting {
                let w = size.x - z * 0.5;
                let center = vec2(s0.x + z * 0.25 + w / 2.0, s0.y + size.y - z * 0.14);
                world_bar(center, w, c.progress, HONEY, z);
            }
        }
        let starving = e.kind.draw_kw() > 0.0
            && match e.net {
                None => true,
                Some(n) => e.active && world.power.nets.get(n).is_some_and(|n| n.sat < 0.5),
            };
        if starving && e.kind != Kind::Arm && (t * 1.5).fract() < 0.65 {
            a.draw_centered("icon_bolt", s0 + size * 0.5, Vec2::splat(z * 0.55), 0.0, WHITE);
        }
        if e.hp < e.kind.max_hp() {
            let frac = e.hp / e.kind.max_hp();
            let col = if frac > 0.5 { SAGE } else { TERRA };
            let center = vec2(s0.x + size.x / 2.0, s0.y - z * 0.16);
            world_bar(center, size.x - 4.0, frac, col, z);
        }
    }
    for s in &world.shots {
        let (p, q) = (cam.w2s(s.from), cam.w2s(s.to));
        draw_line(p.x, p.y, q.x, q.y, (k * 4.0).max(1.5), theme::alpha(theme::LAMP, 0.9));
    }
}

fn draw_entity(a: &Assets, e: &Entity, cam: &Cam, t: f32) {
    let z = cam.zoom;
    let k = z / PX;
    let s0 = cam.w2s(e.pos.as_vec2());
    let at = |px: f32, py: f32| s0 + vec2(px, py) * k;
    let spin = |speed: f32| if e.active { t * speed } else { 0.0 };
    match &e.state {
        State::Arm { held, swing } => {
            let c = s0 + Vec2::splat(z / 2.0);
            a.draw("arm_base", s0, Vec2::splat(z), SpriteParams::default());
            let ang = arm_limb_angle(Some(*swing), e.dir);
            draw_arm_limb(a, c, ang, z, WHITE);
            if let Some(item) = held {
                draw_item(a, c + Vec2::from_angle(ang) * 48.0 * k, z * 0.36, *item);
            }
            return;
        }
        State::Sentry { aim, .. } => {
            draw_kind(a, e.kind, e.dir, s0, z, WHITE);
            a.draw_centered("sentry_head", at(64.0, 46.0), Vec2::splat(88.0 * k), *aim, WHITE);
            return;
        }
        _ => {}
    }
    if e.kind == Kind::Hive {
        let pulse = 1.0 + 0.03 * (t * 2.0 + e.pos.x as f32).sin();
        let size = vec2(192.0, 192.0) * k * pulse;
        a.draw_centered("hive", s0 + vec2(96.0, 96.0) * k, size, 0.0, WHITE);
        return;
    }
    if e.kind == Kind::Pole {
        // The lantern is its own sprite so it can swing gently on its hook.
        let sw = sway(t, lantern_seed(e.pos.as_vec2()), 0.06, 3.5);
        a.draw("pole", s0 - vec2(0.0, 48.0 * k), vec2(64.0, 112.0) * k, SpriteParams::default());
        draw_lantern(a, s0, k, sw, WHITE);
        return;
    }
    draw_kind(a, e.kind, e.dir, s0, z, WHITE);
    match e.kind {
        Kind::Drill => a.draw_centered("drill_bit", at(63.0, 47.0), Vec2::splat(44.0 * k), spin(5.0), WHITE),
        Kind::Smelter if e.active => {
            let breathe = 0.925 + 0.075 * (t * 1.1 + e.pos.x as f32 * 2.3).sin();
            a.draw_centered("glow", at(64.0, 104.0), vec2(70.0, 40.0) * k, 0.0, theme::rgba(1.0, 0.55, 0.2, 0.75 * breathe));
        }
        Kind::Assembler => a.draw_centered("assembler_gear", at(97.0, 68.0), Vec2::splat(80.0 * k), spin(1.6), WHITE),
        Kind::Lab => {
            let pulse = if e.active { 1.0 + 0.1 * (t * 4.0).sin() } else { 0.75 };
            let tint = if e.active { WHITE } else { theme::rgba(0.7, 0.75, 0.75, 0.6) };
            a.draw_centered("lab_orb", at(95.0, 92.0), Vec2::splat(64.0 * k * pulse), 0.0, tint);
        }
        Kind::Generator => a.draw_centered("generator_wheel", at(87.0, 63.0), Vec2::splat(44.0 * k), spin(6.0), WHITE),
        Kind::Wellhead => {
            let rock = if e.active { (t * 2.5).sin() * 0.22 } else { 0.0 };
            let pivot = at(64.0, 30.0);
            a.draw("wellhead_beam", pivot - vec2(62.0, 20.0) * k, vec2(112.0, 40.0) * k, SpriteParams {
                rotation: rock,
                pivot: Some(pivot),
                ..Default::default()
            });
        }
        Kind::Mixer => a.draw_centered("mixer_paddle", at(64.0, 33.0), Vec2::splat(56.0 * k), spin(4.0), WHITE),
        Kind::GrandEngine => {
            let cores = match e.state {
                State::Engine { cores } => cores as f32 / ENGINE_CORES as f32,
                _ => 0.0,
            };
            a.draw_centered("engine_ring", at(128.0, 97.0), Vec2::splat(132.0 * k), t * (0.2 + cores * 2.0), WHITE);
            let pulse = 0.7 + 0.3 * cores + 0.05 * (t * 3.0).sin();
            a.draw_centered("engine_core", at(128.0, 97.0), Vec2::splat(76.0 * k * pulse), 0.0, WHITE);
        }
        _ => {}
    }
}

/// Draws the lantern hanging from a pole at `s0` (the pole's footprint top-left)
/// with a scale factor `k`. It is its own sprite (64x48) whose hook top sits at
/// local (32, 4); we place that point on the pole's hook and rotate about it.
fn draw_lantern(a: &Assets, s0: Vec2, k: f32, rotation: f32, tint: Color) {
    let hook = s0 + vec2(46.0, -27.0) * k;
    let top_left = hook - vec2(32.0, 4.0) * k;
    a.draw("lantern", top_left, vec2(64.0, 48.0) * k, SpriteParams {
        rotation,
        pivot: Some(hook),
        tint,
        ..Default::default()
    });
}

/// Steam vent point per machine, in sprite pixels from the sprite's top-left.
/// Each matches the brass vent painted into the corresponding sprite in
/// `gen_assets`. Drives the steam puffs that rise from active machines.
pub const VENTS: &[(Kind, Vec2)] = &[
    (Kind::Smelter, Vec2::new(97.0, 6.0)),
    (Kind::Assembler, Vec2::new(154.0, 24.0)),
    (Kind::Lab, Vec2::new(150.0, 32.0)),
    (Kind::Generator, Vec2::new(31.0, 20.0)),
    (Kind::Refinery, Vec2::new(58.0, 36.0)),
    (Kind::Mixer, Vec2::new(94.0, 24.0)),
    (Kind::Drill, Vec2::new(96.0, 22.0)),
    (Kind::Wellhead, Vec2::new(46.0, 24.0)),
    (Kind::GrandEngine, Vec2::new(128.0, 22.0)),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arm_ghost_matches_placed_arm() {
        for d in Dir::ALL {
            let idle = arm_idle_angle(d);
            assert!((arm_limb_angle(None, d) - idle).abs() < 1e-5, "ghost {d:?} != idle");
            assert!((arm_limb_angle(Some(0.0), d) - idle).abs() < 1e-5, "placed {d:?} != idle");
        }
        let west = arm_idle_angle(Dir::E);
        assert!((west.abs() - PI).abs() < 1e-5, "Dir::E should point west, got {west}");
    }
}

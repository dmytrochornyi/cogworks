use std::f32::consts::{FRAC_PI_2, PI, TAU};

use macroquad::prelude::*;

use crate::assets::{Assets, SpriteParams};
use crate::items::*;
use crate::theme::{self, line};
use crate::ui::{circle, circle_lines, rrect, rrect_dashed_lines, rrect_lines};
use crate::world::*;

/// Sprite pixels per tile.
const PX: f32 = 64.0;
/// Seconds per full day.
pub const DAY_LENGTH: f32 = 480.0;
const WIND: Vec2 = Vec2::new(0.35, -0.12);

pub struct Cam {
    /// World position (in tiles) at the centre of the screen.
    pub pos: Vec2,
    /// Pixels per tile.
    pub zoom: f32,
}

impl Cam {
    fn half_screen() -> Vec2 {
        vec2(screen_width(), screen_height()) * 0.5
    }

    pub fn w2s(&self, p: Vec2) -> Vec2 {
        (p - self.pos) * self.zoom + Self::half_screen()
    }

    pub fn s2w(&self, s: Vec2) -> Vec2 {
        (s - Self::half_screen()) / self.zoom + self.pos
    }

    /// Inclusive range of tiles on screen, clamped to the map.
    fn visible(&self) -> (IVec2, IVec2) {
        let lo = self.s2w(Vec2::ZERO).floor().as_ivec2().max(IVec2::ZERO);
        let hi = self
            .s2w(vec2(screen_width(), screen_height()))
            .floor()
            .as_ivec2()
            .min(ivec2(MAP_W - 1, MAP_H - 1));
        (lo, hi)
    }

    fn on_screen(&self, p: Vec2, margin: f32) -> bool {
        let s = self.w2s(p);
        let m = margin * self.zoom;
        s.x > -m && s.y > -m && s.x < screen_width() + m && s.y < screen_height() + m
    }
}

fn hash(x: i32, y: i32) -> u32 {
    let mut h = (x as u32).wrapping_mul(374_761_393) ^ (y as u32).wrapping_mul(668_265_263);
    h = (h ^ (h >> 13)).wrapping_mul(1_274_126_177);
    h ^ (h >> 16)
}

fn rgba(c: Color, a: f32) -> Color {
    theme::with_alpha(c, a)
}

const INK: Color = theme::color(theme::INK);
const HONEY: Color = theme::color(theme::HONEY);
const TERRA: Color = theme::color(theme::TERRA);
const SAGE: Color = theme::color(theme::SAGE);
const BADGE: Color = theme::color(theme::BADGE);

/// Height of an in-world bar in screen pixels: follows the camera but never
/// drops below 3 px, so damage stays readable on a zoomed-out map.
fn bar_height(zoom: f32) -> f32 {
    (zoom * 0.075).clamp(3.0, 11.0)
}

/// Rounded, ink-outlined bar drawn in screen space. Its height follows the camera
/// zoom while never dropping below 3 px, so health stays readable when zoomed out.
fn world_bar(center: Vec2, w: f32, frac: f32, col: Color, zoom: f32) {
    let h = bar_height(zoom);
    let r = Rect::new(center.x - w / 2.0, center.y - h / 2.0, w.max(h), h);
    rrect(r, h / 2.0, rgba(INK, 0.82));
    let f = frac.clamp(0.0, 1.0);
    if f > 0.0 {
        let fw = (r.w * f).max(h);
        rrect(Rect::new(r.x, r.y, fw, h), h / 2.0, col);
        if fw > h + 3.0 {
            rrect(
                Rect::new(r.x + 1.5, r.y + 1.0, fw - 3.0, (h / 2.0 - 1.0).max(1.0)),
                h / 2.0,
                rgba(theme::color(theme::WHITE), 0.18),
            );
        }
    }
    rrect_lines(r, h / 2.0, line::HAIR * 1.2, rgba(INK, 0.95));
}

fn angle(d: Dir) -> f32 {
    match d {
        Dir::E => 0.0,
        Dir::S => FRAC_PI_2,
        Dir::W => PI,
        Dir::N => -FRAC_PI_2,
    }
}

// ------------------------------------------------------------------ time of day

pub struct Sky {
    /// Ambient light multiplier.
    pub ambient: Color,
    /// 0 at noon, 1 at midnight.
    pub night: f32,
    pub label: &'static str,
}

pub fn sky(time: f32) -> Sky {
    // Start the first day in the morning.
    let p = (time / DAY_LENGTH + 0.3).fract();
    const KEYS: [(f32, [f32; 3], &str); 8] = [
        (0.00, [0.30, 0.30, 0.50], "Night"),
        (0.22, [0.78, 0.58, 0.60], "Dawn"),
        (0.30, [1.00, 0.92, 0.82], "Morning"),
        (0.50, [1.00, 0.98, 0.93], "Midday"),
        (0.68, [1.00, 0.84, 0.64], "Golden hour"),
        (0.77, [0.88, 0.56, 0.56], "Sunset"),
        (0.85, [0.46, 0.40, 0.64], "Dusk"),
        (1.00, [0.30, 0.30, 0.50], "Night"),
    ];
    let k = KEYS.iter().rposition(|(t, _, _)| *t <= p).unwrap_or(0).min(KEYS.len() - 2);
    let (t0, c0, label) = KEYS[k];
    let (t1, c1, _) = KEYS[k + 1];
    let f = ((p - t0) / (t1 - t0)).clamp(0.0, 1.0);
    let f = f * f * (3.0 - 2.0 * f);
    let c: Vec<f32> = (0..3).map(|i| c0[i] + (c1[i] - c0[i]) * f).collect();
    let lum = (c[0] + c[1] + c[2]) / 3.0;
    Sky {
        ambient: theme::rgba(c[0], c[1], c[2], 1.0),
        night: ((0.95 - lum) / 0.55).clamp(0.0, 1.0),
        label,
    }
}

// ------------------------------------------------------------------ sprites

fn ore_key(item: Item) -> &'static str {
    match item {
        Item::IronOre => "iron",
        Item::CopperOre => "copper",
        Item::Coal => "coal",
        Item::Stone => "stone",
        _ => "quartz",
    }
}

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
            draw_arm_limb(a, c, angle(dir) - FRAC_PI_2, z, tint);
        }
        Kind::Pipe => a.draw("pipe_hub", s0, Vec2::splat(z), p),
        Kind::Pole => a.draw("pole", s0 - vec2(0.0, 48.0 * k), vec2(64.0, 112.0) * k, p),
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

// ------------------------------------------------------------------ particles

#[derive(Clone, Copy, PartialEq)]
enum PKind {
    Smoke,
    Firefly,
    Mote,
    Spark,
}

struct Particle {
    pos: Vec2,
    vel: Vec2,
    life: f32,
    max: f32,
    size: f32,
    kind: PKind,
    seed: f32,
}

pub struct Renderer {
    world_rt: Option<RenderTarget>,
    light_rt: Option<RenderTarget>,
    rt_size: (u32, u32),
    particles: Vec<Particle>,
    rng: u64,
}

impl Renderer {
    pub fn new() -> Self {
        Renderer { world_rt: None, light_rt: None, rt_size: (0, 0), particles: Vec::new(), rng: 0x1234_5678 }
    }

    fn rand(&mut self) -> f32 {
        self.rng ^= self.rng << 13;
        self.rng ^= self.rng >> 7;
        self.rng ^= self.rng << 17;
        (self.rng >> 40) as f32 / (1u64 << 24) as f32
    }

    fn ensure_targets(&mut self) {
        let size = (screen_width().max(1.0) as u32, screen_height().max(1.0) as u32);
        if self.rt_size != size || self.world_rt.is_none() {
            let make = || {
                let rt = render_target(size.0, size.1);
                rt.texture.set_filter(FilterMode::Linear);
                rt
            };
            self.world_rt = Some(make());
            self.light_rt = Some(make());
            self.rt_size = size;
        }
    }

    fn update_particles(&mut self, world: &World, cam: &Cam, dt: f32, night: f32) {
        for p in &mut self.particles {
            p.life -= dt;
            p.pos += p.vel * dt;
            match p.kind {
                PKind::Smoke => p.vel += WIND * dt * 0.6,
                PKind::Firefly | PKind::Mote => {
                    p.vel = Vec2::from_angle(p.seed + p.life * 0.9) * if p.kind == PKind::Mote { 0.25 } else { 0.5 };
                }
                PKind::Spark => p.vel *= 0.9,
            }
        }
        self.particles.retain(|p| p.life > 0.0);

        // Smoke from working furnaces and engines.
        for e in world.entities.iter().flatten() {
            if !e.active || !cam.on_screen(e.center(), 4.0) {
                continue;
            }
            let (vent, rate) = match e.kind {
                Kind::Smelter => (e.pos.as_vec2() + vec2(97.0, 6.0) / PX, 3.0),
                Kind::Generator => (e.pos.as_vec2() + vec2(31.0, 80.0) / PX, 2.0),
                Kind::Refinery => (e.pos.as_vec2() + vec2(58.0, 36.0) / PX, 1.5),
                _ => continue,
            };
            if self.rand() < rate * dt {
                let (r1, r2) = (self.rand(), self.rand());
                self.particles.push(Particle {
                    pos: vent,
                    vel: vec2(r1 * 0.3 - 0.15, -0.5 - r2 * 0.3),
                    life: 3.0,
                    max: 3.0,
                    size: 0.5 + r1 * 0.3,
                    kind: PKind::Smoke,
                    seed: r2,
                });
            }
        }
        // Fireflies at night, golden dust by day, around the camera.
        let half = Cam::half_screen() / cam.zoom;
        for (kind, want) in [(PKind::Firefly, (night * 40.0) as usize), (PKind::Mote, ((1.0 - night) * 22.0) as usize)] {
            let have = self.particles.iter().filter(|p| p.kind == kind).count();
            if have < want {
                let (r1, r2, r3) = (self.rand(), self.rand(), self.rand());
                let pos = cam.pos + vec2(r1 * 2.0 - 1.0, r2 * 2.0 - 1.0) * half;
                self.particles.push(Particle { pos, vel: Vec2::ZERO, life: 6.0 + r3 * 6.0, max: 12.0, size: 0.08, kind, seed: r3 * TAU });
            }
        }
        for s in &world.shots {
            for _ in 0..2 {
                let (r1, r2) = (self.rand(), self.rand());
                self.particles.push(Particle {
                    pos: s.to,
                    vel: Vec2::from_angle(r1 * TAU) * (1.0 + r2 * 2.0),
                    life: 0.25,
                    max: 0.25,
                    size: 0.06,
                    kind: PKind::Spark,
                    seed: r1,
                });
            }
        }
    }

    /// Renders the lit, graded world to the screen.
    pub fn frame(&mut self, a: &Assets, world: &World, cam: &Cam, t: f32, dt: f32, show_pollution: bool) {
        self.ensure_targets();
        let sky = sky(world.time);
        self.update_particles(world, cam, dt, sky.night);
        let (w, h) = (self.rt_size.0 as f32, self.rt_size.1 as f32);
        let world_rt = self.world_rt.clone().unwrap();
        let light_rt = self.light_rt.clone().unwrap();

        let mut rt_cam = Camera2D::from_display_rect(Rect::new(0.0, 0.0, w, h));
        rt_cam.render_target = Some(world_rt.clone());
        set_camera(&rt_cam);
        clear_background(theme::color(theme::WORLD_CLEAR));
        draw_world(a, world, cam, t);
        self.draw_particles(a, cam);
        draw_clouds(a, cam, world.time, 1.0 - sky.night);
        if show_pollution {
            draw_pollution(world, cam);
        }

        let mut light_cam = Camera2D::from_display_rect(Rect::new(0.0, 0.0, w, h));
        light_cam.render_target = Some(light_rt.clone());
        set_camera(&light_cam);
        let amb = sky.ambient;
        clear_background(theme::rgba(amb.r * 0.5, amb.g * 0.5, amb.b * 0.5, 1.0));
        gl_use_material(&a.additive);
        draw_lights(a, world, cam, t, sky.night);
        for p in self.particles.iter().filter(|p| p.kind == PKind::Firefly) {
            let blink = 0.5 + 0.5 * (t * 3.0 + p.seed * 10.0).sin();
            light(a, cam, p.pos, 0.7, theme::rgba(0.9, 1.0, 0.5, 0.5 * blink * sky.night));
        }
        gl_use_default_material();

        set_default_camera();
        a.composite.set_texture("Light", light_rt.texture.clone());
        a.composite.set_uniform("Params", vec4(0.0, 0.0, 0.55, 0.35));
        a.composite.set_uniform("Screen", vec2(w, h));
        gl_use_material(&a.composite);
        // The render targets are stored bottom-up relative to the default
        // framebuffer, so flip the composited world back upright. Without this,
        // the world (and thus movement) appears vertically inverted.
        draw_texture_ex(&world_rt.texture, 0.0, 0.0, WHITE, DrawTextureParams {
            dest_size: Some(vec2(screen_width(), screen_height())),
            flip_y: true,
            ..Default::default()
        });
        gl_use_default_material();
    }

    fn draw_particles(&self, a: &Assets, cam: &Cam) {
        let z = cam.zoom;
        for p in &self.particles {
            let s = cam.w2s(p.pos);
            let f = p.life / p.max;
            match p.kind {
                PKind::Smoke => {
                    let size = (p.size + (1.0 - f) * 0.9) * z;
                    a.draw_centered("smoke", s, Vec2::splat(size), p.seed * TAU, rgba(WHITE, f * 0.7));
                }
                PKind::Firefly => draw_circle(s.x, s.y, 0.05 * z, theme::rgba(1.0, 1.0, 0.7, 0.9)),
                PKind::Mote => draw_circle(s.x, s.y, 0.035 * z, theme::rgba(1.0, 0.95, 0.8, 0.35 * f.min(1.0 - f) * 2.0)),
                PKind::Spark => draw_circle(s.x, s.y, 0.05 * z, theme::rgba(1.0, 0.85, 0.5, f)),
            }
        }
    }
}

// ------------------------------------------------------------------ world

fn draw_ground(a: &Assets, world: &World, cam: &Cam, lo: IVec2, hi: IVec2) {
    let z = cam.zoom;
    for y in lo.y..=hi.y {
        for x in lo.x..=hi.x {
            let s = cam.w2s(ivec2(x, y).as_vec2());
            let key = ["grass_0", "grass_1", "grass_2", "grass_3"][(hash(x, y) % 4) as usize];
            a.draw(key, s, Vec2::splat(z + 0.6), SpriteParams::default());
        }
    }
    // Overlays can spill onto neighbours, so they go in a second pass.
    for y in lo.y..=hi.y {
        for x in lo.x..=hi.x {
            let p = ivec2(x, y);
            let s = cam.w2s(p.as_vec2());
            let h = hash(x * 3 + 1, y * 7 + 2);
            match world.res_at(p) {
                Some(Resource::Ore(o)) => {
                    // Disturbed soil under the rocks, then a jittered cluster so fields don't look gridded.
                    draw_rectangle(s.x, s.y, z + 0.6, z + 0.6, theme::alpha(theme::DIRT, 0.16));
                    let key = format!("ore_{}_{}", ore_key(o), h % 3);
                    let jitter = vec2(((h >> 4) % 100) as f32 - 50.0, ((h >> 11) % 100) as f32 - 50.0) / 100.0 * 0.35 * z;
                    let scale = 0.8 + ((h >> 18) % 40) as f32 / 100.0;
                    let size = Vec2::splat(z * scale);
                    a.draw(&key, s + Vec2::splat(z * 0.5) - size * 0.5 + jitter, size, SpriteParams {
                        flip_x: (h >> 25) & 1 == 1,
                        ..Default::default()
                    });
                }
                Some(Resource::Tar) => a.draw(["tar_0", "tar_1"][(h % 2) as usize], s, Vec2::splat(z), SpriteParams::default()),
                None if h % 100 < 9 && world.entity_at(p).is_none() => {
                    let key = format!("decor_{}", (h >> 8) % 8);
                    let off = vec2(((h >> 12) % 32) as f32, ((h >> 17) % 32) as f32) / 64.0 * z;
                    a.draw(&key, s + off, Vec2::splat(z * 0.5), SpriteParams::default());
                }
                None => {}
            }
        }
    }
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
    draw_ground(a, world, cam, lo, hi);

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
            let ang = (-e.dir.vec().as_vec2()).to_angle() + swing * PI;
            draw_arm_limb(a, c, ang, z, WHITE);
            if let Some(item) = held {
                draw_item(a, c + Vec2::from_angle(ang) * 48.0 * k, z * 0.36, *item);
            }
            return;
        }
        State::Sentry { aim, .. } => {
            draw_kind(a, e.kind, e.dir, s0, z, WHITE);
            a.draw_centered("sentry_head", at(64.0, 57.0), Vec2::splat(96.0 * k), *aim, WHITE);
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
    draw_kind(a, e.kind, e.dir, s0, z, WHITE);
    match e.kind {
        Kind::Drill => a.draw_centered("drill_bit", at(64.0, 56.0), Vec2::splat(64.0 * k), spin(5.0), WHITE),
        Kind::Smelter if e.active => {
            let flick = 0.8 + 0.2 * (t * 9.0 + e.pos.x as f32).sin();
            a.draw_centered("glow", at(64.0, 104.0), vec2(70.0, 40.0) * k, 0.0, theme::rgba(1.0, 0.55, 0.2, 0.75 * flick));
        }
        Kind::Assembler => a.draw_centered("assembler_gear", at(96.0, 86.0), Vec2::splat(80.0 * k), spin(1.6), WHITE),
        Kind::Lab => {
            let pulse = if e.active { 1.0 + 0.1 * (t * 4.0).sin() } else { 0.75 };
            let tint = if e.active { WHITE } else { theme::rgba(0.7, 0.75, 0.75, 0.6) };
            a.draw_centered("lab_orb", at(96.0, 87.0), Vec2::splat(64.0 * k * pulse), 0.0, tint);
        }
        Kind::Generator => a.draw_centered("generator_wheel", at(74.0, 61.0), Vec2::splat(64.0 * k), spin(6.0), WHITE),
        Kind::Wellhead => {
            let rock = if e.active { (t * 2.5).sin() * 0.22 } else { 0.0 };
            let pivot = at(78.0, 52.0);
            a.draw("wellhead_beam", pivot - vec2(62.0, 20.0) * k, vec2(112.0, 40.0) * k, SpriteParams {
                rotation: rock,
                pivot: Some(pivot),
                ..Default::default()
            });
        }
        Kind::Mixer => a.draw_centered("mixer_paddle", at(64.0, 57.0), Vec2::splat(56.0 * k), spin(4.0), WHITE),
        Kind::GrandEngine => {
            let cores = match e.state {
                State::Engine { cores } => cores as f32 / ENGINE_CORES as f32,
                _ => 0.0,
            };
            a.draw_centered("engine_ring", at(128.0, 115.0), Vec2::splat(160.0 * k), t * (0.2 + cores * 2.0), WHITE);
            let pulse = 0.7 + 0.3 * cores + 0.05 * (t * 3.0).sin();
            a.draw_centered("engine_core", at(128.0, 115.0), Vec2::splat(96.0 * k * pulse), 0.0, WHITE);
        }
        _ => {}
    }
}

fn draw_enemy(a: &Assets, en: &Enemy, cam: &Cam) {
    let z = cam.zoom;
    let c = cam.w2s(en.pos);
    let frame = (en.phase * 0.6) as i32 % 2;
    let (key, px) = if en.brute { (["brute_0", "brute_1"][frame as usize], 72.0) } else { (["mite_0", "mite_1"][frame as usize], 48.0) };
    let heading = (en.target - en.pos).to_angle();
    a.draw_centered(key, c, Vec2::splat(px / PX * z), heading, WHITE);
    if en.hp < en.max_hp() {
        let w = px / PX * z * 0.6;
        world_bar(vec2(c.x, c.y - w * 0.8), w, en.hp / en.max_hp(), TERRA, z);
    }
}

fn draw_player(a: &Assets, player: &Player, cam: &Cam, t: f32) {
    let walking = player.walk != Vec2::ZERO;
    let frame = if walking { (t * 8.0) as i32 % 2 } else { 0 };
    let c = cam.w2s(player.pos - vec2(0.0, 0.2));
    let size = Vec2::splat(cam.zoom * 1.05);
    a.draw(["player_0", "player_1"][frame as usize], c - size * 0.5, size, SpriteParams {
        flip_x: player.walk.x < 0.0,
        ..Default::default()
    });
}

/// Lantern position on a pole, in tiles from its top-left.
const LANTERN: Vec2 = Vec2::new(46.0 / PX, (35.0 - 48.0) / PX);
const WIRE_ANCHOR: Vec2 = Vec2::new(0.5, (16.0 - 48.0) / PX);

fn draw_wires(world: &World, cam: &Cam) {
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

fn draw_clouds(a: &Assets, cam: &Cam, time: f32, day: f32) {
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

fn draw_pollution(world: &World, cam: &Cam) {
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

fn light(a: &Assets, cam: &Cam, pos: Vec2, radius: f32, col: Color) {
    if !cam.on_screen(pos, radius) {
        return;
    }
    let size = Vec2::splat(radius * 2.0 * cam.zoom);
    a.draw_centered("glow", cam.w2s(pos), size, 0.0, theme::rgba(col.r * 0.5, col.g * 0.5, col.b * 0.5, col.a));
}

fn draw_lights(a: &Assets, world: &World, cam: &Cam, t: f32, night: f32) {
    let warm = theme::color(theme::LAMP);
    for e in world.entities.iter().flatten() {
        let p = e.pos.as_vec2();
        let flick = 0.92 + 0.08 * (t * 7.0 + p.x * 3.1).sin();
        let px = |x: f32, y: f32| p + vec2(x, y) / PX;
        match e.kind {
            Kind::Pole => light(a, cam, p + LANTERN, 3.6, rgba(warm, night * 0.9 * flick)),
            Kind::Smelter if e.active => light(a, cam, px(64.0, 104.0), 2.6, theme::rgba(1.0, 0.5, 0.2, (0.3 + night * 0.7) * flick)),
            Kind::Generator if e.active => light(a, cam, px(74.0, 61.0), 2.0, theme::rgba(1.0, 0.7, 0.35, 0.15 + night * 0.5)),
            Kind::Lab if e.active => light(a, cam, px(96.0, 87.0), 3.0, theme::rgba(0.5, 0.95, 0.9, 0.25 + night * 0.6)),
            Kind::Assembler if e.active => light(a, cam, px(96.0, 86.0), 1.8, theme::rgba(0.9, 0.8, 0.5, night * 0.5)),
            Kind::Sentry if e.active => light(a, cam, px(64.0, 57.0), 1.6, theme::rgba(1.0, 0.85, 0.5, 0.5 * flick)),
            Kind::GrandEngine => light(a, cam, px(128.0, 115.0), 5.0, theme::rgba(0.75, 0.5, 1.0, 0.3 + night * 0.6)),
            Kind::Hive => light(a, cam, px(96.0, 96.0), 3.0, theme::rgba(0.95, 0.4, 0.7, 0.15 + night * 0.45)),
            _ => {}
        }
    }
    if world.player.alive() {
        light(a, cam, world.player.pos, 4.5, rgba(warm, night * 0.75));
    }
    for s in &world.shots {
        light(a, cam, s.from, 1.4, theme::rgba(1.0, 0.85, 0.5, 0.6));
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
    if kind.directional() && !matches!(kind, Kind::Belt | Kind::Drill | Kind::Tunnel | Kind::Splitter | Kind::Arm) {
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
    fn world_bar_height_scales_with_zoom_and_has_a_floor() {
        assert_eq!(bar_height(0.0), 3.0);
        assert!((bar_height(52.0) - 3.9).abs() < 1e-6);
        assert_eq!(bar_height(10_000.0), 11.0);
    }

    #[test]
    fn preview_corners_stay_small() {
        assert_eq!(radius_of(Vec2::splat(20.0)), 4.0);
        assert_eq!(radius_of(Vec2::splat(200.0)), 12.0);
    }
}

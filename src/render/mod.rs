use std::f32::consts::{FRAC_PI_2, PI, TAU};

use macroquad::prelude::*;

use crate::assets::{Assets, SpriteParams};
use crate::theme::{self, line};
use crate::ui::{rrect, rrect_lines};
use crate::world::*;

mod actors;
mod camera;
mod entities;
mod ground;
mod lights;
mod overlays;
mod particles;
mod sky;

pub use camera::Cam;
pub use entities::{draw_item, draw_item_tint, draw_world, VENTS};
pub use overlays::draw_ghost;
pub use sky::{DAY_LENGTH, sky};

// Re-exported so `crate::render` keeps the same public API as before the
// split. These items aren't referenced through this path inside the crate, so
// silence the import lint for them.
#[allow(unused_imports)]
pub use entities::draw_kind;
#[allow(unused_imports)]
pub use particles::{MAX_BUTTERFLIES, MAX_PETALS, MAX_STEAM};
#[allow(unused_imports)]
pub use sky::{Sky, composite_rgb, sky_at_phase};

use lights::{draw_lights, light};
use overlays::{draw_clouds, draw_pollution};
use particles::{PKind, Particle, SpawnEnv, advance_particles, butterfly_color, spawn_particles, vent_rate};

use actors::walk_footfalls;

/// Sprite pixels per tile.
const PX: f32 = 64.0;
const WIND: Vec2 = Vec2::new(0.35, -0.12);

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

/// A smooth sine sway with a per-`seed` phase. `amp` is the amplitude (radians
/// where used for rotation, tiles where used for offsets) and `period` the full
/// cycle time in seconds. Everything in the world moves through this one gentle
/// curve, so nothing flickers.
pub fn sway(t: f32, seed: f32, amp: f32, period: f32) -> f32 {
    (t / period * TAU + seed).sin() * amp
}

/// Per-pole phase so neighbouring lanterns don't swing in lockstep.
fn lantern_seed(pos: Vec2) -> f32 {
    pos.x * 1.7 + pos.y * 0.9
}

pub struct Renderer {
    world_rt: Option<RenderTarget>,
    light_rt: Option<RenderTarget>,
    rt_size: (u32, u32),
    particles: Vec<Particle>,
    rng: u64,
    /// Seconds until the next bird shadow crosses the view.
    bird_timer: f32,
    /// Footfall pairs seen so far, so a dust puff fires once per pair.
    player_foot_pairs: u64,
}

impl Renderer {
    pub fn new() -> Self {
        Renderer { world_rt: None, light_rt: None, rt_size: (0, 0), particles: Vec::new(), rng: 0x1234_5678, bird_timer: 6.0, player_foot_pairs: 0 }
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
        advance_particles(&mut self.particles, dt);

        // Collect the active machines' spawn sources in world coordinates, so
        // the pure spawn function below can be unit-tested without a window.
        let mut vents = Vec::new();
        let mut furnaces = Vec::new();
        for e in world.entities.iter().flatten() {
            if !e.active || !cam.on_screen(e.center(), 4.0) {
                continue;
            }
            if let Some(&(_, vp)) = VENTS.iter().find(|(k, _)| *k == e.kind) {
                vents.push((e.pos.as_vec2() + vp / PX, vent_rate(e.kind)));
            }
            if e.kind == Kind::Smelter {
                furnaces.push(e.pos.as_vec2() + vec2(64.0, 104.0) / PX);
            }
        }
        let half = Cam::half_screen() / cam.zoom;
        let shot_pos: Vec<Vec2> = world.shots.iter().map(|s| s.to).collect();
        // Dust puffs under the player's feet, once per pair of footfalls. Stride
        // only advances on real movement, so standing still never puffs.
        let mut footfalls = Vec::new();
        if world.player.alive() {
            let pairs = walk_footfalls(world.player.stride) / 2;
            if pairs > self.player_foot_pairs {
                footfalls.push(world.player.pos + vec2(0.0, 0.25));
            }
            self.player_foot_pairs = pairs;
        }
        let env = SpawnEnv { dt, night, cam_pos: cam.pos, half, vents: &vents, furnaces: &furnaces, shots: &shot_pos, footfalls: &footfalls };
        spawn_particles(&mut self.rng, &mut self.particles, &env, &mut self.bird_timer);
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
        self.draw_particles(a, cam, t);
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
        draw_lights(a, world, cam, t, sky.glow);
        for p in self.particles.iter().filter(|p| p.kind == PKind::Firefly) {
            let blink = 0.5 + 0.5 * (t * 3.0 + p.seed * 10.0).sin();
            light(a, cam, p.pos, 0.45, theme::rgba(1.0, 0.88, 0.5, 0.5 * blink * sky.glow));
        }
        gl_use_default_material();

        set_default_camera();
        a.composite.set_texture("Light", light_rt.texture.clone());
        a.composite.set_uniform("Params", vec4(0.0, 0.0, 0.55, 0.35));
        a.composite.set_uniform("Tint", vec4(sky.tint.r, sky.tint.g, sky.tint.b, sky.tint.a));
        a.composite.set_uniform("Ambient", vec4(amb.r, amb.g, amb.b, 0.0));
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

    fn draw_particles(&self, a: &Assets, cam: &Cam, t: f32) {
        let z = cam.zoom;
        for p in &self.particles {
            let s = cam.w2s(p.pos);
            let f = p.life / p.max;
            match p.kind {
                PKind::Steam => {
                    // Soft white, rising, growing and fading over its 2-3 s life.
                    let grow = 1.0 - f;
                    let size = (p.size + grow * 0.8) * z;
                    a.draw_centered("steam", s, Vec2::splat(size), p.seed * TAU, rgba(WHITE, f * 0.6));
                }
                PKind::Dust => {
                    // Small, short-lived dirt puff under the player's feet.
                    let grow = 1.0 - f;
                    let size = (p.size + grow * 0.5) * z;
                    a.draw_centered("steam", s, Vec2::splat(size), p.seed * TAU, theme::alpha(theme::DIRT, f * 0.35));
                }
                PKind::Firefly => draw_circle(s.x, s.y, 0.05 * z, theme::rgba(1.0, 1.0, 0.7, 0.9)),
                PKind::Mote => draw_circle(s.x, s.y, 0.035 * z, theme::rgba(1.0, 0.95, 0.8, 0.45 * f.min(1.0 - f) * 2.0)),
                PKind::Spark => draw_circle(s.x, s.y, 0.05 * z, theme::rgba(1.0, 0.85, 0.5, f)),
                PKind::Butterfly => {
                    let frame = ((t * 6.0) as i32).rem_euclid(2) as usize;
                    let key = ["butterfly_0", "butterfly_1"][frame];
                    let bank = (p.seed + p.life * 1.5).sin() * 0.4;
                    a.draw_centered(key, s, Vec2::splat(p.size * z), bank, butterfly_color(p.seed));
                }
                PKind::Petal => {
                    let spin = p.seed + (1.0 - f) * 4.0;
                    a.draw_centered("petal", s, Vec2::splat(p.size * z), spin, rgba(WHITE, f));
                }
                PKind::BirdShadow => {
                    let native = a.size("bird_shadow");
                    let w = p.size * z;
                    let size = vec2(w, w * native.y / native.x);
                    a.draw("bird_shadow", s - size * 0.5, size, SpriteParams {
                        flip_x: p.seed > 0.5,
                        tint: theme::alpha(theme::SHADOW, 0.16),
                        ..Default::default()
                    });
                }
            }
        }
    }
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
    fn sway_is_gentle() {
        let dt = 1.0 / 60.0;
        for seed in [0.0, 2.5, 4.1] {
            let mut prev = sway(0.0, seed, 0.08, 3.0);
            for i in 1..=600 {
                let t = i as f32 * dt;
                let s = sway(t, seed, 0.08, 3.0);
                assert!(s.abs() <= 0.08 + 1e-6, "sway exceeded its amplitude: {s}");
                assert!((s - prev).abs() <= 0.01, "sway moved too fast per frame: {}", (s - prev).abs());
                prev = s;
            }
        }
    }
}

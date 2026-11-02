//! Feedback for the player's own actions: buildings pop in when placed, ore
//! chips fly off a rock being mined, and each mined ore arcs into the player.

use std::f32::consts::TAU;

use macroquad::prelude::*;

use super::actors::Facing;
use super::camera::Cam;
use super::entities::draw_item;
use super::particles::rand01;
use crate::assets::Assets;
use crate::items::Item;
use crate::theme;

/// How long a freshly placed building springs before settling, in seconds.
pub const POP_TIME: f32 = 0.45;
/// Seconds a mined ore takes to arc from the rock into the player.
const FLY_TIME: f32 = 0.42;
const CHUNK_GRAVITY: f32 = 14.0;

/// Scale of a building `age` seconds after it was placed: it starts small,
/// overshoots and settles at 1 with a damped wobble.
pub fn pop_scale(age: f32) -> f32 {
    if age >= POP_TIME {
        return 1.0;
    }
    let t = age / POP_TIME;
    1.0 - 0.35 * (-t * 5.0).exp() * (t * 13.0).cos()
}

struct Chunk {
    /// Ground position in world tiles.
    pos: Vec2,
    vel: Vec2,
    /// Height above the ground, in tiles, and its vertical speed.
    h: f32,
    vh: f32,
    life: f32,
    rot: f32,
    spin: f32,
    item: Item,
}

struct Flyer {
    from: Vec2,
    age: f32,
    item: Item,
}

pub struct Juice {
    /// The way the player faces; kept while standing still.
    pub facing: Facing,
    /// Footprint top-left and age of each building still popping in.
    pops: Vec<(IVec2, f32)>,
    chunks: Vec<Chunk>,
    flyers: Vec<Flyer>,
}

impl Default for Juice {
    fn default() -> Self {
        Juice { facing: Facing::Front, pops: Vec::new(), chunks: Vec::new(), flyers: Vec::new() }
    }
}

impl Juice {
    /// Turns the player toward their movement; standing still keeps the facing.
    pub fn face(&mut self, vel: Vec2) {
        if let Some(f) = Facing::from_vel(vel) {
            self.facing = f;
        }
    }

    pub fn built(&mut self, pos: IVec2) {
        self.pops.retain(|(p, _)| *p != pos);
        self.pops.push((pos, 0.0));
    }

    /// Chips fly off the mined tile and the ore arcs toward the player.
    pub fn mined(&mut self, rng: &mut u64, tile: IVec2, item: Item) {
        let c = tile.as_vec2() + Vec2::splat(0.5);
        for _ in 0..5 {
            let (r1, r2, r3) = (rand01(rng), rand01(rng), rand01(rng));
            self.chunks.push(Chunk {
                pos: c + vec2(r1 - 0.5, r2 - 0.5) * 0.3,
                vel: Vec2::from_angle(r1 * TAU) * (0.8 + r2 * 1.4),
                h: 0.1,
                vh: 3.0 + r3 * 2.5,
                life: 0.9,
                rot: r2 * TAU,
                spin: (r3 - 0.5) * 18.0,
                item,
            });
        }
        self.flyers.push(Flyer { from: c, age: 0.0, item });
    }

    pub fn update(&mut self, dt: f32) {
        for p in &mut self.pops {
            p.1 += dt;
        }
        self.pops.retain(|p| p.1 < POP_TIME);
        for c in &mut self.chunks {
            c.life -= dt;
            c.pos += c.vel * dt;
            c.vh -= CHUNK_GRAVITY * dt;
            c.h += c.vh * dt;
            c.rot += c.spin * dt;
            if c.h <= 0.0 {
                // Bounce once, losing most of the energy, then rest.
                c.h = 0.0;
                c.vh = -c.vh * 0.3;
                c.vel *= 0.5;
                c.spin *= 0.5;
            }
        }
        self.chunks.retain(|c| c.life > 0.0);
        for f in &mut self.flyers {
            f.age += dt;
        }
        self.flyers.retain(|f| f.age < FLY_TIME);
    }

    /// The pop scale for a building whose footprint starts at `pos`.
    pub fn scale_at(&self, pos: IVec2) -> f32 {
        self.pops.iter().find(|(p, _)| *p == pos).map_or(1.0, |&(_, age)| pop_scale(age))
    }

    pub fn draw(&self, a: &Assets, cam: &Cam, player: Vec2) {
        let z = cam.zoom;
        for c in &self.chunks {
            let fade = (c.life / 0.3).min(1.0);
            let ground = cam.w2s(c.pos);
            let size = z * 0.22;
            // Soft shadow on the ground, shrinking as the chip rises.
            let sh = (1.0 - c.h * 0.6).clamp(0.3, 1.0);
            draw_ellipse(ground.x, ground.y + size * 0.3, size * 0.4 * sh, size * 0.18 * sh, 0.0, theme::rgba(0.1, 0.06, 0.04, 0.25 * fade));
            let s = ground - vec2(0.0, c.h * z);
            a.draw_centered(item_sprite(c.item), s, Vec2::splat(size), c.rot, theme::rgba(1.0, 1.0, 1.0, fade));
        }
        for f in &self.flyers {
            let t = (f.age / FLY_TIME).clamp(0.0, 1.0);
            // Ease in toward the player, along an arc that peaks mid-flight.
            let e = t * t * (3.0 - 2.0 * t);
            let to = player - vec2(0.0, 0.35);
            let p = f.from.lerp(to, e) - vec2(0.0, (t * (1.0 - t)) * 4.0 * 0.9);
            let size = z * (0.42 - 0.18 * t);
            draw_item(a, cam.w2s(p), size, f.item);
        }
    }
}

/// The atlas key of an item's icon; every item that can be mined has one.
fn item_sprite(item: Item) -> &'static str {
    super::entities::item_key(item).unwrap_or("item_stone")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pop_starts_small_overshoots_and_settles() {
        assert!(pop_scale(0.0) < 0.7, "a new building starts visibly small");
        let peak = (1..40).map(|i| pop_scale(i as f32 * POP_TIME / 40.0)).fold(0.0, f32::max);
        assert!(peak > 1.03, "it overshoots for a springy feel: peak {peak}");
        assert!(peak < 1.25, "but not so much it looks broken: peak {peak}");
        assert_eq!(pop_scale(POP_TIME), 1.0);
        assert_eq!(pop_scale(10.0), 1.0);
    }

    #[test]
    fn effects_expire() {
        let mut j = Juice::default();
        let mut rng = 7u64;
        j.built(IVec2::new(3, 4));
        j.mined(&mut rng, IVec2::new(1, 1), Item::IronOre);
        assert_eq!(j.chunks.len(), 5);
        assert_eq!(j.flyers.len(), 1);
        for _ in 0..120 {
            j.update(1.0 / 60.0);
        }
        assert!(j.pops.is_empty() && j.chunks.is_empty() && j.flyers.is_empty());
        assert_eq!(j.scale_at(IVec2::new(3, 4)), 1.0);
    }
}

use macroquad::prelude::*;

use crate::assets::{Assets, SpriteParams};
use crate::world::{Enemy, Player};

use super::camera::Cam;
use super::{PX, TERRA, world_bar};

/// Tiles walked per footfall; drives the dust puffs under the robot.
pub const WALK_STEP: f32 = 0.35;
/// Lean at full horizontal speed, in radians.
const MAX_LEAN: f32 = 0.17;

/// Number of footfalls (dust puffs) at `stride` tiles walked.
pub fn walk_footfalls(stride: f32) -> u64 {
    (stride / WALK_STEP).floor().max(0.0) as u64
}

/// How far the hovering robot tips into its horizontal motion, in radians.
pub fn lean(vel: Vec2) -> f32 {
    (vel.x * 0.028).clamp(-MAX_LEAN, MAX_LEAN)
}

pub(super) fn draw_enemy(a: &Assets, en: &Enemy, cam: &Cam) {
    let z = cam.zoom;
    let c = cam.w2s(en.pos);
    let frame = (en.phase * 0.6) as i32 % 2;
    let (key, px) = if en.brute { (["brute_0", "brute_1"][frame as usize], 72.0) } else { (["mite_0", "mite_1"][frame as usize], 48.0) };
    // Bugs are painted facing the viewer, so they stay upright and only
    // mirror to face the side they're heading toward.
    let size = Vec2::splat(px / PX * z);
    draw_ellipse(c.x, c.y + size.y * 0.42, size.x * 0.32, size.y * 0.1, 0.0, Color::new(0.1, 0.06, 0.04, 0.22));
    a.draw(key, c - size * 0.5, size, SpriteParams { flip_x: en.target.x < en.pos.x, ..Default::default() });
    if en.hp < en.max_hp() {
        let w = px / PX * z * 0.6;
        world_bar(vec2(c.x, c.y - w * 0.8), w, en.hp / en.max_hp(), TERRA, z);
    }
}

/// Which way the player faces, kept between frames so standing still holds
/// the last direction. Screen y points down, so "front" is toward the viewer.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Facing {
    Front,
    FrontRight,
    Right,
    BackRight,
    Back,
    BackLeft,
    Left,
    FrontLeft,
}

impl Facing {
    /// Clockwise from facing the viewer (moving down the screen).
    const RING: [Facing; 8] = [
        Facing::Front,
        Facing::FrontLeft,
        Facing::Left,
        Facing::BackLeft,
        Facing::Back,
        Facing::BackRight,
        Facing::Right,
        Facing::FrontRight,
    ];

    /// The facing for a movement velocity, or `None` when standing still:
    /// whichever of the eight compass directions is nearest the motion.
    pub fn from_vel(vel: Vec2) -> Option<Facing> {
        if vel.length_squared() < 1e-4 {
            return None;
        }
        // Angle from straight down (+y), turning toward -x.
        let a = (-vel.x).atan2(vel.y);
        let k = (a / std::f32::consts::FRAC_PI_4).round().rem_euclid(8.0) as usize;
        Some(Self::RING[k])
    }

    /// Sprite and whether to mirror it. The side and 3/4 art faces one way and
    /// is mirrored for the other: `player_side` faces left, the 3/4 views right.
    fn sprite(self) -> (&'static str, bool) {
        match self {
            Facing::Front => ("player_0", false),
            Facing::Back => ("player_back", false),
            Facing::Left => ("player_side", false),
            Facing::Right => ("player_side", true),
            Facing::FrontRight => ("player_front_right", false),
            Facing::FrontLeft => ("player_front_right", true),
            Facing::BackRight => ("player_back_right", false),
            Facing::BackLeft => ("player_back_right", true),
        }
    }
}

/// The robot hovers: a slow float when idle, a quicker bob and a lean into
/// the motion when moving, a flickering thruster glow under it and a soft
/// shadow on the ground that shrinks as it rises.
pub(super) fn draw_player(a: &Assets, player: &Player, cam: &Cam, t: f32, facing: Facing) {
    let moving = player.vel != Vec2::ZERO;
    let z = cam.zoom;
    let float = 0.06 * (t * 2.6).sin() + if moving { 0.025 * (t * 11.0).sin() } else { 0.0 };
    let lift = 0.16 + float;
    let ground = cam.w2s(player.draw_pos + vec2(0.0, 0.3));
    let size = Vec2::splat(z * 1.05);

    let sh = 1.0 - float * 2.5;
    draw_ellipse(ground.x, ground.y, size.x * 0.26 * sh, size.y * 0.085 * sh, 0.0, Color::new(0.1, 0.06, 0.04, 0.24));

    let base = ground - vec2(0.0, lift * z);
    let flicker = 0.8 + 0.2 * (t * 37.0).sin() * (t * 23.0).cos();
    a.draw_centered("glow", base + vec2(0.0, z * 0.05), vec2(z * 0.55, z * 0.32) * flicker, 0.0, Color::new(1.0, 0.62, 0.22, 0.55 * flicker));

    let (key, flip) = facing.sprite();
    // Lean into horizontal motion, tipping forward a touch when moving up or down.
    let lean = lean(player.vel);
    a.draw(key, base - vec2(size.x * 0.5, size.y), size, SpriteParams {
        rotation: lean,
        pivot: Some(base),
        flip_x: flip,
        ..Default::default()
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn robot_leans_into_motion() {
        assert!(lean(vec2(5.0, 0.0)) > 0.0 && lean(vec2(5.0, 0.0)) <= MAX_LEAN);
        assert!(lean(vec2(-5.0, 0.0)) < 0.0 && lean(vec2(-5.0, 0.0)) >= -MAX_LEAN);
        assert_eq!(lean(vec2(0.0, 5.0)), 0.0, "vertical motion stays upright");
        assert_eq!(lean(vec2(1000.0, 0.0)), MAX_LEAN);
    }

    #[test]
    fn facing_follows_motion() {
        assert_eq!(Facing::from_vel(vec2(0.0, 3.0)), Some(Facing::Front));
        assert_eq!(Facing::from_vel(vec2(0.0, -3.0)), Some(Facing::Back));
        assert_eq!(Facing::from_vel(vec2(3.0, 1.0)), Some(Facing::Right));
        assert_eq!(Facing::from_vel(vec2(-3.0, -1.0)), Some(Facing::Left));
        assert_eq!(Facing::from_vel(vec2(3.0, 3.0)), Some(Facing::FrontRight));
        assert_eq!(Facing::from_vel(vec2(-3.0, 3.0)), Some(Facing::FrontLeft));
        assert_eq!(Facing::from_vel(vec2(3.0, -3.0)), Some(Facing::BackRight));
        assert_eq!(Facing::from_vel(vec2(-3.0, -3.0)), Some(Facing::BackLeft));
        assert_eq!(Facing::from_vel(Vec2::ZERO), None, "standing still keeps the last facing");
    }

    #[test]
    fn footfalls_count_whole_steps() {
        assert_eq!(walk_footfalls(0.0), 0);
        assert_eq!(walk_footfalls(WALK_STEP * 2.5), 2);
    }
}

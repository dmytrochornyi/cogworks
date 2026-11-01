use std::f32::consts::TAU;

use macroquad::prelude::*;

use crate::assets::{Assets, SpriteParams};
use crate::world::{Enemy, Player};

use super::camera::Cam;
use super::{PX, TERRA, world_bar};

/// One walk step is this many tiles; a full two-foot cycle is four frames.
pub const WALK_STEP: f32 = 0.35;
/// Walk bob amplitude in tiles: visible but gentle (tested to stay 0.04..0.08).
const WALK_BOB_AMP: f32 = 0.06;
/// Lean at full horizontal speed, in radians (tested to stay within ±0.12).
const WALK_MAX_LEAN: f32 = 0.12;
/// Idle breathing period, seconds (tested to stay ≥ 2 s).
const IDLE_PERIOD: f32 = 2.5;
/// Idle breathing amplitude in tiles (tested to stay ≤ 0.02).
const IDLE_BOB_AMP: f32 = 0.015;

/// Everything the player's draw step needs, computed purely from distance walked
/// and velocity so the animation follows the feet rather than wall time.
pub struct Pose {
    /// Walk-cycle frame 0..=3; contact frames are 0 and 2.
    pub frame: usize,
    /// Vertical offset in tiles (up is positive screen space).
    pub bob: f32,
    /// Rotation in radians, leaning into horizontal motion (about the feet).
    pub lean: f32,
    /// Footfall squash amount in 0..=1; 0 = no squash, 1 = planted foot.
    pub squash: f32,
    /// Whether to mirror the sprite (moving left).
    pub face_left: bool,
}

/// Number of footfalls (contacts) at `stride` tiles walked.
pub fn walk_footfalls(stride: f32) -> u64 {
    (stride / WALK_STEP).floor().max(0.0) as u64
}

/// The pure walk cycle: frame from distance, bob from the step phase, lean from
/// horizontal velocity, and a gentle idle breath when standing still.
pub fn walk_pose(stride: f32, vel: Vec2, t: f32) -> Pose {
    if vel == Vec2::ZERO {
        // Slow idle breathing: tiny amplitude, long period, no lean or squash.
        let bob = IDLE_BOB_AMP * (t / IDLE_PERIOD * TAU).sin();
        return Pose { frame: 0, bob, lean: 0.0, squash: 0.0, face_left: false };
    }
    let steps = stride / WALK_STEP;
    let frame = walk_footfalls(stride) % 4;
    // Bob dips as a foot plants (phase 0) and rises as it passes under the body.
    let bob = -WALK_BOB_AMP * (TAU * steps).cos();
    // Squash peaks on the planted foot and eases off through the passing frame.
    let phase = steps.fract();
    let squash = (1.0 - phase) * (1.0 - phase);
    let lean = (vel.x * 0.02).clamp(-WALK_MAX_LEAN, WALK_MAX_LEAN);
    let face_left = vel.x < 0.0;
    Pose { frame: frame as usize, bob, lean, squash, face_left }
}

pub(super) fn draw_enemy(a: &Assets, en: &Enemy, cam: &Cam) {
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

pub(super) fn draw_player(a: &Assets, player: &Player, cam: &Cam, t: f32) {
    let pose = walk_pose(player.stride, player.vel, t);
    let walking = player.vel != Vec2::ZERO;
    // Walking uses the four-frame cycle; idle keeps the breathing pair.
    let key = if walking {
        ["player_walk_0", "player_walk_1", "player_walk_2", "player_walk_3"][pose.frame]
    } else {
        ["player_0", "player_1"][if pose.bob >= 0.0 { 0 } else { 1 }]
    };

    let z = cam.zoom;
    // Slight squash on footfalls: shorter and wider by a hair, not a full scale.
    let sx = z * 1.05 * (1.0 + 0.04 * pose.squash);
    let sy = z * 1.05 * (1.0 - 0.05 * pose.squash);
    let bob_px = pose.bob * z;
    let c = cam.w2s(player.pos - vec2(0.0, 0.2));
    let top_left = c - vec2(sx, sy) * 0.5 - vec2(0.0, bob_px);
    // Rotate about the feet (bottom centre), which move with the bob offset.
    let pivot = vec2(c.x, c.y + sy * 0.5 - bob_px);
    a.draw(key, top_left, vec2(sx, sy), SpriteParams {
        rotation: pose.lean,
        pivot: Some(pivot),
        flip_x: pose.face_left,
        ..Default::default()
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn walk_frame_follows_distance() {
        // The frame advances once per WALK_STEP tiles, independent of wall time.
        let vel = vec2(1.0, 0.0);
        assert_eq!(walk_pose(0.0, vel, 0.0).frame, 0);
        assert_eq!(walk_pose(WALK_STEP * 0.5, vel, 0.0).frame, 0);
        assert_eq!(walk_pose(WALK_STEP, vel, 0.0).frame, 1);
        assert_eq!(walk_pose(WALK_STEP * 2.0, vel, 0.0).frame, 2);
        assert_eq!(walk_pose(WALK_STEP * 3.0, vel, 0.0).frame, 3);
        assert_eq!(walk_pose(WALK_STEP * 4.0, vel, 0.0).frame, 0);
        // Same stride → same frame at any t.
        assert_eq!(walk_pose(WALK_STEP * 1.5, vel, 0.0).frame, walk_pose(WALK_STEP * 1.5, vel, 99.0).frame);
    }

    #[test]
    fn walk_bob_is_visible_but_gentle() {
        let mut peak = 0.0f32;
        for i in 0..400 {
            let stride = i as f32 * 0.01;
            let bob = walk_pose(stride, vec2(1.0, 0.0), 0.0).bob.abs();
            peak = peak.max(bob);
        }
        assert!(peak >= 0.04, "bob too subtle: {peak}");
        assert!(peak <= 0.08, "bob too strong: {peak}");
    }

    #[test]
    fn walk_leans_into_motion() {
        let right = walk_pose(1.0, vec2(1.0, 0.0), 0.0);
        assert!(right.lean > 0.0 && right.lean <= WALK_MAX_LEAN + 1e-6, "right lean {}", right.lean);
        let left = walk_pose(1.0, vec2(-1.0, 0.0), 0.0);
        assert!(left.lean < 0.0 && left.lean >= -WALK_MAX_LEAN - 1e-6, "left lean {}", left.lean);
        // Vertical motion stays upright.
        assert_eq!(walk_pose(1.0, vec2(0.0, 1.0), 0.0).lean, 0.0);
    }

    #[test]
    fn idle_breathes_slowly() {
        // Tiny amplitude at all times.
        for i in 0..300 {
            let t = i as f32 * 0.02;
            let bob = walk_pose(0.0, Vec2::ZERO, t).bob;
            assert!(bob.abs() <= 0.02 + 1e-6, "idle bob {bob} at t={t}");
        }
        // Period ≥ 2 s: the bob stays ≥ 0 through the first second (half period ≥ 1 s)…
        for i in 0..=50 {
            let t = i as f32 * 0.02;
            assert!(walk_pose(0.0, Vec2::ZERO, t).bob >= -1e-6, "idle bob went negative too soon at t={t}");
        }
        // …but does breathe: it dips below zero by 1.5 s.
        assert!(walk_pose(0.0, Vec2::ZERO, 1.5).bob < 0.0);
    }
}

use macroquad::prelude::*;

use crate::assets::{Assets, SpriteParams};
use crate::world::{Enemy, Player};

use super::camera::Cam;
use super::{PX, TERRA, world_bar};

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
    let walking = player.walk != Vec2::ZERO;
    let frame = if walking { (t * 8.0) as i32 % 2 } else { 0 };
    let c = cam.w2s(player.pos - vec2(0.0, 0.2));
    let size = Vec2::splat(cam.zoom * 1.05);
    a.draw(["player_0", "player_1"][frame as usize], c - size * 0.5, size, SpriteParams {
        flip_x: player.walk.x < 0.0,
        ..Default::default()
    });
}

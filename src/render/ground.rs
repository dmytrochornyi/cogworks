use macroquad::prelude::*;

use crate::assets::{Assets, SpriteParams};
use crate::items::Item;
use crate::theme;
use crate::world::*;

use super::camera::Cam;
use super::{hash, wind};

/// The sixteen seamless ground tiles, indexed `[x][y]` by `x.rem_euclid(4)` and
/// `y.rem_euclid(4)`. All are cut from one periodic 256x256 sheet, so touching
/// tiles always continue each other.
const GRASS_TILES: [[&str; 4]; 4] = [
    ["grass_0_0", "grass_0_1", "grass_0_2", "grass_0_3"],
    ["grass_1_0", "grass_1_1", "grass_1_2", "grass_1_3"],
    ["grass_2_0", "grass_2_1", "grass_2_2", "grass_2_3"],
    ["grass_3_0", "grass_3_1", "grass_3_2", "grass_3_3"],
];

fn ore_key(item: Item) -> &'static str {
    match item {
        Item::IronOre => "iron",
        Item::CopperOre => "copper",
        Item::Coal => "coal",
        Item::Stone => "stone",
        _ => "quartz",
    }
}

pub(super) fn draw_ground(a: &Assets, world: &World, cam: &Cam, lo: IVec2, hi: IVec2, t: f32) {
    let z = cam.zoom;
    for y in lo.y..=hi.y {
        for x in lo.x..=hi.x {
            let s = cam.w2s(ivec2(x, y).as_vec2());
            let key = GRASS_TILES[x.rem_euclid(4) as usize][y.rem_euclid(4) as usize];
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
                    // A soft round tint under the ore field so it fades into the
                    // grass, instead of the old hard square soil patch.
                    a.draw_centered("glow", s + Vec2::splat(z * 0.5), Vec2::splat(z * 1.5), 0.0, theme::alpha(theme::DIRT, 0.10));
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
                None if h % 100 < 13 && world.entity_at(p).is_none() && !world.has_tree(p) => {
                    let key = format!("decor_{}", (h >> 8) % 8);
                    let off = vec2(((h >> 12) % 32) as f32, ((h >> 17) % 32) as f32) / 64.0 * z;
                    // Grass tufts, flowers and mushrooms bend about their base in
                    // the wind; mushrooms are stiffer than grass.
                    let stiff = if (h >> 8) % 8 == 5 { 0.35 } else { 1.0 };
                    let sw = wind(t, x as f32, y as f32) * 0.32 * stiff;
                    let base = s + off + vec2(z * 0.25, z * 0.5);
                    a.draw(&key, s + off, Vec2::splat(z * 0.5), SpriteParams {
                        rotation: sw,
                        pivot: Some(base),
                        ..Default::default()
                    });
                }
                None => {}
            }
        }
    }
}

use macroquad::prelude::*;

use crate::assets::{Assets, SpriteParams};
use crate::items::Item;
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

/// Neighbour offsets for the ore-field mask, clockwise from north; bit `k`
/// of the mask is set when neighbour `k` holds the same ore (see `ORE_FIELD`).
const NEIGHBOURS: [IVec2; 8] = [
    IVec2::new(0, -1),
    IVec2::new(1, -1),
    IVec2::new(1, 0),
    IVec2::new(1, 1),
    IVec2::new(0, 1),
    IVec2::new(-1, 1),
    IVec2::new(-1, 0),
    IVec2::new(-1, -1),
];

/// Which neighbours of `p` hold the same ore, as the shader's 8-bit mask.
pub(super) fn ore_mask(world: &World, p: IVec2, ore: Item) -> u8 {
    NEIGHBOURS
        .iter()
        .enumerate()
        .filter(|(_, d)| world.ore_at(p + **d) == Some(ore))
        .fold(0, |m, (k, _)| m | (1 << k))
}

/// Ore deposits are drawn as one continuous field per ore: every ore tile is a
/// quad cut from that ore's seamless 4x4-tile texture, and the `ore_field`
/// shader fades it out toward neighbours that aren't the same ore. All tiles of
/// one ore go into a single mesh.
fn draw_ore_fields(a: &Assets, world: &World, cam: &Cam, lo: IVec2, hi: IVec2) {
    const ORES: [Item; 5] = [Item::IronOre, Item::CopperOre, Item::Coal, Item::Stone, Item::Quartz];
    gl_use_material(&a.ore_field);
    for ore in ORES {
        let sheet = a.uv_rect(&format!("orefield_{}", ore_key(ore)));
        let cell = vec2(sheet.w, sheet.h) / 4.0;
        let mut verts: Vec<Vertex> = Vec::new();
        let mut idx: Vec<u16> = Vec::new();
        let flush = |verts: &mut Vec<Vertex>, idx: &mut Vec<u16>| {
            if !verts.is_empty() {
                draw_mesh(&Mesh { vertices: std::mem::take(verts), indices: std::mem::take(idx), texture: Some(a.atlas.clone()) });
            }
        };
        for y in lo.y - 1..=hi.y + 1 {
            for x in lo.x - 1..=hi.x + 1 {
                let p = ivec2(x, y);
                if world.ore_at(p) != Some(ore) {
                    continue;
                }
                let mask = ore_mask(world, p, ore) as f32;
                let uv0 = vec2(sheet.x, sheet.y) + vec2(x.rem_euclid(4) as f32, y.rem_euclid(4) as f32) * cell;
                let base = verts.len() as u16;
                for (cx, cy) in [(0, 0), (1, 0), (1, 1), (0, 1)] {
                    let s = cam.w2s((p + ivec2(cx, cy)).as_vec2());
                    let mut v = Vertex::new(s.x, s.y, 0.0, uv0.x + cx as f32 * cell.x, uv0.y + cy as f32 * cell.y, WHITE);
                    v.normal = vec4(cx as f32, cy as f32, mask, 0.0);
                    verts.push(v);
                }
                idx.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
                // macroquad caps one draw call at 5000 indices (833 quads).
                if idx.len() >= 800 * 6 {
                    flush(&mut verts, &mut idx);
                }
            }
        }
        flush(&mut verts, &mut idx);
    }
    gl_use_default_material();
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
    draw_ore_fields(a, world, cam, lo, hi);
    // Overlays can spill onto neighbours, so they go in a second pass.
    for y in lo.y..=hi.y {
        for x in lo.x..=hi.x {
            let p = ivec2(x, y);
            let s = cam.w2s(p.as_vec2());
            let h = hash(x * 3 + 1, y * 7 + 2);
            match world.res_at(p) {
                // Ore is drawn as continuous fields by `draw_ore_fields`.
                Some(Resource::Ore(_)) => {}
                Some(Resource::Tar) => a.draw(["tar_0", "tar_1"][(h % 2) as usize], s, Vec2::splat(z), SpriteParams::default()),
                None if h % 100 < 13 && world.entity_at(p).is_none() && !world.has_tree(p) => {
                    let key = format!("decor_{}", (h >> 8) % 8);
                    let off = vec2(((h >> 12) % 32) as f32, ((h >> 17) % 32) as f32) / 64.0 * z;
                    // Grass tufts, flowers and mushrooms bend about their base in
                    // the wind; mushrooms are stiffer than grass, and pebbles
                    // (decor_3) and the fallen leaf (decor_7) lie still.
                    let stiff = match (h >> 8) % 8 {
                        3 | 7 => 0.0,
                        5 => 0.35,
                        _ => 1.0,
                    };
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ore_mask_bits_follow_the_shader_order() {
        let mut w = World::new(7);
        w.res.fill(None);
        let c = ivec2(50, 50);
        let set = |w: &mut World, p: IVec2, ore: Item| w.res[(p.y * MAP_W + p.x) as usize] = Some(Resource::Ore(ore));
        set(&mut w, c, Item::IronOre);
        assert_eq!(ore_mask(&w, c, Item::IronOre), 0, "a lone tile fades on every side");
        set(&mut w, c + ivec2(0, -1), Item::IronOre); // N  -> bit 0
        set(&mut w, c + ivec2(1, 1), Item::IronOre); // SE -> bit 3
        set(&mut w, c + ivec2(-1, 0), Item::IronOre); // W  -> bit 6
        set(&mut w, c + ivec2(1, 0), Item::Coal); // E is a different ore: not set
        assert_eq!(ore_mask(&w, c, Item::IronOre), 0b0100_1001);
    }
}

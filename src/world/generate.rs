use std::f32::consts::TAU;

use serde::{Deserialize, Serialize};

use super::*;

impl World {
    pub fn new(seed: u64) -> Self {
        let n = (MAP_W * MAP_H) as usize;
        let spawn = vec2(MAP_W as f32 / 2.0, MAP_H as f32 / 2.0);
        let mut w = World {
            res: vec![None; n],
            grid: vec![None; n],
            entities: Vec::new(),
            free: Vec::new(),
            player: Player::new(spawn),
            research: Research { current: None, progress: 0, done: vec![false; TECHS.len()] },
            enemies: Vec::new(),
            pollution: vec![0.0; combat::chunk_count()],
            evolution: 0.0,
            peaceful: false,
            time: 0.0,
            won: false,
            spawn,
            pollution_timer: 0.0,
            rng: Rng(seed.rotate_left(17) | 1),
            events: Vec::new(),
            sounds: Vec::new(),
            fx: Vec::new(),
            power: Power::default(),
            fluids: FluidNets::default(),
            shots: Vec::new(),
        };
        w.generate(seed);
        w
    }

    fn generate(&mut self, seed: u64) {
        let mut rng = Rng(seed | 1);
        let center = self.spawn;
        let ores = [
            Resource::Ore(Item::IronOre),
            Resource::Ore(Item::CopperOre),
            Resource::Ore(Item::Coal),
            Resource::Ore(Item::Stone),
            Resource::Ore(Item::Quartz),
            Resource::Tar,
        ];

        for _ in 0..170 {
            let c = vec2(rng.range(4.0, MAP_W as f32 - 4.0), rng.range(4.0, MAP_H as f32 - 4.0));
            if c.distance(center) < 30.0 {
                continue;
            }
            let res = ores[(rng.next() % ores.len() as u64) as usize];
            let r = if res == Resource::Tar { rng.range(1.5, 3.0) } else { rng.range(3.0, 9.0) };
            self.blob(&mut rng, c, r, res);
        }
        // Starter patches: the four basics close by, quartz and tar a short walk away.
        let base = rng.range(0.0, TAU);
        for (k, &res) in ores.iter().enumerate() {
            let (a, dist, r) = if k < 4 {
                (base + k as f32 * TAU / 4.0 + rng.range(-0.3, 0.3), rng.range(11.0, 16.0), 5.0)
            } else {
                (base + TAU / 8.0 + (k - 4) as f32 * TAU / 2.0, rng.range(28.0, 36.0), 3.5)
            };
            let c = center + vec2(a.cos(), a.sin()) * dist;
            self.blob(&mut rng, c, r, res);
        }
        // Keep the landing spot clear.
        for y in -3..=3 {
            for x in -3..=3 {
                let p = center.as_ivec2() + ivec2(x, y);
                self.res[idx(p)] = None;
            }
        }
        // Hive clusters, far from spawn.
        let mut placed = 0;
        let mut attempts = 0;
        while placed < 30 && attempts < 2000 {
            attempts += 1;
            let c = vec2(rng.range(6.0, MAP_W as f32 - 6.0), rng.range(6.0, MAP_H as f32 - 6.0));
            if c.distance(center) < 75.0 {
                continue;
            }
            let count = 1 + rng.next() % 3;
            for _ in 0..count {
                let p = (c + vec2(rng.range(-6.0, 6.0), rng.range(-6.0, 6.0))).as_ivec2();
                if self.footprint_free(Kind::Hive, p, Dir::N) {
                    self.spawn_entity(Kind::Hive, p, Dir::N);
                }
            }
            placed += 1;
        }
    }

    fn blob(&mut self, rng: &mut Rng, c: Vec2, r: f32, res: Resource) {
        let (p1, p2) = (rng.range(0.0, TAU), rng.range(0.0, TAU));
        let ext = (r * 1.4).ceil() as i32;
        let ci = c.as_ivec2();
        for y in ci.y - ext..=ci.y + ext {
            for x in ci.x - ext..=ci.x + ext {
                let p = ivec2(x, y);
                if !in_bounds(p) {
                    continue;
                }
                let d = p.as_vec2() + vec2(0.5, 0.5) - c;
                let a = d.y.atan2(d.x);
                let edge = r * (1.0 + 0.22 * (3.0 * a + p1).sin() + 0.12 * (5.0 * a + p2).sin());
                if d.length() < edge {
                    self.res[idx(p)] = Some(res);
                }
            }
        }
    }
}

/// xorshift64* — small deterministic RNG.
#[derive(Clone, Serialize, Deserialize)]
pub struct Rng(pub u64);

impl Rng {
    pub fn next(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    pub fn range(&mut self, lo: f32, hi: f32) -> f32 {
        let f = (self.next() >> 40) as f32 / (1u64 << 24) as f32;
        lo + (hi - lo) * f
    }
}

use std::collections::VecDeque;

use serde::{Deserialize, Serialize};

use super::*;

pub const PLAYER_SPEED: f32 = 6.0;
const PLAYER_RADIUS: f32 = 0.3;
pub const PLAYER_HP: f32 = 100.0;
/// Seconds to mine one ore by hand.
pub const MINE_TIME: f32 = 0.6;

#[derive(Clone, Serialize, Deserialize)]
pub struct Craft {
    pub recipe: usize,
    /// Seconds spent so far; ingredients are taken when this leaves zero.
    pub progress: f32,
    pub started: bool,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Player {
    pub pos: Vec2,
    pub hp: f32,
    pub inv: Stock,
    pub queue: VecDeque<Craft>,
    pub quickbar: [Option<Item>; 10],
    /// Movement input for this tick, set by the game loop.
    #[serde(skip)]
    pub walk: Vec2,
    /// Tiles actually walked so far (after collision), driving the walk cycle.
    #[serde(default)]
    pub stride: f32,
    /// Last frame's actual velocity in tiles/second, so leaning and the walk
    /// cycle reflect real movement (pushing a wall doesn't animate). Not saved.
    #[serde(skip)]
    pub vel: Vec2,
    /// Hand-mining progress on a tile.
    #[serde(skip)]
    pub mining: Option<(IVec2, f32)>,
    pub calm: f32,
    pub respawn: f32,
    gun_cd: f32,
    shots: u32,
    dmg: f32,
}

impl Player {
    pub fn new(pos: Vec2) -> Self {
        let mut inv = Stock::default();
        for (item, n) in [
            (Item::IronPlate, 60),
            (Item::CopperPlate, 30),
            (Item::Brick, 20),
            (Item::Gear, 10),
            (Item::Circuit, 15),
            (Item::Coal, 20),
            (Item::Belt, 60),
            (Item::Arm, 12),
            (Item::Drill, 4),
            (Item::Smelter, 3),
            (Item::Generator, 1),
            (Item::Pole, 12),
            (Item::Chest, 3),
            (Item::Assembler, 1),
            (Item::Lab, 1),
            (Item::SlugClip, 10),
        ] {
            inv.add(item, n);
        }
        let mut quickbar = [None; 10];
        let defaults = [
            Item::Belt,
            Item::Arm,
            Item::Pole,
            Item::Drill,
            Item::Smelter,
            Item::Assembler,
            Item::Chest,
            Item::Generator,
            Item::Lab,
            Item::Splitter,
        ];
        for (slot, item) in quickbar.iter_mut().zip(defaults) {
            *slot = Some(item);
        }
        Player {
            pos,
            hp: PLAYER_HP,
            inv,
            queue: VecDeque::new(),
            quickbar,
            walk: Vec2::ZERO,
            stride: 0.0,
            vel: Vec2::ZERO,
            mining: None,
            calm: 100.0,
            respawn: 0.0,
            gun_cd: 0.0,
            shots: 0,
            dmg: 0.0,
        }
    }

    pub fn alive(&self) -> bool {
        self.hp > 0.0
    }

    pub fn bounds(&self) -> (Vec2, Vec2) {
        (self.pos - Vec2::splat(PLAYER_RADIUS), self.pos + Vec2::splat(PLAYER_RADIUS))
    }

    pub fn damage(&mut self, dmg: f32) {
        if self.alive() {
            self.hp -= dmg;
            self.calm = 0.0;
            if !self.alive() {
                self.respawn = 6.0;
            }
        }
    }

    pub fn in_reach(&self, tile: IVec2) -> bool {
        (tile.as_vec2() + Vec2::splat(0.5)).distance(self.pos) <= REACH
    }
}

impl World {
    pub(super) fn update_player(&mut self, dt: f32) {
        if !self.player.alive() {
            self.player.respawn -= dt;
            if self.player.respawn <= 0.0 {
                self.player.pos = self.spawn;
                self.player.hp = PLAYER_HP;
                self.events.push("You respawned at the landing site".into());
            }
            return;
        }
        self.player.calm += dt;
        if self.player.calm > 5.0 {
            self.player.hp = (self.player.hp + 5.0 * dt).min(PLAYER_HP);
        }
        self.walk_player(dt);
        self.update_crafting(dt);
        self.update_mining(dt);
        self.update_gun(dt);
    }

    fn blocked(&self, p: Vec2) -> bool {
        let (lo, hi) = (p - Vec2::splat(PLAYER_RADIUS), p + Vec2::splat(PLAYER_RADIUS));
        if lo.x < 0.0 || lo.y < 0.0 || hi.x >= MAP_W as f32 || hi.y >= MAP_H as f32 {
            return true;
        }
        let (a, b) = (lo.floor().as_ivec2(), hi.floor().as_ivec2());
        (a.y..=b.y).any(|y| {
            (a.x..=b.x).any(|x| self.entity_ref_at(ivec2(x, y)).is_some_and(|e| e.kind.solid()))
        })
    }

    fn walk_player(&mut self, dt: f32) {
        let step = self.player.walk.normalize_or_zero() * PLAYER_SPEED * dt;
        let p = self.player.pos;
        // Move each axis separately so we slide along walls.
        let x = p + vec2(step.x, 0.0);
        let p = if self.blocked(x) { p } else { x };
        let y = p + vec2(0.0, step.y);
        self.player.pos = if self.blocked(y) { p } else { y };
        // Accumulate the distance actually moved (after collision) and keep the
        // real velocity, so the walk animation follows feet on the ground rather
        // than the requested step (pushing a wall doesn't walk).
        let moved = self.player.pos - p;
        self.player.stride += moved.length();
        self.player.vel = if dt > 0.0 { moved / dt } else { Vec2::ZERO };
    }

    fn update_crafting(&mut self, dt: f32) {
        let Some(front) = self.player.queue.front_mut() else {
            return;
        };
        let r = &RECIPES[front.recipe];
        if !front.started {
            if !self.player.inv.has(r.inputs) {
                self.player.queue.pop_front();
                self.events.push(format!("Missing ingredients for {}", r.name));
                return;
            }
            for &(i, n) in r.inputs {
                self.player.inv.take(i, n);
            }
            front.started = true;
        }
        front.progress += dt;
        if front.progress >= r.time {
            for &(i, n) in r.outputs {
                self.player.inv.add(i, n);
            }
            self.player.queue.pop_front();
            self.sounds.push(Sfx::Craft);
        }
    }

    /// Queues `count` hand crafts of a recipe, plus any intermediate crafts needed.
    pub fn queue_craft(&mut self, recipe: usize, count: u32) -> Result<(), String> {
        let mut sim = self.player.inv.clone();
        // Ingredients of queued crafts that haven't started yet are already spoken for.
        for c in self.player.queue.iter().filter(|c| !c.started) {
            let r = &RECIPES[c.recipe];
            for &(i, n) in r.inputs {
                sim.take(i, n);
            }
            for &(i, n) in r.outputs {
                sim.add(i, n);
            }
        }
        let mut plan = Vec::new();
        for _ in 0..count {
            if !self.plan_craft(&mut sim, recipe, &mut plan, 0) {
                break;
            }
        }
        if plan.is_empty() {
            let missing: Vec<String> = RECIPES[recipe]
                .inputs
                .iter()
                .filter(|&&(i, n)| self.player.inv.get(i) < n)
                .map(|(i, _)| i.name().to_string())
                .collect();
            return Err(format!("Can't craft {}: need {}", RECIPES[recipe].name, missing.join(", ")));
        }
        for r in plan {
            self.player.queue.push_back(Craft { recipe: r, progress: 0.0, started: false });
        }
        Ok(())
    }

    fn plan_craft(&self, inv: &mut Stock, recipe: usize, plan: &mut Vec<usize>, depth: u32) -> bool {
        let r = &RECIPES[recipe];
        if !r.hand || !self.recipe_unlocked(recipe) || depth > 5 {
            return false;
        }
        let mut trial = inv.clone();
        let mut steps = Vec::new();
        for &(item, n) in r.inputs {
            while trial.get(item) < n {
                let Some(sub) = self.hand_recipe_for(item) else {
                    return false;
                };
                if !self.plan_craft(&mut trial, sub, &mut steps, depth + 1) {
                    return false;
                }
            }
            trial.take(item, n);
        }
        for &(i, n) in r.outputs {
            trial.add(i, n);
        }
        *inv = trial;
        plan.extend(steps);
        plan.push(recipe);
        true
    }

    /// Whether the player could hand craft one of `recipe` right now.
    pub fn can_craft(&self, recipe: usize) -> bool {
        let mut sim = self.player.inv.clone();
        self.plan_craft(&mut sim, recipe, &mut Vec::new(), 0)
    }

    /// Whether the player can hand-craft enough of `item` to hold at least `n`,
    /// reusing the workbench's auto-craft planning.
    pub fn can_cover(&self, item: Item, n: u32) -> bool {
        let Some(sub) = self.hand_recipe_for(item) else {
            return false;
        };
        let mut sim = self.player.inv.clone();
        let mut plan = Vec::new();
        while sim.get(item) < n {
            if !self.plan_craft(&mut sim, sub, &mut plan, 0) {
                return false;
            }
        }
        true
    }

    fn hand_recipe_for(&self, item: Item) -> Option<usize> {
        (0..RECIPES.len()).find(|&r| {
            RECIPES[r].hand && RECIPES[r].main_output() == Some(item) && self.recipe_unlocked(r)
        })
    }

    pub fn cancel_craft(&mut self, index: usize) {
        if let Some(c) = self.player.queue.remove(index)
            && c.started
        {
            for &(i, n) in RECIPES[c.recipe].inputs {
                self.player.inv.add(i, n);
            }
        }
    }

    fn update_mining(&mut self, dt: f32) {
        let Some((tile, progress)) = self.player.mining else {
            return;
        };
        let progress = progress + dt / MINE_TIME;
        if self.has_tree(tile) {
            if progress >= 1.0 {
                self.chop(tile);
                self.player.mining = self.has_tree(tile).then_some((tile, 0.0));
            } else {
                self.player.mining = Some((tile, progress));
            }
            return;
        }
        let Some(ore) = self.ore_at(tile).filter(|_| self.entity_at(tile).is_none()) else {
            self.player.mining = None;
            return;
        };
        if progress >= 1.0 {
            self.player.inv.add(ore, 1);
            self.fx.push(Fx::Mined { tile, item: ore });
            self.player.mining = Some((tile, 0.0));
        } else {
            self.player.mining = Some((tile, progress));
        }
    }

    fn update_gun(&mut self, dt: f32) {
        self.player.gun_cd -= dt;
        if self.player.gun_cd > 0.0 {
            return;
        }
        let Some(t) = self.nearest_target(self.player.pos, combat::GUN_RANGE) else {
            return;
        };
        if self.player.shots == 0 {
            let Some(clip) = [Item::HeavyClip, Item::SlugClip]
                .into_iter()
                .find(|&i| self.player.inv.get(i) > 0)
            else {
                return;
            };
            self.player.inv.take(clip, 1);
            let (n, d) = clip.ammo().unwrap_or((0, 0.0));
            self.player.shots = n;
            self.player.dmg = d;
        }
        self.player.shots -= 1;
        self.player.gun_cd = 0.3;
        let (from, dmg) = (self.player.pos, self.player.dmg);
        self.hit(from, t, dmg);
    }
}

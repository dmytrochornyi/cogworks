use std::f32::consts::TAU;

use serde::{Deserialize, Serialize};

use super::*;

/// Pollution is tracked on a coarse grid of CHUNK x CHUNK tiles.
pub const CHUNK: i32 = 8;
const CHUNKS_W: i32 = MAP_W / CHUNK;
const CHUNKS_H: i32 = MAP_H / CHUNK;
/// Agitation a hive needs to send out an attack wave.
const WAVE_COST: f32 = 60.0;
pub const SENTRY_RANGE: f32 = 16.0;
pub const GUN_RANGE: f32 = 12.0;
/// Enemies chase the player when this close.
const AGGRO: f32 = 9.0;

pub fn chunk_count() -> usize {
    (CHUNKS_W * CHUNKS_H) as usize
}

pub fn chunk_of(p: Vec2) -> Option<usize> {
    let c = (p / CHUNK as f32).floor().as_ivec2();
    (c.x >= 0 && c.y >= 0 && c.x < CHUNKS_W && c.y < CHUNKS_H).then(|| (c.y * CHUNKS_W + c.x) as usize)
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Enemy {
    pub pos: Vec2,
    pub hp: f32,
    pub brute: bool,
    /// Where it's heading; a building tile or the player.
    pub target: Vec2,
    pub home: Vec2,
    pub cooldown: f32,
    /// Seconds left before it gives up and despawns, when it has nothing to attack.
    pub patience: f32,
    /// Animation phase.
    pub phase: f32,
}

impl Enemy {
    pub fn max_hp(&self) -> f32 {
        if self.brute { 150.0 } else { 30.0 }
    }

    fn speed(&self) -> f32 {
        if self.brute { 2.2 } else { 3.2 }
    }

    fn bite(&self) -> f32 {
        if self.brute { 30.0 } else { 7.0 }
    }

    pub fn radius(&self) -> f32 {
        if self.brute { 0.45 } else { 0.28 }
    }
}

/// A short-lived tracer line for drawing gunfire.
#[derive(Clone)]
pub struct Shot {
    pub from: Vec2,
    pub to: Vec2,
    pub ttl: f32,
}

impl World {
    pub fn pollute(&mut self, at: Vec2, amount: f32) {
        if let Some(c) = chunk_of(at) {
            self.pollution[c] += amount;
            self.evolution = (self.evolution + amount * 2.0e-7).min(1.0);
        }
    }

    pub fn pollution_at(&self, at: Vec2) -> f32 {
        chunk_of(at).map_or(0.0, |c| self.pollution[c])
    }

    pub(super) fn update_pollution(&mut self, dt: f32) {
        self.evolution = (self.evolution + dt * 1.5e-5).min(1.0);
        self.pollution_timer += dt;
        if self.pollution_timer < 1.0 {
            return;
        }
        self.pollution_timer -= 1.0;
        let old = self.pollution.clone();
        const SPREAD: f32 = 0.04;
        for cy in 0..CHUNKS_H {
            for cx in 0..CHUNKS_W {
                let i = (cy * CHUNKS_W + cx) as usize;
                let mut v = old[i] * (1.0 - 4.0 * SPREAD);
                for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
                    let (nx, ny) = (cx + dx, cy + dy);
                    if nx >= 0 && ny >= 0 && nx < CHUNKS_W && ny < CHUNKS_H {
                        v += old[(ny * CHUNKS_W + nx) as usize] * SPREAD;
                    }
                }
                // Ground absorbs a little every second.
                self.pollution[i] = (v * 0.985 - 0.05).max(0.0);
            }
        }
    }

    pub(super) fn update_hive(&mut self, id: usize, dt: f32) {
        if self.peaceful {
            return;
        }
        let center = self.entity(id).center();
        let chunk = chunk_of(center);
        let near_player = self.player.alive() && self.player.pos.distance(center) < 14.0;
        let absorbed = chunk.map_or(0.0, |c| self.pollution[c].min(15.0 * dt));
        if let Some(c) = chunk {
            self.pollution[c] -= absorbed;
        }
        let wave_size = 3 + (self.evolution * 10.0) as usize;
        let State::Hive { agitation, cooldown, guard_cd } = &mut self.entity_mut(id).state else {
            return;
        };
        *cooldown -= dt;
        *guard_cd -= dt;
        *agitation += absorbed;
        let mut wave = 0;
        let mut guards = 0;
        if *agitation >= WAVE_COST && *cooldown <= 0.0 {
            *agitation -= WAVE_COST;
            *cooldown = 25.0;
            wave = wave_size;
        }
        if near_player && *guard_cd <= 0.0 {
            *guard_cd = 6.0;
            guards = 2;
        }
        if wave > 0
            && let Some(target) = self.attack_target(center)
        {
            self.spawn_enemies(center, wave, target);
        }
        if guards > 0 {
            let target = self.player.pos;
            self.spawn_enemies(center, guards, target);
        }
    }

    /// The nearest polluting building to `from`, or failing that any building, within range.
    fn attack_target(&self, from: Vec2) -> Option<Vec2> {
        let mut best: Option<(f32, Vec2)> = None;
        for e in self.entities.iter().flatten() {
            if e.kind == Kind::Hive {
                continue;
            }
            let c = e.center();
            let mut d = c.distance(from);
            if e.kind.pollution() <= 0.0 {
                d += 40.0;
            }
            if d < 130.0 && best.is_none_or(|(bd, _)| d < bd) {
                best = Some((d, c));
            }
        }
        best.map(|(_, c)| c)
    }

    fn spawn_enemies(&mut self, at: Vec2, count: usize, target: Vec2) {
        for _ in 0..count {
            let brute = self.rng.range(0.0, 1.0) < self.evolution * 0.6;
            let offset = vec2(self.rng.range(-2.0, 2.0), self.rng.range(-2.0, 2.0));
            let mut e = Enemy {
                pos: at + offset,
                hp: 0.0,
                brute,
                target,
                home: at,
                cooldown: 0.0,
                patience: 90.0,
                phase: self.rng.range(0.0, TAU),
            };
            e.hp = e.max_hp();
            self.enemies.push(e);
        }
    }

    pub(super) fn update_enemies(&mut self, dt: f32) {
        for s in &mut self.shots {
            s.ttl -= dt;
        }
        self.shots.retain(|s| s.ttl > 0.0);
        if self.peaceful {
            self.enemies.clear();
            return;
        }
        let mut enemies = std::mem::take(&mut self.enemies);
        for en in &mut enemies {
            en.phase += dt * 10.0;
            en.cooldown -= dt;
            en.patience -= dt;
            let player_close = self.player.alive() && self.player.pos.distance(en.pos) < AGGRO;
            if player_close {
                en.target = self.player.pos;
                en.patience = 60.0;
            }
            let to = en.target - en.pos;
            if player_close && to.length() < 0.6 + en.radius() {
                if en.cooldown <= 0.0 {
                    en.cooldown = 0.8;
                    self.player.damage(en.bite());
                }
                continue;
            }
            // Bite whatever building is directly ahead, or the one we came for.
            let ahead = en.pos + to.normalize_or_zero() * (en.radius() + 0.3);
            let target_tile = en.target.floor().as_ivec2();
            let blocker = self
                .entity_at(ahead.floor().as_ivec2())
                .filter(|&id| self.entity(id).kind != Kind::Hive)
                .filter(|&id| self.entity(id).kind.solid() || self.entity(id).tiles().any(|t| t == target_tile));
            if let Some(id) = blocker {
                if en.cooldown <= 0.0 {
                    en.cooldown = 0.8;
                    self.damage_entity(id, en.bite());
                }
                en.patience = 60.0;
                continue;
            }
            if to.length() < 0.5 {
                // Target is gone: look for something else nearby, or head home.
                en.target = self.attack_target_near(en.pos).unwrap_or(en.home);
                if en.target == en.home && en.pos.distance(en.home) < 2.0 {
                    en.patience = 0.0;
                }
                continue;
            }
            en.pos += to.normalize() * en.speed() * dt;
        }
        enemies.retain(|e| e.hp > 0.0 && e.patience > 0.0);
        // Gently push apart overlapping enemies so groups don't collapse into one dot.
        for i in 0..enemies.len() {
            for j in i + 1..enemies.len() {
                let d = enemies[j].pos - enemies[i].pos;
                let min = enemies[i].radius() + enemies[j].radius();
                let len = d.length();
                if len < min && len > 1e-4 {
                    let push = d / len * (min - len) * 0.5;
                    enemies[i].pos -= push;
                    enemies[j].pos += push;
                }
            }
        }
        self.enemies = enemies;
    }

    fn attack_target_near(&self, from: Vec2) -> Option<Vec2> {
        self.entities
            .iter()
            .flatten()
            .filter(|e| e.kind != Kind::Hive)
            .map(|e| e.center())
            .filter(|c| c.distance(from) < 20.0)
            .min_by(|a, b| a.distance(from).total_cmp(&b.distance(from)))
    }

    pub fn damage_entity(&mut self, id: usize, dmg: f32) {
        let e = self.entity_mut(id);
        e.hp -= dmg;
        e.calm = 0.0;
        if e.hp <= 0.0 {
            let kind = e.kind;
            self.delete(id);
            if kind == Kind::Hive {
                self.evolution = (self.evolution + 0.01).min(1.0);
                self.events.push("Hive destroyed".into());
            } else {
                self.events.push(format!("{} was destroyed!", kind.name()));
            }
        }
    }

    /// The closest enemy (mite or hive) to `from` within `range`.
    pub fn nearest_target(&self, from: Vec2, range: f32) -> Option<Target> {
        let mite = self
            .enemies
            .iter()
            .enumerate()
            .filter(|(_, e)| e.hp > 0.0)
            .map(|(k, e)| (e.pos.distance(from), k))
            .filter(|(d, _)| *d <= range)
            .min_by(|a, b| a.0.total_cmp(&b.0));
        if let Some((_, k)) = mite {
            return Some(Target::Enemy(k));
        }
        self.entities
            .iter()
            .enumerate()
            .filter_map(|(id, e)| e.as_ref().filter(|e| e.kind == Kind::Hive).map(|e| (id, e)))
            .map(|(id, e)| (e.center().distance(from), id))
            .filter(|(d, _)| *d <= range)
            .min_by(|a, b| a.0.total_cmp(&b.0))
            .map(|(_, id)| Target::Hive(id))
    }

    pub fn target_pos(&self, t: Target) -> Vec2 {
        match t {
            Target::Enemy(k) => self.enemies[k].pos,
            Target::Hive(id) => self.entity(id).center(),
        }
    }

    pub fn hit(&mut self, from: Vec2, t: Target, dmg: f32) {
        let to = self.target_pos(t);
        self.shots.push(Shot { from, to, ttl: 0.08 });
        self.sounds.push(Sfx::Shot(from));
        match t {
            Target::Enemy(k) => self.enemies[k].hp -= dmg,
            Target::Hive(id) => self.damage_entity(id, dmg),
        }
    }

    pub(super) fn update_sentry(&mut self, id: usize, dt: f32) {
        let center = self.entity(id).center();
        let target = self.nearest_target(center, SENTRY_RANGE);
        let e = self.entity_mut(id);
        let State::Sentry { ammo, shots, dmg, cooldown, .. } = &mut e.state else {
            return;
        };
        *cooldown -= dt;
        e.active = false;
        let Some(t) = target else {
            return;
        };
        if *shots == 0 {
            let Some(clip) = [Item::HeavyClip, Item::SlugClip].into_iter().find(|&i| ammo.get(i) > 0)
            else {
                return;
            };
            ammo.take(clip, 1);
            let (n, d) = clip.ammo().unwrap_or((0, 0.0));
            *shots = n;
            *dmg = d;
        }
        e.active = true;
        if *cooldown > 0.0 {
            return;
        }
        *cooldown = 0.25;
        *shots -= 1;
        let damage = *dmg;
        let to = self.target_pos(t);
        if let State::Sentry { aim, .. } = &mut self.entity_mut(id).state {
            *aim = (to - center).to_angle();
        }
        self.hit(center, t, damage);
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Target {
    Enemy(usize),
    Hive(usize),
}

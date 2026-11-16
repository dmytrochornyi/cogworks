//! Load-time validation and repair.
//!
//! A save that deserializes can still be structurally broken — a hand edit, a
//! version skew, or a truncation that happened to stay valid JSON. `repair`
//! catches the things that would otherwise panic deep inside the simulation
//! later, fixes what it can, and reports what it fixed as short notes. Anything
//! it can't fix becomes an `Err` so the caller can fall back to another save.

use super::*;

impl World {
    /// Validates a freshly loaded world and repairs what it can.
    ///
    /// `Err` means the save is unusable; `Ok(notes)` lists the fixes applied.
    pub fn repair(&mut self) -> Result<Vec<String>, String> {
        let tiles = (MAP_W * MAP_H) as usize;

        // Fatal: these lengths are structural and can't be repaired.
        if self.res.len() != tiles {
            return Err(format!("save has {} resource tiles, expected {tiles}", self.res.len()));
        }
        if self.grid.len() != tiles {
            return Err(format!("save has {} grid tiles, expected {tiles}", self.grid.len()));
        }
        if self.pollution.len() != combat::chunk_count() {
            return Err(format!(
                "save has {} pollution cells, expected {}",
                self.pollution.len(),
                combat::chunk_count()
            ));
        }
        if !self.trees.is_empty() && self.trees.len() != tiles {
            return Err(format!("save has {} tree tiles, expected {tiles}", self.trees.len()));
        }

        let mut notes = Vec::new();

        // Research: an older save may predate a tech, or target one already done.
        if self.research.done.len() != TECHS.len() {
            self.research.done.resize(TECHS.len(), false);
            notes.push("Repaired research progress".to_string());
        }
        if self.research.current.is_some_and(|c| c >= TECHS.len() || self.research.done[c]) {
            self.research.current = None;
            notes.push("Cleared an invalid research target".to_string());
        }

        // Buildings and the tile grid.
        let dropped = self.repair_buildings();
        if dropped > 0 {
            notes.push(format!("Removed {dropped} broken building{}", plural(dropped)));
        }

        // Numbers that would otherwise spread NaN through the simulation.
        let fixed = self.repair_numbers();
        if fixed > 0 {
            notes.push(format!("Repaired {fixed} invalid number{}", plural(fixed)));
        }

        // The player's position, and the landing site it respawns at.
        if !self.spawn.is_finite() {
            self.spawn = vec2(MAP_W as f32 / 2.0, MAP_H as f32 / 2.0);
        }
        if !self.player.pos.is_finite() {
            self.player.pos = self.spawn;
            notes.push("Moved the player to the landing site".to_string());
        }

        // Enemies with a broken position or health can't be simulated.
        let before = self.enemies.len();
        self.enemies.retain(|e| e.pos.is_finite() && e.hp.is_finite());
        let removed = before - self.enemies.len();
        if removed > 0 {
            notes.push(format!("Removed {removed} broken enem{}", if removed == 1 { "y" } else { "ies" }));
        }

        Ok(notes)
    }

    /// Drops entities whose footprint leaves the map or overlaps an earlier
    /// entity, then rebuilds `grid` and `free` if they no longer agree with the
    /// surviving entities. Returns how many entities were dropped.
    fn repair_buildings(&mut self) -> usize {
        let mut expected = vec![None; self.grid.len()];
        let mut dropped = 0;
        for id in 0..self.entities.len() {
            let tiles: Vec<IVec2> = match &self.entities[id] {
                Some(entity) => entity.tiles().collect(),
                None => continue,
            };
            let fits = tiles.iter().all(|&t| in_bounds(t))
                && tiles.iter().all(|&t| expected[idx(t)].is_none());
            if !fits {
                self.entities[id] = None;
                dropped += 1;
                continue;
            }
            for t in tiles {
                expected[idx(t)] = Some(id);
            }
        }
        if self.grid != expected {
            self.grid = expected;
        }
        if dropped > 0 || !self.free_consistent() {
            self.free = (0..self.entities.len())
                .filter(|&i| self.entities[i].is_none())
                .collect();
        }
        dropped
    }

    /// Whether `free` holds exactly the id of every `None` slot, once each.
    fn free_consistent(&self) -> bool {
        let mut in_free = vec![false; self.entities.len()];
        for &id in &self.free {
            if id >= self.entities.len() || in_free[id] {
                return false;
            }
            in_free[id] = true;
        }
        (0..self.entities.len()).all(|i| in_free[i] == self.entities[i].is_none())
    }

    /// Replaces non-finite floats with safe values and clamps health to a sane
    /// range. Returns how many values were repaired.
    fn repair_numbers(&mut self) -> usize {
        let mut fixed = 0;
        for value in [&mut self.time, &mut self.evolution, &mut self.pollution_timer] {
            if !value.is_finite() {
                *value = 0.0;
                fixed += 1;
            }
        }
        for cell in &mut self.pollution {
            if !cell.is_finite() {
                *cell = 0.0;
                fixed += 1;
            }
        }
        for entity in self.entities.iter_mut().flatten() {
            let max = entity.kind.max_hp();
            if !entity.hp.is_finite() {
                entity.hp = max;
                fixed += 1;
            } else if entity.hp < 0.0 || entity.hp > max {
                entity.hp = entity.hp.clamp(0.0, max);
                fixed += 1;
            }
            if !entity.calm.is_finite() {
                entity.calm = 0.0;
                fixed += 1;
            }
        }
        let player = &mut self.player;
        if !player.hp.is_finite() || player.hp > PLAYER_HP {
            player.hp = PLAYER_HP;
            fixed += 1;
        } else if player.hp < 0.0 {
            // A player saved while dead legitimately has negative hp; clamp it
            // without counting it as a repair so loading such a save doesn't
            // toast "Save repaired".
            player.hp = 0.0;
        }
        for value in [&mut player.calm, &mut player.respawn, &mut player.stride] {
            if !value.is_finite() {
                *value = 0.0;
                fixed += 1;
            }
        }
        fixed
    }
}

fn plural(n: usize) -> &'static str {
    if n == 1 { "" } else { "s" }
}

#[cfg(test)]
impl World {
    /// Structural self-check shared with [`World::repair`]: the tile grid and the
    /// entity list agree in both directions, `free` holds exactly the empty
    /// slots, every simulation float is finite, and research progress has one
    /// entry per tech. The soak and corruption tests call this after every
    /// action so a NaN or a desynced grid surfaces here, at its source, instead
    /// of as a panic deep inside a later tick.
    pub fn check_invariants(&self) -> Result<(), String> {
        // Grid -> entities: every marked tile points at a live entity covering it.
        for (i, &cell) in self.grid.iter().enumerate() {
            let Some(id) = cell else { continue };
            let p = ivec2(i as i32 % MAP_W, i as i32 / MAP_W);
            let Some(e) = self.entities.get(id).and_then(|s| s.as_ref()) else {
                return Err(format!("grid[{i}] points at missing entity {id}"));
            };
            if !e.tiles().any(|t| t == p) {
                return Err(format!("grid[{i}] -> {id} doesn't cover tile {p}"));
            }
        }

        // Entities -> grid: every live entity's tile points back at it, in bounds.
        for (id, slot) in self.entities.iter().enumerate() {
            let Some(e) = slot else { continue };
            for t in e.tiles() {
                if !in_bounds(t) {
                    return Err(format!("entity {id} has an out-of-bounds tile {t}"));
                }
                if self.grid[idx(t)] != Some(id) {
                    return Err(format!("entity {id}'s tile {t} isn't marked in the grid"));
                }
            }
        }

        if !self.free_consistent() {
            return Err("free list disagrees with the entity slots".into());
        }

        if self.research.done.len() != TECHS.len() {
            return Err(format!(
                "research.done has {} entries, expected {}",
                self.research.done.len(),
                TECHS.len()
            ));
        }

        // `fin` only formats a message the moment it finds a non-finite value, so
        // the common all-finite case allocates nothing even over thousands of steps.
        let mut bad: Option<String> = None;
        macro_rules! fin {
            ($v:expr, $($label:tt)*) => {
                if !($v).is_finite() && bad.is_none() {
                    bad = Some(format!($($label)*));
                }
            };
        }

        fin!(self.spawn.x, "spawn.x is {}", self.spawn.x);
        fin!(self.spawn.y, "spawn.y is {}", self.spawn.y);
        fin!(self.time, "time is {}", self.time);
        fin!(self.evolution, "evolution is {}", self.evolution);
        fin!(self.pollution_timer, "pollution_timer is {}", self.pollution_timer);
        for (i, &c) in self.pollution.iter().enumerate() {
            fin!(c, "pollution[{i}] is {c}");
        }

        fin!(self.player.pos.x, "player.pos.x is {}", self.player.pos.x);
        fin!(self.player.pos.y, "player.pos.y is {}", self.player.pos.y);
        fin!(self.player.hp, "player.hp is {}", self.player.hp);
        fin!(self.player.calm, "player.calm is {}", self.player.calm);
        fin!(self.player.respawn, "player.respawn is {}", self.player.respawn);
        fin!(self.player.stride, "player.stride is {}", self.player.stride);

        for (id, slot) in self.entities.iter().enumerate() {
            let Some(e) = slot else { continue };
            fin!(e.hp, "entity {id} hp is {}", e.hp);
            fin!(e.calm, "entity {id} calm is {}", e.calm);
            match &e.state {
                State::Belt(b) => {
                    fin!(b.len, "entity {id} belt len is {}", b.len);
                    for (lane, items) in b.lanes.iter().enumerate() {
                        for (k, &(_, p)) in items.iter().enumerate() {
                            fin!(p, "entity {id} belt lane {lane} item {k} is {p}");
                        }
                    }
                }
                State::Splitter { .. } => {}
                State::Arm { swing, .. } => fin!(*swing, "entity {id} arm swing is {}", *swing),
                State::Chest { .. } => {}
                State::Drill { timer, .. } => fin!(*timer, "entity {id} drill timer is {}", *timer),
                State::Crafter(c) => {
                    fin!(c.progress, "entity {id} crafter progress is {}", c.progress);
                    fin!(c.fluid_in, "entity {id} crafter fluid_in is {}", c.fluid_in);
                    fin!(c.fluid_out, "entity {id} crafter fluid_out is {}", c.fluid_out);
                }
                State::Lab { progress, .. } => fin!(*progress, "entity {id} lab progress is {}", *progress),
                State::Generator { energy, load, .. } => {
                    fin!(*energy, "entity {id} generator energy is {}", *energy);
                    fin!(*load, "entity {id} generator load is {}", *load);
                }
                State::Pole | State::Solar | State::Wall => {}
                State::Pipe { amount, .. } => fin!(*amount, "entity {id} pipe amount is {}", *amount),
                State::Wellhead { amount } => fin!(*amount, "entity {id} wellhead amount is {}", *amount),
                State::Sentry { dmg, cooldown, aim, .. } => {
                    fin!(*dmg, "entity {id} sentry dmg is {}", *dmg);
                    fin!(*cooldown, "entity {id} sentry cooldown is {}", *cooldown);
                    fin!(*aim, "entity {id} sentry aim is {}", *aim);
                }
                State::Engine { .. } => {}
                State::Hive { agitation, cooldown, guard_cd } => {
                    fin!(*agitation, "entity {id} hive agitation is {}", *agitation);
                    fin!(*cooldown, "entity {id} hive cooldown is {}", *cooldown);
                    fin!(*guard_cd, "entity {id} hive guard_cd is {}", *guard_cd);
                }
            }
        }

        for (i, e) in self.enemies.iter().enumerate() {
            fin!(e.pos.x, "enemy {i} pos.x is {}", e.pos.x);
            fin!(e.pos.y, "enemy {i} pos.y is {}", e.pos.y);
            fin!(e.hp, "enemy {i} hp is {}", e.hp);
            fin!(e.target.x, "enemy {i} target.x is {}", e.target.x);
            fin!(e.target.y, "enemy {i} target.y is {}", e.target.y);
            fin!(e.home.x, "enemy {i} home.x is {}", e.home.x);
            fin!(e.home.y, "enemy {i} home.y is {}", e.home.y);
            fin!(e.cooldown, "enemy {i} cooldown is {}", e.cooldown);
            fin!(e.patience, "enemy {i} patience is {}", e.patience);
            fin!(e.phase, "enemy {i} phase is {}", e.phase);
        }

        match bad {
            None => Ok(()),
            Some(msg) => Err(msg),
        }
    }
}

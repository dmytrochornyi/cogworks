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

use super::logistics::Src;
use super::*;

/// Seconds per item mined by a drill.
pub const DRILL_TIME: f32 = 1.5;
pub const DRILL_BUFFER: usize = 4;
/// Tar per second per seep tile under a wellhead.
pub const WELL_RATE: f32 = 3.0;

impl World {
    pub(super) fn update_drill(&mut self, id: usize, dt: f32) {
        let e = self.entity(id);
        let ores: Vec<Item> = e.tiles().filter_map(|p| self.ore_at(p)).collect();
        let front = e.front();
        let pos = e.pos;
        let e = self.entity_mut(id);
        let State::Drill { timer, out, cursor } = &mut e.state else {
            return;
        };
        e.active = out.len() < DRILL_BUFFER && !ores.is_empty();
        if e.active {
            *timer += dt;
            if *timer >= DRILL_TIME {
                *timer -= DRILL_TIME;
                *cursor = (*cursor + 1) % ores.len();
                out.push(ores[*cursor]);
            }
        }
        if let Some(&item) = out.first() {
            // Drills drop onto whatever is in front, from the footprint cell touching it.
            let src_tile = self.adjacent_cell(id, front).unwrap_or(pos);
            if self.give(front, item, Src { tile: src_tile, lane: None })
                && let State::Drill { out, .. } = &mut self.entity_mut(id).state
            {
                out.remove(0);
            }
        }
    }

    /// The footprint cell of `id` that touches tile `outside`.
    fn adjacent_cell(&self, id: usize, outside: IVec2) -> Option<IVec2> {
        self.entity(id)
            .tiles()
            .find(|t| (*t - outside).abs().element_sum() == 1)
    }

    pub(super) fn update_crafter(&mut self, id: usize, dt: f32) {
        self.pull_fluid_input(id);
        let e = self.entity_mut(id);
        let State::Crafter(c) = &mut e.state else {
            return;
        };
        let Some(r) = c.recipe.map(|r| &RECIPES[r]) else {
            e.active = false;
            return;
        };
        if !c.crafting {
            let out_room = r.outputs.iter().all(|&(i, n)| c.out.get(i) + n <= n * 3);
            let fluid_room = r.fluid_out.is_none_or(|(_, n)| c.fluid_out + n <= n * 3.0);
            let fluid_ok = r.fluid_in.is_none_or(|(_, n)| c.fluid_in >= n);
            if c.input.has(r.inputs) && fluid_ok && out_room && fluid_room {
                for &(i, n) in r.inputs {
                    c.input.take(i, n);
                }
                if let Some((_, n)) = r.fluid_in {
                    c.fluid_in -= n;
                }
                c.crafting = true;
                c.progress = 0.0;
            }
        }
        if c.crafting {
            c.progress += dt / r.time;
            if c.progress >= 1.0 {
                c.crafting = false;
                c.progress = 0.0;
                for &(i, n) in r.outputs {
                    c.out.add(i, n);
                }
                if let Some((_, n)) = r.fluid_out {
                    c.fluid_out += n;
                }
            }
        }
        e.active = c.crafting;
        self.push_fluid_output(id);
    }

    pub(super) fn update_lab(&mut self, id: usize, dt: f32) {
        let Some(tech) = self.research.current else {
            self.entity_mut(id).active = false;
            return;
        };
        let t = &TECHS[tech];
        let e = self.entity_mut(id);
        let State::Lab { input, progress } = &mut e.state else {
            return;
        };
        if *progress <= 0.0 {
            if !t.science.iter().all(|&s| input.get(s) > 0) {
                e.active = false;
                return;
            }
            for &s in t.science {
                input.take(s, 1);
            }
            *progress = f32::EPSILON;
        }
        e.active = true;
        *progress += dt / t.time;
        if *progress >= 1.0 {
            *progress = 0.0;
            self.add_research(1);
        }
    }

    pub fn add_research(&mut self, units: u32) {
        let Some(tech) = self.research.current else {
            return;
        };
        self.research.progress += units;
        if self.research.progress >= TECHS[tech].units {
            self.research.done[tech] = true;
            self.research.current = None;
            self.research.progress = 0;
            self.events.push(format!("Research complete: {}", TECHS[tech].name));
            self.sounds.push(Sfx::Research);
        }
    }

    pub fn tech_available(&self, tech: usize) -> bool {
        !self.research.done[tech] && TECHS[tech].prereqs.iter().all(|&p| self.research.done[p])
    }

    pub fn set_research(&mut self, tech: usize) {
        if self.tech_available(tech) && self.research.current != Some(tech) {
            self.research.current = Some(tech);
            self.research.progress = 0;
            // Labs mid-unit restart on the new technology.
            for e in self.entities.iter_mut().flatten() {
                if let State::Lab { progress, .. } = &mut e.state {
                    *progress = 0.0;
                }
            }
        }
    }

    pub(super) fn update_wellhead(&mut self, id: usize, dt: f32) {
        let e = self.entity(id);
        let seeps = e.tiles().filter(|&p| self.res_at(p) == Some(Resource::Tar)).count();
        let e = self.entity_mut(id);
        let State::Wellhead { amount } = &mut e.state else {
            return;
        };
        e.active = *amount < 50.0 && seeps > 0;
        if e.active {
            *amount = (*amount + WELL_RATE * seeps as f32 * dt).min(50.0);
        }
        self.push_fluid_output(id);
    }

    /// Recipes a machine of this kind can be set to right now.
    pub fn machine_recipes(&self, kind: Kind) -> Vec<usize> {
        let Some(cat) = kind.category() else {
            return Vec::new();
        };
        (0..RECIPES.len())
            .filter(|&r| RECIPES[r].category == cat && self.recipe_unlocked(r))
            .collect()
    }

    /// Sets a machine's recipe, returning its old contents to the player.
    pub fn set_recipe(&mut self, id: usize, recipe: usize) {
        let refund = self.contents(id);
        let State::Crafter(c) = &mut self.entity_mut(id).state else {
            return;
        };
        if c.recipe == Some(recipe) {
            return;
        }
        *c = Crafter { recipe: Some(recipe), ..Default::default() };
        for (i, n) in refund.iter() {
            self.player.inv.add(i, n);
        }
    }

    /// Moves whatever the building can use from the player's inventory into it.
    /// Returns how many items were moved.
    pub fn insert_from_player(&mut self, id: usize) -> u32 {
        let e = self.entity(id);
        let wanted: Vec<(Item, u32)> = match &e.state {
            State::Crafter(c) => match c.recipe {
                Some(r) => RECIPES[r]
                    .inputs
                    .iter()
                    .map(|&(i, n)| (i, (n * 10).saturating_sub(c.input.get(i))))
                    .collect(),
                None => Vec::new(),
            },
            State::Lab { input, .. } => Item::ALL
                .iter()
                .filter(|i| i.is_science())
                .map(|&i| (i, 20u32.saturating_sub(input.get(i))))
                .collect(),
            State::Generator { fuel, .. } => [Item::FuelCell, Item::Coal]
                .iter()
                .map(|&i| (i, 20u32.saturating_sub(fuel.get(i))))
                .collect(),
            State::Sentry { ammo, .. } => [Item::HeavyClip, Item::SlugClip]
                .iter()
                .map(|&i| (i, 20u32.saturating_sub(ammo.get(i))))
                .collect(),
            State::Engine { cores } => vec![(Item::AetherCore, ENGINE_CORES - cores)],
            State::Chest { stock } => {
                // Crates take everything that isn't a building or ammo.
                let room = CHEST_CAP.saturating_sub(stock.total());
                let mut left = room;
                self.player
                    .inv
                    .iter()
                    .filter(|(i, _)| i.shape() != Shape::Building && i.ammo().is_none())
                    .map(|(i, n)| {
                        let t = n.min(left);
                        left -= t;
                        (i, t)
                    })
                    .collect()
            }
            _ => Vec::new(),
        };
        let mut moved = 0;
        for (item, want) in wanted {
            let n = self.player.inv.take(item, want);
            moved += n;
            match &mut self.entity_mut(id).state {
                State::Crafter(c) => c.input.add(item, n),
                State::Lab { input, .. } => input.add(item, n),
                State::Generator { fuel, .. } => fuel.add(item, n),
                State::Sentry { ammo, .. } => ammo.add(item, n),
                State::Chest { stock } => stock.add(item, n),
                State::Engine { .. } => {
                    for _ in 0..n {
                        self.insert(id, item, Src { tile: IVec2::ZERO, lane: None });
                    }
                }
                _ => {}
            }
        }
        moved
    }

    /// Moves a building's output (or a crate's contents) into the player's inventory.
    pub fn take_to_player(&mut self, id: usize) -> u32 {
        let taken = match &mut self.entity_mut(id).state {
            State::Crafter(c) => std::mem::take(&mut c.out),
            State::Chest { stock } => std::mem::take(stock),
            State::Drill { out, .. } => {
                let mut s = Stock::default();
                out.drain(..).for_each(|i| s.add(i, 1));
                s
            }
            _ => Stock::default(),
        };
        for (i, n) in taken.iter() {
            self.player.inv.add(i, n);
        }
        taken.total()
    }
}

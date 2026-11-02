use super::*;

impl World {
    pub fn entity_at(&self, p: IVec2) -> Option<usize> {
        if in_bounds(p) { self.grid[idx(p)] } else { None }
    }

    pub fn entity(&self, id: usize) -> &Entity {
        self.entities[id].as_ref().expect("dangling entity id")
    }

    pub fn entity_mut(&mut self, id: usize) -> &mut Entity {
        self.entities[id].as_mut().expect("dangling entity id")
    }

    pub fn entity_ref_at(&self, p: IVec2) -> Option<&Entity> {
        self.entity_at(p).map(|id| self.entity(id))
    }

    pub(super) fn footprint_free(&self, kind: Kind, pos: IVec2, dir: Dir) -> bool {
        footprint(kind, pos, dir).all(|p| in_bounds(p) && self.grid[idx(p)].is_none())
    }

    pub(super) fn spawn_entity(&mut self, kind: Kind, pos: IVec2, dir: Dir) -> usize {
        let state = match kind {
            Kind::Belt | Kind::Tunnel => State::Belt(BeltState { len: 1.0, ..Default::default() }),
            Kind::Splitter => State::Splitter { lanes: Default::default(), toggle: [false; 2] },
            Kind::Arm => State::Arm { held: None, swing: 0.0 },
            Kind::Chest => State::Chest { stock: Stock::default() },
            Kind::Drill => State::Drill { timer: 0.0, out: Vec::new(), cursor: 0 },
            Kind::Smelter | Kind::Assembler | Kind::Refinery | Kind::Mixer => {
                State::Crafter(Crafter::default())
            }
            Kind::Lab => State::Lab { input: Stock::default(), progress: 0.0 },
            Kind::Generator => State::Generator { fuel: Stock::default(), energy: 0.0, load: 0.0 },
            Kind::Pole => State::Pole,
            Kind::Solar => State::Solar,
            Kind::Wall => State::Wall,
            Kind::Pipe => State::Pipe { fluid: None, amount: 0.0 },
            Kind::Wellhead => State::Wellhead { amount: 0.0 },
            Kind::Sentry => State::Sentry {
                ammo: Stock::default(),
                shots: 0,
                dmg: 0.0,
                cooldown: 0.0,
                aim: 0.0,
            },
            Kind::GrandEngine => State::Engine { cores: 0 },
            Kind::Hive => State::Hive { agitation: 0.0, cooldown: 0.0, guard_cd: 0.0 },
        };
        let e = Entity {
            kind,
            pos,
            dir,
            hp: kind.max_hp(),
            calm: 100.0,
            active: false,
            state,
            net: None,
            fnet: None,
        };
        let id = match self.free.pop() {
            Some(id) => {
                self.entities[id] = Some(e);
                id
            }
            None => {
                self.entities.push(Some(e));
                self.entities.len() - 1
            }
        };
        for p in footprint(kind, pos, dir) {
            self.grid[idx(p)] = Some(id);
        }
        self.mark_dirty(kind);
        id
    }

    fn mark_dirty(&mut self, kind: Kind) {
        if kind.draw_kw() > 0.0 || matches!(kind, Kind::Pole | Kind::Generator | Kind::Solar) {
            self.power.dirty = true;
        }
        if kind == Kind::Pipe || kind.has_fluid_in() || kind.has_fluid_out() {
            self.fluids.dirty = true;
        }
    }

    /// Whether `kind` can go at `pos` right now. Doesn't check the player's inventory.
    pub fn check_place(&self, kind: Kind, pos: IVec2, dir: Dir) -> Result<(), String> {
        if !self.footprint_free(kind, pos, dir) {
            return Err("Space is occupied".into());
        }
        let tiles: Vec<IVec2> = footprint(kind, pos, dir).collect();
        if kind == Kind::Drill && !tiles.iter().any(|&p| self.ore_at(p).is_some()) {
            return Err("Drills must be placed over ore".into());
        }
        if kind == Kind::Wellhead && !tiles.iter().any(|&p| self.res_at(p) == Some(Resource::Tar)) {
            return Err("Wellheads must be placed over a tar seep".into());
        }
        if kind == Kind::Pipe {
            self.check_fluid_mix(pos)?;
        }
        if kind.solid() && self.player.alive() {
            let (lo, hi) = self.player.bounds();
            let (tlo, thi) = (pos.as_vec2(), (pos + dims(kind, dir)).as_vec2());
            if lo.x < thi.x && hi.x > tlo.x && lo.y < thi.y && hi.y > tlo.y {
                return Err("You're standing in the way".into());
            }
        }
        Ok(())
    }

    /// Rejects a pipe that would join two networks holding different fluids.
    fn check_fluid_mix(&self, pos: IVec2) -> Result<(), String> {
        let mut fluids: Vec<Fluid> = Vec::new();
        for d in Dir::ALL {
            let Some(net) = self.pipe_net(pos + d.vec()) else {
                continue;
            };
            let Some(f) = self.fluids.nets[net].fluid else {
                continue;
            };
            if !fluids.contains(&f) {
                fluids.push(f);
            }
        }
        fluids.sort_by_key(|f| *f as usize);
        if fluids.len() >= 2 {
            return Err(format!("Can't mix {} and {}", fluids[0].short_name(), fluids[1].short_name()));
        }
        Ok(())
    }

    /// Places a building from the player's inventory.
    pub fn place(&mut self, kind: Kind, pos: IVec2, dir: Dir) -> Result<(), String> {
        let item = kind.item().ok_or("Can't build that")?;
        if self.player.inv.get(item) == 0 {
            return Err(format!("Out of {}", item.name()));
        }
        self.check_place(kind, pos, dir)?;
        self.player.inv.take(item, 1);
        let id = self.spawn_entity(kind, pos, dir);
        self.sounds.push(Sfx::Place);
        let e = self.entity(id);
        self.fx.push(Fx::Built { pos: e.pos, size: e.dims() });
        if kind == Kind::Tunnel {
            self.pair_tunnel(id);
        }
        Ok(())
    }

    /// Picks up the building at `pos` into the player's inventory, along with its contents.
    pub fn deconstruct(&mut self, pos: IVec2) -> Option<Kind> {
        let id = self.entity_at(pos)?;
        let kind = self.entity(id).kind;
        let item = kind.item()?;
        let (epos, esize) = (self.entity(id).pos, self.entity(id).dims());
        let contents = self.contents(id);
        for (i, n) in contents.iter() {
            self.player.inv.add(i, n);
        }
        // Picking up one end of a tunnel also returns its in-flight underground
        // items, which `contents` can't see (they live on the partner's lanes).
        if kind == Kind::Tunnel {
            for (i, n) in self.drain_underground(id).iter() {
                self.player.inv.add(i, n);
            }
        }
        self.player.inv.add(item, 1);
        self.delete(id);
        self.sounds.push(Sfx::Pickup);
        self.fx.push(Fx::Removed { pos: epos, size: esize });
        Some(kind)
    }

    /// Every item held inside an entity.
    pub fn contents(&self, id: usize) -> Stock {
        let mut s = Stock::default();
        match &self.entity(id).state {
            State::Belt(b) => b.lanes.iter().flatten().for_each(|(i, _)| s.add(*i, 1)),
            State::Splitter { lanes, .. } => lanes.iter().flatten().for_each(|i| s.add(*i, 1)),
            State::Arm { held: Some(i), .. } => s.add(*i, 1),
            State::Chest { stock } => s = stock.clone(),
            State::Drill { out, .. } => out.iter().for_each(|i| s.add(*i, 1)),
            State::Crafter(c) => {
                for (i, n) in c.input.iter().chain(c.out.iter()) {
                    s.add(i, n);
                }
            }
            State::Lab { input, .. } => s = input.clone(),
            State::Generator { fuel, .. } => s = fuel.clone(),
            State::Sentry { ammo, .. } => s = ammo.clone(),
            _ => {}
        }
        s
    }

    /// Removes an entity without refunding anything.
    pub fn delete(&mut self, id: usize) {
        let Some(e) = self.entities[id].take() else {
            return;
        };
        let fnet = e.fnet;
        for p in e.tiles() {
            self.grid[idx(p)] = None;
        }
        self.free.push(id);
        self.mark_dirty(e.kind);
        // A removed pipe stops counting toward its network right away, so the
        // surviving pipes keep the rest of the fluid (minus this pipe's share).
        if let Some(net) = fnet {
            self.detach_pipe(id, net);
        }
        if let State::Belt(BeltState { partner: Some(other), .. }) = e.state {
            self.unpair_tunnel(other);
        }
    }

    pub fn rotate(&mut self, pos: IVec2, cw: bool) {
        let Some(id) = self.entity_at(pos) else {
            return;
        };
        let e = self.entity_mut(id);
        if !e.kind.rotatable() {
            return;
        }
        // Only square buildings rotate in place.
        e.dir = if cw { e.dir.cw() } else { e.dir.ccw() };
        let kind = e.kind;
        self.mark_dirty(kind);
    }

    pub fn set_dir(&mut self, pos: IVec2, dir: Dir) {
        if let Some(id) = self.entity_at(pos) {
            let e = self.entity_mut(id);
            if e.kind == Kind::Belt {
                e.dir = dir;
            }
        }
    }
}

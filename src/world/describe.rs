use super::*;

impl World {
    /// Lines of text describing the tile at `pos`; the first is its name.
    pub fn describe(&self, pos: IVec2) -> Vec<String> {
        self.describe_rows(pos).into_iter().map(|(_, l)| l).collect()
    }

    /// The icon that stands for the tile at `pos` in its tooltip: the building's
    /// item, the ore, or wood for a tree.
    pub fn describe_icon(&self, pos: IVec2) -> Option<Item> {
        match self.entity_ref_at(pos) {
            Some(e) => e.kind.item(),
            None => match self.res_at(pos) {
                Some(Resource::Ore(i)) => Some(i),
                _ => None,
            },
        }
    }

    /// Like [`World::describe`], with an item icon on each row that is about one.
    pub fn describe_rows(&self, pos: IVec2) -> Vec<(Option<Item>, String)> {
        let mut lines: Vec<(Option<Item>, String)> = Vec::new();
        let Some(id) = self.entity_at(pos) else {
            if let Some(r) = self.res_at(pos) {
                lines.push((None, r.name().to_string()));
                lines.push((None, match r {
                    Resource::Ore(_) => "Hold right click to mine".into(),
                    Resource::Tar => "Place a Wellhead here".into(),
                }));
            }
            return lines;
        };
        let e = self.entity(id);
        lines.push((None, e.kind.name().to_string()));
        if e.hp < e.kind.max_hp() {
            lines.push((None, format!("HP {:.0}/{:.0}", e.hp, e.kind.max_hp())));
        }
        if e.kind.draw_kw() > 0.0 {
            match e.net {
                None => lines.push((None, "No power - place a Power Pole nearby".into())),
                Some(n) => {
                    let sat = self.power.nets[n].sat;
                    if sat < 0.99 {
                        lines.push((None, format!("Low power: {:.0}%", sat * 100.0)));
                    }
                }
            }
        }
        match &e.state {
            State::Belt(b) => {
                let n: usize = b.lanes.iter().map(Vec::len).sum();
                lines.push((None, format!("Carrying {n} item(s)")));
                if e.kind == Kind::Tunnel {
                    lines.push((None, match (b.exit, b.partner) {
                        (_, None) => format!("Unpaired - place another within {TUNNEL_MAX} tiles"),
                        (true, _) => "Tunnel exit".into(),
                        (false, _) => "Tunnel entrance".into(),
                    }));
                }
            }
            State::Arm { held, .. } => {
                lines.push((None, "Moves items from behind to the front".into()));
                if let Some(i) = held {
                    lines.push((Some(*i), format!("Holding {}", i.name())));
                }
            }
            State::Drill { out, .. } => {
                let ores: Vec<&str> = e
                    .tiles()
                    .filter_map(|p| self.ore_at(p))
                    .fold(Vec::new(), |mut v, i| {
                        if !v.contains(&i.name()) {
                            v.push(i.name());
                        }
                        v
                    });
                lines.push((None, format!("Mining {}", ores.join(", "))));
                lines.push((None, format!("Buffered: {}", out.len())));
            }
            State::Crafter(c) => match c.recipe {
                Some(r) => {
                    let r = &RECIPES[r];
                    lines.push((None, format!("Recipe: {} ({:.0}%)", r.name, c.progress * 100.0)));
                    if let Some((f, n)) = r.fluid_in {
                        lines.push((None, format!("{} {:.0}/{n}", f.name(), c.fluid_in)));
                    }
                    for (i, n) in r.inputs {
                        lines.push((Some(*i), format!("{} {}/{}", i.name(), c.input.get(*i), n)));
                    }
                    for (i, n) in c.out.iter() {
                        lines.push((Some(i), format!("Output: {} x{n}", i.name())));
                    }
                    if let Some((f, _)) = r.fluid_out {
                        lines.push((None, format!("Output: {} {:.0}", f.name(), c.fluid_out)));
                    }
                }
                None if e.kind == Kind::Smelter => {
                    lines.push((None, "Picks a recipe from what it's fed".into()))
                }
                None => lines.push((None, "No recipe - click to choose".into())),
            },
            State::Lab { input, .. } => {
                for (i, n) in input.iter() {
                    lines.push((Some(i), format!("{} x{n}", i.name())));
                }
                if self.research.current.is_none() {
                    lines.push((None, "No research selected (T)".into()));
                }
            }
            State::Generator { fuel, load, .. } => {
                lines.push((None, format!("Load: {:.0}%", load * 100.0)));
                for (i, n) in fuel.iter() {
                    lines.push((Some(i), format!("{} x{n}", i.name())));
                }
            }
            State::Chest { stock } => lines.push((None, format!("{} items stored", stock.total()))),
            State::Pipe { .. } => {
                if let Some(n) = e.fnet.and_then(|n| self.fluids.nets.get(n)) {
                    let name = n.fluid.map_or("Empty", Fluid::name);
                    lines.push((None, format!("{name}: {:.0}/{:.0}", n.amount, n.cap)));
                }
            }
            State::Wellhead { amount } => lines.push((None, format!("Tar buffered: {amount:.0}"))),
            State::Sentry { ammo, shots, .. } => {
                let clips: u32 = ammo.iter().map(|(_, n)| n).sum();
                lines.push((None, format!("Ammo: {clips} clips + {shots} shots")));
            }
            State::Engine { cores } => {
                lines.push((None, format!("Aether Cores: {cores}/{ENGINE_CORES}")))
            }
            State::Hive { .. } => lines.push((None, "Spawns mites when pollution reaches it".into())),
            _ => {}
        }
        lines
    }
}

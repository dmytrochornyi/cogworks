use super::*;

impl World {
    /// Lines of text describing the tile at `pos`, for the hover tooltip.
    pub fn describe(&self, pos: IVec2) -> Vec<String> {
        let mut lines = Vec::new();
        let Some(id) = self.entity_at(pos) else {
            if let Some(r) = self.res_at(pos) {
                lines.push(r.name().to_string());
                lines.push(match r {
                    Resource::Ore(_) => "Hold right click to mine".into(),
                    Resource::Tar => "Place a Wellhead here".into(),
                });
            }
            return lines;
        };
        let e = self.entity(id);
        lines.push(e.kind.name().to_string());
        if e.hp < e.kind.max_hp() {
            lines.push(format!("HP {:.0}/{:.0}", e.hp, e.kind.max_hp()));
        }
        if e.kind.draw_kw() > 0.0 {
            match e.net {
                None => lines.push("No power - place a Power Pole nearby".into()),
                Some(n) => {
                    let sat = self.power.nets[n].sat;
                    if sat < 0.99 {
                        lines.push(format!("Low power: {:.0}%", sat * 100.0));
                    }
                }
            }
        }
        match &e.state {
            State::Belt(b) => {
                let n: usize = b.lanes.iter().map(Vec::len).sum();
                lines.push(format!("Carrying {n} item(s)"));
                if e.kind == Kind::Tunnel {
                    lines.push(match (b.exit, b.partner) {
                        (_, None) => format!("Unpaired - place another within {TUNNEL_MAX} tiles"),
                        (true, _) => "Tunnel exit".into(),
                        (false, _) => "Tunnel entrance".into(),
                    });
                }
            }
            State::Arm { held, .. } => {
                lines.push("Moves items from behind to the front".into());
                if let Some(i) = held {
                    lines.push(format!("Holding {}", i.name()));
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
                lines.push(format!("Mining {}", ores.join(", ")));
                lines.push(format!("Buffered: {}", out.len()));
            }
            State::Crafter(c) => match c.recipe {
                Some(r) => {
                    let r = &RECIPES[r];
                    lines.push(format!("Recipe: {} ({:.0}%)", r.name, c.progress * 100.0));
                    if let Some((f, n)) = r.fluid_in {
                        lines.push(format!("  {} {:.0}/{n}", f.name(), c.fluid_in));
                    }
                    for (i, n) in r.inputs {
                        lines.push(format!("  {} {}/{}", i.name(), c.input.get(*i), n));
                    }
                    for (i, n) in c.out.iter() {
                        lines.push(format!("  Output: {} x{n}", i.name()));
                    }
                    if let Some((f, _)) = r.fluid_out {
                        lines.push(format!("  Output: {} {:.0}", f.name(), c.fluid_out));
                    }
                }
                None if e.kind == Kind::Smelter => {
                    lines.push("Picks a recipe from what it's fed".into())
                }
                None => lines.push("No recipe - click to choose".into()),
            },
            State::Lab { input, .. } => {
                for (i, n) in input.iter() {
                    lines.push(format!("  {} x{n}", i.name()));
                }
                if self.research.current.is_none() {
                    lines.push("No research selected (T)".into());
                }
            }
            State::Generator { fuel, load, .. } => {
                lines.push(format!("Load: {:.0}%", load * 100.0));
                for (i, n) in fuel.iter() {
                    lines.push(format!("  {} x{n}", i.name()));
                }
            }
            State::Chest { stock } => lines.push(format!("{} items stored", stock.total())),
            State::Pipe { .. } => {
                if let Some(n) = e.fnet.and_then(|n| self.fluids.nets.get(n)) {
                    let name = n.fluid.map_or("Empty", Fluid::name);
                    lines.push(format!("{name}: {:.0}/{:.0}", n.amount, n.cap));
                }
            }
            State::Wellhead { amount } => lines.push(format!("Tar buffered: {amount:.0}")),
            State::Sentry { ammo, shots, .. } => {
                let clips: u32 = ammo.iter().map(|(_, n)| n).sum();
                lines.push(format!("Ammo: {clips} clips + {shots} shots"));
            }
            State::Engine { cores } => {
                lines.push(format!("Aether Cores: {cores}/{ENGINE_CORES}"))
            }
            State::Hive { .. } => lines.push("Spawns mites when pollution reaches it".into()),
            _ => {}
        }
        lines
    }
}

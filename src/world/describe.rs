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
        self.describe_rows_raw(pos).into_iter().map(|(i, l, _)| (i, l)).collect()
    }

    /// [`World::describe_rows`] with the action-hint rows dropped, for the map
    /// tooltip: the map can't mine, place, choose or pick things up, so hints
    /// like "Hold right click to mine" would mislead there.
    pub fn describe_rows_for_map(&self, pos: IVec2) -> Vec<(Option<Item>, String)> {
        self.describe_rows_raw(pos).into_iter().filter(|(_, _, hint)| !hint).map(|(i, l, _)| (i, l)).collect()
    }

    /// [`World::describe_rows`] with each row's action hint flagged as a third
    /// field: `true` when the row tells the player to do something only the
    /// world lets them do, rather than describing the tile.
    fn describe_rows_raw(&self, pos: IVec2) -> Vec<(Option<Item>, String, bool)> {
        let mut lines: Vec<(Option<Item>, String, bool)> = Vec::new();
        let Some(id) = self.entity_at(pos) else {
            if let Some(r) = self.res_at(pos) {
                lines.push((None, r.name().to_string(), false));
                lines.push((None, match r {
                    Resource::Ore(_) => "Hold right click to mine".into(),
                    Resource::Tar => "Place a Wellhead here".into(),
                }, true));
            }
            return lines;
        };
        let e = self.entity(id);
        lines.push((None, e.kind.name().to_string(), false));
        if e.hp < e.kind.max_hp() {
            lines.push((None, format!("HP {:.0}/{:.0}", e.hp, e.kind.max_hp()), false));
        }
        if e.kind.draw_kw() > 0.0 {
            match e.net {
                None => lines.push((None, "No power - place a Power Pole nearby".into(), true)),
                Some(n) => {
                    let sat = self.power.nets[n].sat;
                    if sat < 0.99 {
                        lines.push((None, format!("Low power: {:.0}%", sat * 100.0), false));
                    }
                }
            }
        }
        match &e.state {
            State::Belt(b) => {
                let n: usize = b.lanes.iter().map(Vec::len).sum();
                let items = if n == 1 { "item" } else { "items" };
                lines.push((None, format!("Carrying {n} {items}"), false));
                if e.kind == Kind::Tunnel {
                    match (b.exit, b.partner) {
                        (_, None) => lines.push((None, format!("Unpaired - place another within {TUNNEL_MAX} tiles"), true)),
                        (true, _) => lines.push((None, "Tunnel exit".into(), false)),
                        (false, _) => lines.push((None, "Tunnel entrance".into(), false)),
                    }
                }
            }
            State::Arm { held, .. } => {
                lines.push((None, "Moves items from behind to the front".into(), false));
                if let Some(i) = held {
                    lines.push((Some(*i), format!("Holding {}", i.name()), false));
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
                lines.push((None, format!("Mining {}", ores.join(", ")), false));
                lines.push((None, format!("Buffered: {}", out.len()), false));
            }
            State::Crafter(c) => match c.recipe {
                Some(r) => {
                    let r = &RECIPES[r];
                    lines.push((None, format!("Recipe: {} ({:.0}%)", r.name, c.progress * 100.0), false));
                    if let Some((f, n)) = r.fluid_in {
                        lines.push((None, format!("{} {:.0}/{n}", f.name(), c.fluid_in), false));
                    }
                    for (i, n) in r.inputs {
                        lines.push((Some(*i), format!("{} {}/{}", i.name(), c.input.get(*i), n), false));
                    }
                    for (i, n) in c.out.iter() {
                        lines.push((Some(i), format!("Output: {} x{n}", i.name()), false));
                    }
                    if let Some((f, _)) = r.fluid_out {
                        lines.push((None, format!("Output: {} {:.0}", f.name(), c.fluid_out), false));
                    }
                }
                None if e.kind == Kind::Smelter => {
                    lines.push((None, "Picks a recipe from what it's fed".into(), false))
                }
                None => lines.push((None, "No recipe - click to choose".into(), true)),
            },
            State::Lab { input, .. } => {
                for (i, n) in input.iter() {
                    lines.push((Some(i), format!("{} x{n}", i.name()), false));
                }
                if self.research.current.is_none() {
                    lines.push((None, "No research selected (T)".into(), true));
                }
            }
            State::Generator { fuel, load, .. } => {
                lines.push((None, format!("Load: {:.0}%", load * 100.0), false));
                for (i, n) in fuel.iter() {
                    lines.push((Some(i), format!("{} x{n}", i.name()), false));
                }
            }
            State::Chest { stock } => lines.push((None, format!("{} items stored", stock.total()), false)),
            State::Pipe { .. } => {
                if let Some(n) = e.fnet.and_then(|n| self.fluids.nets.get(n)) {
                    let name = n.fluid.map_or("Empty", Fluid::name);
                    lines.push((None, format!("{name}: {:.0}/{:.0}", n.amount, n.cap), false));
                }
            }
            State::Wellhead { amount } => lines.push((None, format!("Tar buffered: {amount:.0}"), false)),
            State::Sentry { ammo, shots, .. } => {
                let clips: u32 = ammo.iter().map(|(_, n)| n).sum();
                lines.push((None, format!("Ammo: {clips} clips + {shots} shots"), false));
            }
            State::Engine { cores } => {
                lines.push((None, format!("Aether Cores: {cores}/{ENGINE_CORES}"), false))
            }
            State::Hive { .. } => lines.push((None, "Spawns mites when pollution reaches it".into(), false)),
            _ => {}
        }
        lines
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn first_ore(w: &World) -> IVec2 {
        (0..MAP_H)
            .flat_map(|y| (0..MAP_W).map(move |x| ivec2(x, y)))
            .find(|&p| matches!(w.res_at(p), Some(Resource::Ore(_))) && w.entity_at(p).is_none())
            .expect("a generated world always has ore")
    }

    #[test]
    fn map_rows_drop_action_hints() {
        let w = World::new(7);
        let p = first_ore(&w);

        let world_rows = w.describe_rows(p);
        assert!(
            world_rows.iter().any(|(_, l)| l == "Hold right click to mine"),
            "the world tooltip keeps the mining hint: {world_rows:?}"
        );

        let map_rows = w.describe_rows_for_map(p);
        assert!(
            map_rows.iter().all(|(_, l)| l != "Hold right click to mine"),
            "the map tooltip drops the mining hint: {map_rows:?}"
        );
        assert_eq!(map_rows.first().map(|(_, l)| l.as_str()), world_rows.first().map(|(_, l)| l.as_str()));
    }

    #[test]
    fn hint_flag_marks_only_instructions() {
        let w = World::new(7);
        let p = first_ore(&w);
        let rows = w.describe_rows_raw(p);

        let name = rows.first().expect("a name row");
        assert!(!name.2, "the name row is not an action hint");

        let mine = rows.iter().find(|(_, l, _)| l == "Hold right click to mine").expect("a mining hint");
        assert!(mine.2, "the mining hint is flagged");
    }
}

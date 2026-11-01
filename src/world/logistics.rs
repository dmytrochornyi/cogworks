use macroquad::prelude::*;

use super::*;

/// Arm swings per second (one way).
pub const ARM_SPEED: f32 = 1.8;

/// Where an item handed to another entity comes from.
#[derive(Clone, Copy, Debug)]
pub struct Src {
    /// Tile of the giver, adjacent to the receiver (or the tunnel partner).
    pub tile: IVec2,
    /// Lane the item travels in, for belt-like givers.
    pub lane: Option<usize>,
}

impl World {
    /// Whether the belt-like entity at `tile` outputs straight into `target`.
    pub fn feeds(&self, tile: IVec2, target: IVec2) -> bool {
        let Some(e) = self.entity_ref_at(tile) else {
            return false;
        };
        match &e.state {
            State::Belt(b) if b.exit || e.kind == Kind::Belt => tile + e.dir.vec() == target,
            State::Splitter { .. } => tile + e.dir.vec() == target,
            _ => false,
        }
    }

    /// For a plain belt fed only from one side, the offset to that side: it then acts as a curve.
    pub fn curve_side(&self, id: usize) -> Option<IVec2> {
        let e = self.entity(id);
        if e.kind != Kind::Belt {
            return None;
        }
        let f = e.dir.vec();
        if self.feeds(e.pos - f, e.pos) {
            return None;
        }
        let l = e.dir.left();
        let (a, b) = (self.feeds(e.pos + l, e.pos), self.feeds(e.pos - l, e.pos));
        match (a, b) {
            (true, false) => Some(l),
            (false, true) => Some(-l),
            _ => None,
        }
    }

    /// Lane and entry position an item from `src` would take on belt `id`, ignoring spacing.
    fn belt_entry(&self, id: usize, src: Src) -> Option<(usize, f32)> {
        let e = self.entity(id);
        let State::Belt(b) = &e.state else {
            return None;
        };
        let f = e.dir.vec();
        let rel = src.tile - e.pos;
        let behind = rel == -f;
        let side_lane = |rel: IVec2| if rel == e.dir.left() { 0 } else { 1 };
        match src.lane {
            Some(lane) => {
                if e.kind == Kind::Tunnel {
                    // Exits take items only from their entrance; entrances only from behind.
                    let ok = if b.exit { b.partner == Some(src.tile) } else { behind };
                    return ok.then_some((lane, 0.0));
                }
                if behind {
                    Some((lane, 0.0))
                } else if rel == f {
                    None
                } else if self.curve_side(id) == Some(rel) {
                    Some((lane, 0.0))
                } else {
                    Some((side_lane(rel), 0.5))
                }
            }
            // Arms and drills drop onto the far lane, or the right lane from behind/front.
            None => {
                if rel == f || behind {
                    Some((1, if behind { 0.0 } else { 0.5 }))
                } else {
                    Some((1 - side_lane(rel), 0.5))
                }
            }
        }
    }

    pub fn can_insert(&self, id: usize, item: Item, src: Src) -> bool {
        let e = self.entity(id);
        match &e.state {
            State::Belt(b) => self
                .belt_entry(id, src)
                .is_some_and(|(lane, p)| lane_has_room(&b.lanes[lane], p)),
            State::Splitter { lanes, .. } => {
                let lane = src.lane.unwrap_or(1);
                e.tiles().any(|t| t - e.dir.vec() == src.tile) && lanes[lane].len() < 2
            }
            State::Chest { stock } => stock.total() < CHEST_CAP,
            State::Crafter(c) => {
                let recipe = c.recipe.or_else(|| self.auto_recipe(e.kind, item));
                recipe.and_then(|r| RECIPES[r].needs(item)).is_some_and(|n| c.input.get(item) < n * 2)
            }
            State::Lab { input, .. } => item.is_science() && input.get(item) < 4,
            State::Generator { fuel, .. } => item.fuel_value().is_some() && fuel.get(item) < 10,
            State::Sentry { ammo, .. } => item.ammo().is_some() && ammo.get(item) < 10,
            State::Engine { cores } => item == Item::AetherCore && *cores < ENGINE_CORES,
            _ => false,
        }
    }

    /// Hands over an item; callers must check `can_insert` first.
    pub fn insert(&mut self, id: usize, item: Item, src: Src) {
        let entry = self.belt_entry(id, src);
        let auto = self.auto_recipe(self.entity(id).kind, item);
        let e = self.entity_mut(id);
        match &mut e.state {
            State::Belt(b) => {
                if let Some((lane, p)) = entry {
                    let items = &mut b.lanes[lane];
                    let at = items.iter().position(|(_, q)| *q < p).unwrap_or(items.len());
                    items.insert(at, (item, p));
                }
            }
            State::Splitter { lanes, .. } => lanes[src.lane.unwrap_or(1)].push(item),
            State::Chest { stock } => stock.add(item, 1),
            State::Crafter(c) => {
                if c.recipe.is_none() {
                    c.recipe = auto;
                }
                c.input.add(item, 1);
            }
            State::Lab { input, .. } => input.add(item, 1),
            State::Generator { fuel, .. } => fuel.add(item, 1),
            State::Sentry { ammo, .. } => ammo.add(item, 1),
            State::Engine { cores } => {
                *cores += 1;
                if *cores >= ENGINE_CORES && !self.won {
                    self.won = true;
                    self.events.push("The Grand Engine awakens! You win!".into());
                }
            }
            _ => {}
        }
    }

    /// Gives `item` to whatever occupies `to`.
    pub fn give(&mut self, to: IVec2, item: Item, src: Src) -> bool {
        let Some(target) = self.entity_at(to) else {
            return false;
        };
        if !self.can_insert(target, item, src) {
            return false;
        }
        self.insert(target, item, src);
        true
    }

    /// A smelter with no recipe picks the only unlocked smelting recipe that uses `item`.
    pub fn auto_recipe(&self, kind: Kind, item: Item) -> Option<usize> {
        if kind != Kind::Smelter {
            return None;
        }
        let mut found = RECIPES.iter().enumerate().filter(|(id, r)| {
            r.category == Category::Smelting && r.needs(item).is_some() && self.recipe_unlocked(*id)
        });
        let first = found.next()?;
        if found.next().is_some() { None } else { Some(first.0) }
    }

    /// Items an arm could take from an entity, best candidates first.
    fn takeable(&self, id: usize) -> Vec<Item> {
        match &self.entity(id).state {
            State::Belt(b) => {
                let mut items: Vec<(Item, f32)> = b
                    .lanes
                    .iter()
                    .flatten()
                    .filter(|(_, p)| *p <= 1.0)
                    .copied()
                    .collect();
                items.sort_by(|a, b| b.1.total_cmp(&a.1));
                items.into_iter().map(|(i, _)| i).collect()
            }
            State::Splitter { lanes, .. } => lanes.iter().flatten().copied().collect(),
            State::Chest { stock } => stock.iter().map(|(i, _)| i).collect(),
            State::Drill { out, .. } => out.first().copied().into_iter().collect(),
            State::Crafter(c) => c.out.iter().map(|(i, _)| i).collect(),
            _ => Vec::new(),
        }
    }

    fn take_item(&mut self, id: usize, item: Item) {
        match &mut self.entity_mut(id).state {
            State::Belt(b) => {
                let best = (0..2)
                    .flat_map(|l| b.lanes[l].iter().enumerate().map(move |(k, x)| (l, k, *x)))
                    .filter(|(_, _, (i, p))| *i == item && *p <= 1.0)
                    .max_by(|a, b| a.2.1.total_cmp(&b.2.1));
                if let Some((l, k, _)) = best {
                    b.lanes[l].remove(k);
                }
            }
            State::Splitter { lanes, .. } => {
                for lane in lanes {
                    if let Some(k) = lane.iter().position(|i| *i == item) {
                        lane.remove(k);
                        return;
                    }
                }
            }
            State::Chest { stock } => {
                stock.take(item, 1);
            }
            State::Drill { out, .. } => {
                out.remove(0);
            }
            State::Crafter(c) => {
                c.out.take(item, 1);
            }
            _ => {}
        }
    }

    pub(super) fn update_belt(&mut self, id: usize, dt: f32) {
        let e = self.entity_mut(id);
        let (pos, dir) = (e.pos, e.dir);
        let State::Belt(b) = &mut e.state else {
            return;
        };
        let target = if e.kind == Kind::Tunnel && !b.exit { b.partner } else { Some(pos + dir.vec()) };
        let len = b.len;
        let mut ready = [None; 2];
        for (lane, items) in b.lanes.iter_mut().enumerate() {
            let mut limit = len;
            for (_, p) in items.iter_mut() {
                *p = (*p + BELT_SPEED * dt).min(limit.max(*p));
                limit = *p - BELT_SPACING;
            }
            ready[lane] = items.first().filter(|(_, p)| *p >= len).map(|(i, _)| *i);
        }
        let Some(target) = target else {
            return;
        };
        for (lane, item) in ready.into_iter().enumerate() {
            if let Some(item) = item
                && self.give(target, item, Src { tile: pos, lane: Some(lane) })
                && let State::Belt(b) = &mut self.entity_mut(id).state
            {
                b.lanes[lane].remove(0);
            }
        }
    }

    pub(super) fn update_splitter(&mut self, id: usize) {
        let e = self.entity(id);
        let cells: Vec<IVec2> = e.tiles().collect();
        let f = e.dir.vec();
        let State::Splitter { lanes, toggle } = &e.state else {
            return;
        };
        let (lanes, mut toggle) = (lanes.clone(), *toggle);
        let mut sent = [false; 2];
        for lane in 0..2 {
            let Some(&item) = lanes[lane].first() else {
                continue;
            };
            let order = if toggle[lane] { [1, 0] } else { [0, 1] };
            for k in order {
                let src = Src { tile: cells[k], lane: Some(lane) };
                if self.give(cells[k] + f, item, src) {
                    sent[lane] = true;
                    toggle[lane] = k == 0;
                    break;
                }
            }
        }
        if let State::Splitter { lanes, toggle: t } = &mut self.entity_mut(id).state {
            for lane in 0..2 {
                if sent[lane] {
                    lanes[lane].remove(0);
                }
            }
            *t = toggle;
        }
    }

    pub(super) fn update_arm(&mut self, id: usize, dt: f32) {
        let e = self.entity(id);
        let (pos, dir) = (e.pos, e.dir);
        let State::Arm { held, swing } = e.state else {
            return;
        };
        let step = ARM_SPEED * dt;
        let (from, to) = (pos - dir.vec(), pos + dir.vec());
        let src = Src { tile: pos, lane: None };
        let mut active = false;
        let (held, swing) = match held {
            Some(item) if swing < 1.0 => {
                active = true;
                (Some(item), (swing + step).min(1.0))
            }
            Some(item) => {
                if self.give(to, item, src) { (None, swing) } else { (Some(item), swing) }
            }
            None if swing > 0.0 => {
                active = true;
                (None, (swing - step).max(0.0))
            }
            None => {
                let pick = match (self.entity_at(from), self.entity_at(to)) {
                    (Some(s), Some(t)) if s != t => self
                        .takeable(s)
                        .into_iter()
                        .find(|&i| self.can_insert(t, i, src))
                        .map(|i| (s, i)),
                    _ => None,
                };
                // Picking up needs power too: a stalled arm shouldn't grab items.
                match pick {
                    Some((s, item)) if dt > 0.0 => {
                        self.take_item(s, item);
                        active = true;
                        (Some(item), 0.0)
                    }
                    _ => (None, 0.0),
                }
            }
        };
        let e = self.entity_mut(id);
        e.state = State::Arm { held, swing };
        // Arms draw power while they have something to do, even if starved.
        e.active = active || held.is_some();
    }

    fn pair_tunnel_at(&mut self, id: usize, other: usize) {
        let (a, b) = (self.entity(id).pos, self.entity(other).pos);
        let dist = (a - b).abs().max_element() as f32;
        if let State::Belt(s) = &mut self.entity_mut(other).state {
            s.partner = Some(a);
            s.len = dist;
            s.exit = false;
        }
        if let State::Belt(s) = &mut self.entity_mut(id).state {
            s.partner = Some(b);
            s.exit = true;
        }
    }

    /// Links a freshly placed tunnel as the exit of an unpaired entrance behind it.
    pub(super) fn pair_tunnel(&mut self, id: usize) {
        let e = self.entity(id);
        let (pos, dir) = (e.pos, e.dir);
        for d in 1..=TUNNEL_MAX {
            let Some(other) = self.entity_at(pos - dir.vec() * d) else {
                continue;
            };
            let o = self.entity(other);
            if o.kind == Kind::Tunnel && o.dir == dir {
                if let State::Belt(b) = &o.state
                    && !b.exit
                    && b.partner.is_none()
                {
                    self.pair_tunnel_at(id, other);
                }
                return;
            }
        }
    }

    pub(super) fn unpair_tunnel(&mut self, pos: IVec2) {
        if let Some(id) = self.entity_at(pos)
            && let State::Belt(b) = &mut self.entity_mut(id).state
        {
            b.partner = None;
            b.len = 1.0;
            for lane in &mut b.lanes {
                lane.retain(|(_, p)| *p <= 1.0);
            }
        }
    }
}

fn lane_has_room(items: &[(Item, f32)], p: f32) -> bool {
    items.iter().all(|(_, q)| (q - p).abs() >= BELT_SPACING)
}

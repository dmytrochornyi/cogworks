mod combat;
mod fluids;
mod logistics;
mod player;
mod power;
mod production;
#[cfg(test)]
mod tests;

use std::f32::consts::TAU;

use macroquad::prelude::*;
use serde::{Deserialize, Serialize};

use crate::items::*;
pub use combat::{Enemy, Shot};
pub use fluids::FluidNets;
pub use player::{PLAYER_HP, Player};
pub use power::{POLE_SUPPLY, Power};

pub const MAP_W: i32 = 256;
pub const MAP_H: i32 = 256;
/// Tiles per second.
pub const BELT_SPEED: f32 = 1.6;
/// Minimum distance between items in one belt lane, in tiles.
pub const BELT_SPACING: f32 = 0.25;
/// Longest tunnel: entrance and exit may be at most this many tiles apart.
pub const TUNNEL_MAX: i32 = 6;
/// Player reach for building, mining and removal, in tiles.
pub const REACH: f32 = 12.0;
pub const CHEST_CAP: u32 = 800;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum Dir {
    N,
    E,
    S,
    W,
}

impl Dir {
    pub const ALL: [Dir; 4] = [Dir::N, Dir::E, Dir::S, Dir::W];

    pub fn vec(self) -> IVec2 {
        match self {
            Dir::N => ivec2(0, -1),
            Dir::E => ivec2(1, 0),
            Dir::S => ivec2(0, 1),
            Dir::W => ivec2(-1, 0),
        }
    }

    pub fn cw(self) -> Dir {
        Dir::ALL[(self as usize + 1) % 4]
    }

    pub fn ccw(self) -> Dir {
        Dir::ALL[(self as usize + 3) % 4]
    }

    pub fn opposite(self) -> Dir {
        Dir::ALL[(self as usize + 2) % 4]
    }

    pub fn from_delta(d: IVec2) -> Option<Dir> {
        Dir::ALL.into_iter().find(|x| x.vec() == d)
    }

    /// Unit vector to the left of this direction (screen y points down).
    pub fn left(self) -> IVec2 {
        let f = self.vec();
        ivec2(f.y, -f.x)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum Kind {
    Belt,
    Tunnel,
    Splitter,
    Arm,
    Chest,
    Drill,
    Smelter,
    Assembler,
    Lab,
    Generator,
    Pole,
    Solar,
    Pipe,
    Wellhead,
    Refinery,
    Mixer,
    Wall,
    Sentry,
    GrandEngine,
    Hive,
}

impl Kind {
    pub const ALL: &'static [Kind] = &[
        Kind::Belt,
        Kind::Tunnel,
        Kind::Splitter,
        Kind::Arm,
        Kind::Chest,
        Kind::Drill,
        Kind::Smelter,
        Kind::Assembler,
        Kind::Lab,
        Kind::Generator,
        Kind::Pole,
        Kind::Solar,
        Kind::Pipe,
        Kind::Wellhead,
        Kind::Refinery,
        Kind::Mixer,
        Kind::Wall,
        Kind::Sentry,
        Kind::GrandEngine,
        Kind::Hive,
    ];

    pub fn item(self) -> Option<Item> {
        Some(match self {
            Kind::Belt => Item::Belt,
            Kind::Tunnel => Item::Tunnel,
            Kind::Splitter => Item::Splitter,
            Kind::Arm => Item::Arm,
            Kind::Chest => Item::Chest,
            Kind::Drill => Item::Drill,
            Kind::Smelter => Item::Smelter,
            Kind::Assembler => Item::Assembler,
            Kind::Lab => Item::Lab,
            Kind::Generator => Item::Generator,
            Kind::Pole => Item::Pole,
            Kind::Solar => Item::Solar,
            Kind::Pipe => Item::Pipe,
            Kind::Wellhead => Item::Wellhead,
            Kind::Refinery => Item::Refinery,
            Kind::Mixer => Item::Mixer,
            Kind::Wall => Item::Wall,
            Kind::Sentry => Item::Sentry,
            Kind::GrandEngine => Item::GrandEngine,
            Kind::Hive => return None,
        })
    }

    pub fn name(self) -> &'static str {
        self.item().map_or("Hive", Item::name)
    }

    /// Footprint (width, height) when facing north.
    pub fn size(self) -> (i32, i32) {
        match self {
            Kind::Splitter => (2, 1),
            Kind::Drill
            | Kind::Smelter
            | Kind::Generator
            | Kind::Solar
            | Kind::Wellhead
            | Kind::Mixer
            | Kind::Sentry => (2, 2),
            Kind::Assembler | Kind::Lab | Kind::Refinery | Kind::Hive => (3, 3),
            Kind::GrandEngine => (4, 4),
            _ => (1, 1),
        }
    }

    pub fn max_hp(self) -> f32 {
        match self {
            Kind::Belt | Kind::Tunnel | Kind::Pipe | Kind::Arm | Kind::Pole => 60.0,
            Kind::Wall => 600.0,
            Kind::Sentry => 400.0,
            Kind::Hive => 700.0,
            Kind::GrandEngine => 3000.0,
            _ => 250.0,
        }
    }

    /// Electric draw while working, in kW.
    pub fn draw_kw(self) -> f32 {
        match self {
            Kind::Arm => 15.0,
            Kind::Drill | Kind::Wellhead => 90.0,
            Kind::Smelter => 120.0,
            Kind::Assembler => 150.0,
            Kind::Lab => 60.0,
            Kind::Refinery => 300.0,
            Kind::Mixer => 200.0,
            _ => 0.0,
        }
    }

    /// Pollution emitted per second while working.
    pub fn pollution(self) -> f32 {
        match self {
            Kind::Drill => 8.0,
            Kind::Smelter => 3.0,
            Kind::Assembler => 1.5,
            Kind::Generator => 7.0,
            Kind::Refinery => 6.0,
            Kind::Mixer => 3.0,
            Kind::Wellhead => 4.0,
            _ => 0.0,
        }
    }

    pub fn category(self) -> Option<Category> {
        match self {
            Kind::Smelter => Some(Category::Smelting),
            Kind::Assembler => Some(Category::Assembly),
            Kind::Refinery => Some(Category::Refining),
            Kind::Mixer => Some(Category::Mixing),
            _ => None,
        }
    }

    /// Whether the player and enemies collide with it.
    pub fn solid(self) -> bool {
        !matches!(self, Kind::Belt | Kind::Tunnel | Kind::Splitter | Kind::Arm | Kind::Pipe)
    }

    pub fn rotatable(self) -> bool {
        matches!(
            self,
            Kind::Belt | Kind::Arm | Kind::Drill | Kind::Wellhead | Kind::Refinery | Kind::Mixer
        )
    }

    /// Whether its direction matters at all (for placement ghosts).
    pub fn directional(self) -> bool {
        self.rotatable() || matches!(self, Kind::Tunnel | Kind::Splitter)
    }

    pub fn has_fluid_in(self) -> bool {
        matches!(self, Kind::Refinery | Kind::Mixer)
    }

    pub fn has_fluid_out(self) -> bool {
        matches!(self, Kind::Wellhead | Kind::Refinery)
    }
}

/// Sound cues raised by the simulation, for the game to play.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Sfx {
    Place,
    Pickup,
    Craft,
    Research,
    /// Gunfire at a world position.
    Shot(Vec2),
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum Resource {
    Ore(Item),
    Tar,
}

impl Resource {
    pub fn name(self) -> &'static str {
        match self {
            Resource::Ore(i) => i.name(),
            Resource::Tar => "Tar Seep",
        }
    }
}

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct BeltState {
    /// Lane 0 is the left lane, lane 1 the right. Items are ordered front-first with their
    /// progress along the belt (0..=len).
    pub lanes: [Vec<(Item, f32)>; 2],
    /// Tile length items travel; longer than 1 for a tunnel entrance.
    pub len: f32,
    /// For tunnels, the paired tunnel's position.
    pub partner: Option<IVec2>,
    /// For tunnels, whether this is the exit end.
    pub exit: bool,
}

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct Crafter {
    pub recipe: Option<usize>,
    pub input: Stock,
    pub out: Stock,
    pub progress: f32,
    pub crafting: bool,
    pub fluid_in: f32,
    pub fluid_out: f32,
}

#[derive(Clone, Serialize, Deserialize)]
pub enum State {
    Belt(BeltState),
    /// Queued items per lane and which output each lane uses next.
    Splitter { lanes: [Vec<Item>; 2], toggle: [bool; 2] },
    /// `swing` goes from 0 (at the pickup side) to 1 (at the drop side).
    Arm { held: Option<Item>, swing: f32 },
    Chest { stock: Stock },
    Drill { timer: f32, out: Vec<Item>, cursor: usize },
    Crafter(Crafter),
    Lab { input: Stock, progress: f32 },
    /// `energy` is joules left in the fuel item currently burning.
    Generator { fuel: Stock, energy: f32, load: f32 },
    Pole,
    Solar,
    Wall,
    Pipe { fluid: Option<Fluid>, amount: f32 },
    Wellhead { amount: f32 },
    /// `shots`/`dmg` describe the loaded clip.
    Sentry { ammo: Stock, shots: u32, dmg: f32, cooldown: f32, aim: f32 },
    Engine { cores: u32 },
    Hive { agitation: f32, cooldown: f32, guard_cd: f32 },
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Entity {
    pub kind: Kind,
    /// Top-left tile.
    pub pos: IVec2,
    pub dir: Dir,
    pub hp: f32,
    /// Seconds since last damaged.
    pub calm: f32,
    /// Whether it worked this tick (drives power demand, pollution and animation).
    pub active: bool,
    pub state: State,
    #[serde(skip)]
    pub net: Option<usize>,
    #[serde(skip)]
    pub fnet: Option<usize>,
}

impl Entity {
    pub fn dims(&self) -> IVec2 {
        dims(self.kind, self.dir)
    }

    pub fn center(&self) -> Vec2 {
        self.pos.as_vec2() + self.dims().as_vec2() * 0.5
    }

    pub fn tiles(&self) -> impl Iterator<Item = IVec2> + use<> {
        footprint(self.kind, self.pos, self.dir)
    }

    /// Tile just outside the middle of the front edge (output side).
    pub fn front(&self) -> IVec2 {
        edge_tile(self.pos, self.dims(), self.dir)
    }

    /// Tile just outside the middle of the back edge.
    pub fn back(&self) -> IVec2 {
        edge_tile(self.pos, self.dims(), self.dir.opposite())
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Research {
    pub current: Option<usize>,
    pub progress: u32,
    pub done: Vec<bool>,
}

#[derive(Serialize, Deserialize)]
pub struct World {
    pub res: Vec<Option<Resource>>,
    grid: Vec<Option<usize>>,
    pub entities: Vec<Option<Entity>>,
    free: Vec<usize>,
    pub player: Player,
    pub research: Research,
    pub enemies: Vec<Enemy>,
    pub pollution: Vec<f32>,
    pub evolution: f32,
    pub peaceful: bool,
    pub time: f32,
    pub won: bool,
    pub spawn: Vec2,
    pollution_timer: f32,
    rng: Rng,
    #[serde(skip)]
    pub events: Vec<String>,
    #[serde(skip)]
    pub sounds: Vec<Sfx>,
    #[serde(skip)]
    pub power: Power,
    #[serde(skip)]
    pub fluids: FluidNets,
    #[serde(skip)]
    pub shots: Vec<Shot>,
}

impl World {
    pub fn new(seed: u64) -> Self {
        let n = (MAP_W * MAP_H) as usize;
        let spawn = vec2(MAP_W as f32 / 2.0, MAP_H as f32 / 2.0);
        let mut w = World {
            res: vec![None; n],
            grid: vec![None; n],
            entities: Vec::new(),
            free: Vec::new(),
            player: Player::new(spawn),
            research: Research { current: None, progress: 0, done: vec![false; TECHS.len()] },
            enemies: Vec::new(),
            pollution: vec![0.0; combat::chunk_count()],
            evolution: 0.0,
            peaceful: false,
            time: 0.0,
            won: false,
            spawn,
            pollution_timer: 0.0,
            rng: Rng(seed.rotate_left(17) | 1),
            events: Vec::new(),
            sounds: Vec::new(),
            power: Power::default(),
            fluids: FluidNets::default(),
            shots: Vec::new(),
        };
        w.generate(seed);
        w
    }

    /// Restores derived state after deserializing.
    pub fn after_load(&mut self) {
        self.power.dirty = true;
        self.fluids.dirty = true;
    }

    fn generate(&mut self, seed: u64) {
        let mut rng = Rng(seed | 1);
        let center = self.spawn;
        let ores = [
            Resource::Ore(Item::IronOre),
            Resource::Ore(Item::CopperOre),
            Resource::Ore(Item::Coal),
            Resource::Ore(Item::Stone),
            Resource::Ore(Item::Quartz),
            Resource::Tar,
        ];

        for _ in 0..170 {
            let c = vec2(rng.range(4.0, MAP_W as f32 - 4.0), rng.range(4.0, MAP_H as f32 - 4.0));
            if c.distance(center) < 30.0 {
                continue;
            }
            let res = ores[(rng.next() % ores.len() as u64) as usize];
            let r = if res == Resource::Tar { rng.range(1.5, 3.0) } else { rng.range(3.0, 9.0) };
            self.blob(&mut rng, c, r, res);
        }
        // Starter patches: the four basics close by, quartz and tar a short walk away.
        let base = rng.range(0.0, TAU);
        for (k, &res) in ores.iter().enumerate() {
            let (a, dist, r) = if k < 4 {
                (base + k as f32 * TAU / 4.0 + rng.range(-0.3, 0.3), rng.range(11.0, 16.0), 5.0)
            } else {
                (base + TAU / 8.0 + (k - 4) as f32 * TAU / 2.0, rng.range(28.0, 36.0), 3.5)
            };
            let c = center + vec2(a.cos(), a.sin()) * dist;
            self.blob(&mut rng, c, r, res);
        }
        // Keep the landing spot clear.
        for y in -3..=3 {
            for x in -3..=3 {
                let p = center.as_ivec2() + ivec2(x, y);
                self.res[idx(p)] = None;
            }
        }
        // Hive clusters, far from spawn.
        let mut placed = 0;
        let mut attempts = 0;
        while placed < 30 && attempts < 2000 {
            attempts += 1;
            let c = vec2(rng.range(6.0, MAP_W as f32 - 6.0), rng.range(6.0, MAP_H as f32 - 6.0));
            if c.distance(center) < 75.0 {
                continue;
            }
            let count = 1 + rng.next() % 3;
            for _ in 0..count {
                let p = (c + vec2(rng.range(-6.0, 6.0), rng.range(-6.0, 6.0))).as_ivec2();
                if self.footprint_free(Kind::Hive, p, Dir::N) {
                    self.spawn_entity(Kind::Hive, p, Dir::N);
                }
            }
            placed += 1;
        }
    }

    fn blob(&mut self, rng: &mut Rng, c: Vec2, r: f32, res: Resource) {
        let (p1, p2) = (rng.range(0.0, TAU), rng.range(0.0, TAU));
        let ext = (r * 1.4).ceil() as i32;
        let ci = c.as_ivec2();
        for y in ci.y - ext..=ci.y + ext {
            for x in ci.x - ext..=ci.x + ext {
                let p = ivec2(x, y);
                if !in_bounds(p) {
                    continue;
                }
                let d = p.as_vec2() + vec2(0.5, 0.5) - c;
                let a = d.y.atan2(d.x);
                let edge = r * (1.0 + 0.22 * (3.0 * a + p1).sin() + 0.12 * (5.0 * a + p2).sin());
                if d.length() < edge {
                    self.res[idx(p)] = Some(res);
                }
            }
        }
    }

    pub fn res_at(&self, p: IVec2) -> Option<Resource> {
        if in_bounds(p) { self.res[idx(p)] } else { None }
    }

    pub fn ore_at(&self, p: IVec2) -> Option<Item> {
        match self.res_at(p) {
            Some(Resource::Ore(i)) => Some(i),
            _ => None,
        }
    }

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

    fn footprint_free(&self, kind: Kind, pos: IVec2, dir: Dir) -> bool {
        footprint(kind, pos, dir).all(|p| in_bounds(p) && self.grid[idx(p)].is_none())
    }

    fn spawn_entity(&mut self, kind: Kind, pos: IVec2, dir: Dir) -> usize {
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

    pub fn recipe_unlocked(&self, id: usize) -> bool {
        let name = RECIPES[id].name;
        starts_unlocked(name)
            || TECHS
                .iter()
                .zip(&self.research.done)
                .any(|(t, &done)| done && t.recipes.contains(&name))
    }

    pub fn has_bonus(&self, bonus: Bonus) -> bool {
        TECHS
            .iter()
            .zip(&self.research.done)
            .any(|(t, &done)| done && t.bonus == Some(bonus))
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
        if kind.solid() && self.player.alive() {
            let (lo, hi) = self.player.bounds();
            let (tlo, thi) = (pos.as_vec2(), (pos + dims(kind, dir)).as_vec2());
            if lo.x < thi.x && hi.x > tlo.x && lo.y < thi.y && hi.y > tlo.y {
                return Err("You're standing in the way".into());
            }
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
        let contents = self.contents(id);
        for (i, n) in contents.iter() {
            self.player.inv.add(i, n);
        }
        self.player.inv.add(item, 1);
        self.delete(id);
        self.sounds.push(Sfx::Pickup);
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
        for p in e.tiles() {
            self.grid[idx(p)] = None;
        }
        self.free.push(id);
        self.mark_dirty(e.kind);
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

    pub fn update(&mut self, dt: f32) {
        self.time += dt;
        if self.power.dirty {
            self.recompute_power();
        }
        if self.fluids.dirty {
            self.recompute_fluids();
        }
        let arm_speed = if self.has_bonus(Bonus::ArmSpeed) { 1.5 } else { 1.0 };
        let drill_speed = if self.has_bonus(Bonus::DrillSpeed) { 1.5 } else { 1.0 };
        for id in 0..self.entities.len() {
            let Some(e) = &mut self.entities[id] else {
                continue;
            };
            e.calm += dt;
            if e.calm > 10.0 && e.hp < e.kind.max_hp() {
                e.hp = (e.hp + 2.0 * dt).min(e.kind.max_hp());
            }
            let kind = e.kind;
            let speed = self.power_factor(id);
            match kind {
                Kind::Belt | Kind::Tunnel => self.update_belt(id, dt),
                Kind::Splitter => self.update_splitter(id),
                Kind::Arm => self.update_arm(id, dt * speed * arm_speed),
                Kind::Drill => self.update_drill(id, dt * speed * drill_speed),
                Kind::Smelter | Kind::Assembler | Kind::Refinery | Kind::Mixer => {
                    self.update_crafter(id, dt * speed)
                }
                Kind::Lab => self.update_lab(id, dt * speed),
                Kind::Wellhead => self.update_wellhead(id, dt * speed),
                Kind::Sentry => self.update_sentry(id, dt),
                Kind::Hive => self.update_hive(id, dt),
                _ => {}
            }
            let e = self.entity(id);
            let emitted = e.kind.pollution() * dt;
            if e.active && emitted > 0.0 {
                let c = e.center();
                self.pollute(c, emitted);
            }
        }
        self.update_power(dt);
        self.update_enemies(dt);
        self.update_pollution(dt);
        self.update_player(dt);
    }

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

pub fn in_bounds(p: IVec2) -> bool {
    p.x >= 0 && p.y >= 0 && p.x < MAP_W && p.y < MAP_H
}

fn idx(p: IVec2) -> usize {
    (p.y * MAP_W + p.x) as usize
}

pub fn dims(kind: Kind, dir: Dir) -> IVec2 {
    let (w, h) = kind.size();
    if matches!(dir, Dir::E | Dir::W) { ivec2(h, w) } else { ivec2(w, h) }
}

pub fn footprint(kind: Kind, pos: IVec2, dir: Dir) -> impl Iterator<Item = IVec2> + use<> {
    let d = dims(kind, dir);
    (0..d.y).flat_map(move |y| (0..d.x).map(move |x| pos + ivec2(x, y)))
}

/// The tile just outside the middle of the edge of a footprint facing `dir`.
pub fn edge_tile(pos: IVec2, dims: IVec2, dir: Dir) -> IVec2 {
    let c = pos + (dims - IVec2::ONE) / 2;
    match dir {
        Dir::N => ivec2(c.x, pos.y - 1),
        Dir::S => ivec2(c.x, pos.y + dims.y),
        Dir::E => ivec2(pos.x + dims.x, c.y),
        Dir::W => ivec2(pos.x - 1, c.y),
    }
}

/// xorshift64* — small deterministic RNG.
#[derive(Clone, Serialize, Deserialize)]
pub struct Rng(pub u64);

impl Rng {
    pub fn next(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    pub fn range(&mut self, lo: f32, hi: f32) -> f32 {
        let f = (self.next() >> 40) as f32 / (1u64 << 24) as f32;
        lo + (hi - lo) * f
    }
}

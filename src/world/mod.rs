mod combat;
mod describe;
mod entity;
mod fluids;
mod generate;
mod logistics;
mod player;
mod power;
mod production;
#[cfg(test)]
mod tests;
mod trees;


use macroquad::prelude::*;
use serde::{Deserialize, Serialize};

use crate::items::*;
use generate::Rng;
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

/// Visual cues raised by the simulation, for the renderer to animate.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Fx {
    /// A building was placed with its footprint's top-left at `pos`.
    Built { pos: IVec2, size: IVec2 },
    /// A building was picked up from this footprint.
    Removed { pos: IVec2, size: IVec2 },
    /// One `item` was hand-mined from the ore at `tile`.
    Mined { tile: IVec2, item: Item },
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
    pub fx: Vec<Fx>,
    /// Trees per tile (see `tree_at`). Empty in saves made before trees.
    #[serde(default)]
    pub trees: Vec<u8>,
    #[serde(skip)]
    pub power: Power,
    #[serde(skip)]
    pub fluids: FluidNets,
    #[serde(skip)]
    pub shots: Vec<Shot>,
}

impl World {
    /// Restores derived state after deserializing.
    pub fn after_load(&mut self) {
        self.power.dirty = true;
        self.fluids.dirty = true;
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

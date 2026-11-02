use macroquad::prelude::*;
use serde::{Deserialize, Serialize};

use crate::world::Kind;
use crate::theme;

macro_rules! items {
    ($($v:ident => $name:literal, $col:expr, $shape:ident;)*) => {
        #[derive(Clone, Copy, PartialEq, Eq, Debug, Hash, Serialize, Deserialize)]
        pub enum Item { $($v),* }

        impl Item {
            pub const ALL: &'static [Item] = &[$(Item::$v),*];

            pub fn name(self) -> &'static str {
                match self { $(Item::$v => $name),* }
            }

            pub fn color(self) -> Color {
                match self { $(Item::$v => crate::theme::color($col)),* }
            }

            pub fn shape(self) -> Shape {
                match self { $(Item::$v => Shape::$shape),* }
            }
        }
    };
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Shape {
    Round,
    Square,
    Diamond,
    Framed,
    Vial,
    Building,
}

items! {
    IronOre => "Iron Ore", theme::ITEM_IRON_ORE, Round;
    CopperOre => "Copper Ore", theme::ITEM_COPPER_ORE, Round;
    Coal => "Coal", theme::ITEM_COAL, Round;
    Stone => "Stone", theme::ITEM_STONE, Round;
    Quartz => "Quartz", theme::ITEM_QUARTZ, Round;
    Wood => "Wood", theme::ITEM_WOOD, Round;
    IronPlate => "Iron Plate", theme::ITEM_IRON_PLATE, Square;
    CopperPlate => "Copper Plate", theme::ITEM_COPPER_PLATE, Square;
    Brick => "Brick", theme::ITEM_BRICK, Square;
    Steel => "Steel", theme::ITEM_STEEL, Square;
    Glass => "Glass", theme::ITEM_GLASS, Square;
    Gear => "Gear", theme::ITEM_GEAR, Diamond;
    Wire => "Wire", theme::ITEM_WIRE, Diamond;
    Rod => "Rod", theme::ITEM_ROD, Diamond;
    Circuit => "Circuit", theme::ITEM_CIRCUIT, Diamond;
    Motor => "Motor", theme::ITEM_MOTOR, Diamond;
    Lens => "Lens", theme::ITEM_LENS, Diamond;
    Polymer => "Polymer", theme::ITEM_POLYMER, Diamond;
    Processor => "Processor", theme::ITEM_PROCESSOR, Framed;
    FuelCell => "Fuel Cell", theme::ITEM_FUEL_CELL, Framed;
    SlugClip => "Slug Clip", theme::ITEM_SLUG_CLIP, Framed;
    HeavyClip => "Heavy Clip", theme::ITEM_HEAVY_CLIP, Framed;
    AetherCore => "Aether Core", theme::ITEM_AETHER_CORE, Framed;
    SurveyData => "Survey Data", theme::ITEM_SURVEY_DATA, Vial;
    MechData => "Mechanism Data", theme::ITEM_MECH_DATA, Vial;
    OpticData => "Optic Data", theme::ITEM_OPTIC_DATA, Vial;
    QuantumData => "Quantum Data", theme::ITEM_QUANTUM_DATA, Vial;
    Belt => "Conveyor", theme::ITEM_BELT, Building;
    Tunnel => "Tunnel Belt", theme::ITEM_TUNNEL, Building;
    Splitter => "Splitter", theme::ITEM_SPLITTER, Building;
    Arm => "Arm", theme::ITEM_ARM, Building;
    Chest => "Crate", theme::ITEM_CHEST, Building;
    Drill => "Drill", theme::ITEM_DRILL, Building;
    Smelter => "Smelter", theme::ITEM_SMELTER, Building;
    Assembler => "Assembler", theme::ITEM_ASSEMBLER, Building;
    Lab => "Lab", theme::ITEM_LAB, Building;
    Generator => "Generator", theme::ITEM_GENERATOR, Building;
    Pole => "Power Pole", theme::ITEM_POLE, Building;
    Solar => "Solar Panel", theme::ITEM_SOLAR, Building;
    Pipe => "Pipe", theme::ITEM_PIPE, Building;
    Wellhead => "Wellhead", theme::ITEM_WELLHEAD, Building;
    Refinery => "Refinery", theme::ITEM_REFINERY, Building;
    Mixer => "Mixer", theme::ITEM_MIXER, Building;
    Wall => "Wall", theme::ITEM_WALL, Building;
    Sentry => "Sentry", theme::ITEM_SENTRY, Building;
    GrandEngine => "Grand Engine", theme::ITEM_GRAND_ENGINE, Building;
}

impl Item {
    pub fn idx(self) -> usize {
        self as usize
    }

    pub fn place_kind(self) -> Option<Kind> {
        Kind::ALL.iter().copied().find(|k| k.item() == Some(self))
    }

    /// Energy released when burned in a generator, in joules.
    pub fn fuel_value(self) -> Option<f32> {
        match self {
            Item::Coal => Some(4.0e6),
            Item::Wood => Some(2.0e6),
            Item::FuelCell => Some(25.0e6),
            _ => None,
        }
    }

    /// Shots and damage per shot when loaded into a sentry or the player's gun.
    pub fn ammo(self) -> Option<(u32, f32)> {
        match self {
            Item::SlugClip => Some((10, 5.0)),
            Item::HeavyClip => Some((10, 12.0)),
            _ => None,
        }
    }

    pub fn is_science(self) -> bool {
        matches!(self, Item::SurveyData | Item::MechData | Item::OpticData | Item::QuantumData)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum Fluid {
    Tar,
    Naphtha,
}

impl Fluid {
    pub fn name(self) -> &'static str {
        match self {
            Fluid::Tar => "Crude Tar",
            Fluid::Naphtha => "Naphtha",
        }
    }

    /// Short name for compact messages like "Can't mix Tar and Naphtha".
    pub fn short_name(self) -> &'static str {
        match self {
            Fluid::Tar => "Tar",
            Fluid::Naphtha => "Naphtha",
        }
    }

    pub fn color(self) -> Color {
        match self {
            Fluid::Tar => crate::theme::color(crate::theme::TAR_LIGHT),
            Fluid::Naphtha => crate::theme::color(crate::theme::HONEY_500),
        }
    }
}

/// Item counts indexed by `Item::idx`.
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct Stock(Vec<u32>);

impl Default for Stock {
    fn default() -> Self {
        Stock(vec![0; Item::ALL.len()])
    }
}

impl Stock {
    pub fn get(&self, item: Item) -> u32 {
        self.0[item.idx()]
    }

    pub fn add(&mut self, item: Item, n: u32) {
        self.0[item.idx()] += n;
    }

    /// Removes up to `n` and returns how many were removed.
    pub fn take(&mut self, item: Item, n: u32) -> u32 {
        let t = self.0[item.idx()].min(n);
        self.0[item.idx()] -= t;
        t
    }

    pub fn has(&self, list: &[(Item, u32)]) -> bool {
        list.iter().all(|&(i, n)| self.get(i) >= n)
    }

    pub fn total(&self) -> u32 {
        self.0.iter().sum()
    }

    pub fn iter(&self) -> impl Iterator<Item = (Item, u32)> + '_ {
        Item::ALL
            .iter()
            .map(|&i| (i, self.get(i)))
            .filter(|&(_, n)| n > 0)
    }

    pub fn clear(&mut self) {
        self.0.fill(0);
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Category {
    Smelting,
    Assembly,
    Refining,
    Mixing,
}

pub struct Recipe {
    pub name: &'static str,
    pub category: Category,
    pub inputs: &'static [(Item, u32)],
    pub fluid_in: Option<(Fluid, f32)>,
    pub outputs: &'static [(Item, u32)],
    pub fluid_out: Option<(Fluid, f32)>,
    pub time: f32,
    /// Whether the player can craft it by hand.
    pub hand: bool,
}

impl Recipe {
    pub fn needs(&self, item: Item) -> Option<u32> {
        self.inputs.iter().find(|(i, _)| *i == item).map(|(_, n)| *n)
    }

    pub fn main_output(&self) -> Option<Item> {
        self.outputs.first().map(|(i, _)| *i)
    }
}

const fn r(
    name: &'static str,
    category: Category,
    inputs: &'static [(Item, u32)],
    outputs: &'static [(Item, u32)],
    time: f32,
) -> Recipe {
    Recipe {
        name,
        category,
        inputs,
        fluid_in: None,
        outputs,
        fluid_out: None,
        time,
        hand: matches!(category, Category::Assembly),
    }
}

const fn machine_only(mut rec: Recipe) -> Recipe {
    rec.hand = false;
    rec
}

use Category::*;
use Item::*;

pub const RECIPES: &[Recipe] = &[
    // 0..5 smelting
    r("Iron Plate", Smelting, &[(IronOre, 1)], &[(IronPlate, 1)], 1.6),
    r("Copper Plate", Smelting, &[(CopperOre, 1)], &[(CopperPlate, 1)], 1.6),
    r("Brick", Smelting, &[(Stone, 2)], &[(Brick, 1)], 1.6),
    r("Steel", Smelting, &[(IronPlate, 5)], &[(Steel, 1)], 8.0),
    r("Glass", Smelting, &[(Quartz, 2)], &[(Glass, 1)], 2.0),
    // 5.. components
    r("Gear", Assembly, &[(IronPlate, 2)], &[(Gear, 1)], 0.5),
    r("Wire", Assembly, &[(CopperPlate, 1)], &[(Wire, 2)], 0.5),
    r("Rod", Assembly, &[(IronPlate, 1)], &[(Rod, 2)], 0.5),
    r("Circuit", Assembly, &[(IronPlate, 1), (Wire, 3)], &[(Circuit, 1)], 1.0),
    r("Motor", Assembly, &[(Steel, 1), (Gear, 2), (Wire, 4)], &[(Motor, 1)], 4.0),
    r("Lens", Assembly, &[(Glass, 2), (Rod, 1)], &[(Lens, 1)], 3.0),
    machine_only(r(
        "Processor",
        Assembly,
        &[(Circuit, 2), (Polymer, 2), (Lens, 1)],
        &[(Processor, 1)],
        8.0,
    )),
    r("Slug Clip", Assembly, &[(IronPlate, 4)], &[(SlugClip, 1)], 1.0),
    r("Heavy Clip", Assembly, &[(SlugClip, 1), (Steel, 1), (CopperPlate, 2)], &[(HeavyClip, 1)], 3.0),
    machine_only(r(
        "Aether Core",
        Assembly,
        &[(Processor, 2), (Motor, 2), (Steel, 4), (FuelCell, 1)],
        &[(AetherCore, 1)],
        15.0,
    )),
    // science
    r("Survey Data", Assembly, &[(CopperPlate, 1), (Gear, 1)], &[(SurveyData, 1)], 5.0),
    r("Mechanism Data", Assembly, &[(Circuit, 1), (Rod, 1), (Brick, 1)], &[(MechData, 1)], 6.0),
    machine_only(r(
        "Optic Data",
        Assembly,
        &[(Lens, 2), (Steel, 1), (Polymer, 1)],
        &[(OpticData, 2)],
        14.0,
    )),
    machine_only(r(
        "Quantum Data",
        Assembly,
        &[(Processor, 1), (FuelCell, 1), (Motor, 1)],
        &[(QuantumData, 2)],
        20.0,
    )),
    // buildings
    r("Conveyor", Assembly, &[(Gear, 1), (IronPlate, 1)], &[(Belt, 2)], 0.5),
    r("Tunnel Belt", Assembly, &[(IronPlate, 5), (Belt, 4)], &[(Tunnel, 2)], 1.0),
    r("Splitter", Assembly, &[(Circuit, 5), (IronPlate, 5), (Belt, 4)], &[(Splitter, 1)], 1.0),
    r("Arm", Assembly, &[(Circuit, 1), (Gear, 1), (IronPlate, 1)], &[(Arm, 1)], 0.5),
    r("Crate", Assembly, &[(IronPlate, 8)], &[(Chest, 1)], 0.5),
    r("Drill", Assembly, &[(Circuit, 3), (Gear, 5), (IronPlate, 10)], &[(Drill, 1)], 2.0),
    r("Smelter", Assembly, &[(Brick, 10), (IronPlate, 4), (Circuit, 2)], &[(Smelter, 1)], 2.0),
    r("Assembler", Assembly, &[(Circuit, 3), (Gear, 5), (IronPlate, 9)], &[(Assembler, 1)], 0.5),
    r("Lab", Assembly, &[(Circuit, 10), (Gear, 10), (Belt, 4)], &[(Lab, 1)], 2.0),
    r("Generator", Assembly, &[(Gear, 5), (IronPlate, 10), (Brick, 10)], &[(Generator, 1)], 2.0),
    r("Power Pole", Assembly, &[(Rod, 1), (Wire, 2)], &[(Pole, 2)], 0.5),
    r("Solar Panel", Assembly, &[(Steel, 5), (Circuit, 15), (Glass, 5)], &[(Solar, 1)], 10.0),
    r("Pipe", Assembly, &[(IronPlate, 1)], &[(Pipe, 1)], 0.5),
    r("Wellhead", Assembly, &[(Steel, 5), (Gear, 10), (Circuit, 5), (Pipe, 10)], &[(Wellhead, 1)], 5.0),
    r(
        "Refinery",
        Assembly,
        &[(Steel, 15), (Gear, 10), (Circuit, 10), (Pipe, 10), (Brick, 10)],
        &[(Refinery, 1)],
        8.0,
    ),
    r("Mixer", Assembly, &[(Steel, 5), (Gear, 5), (Circuit, 5), (Pipe, 5)], &[(Mixer, 1)], 5.0),
    r("Wall", Assembly, &[(Brick, 5)], &[(Wall, 1)], 0.5),
    r("Sentry", Assembly, &[(Gear, 10), (CopperPlate, 10), (IronPlate, 20)], &[(Sentry, 1)], 8.0),
    r(
        "Grand Engine",
        Assembly,
        &[(Steel, 100), (Motor, 50), (Processor, 50), (Brick, 100)],
        &[(GrandEngine, 1)],
        30.0,
    ),
    // fluids
    Recipe {
        name: "Naphtha",
        category: Refining,
        inputs: &[],
        fluid_in: Some((Fluid::Tar, 10.0)),
        outputs: &[],
        fluid_out: Some((Fluid::Naphtha, 8.0)),
        time: 4.0,
        hand: false,
    },
    Recipe {
        name: "Polymer",
        category: Mixing,
        inputs: &[(Coal, 1)],
        fluid_in: Some((Fluid::Naphtha, 20.0)),
        outputs: &[(Polymer, 2)],
        fluid_out: None,
        time: 2.0,
        hand: false,
    },
    Recipe {
        name: "Fuel Cell",
        category: Mixing,
        inputs: &[(IronPlate, 1)],
        fluid_in: Some((Fluid::Naphtha, 10.0)),
        outputs: &[(FuelCell, 1)],
        fluid_out: None,
        time: 3.0,
        hand: false,
    },
];

#[cfg(test)]
pub fn recipe_id(name: &str) -> usize {
    RECIPES
        .iter()
        .position(|r| r.name == name)
        .unwrap_or_else(|| panic!("unknown recipe {name}"))
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Bonus {
    ArmSpeed,
    DrillSpeed,
}

pub struct Tech {
    pub name: &'static str,
    pub units: u32,
    pub science: &'static [Item],
    /// Lab seconds per unit.
    pub time: f32,
    pub prereqs: &'static [usize],
    pub recipes: &'static [&'static str],
    pub bonus: Option<Bonus>,
}

const S: Item = SurveyData;
const M: Item = MechData;
const O: Item = OpticData;
const Q: Item = QuantumData;

pub const TECHS: &[Tech] = &[
    Tech { name: "Logistics", units: 20, science: &[S], time: 10.0, prereqs: &[],
        recipes: &["Tunnel Belt", "Splitter"], bonus: None },
    Tech { name: "Defense", units: 15, science: &[S], time: 10.0, prereqs: &[],
        recipes: &["Wall", "Sentry", "Slug Clip"], bonus: None },
    Tech { name: "Steelworking", units: 25, science: &[S], time: 10.0, prereqs: &[],
        recipes: &["Steel"], bonus: None },
    Tech { name: "Mechanics", units: 40, science: &[S], time: 10.0, prereqs: &[2],
        recipes: &["Motor", "Mechanism Data"], bonus: None },
    Tech { name: "Glassworks", units: 40, science: &[S, M], time: 15.0, prereqs: &[3],
        recipes: &["Glass", "Lens"], bonus: None },
    Tech { name: "Arm Servos", units: 50, science: &[S, M], time: 15.0, prereqs: &[3],
        recipes: &[], bonus: Some(Bonus::ArmSpeed) },
    Tech { name: "Tar Extraction", units: 60, science: &[S, M], time: 15.0, prereqs: &[3],
        recipes: &["Pipe", "Wellhead", "Refinery", "Naphtha"], bonus: None },
    Tech { name: "Polymers", units: 75, science: &[S, M], time: 20.0, prereqs: &[6],
        recipes: &["Mixer", "Polymer", "Fuel Cell"], bonus: None },
    Tech { name: "Heavy Ordnance", units: 75, science: &[S, M], time: 15.0, prereqs: &[1, 3],
        recipes: &["Heavy Clip"], bonus: None },
    Tech { name: "Optics", units: 100, science: &[S, M], time: 20.0, prereqs: &[4, 7],
        recipes: &["Optic Data"], bonus: None },
    Tech { name: "Solar Power", units: 100, science: &[S, M, O], time: 20.0, prereqs: &[9],
        recipes: &["Solar Panel"], bonus: None },
    Tech { name: "Deep Boring", units: 100, science: &[S, M, O], time: 25.0, prereqs: &[9],
        recipes: &[], bonus: Some(Bonus::DrillSpeed) },
    Tech { name: "Processors", units: 150, science: &[S, M, O], time: 25.0, prereqs: &[9],
        recipes: &["Processor"], bonus: None },
    Tech { name: "Quantum Data", units: 150, science: &[S, M, O], time: 30.0, prereqs: &[12],
        recipes: &["Quantum Data"], bonus: None },
    Tech { name: "The Grand Engine", units: 300, science: &[S, M, O, Q], time: 30.0, prereqs: &[13],
        recipes: &["Aether Core", "Grand Engine"], bonus: None },
];

/// Aether Cores the Grand Engine must be fed to win.
pub const ENGINE_CORES: u32 = 50;

/// Whether a recipe is available without research.
pub fn starts_unlocked(recipe: &str) -> bool {
    !TECHS.iter().any(|t| t.recipes.contains(&recipe))
}

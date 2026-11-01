//! Cogworks design tokens: one source of truth for colours, type, spacing, corners
//! and line weights. Shared by the game and by `gen_assets` (via `#[path]`) so the
//! UI and the painted sprites can never drift apart again.
//!
//! Colours are plain `u32` hex literals so the asset generator (which paints in its
//! own float RGBA space) and the game (which draws macroquad `Color`s) can both use
//! them. Keep every raw palette value in this file; the rest of `src` should use the
//! named tokens or the alpha helpers below.

#![allow(dead_code)]

use macroquad::prelude::Color;

/// Expands a packed `0xrrggbb` literal to straight (non-premultiplied) RGBA floats.
pub const fn rgb(hex: u32) -> [f32; 4] {
    [
        ((hex >> 16) & 0xff) as f32 / 255.0,
        ((hex >> 8) & 0xff) as f32 / 255.0,
        (hex & 0xff) as f32 / 255.0,
        1.0,
    ]
}

/// Opaque macroquad colour from a packed hex literal.
pub const fn color(hex: u32) -> Color {
    let c = rgb(hex);
    Color::new(c[0], c[1], c[2], 1.0)
}

/// Token colour with an explicit alpha.
pub const fn alpha(hex: u32, a: f32) -> Color {
    let c = rgb(hex);
    Color::new(c[0], c[1], c[2], a)
}

/// Alpha variant of an already-built colour (for sky/light colours that are computed).
pub fn with_alpha(c: Color, a: f32) -> Color {
    Color::new(c.r, c.g, c.b, a)
}

/// Colour from computed floats. Kept here so raw `Color::new` calls stay in one file.
pub fn rgba(r: f32, g: f32, b: f32, a: f32) -> Color {
    Color::new(r, g, b, a)
}

pub fn mix(a: Color, b: Color, t: f32) -> Color {
    let t = t.clamp(0.0, 1.0);
    Color::new(
        a.r + (b.r - a.r) * t,
        a.g + (b.g - a.g) * t,
        a.b + (b.b - a.b) * t,
        a.a + (b.a - a.a) * t,
    )
}

/// WCAG 2.x relative luminance of a packed `0xrrggbb` colour (0..=1).
pub fn luminance(hex: u32) -> f32 {
    let c = rgb(hex);
    let lin = |v: f32| {
        if v <= 0.03928 {
            v / 12.92
        } else {
            ((v + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * lin(c[0]) + 0.7152 * lin(c[1]) + 0.0722 * lin(c[2])
}

/// WCAG contrast ratio between two packed colours (1.0 ..= 21.0).
pub fn contrast(a: u32, b: u32) -> f32 {
    let (la, lb) = (luminance(a), luminance(b));
    let (hi, lo) = if la >= lb { (la, lb) } else { (lb, la) };
    (hi + 0.05) / (lo + 0.05)
}

/// HSV saturation (0..=1) of a packed `0xrrggbb` colour: how far it sits from
/// grey. Used to keep the lacquer accent a soft rose instead of an aggressive red.
pub fn saturation(hex: u32) -> f32 {
    let c = rgb(hex);
    let max = c[0].max(c[1]).max(c[2]);
    let min = c[0].min(c[1]).min(c[2]);
    if max == 0.0 { 0.0 } else { (max - min) / max }
}

/// The warm lo-fi grade used by the world shader, applied to flat images too.
/// Keeps the minimap's greens in the same light as the ground they represent.
/// `tint` is the mood tint from `sky_at_phase` (rgb colour, strength in alpha),
/// mirroring the `Tint` uniform in `COMPOSITE`: applied mostly into shadows and
/// midtones so lamp-lit areas keep their warmth.
pub fn grade(c: Color, tint: Color) -> Color {
    let luma = 0.299 * c.r + 0.587 * c.g + 0.114 * c.b;
    let warm = Color::new(c.r * 1.06, c.g, c.b * 0.88, c.a);
    let mut out = mix(c, warm, 0.35);
    let lift = (1.0 - luma).clamp(0.0, 1.0);
    out.r = (out.r + 0.030 * lift).min(1.0);
    out.g = (out.g + 0.025 * lift).min(1.0);
    out.b = (out.b + 0.060 * lift).min(1.0);
    // Mood tint, same maths as COMPOSITE: mix into shadows and midtones.
    let tluma = 0.299 * out.r + 0.587 * out.g + 0.114 * out.b;
    let amount = tint.a * (1.0 - tluma * 0.5);
    let tinted = Color::new(
        out.r * tint.r * 1.6 + tint.r * 0.08,
        out.g * tint.g * 1.6 + tint.g * 0.08,
        out.b * tint.b * 1.6 + tint.b * 0.08,
        c.a,
    );
    let out = mix(out, tinted, amount);
    Color::new(out.r.min(1.0), out.g.min(1.0), out.b.min(1.0), c.a)
}

// ---------------------------------------------------------------- type scale

/// Typographic scale. Use these instead of raw pixel sizes.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum TextSize {
    Xs,
    S,
    M,
    L,
    Xl,
    Xxl,
}

impl TextSize {
    pub const fn px(self) -> f32 {
        match self {
            TextSize::Xs => 13.0,
            TextSize::S => 15.0,
            TextSize::M => 17.0,
            TextSize::L => 20.0,
            TextSize::Xl => 28.0,
            TextSize::Xxl => 32.0,
        }
    }

    /// Titles and headings use the Bold cut of the typeface; the rest use Medium.
    pub const fn bold(self) -> bool {
        matches!(self, TextSize::Xl | TextSize::Xxl)
    }
}

// ---------------------------------------------------------------- layout scale

/// Spacing steps. Layouts snap to these values.
pub mod space {
    pub const XS: f32 = 4.0;
    pub const S: f32 = 8.0;
    pub const M: f32 = 12.0;
    pub const L: f32 = 16.0;
    pub const XL: f32 = 24.0;
}

/// Corner radii.
pub mod radius {
    pub const SM: f32 = 6.0;
    pub const MD: f32 = 10.0;
    pub const LG: f32 = 14.0;
}

/// Stroke widths.
pub mod line {
    pub const HAIR: f32 = 1.0;
    pub const BASE: f32 = 1.5;
    pub const STRONG: f32 = 2.0;
}

// ---------------------------------------------------------------- palette ramps

// Plum: surfaces, ink and shadows. 900 is the window fill, 600 is the hairline edge.
pub const PLUM_950: u32 = 0x1e141a;
pub const PLUM_900: u32 = 0x2a2026;
pub const PLUM_850: u32 = 0x33262e;
pub const PLUM_800: u32 = 0x3b2e36;
pub const PLUM_700: u32 = 0x4a3a43;
pub const PLUM_600: u32 = 0x6b5461;
pub const PLUM_500: u32 = 0x8a6d7c;
pub const PLUM_400: u32 = 0xb08fa0;
pub const PLUM_300: u32 = 0xd6bcc8;

// Honey: primary actions, highlights, selection.
pub const HONEY_700: u32 = 0x9a6a43;
pub const HONEY_600: u32 = 0xc9935f;
pub const HONEY_500: u32 = 0xe3aa4c;
pub const HONEY_400: u32 = 0xf2c46d;

// Terra: health, damage, warm accents.
pub const TERRA_700: u32 = 0x7a3f30;
pub const TERRA_600: u32 = 0x9e4f3d;
pub const TERRA_500: u32 = 0xc8694f;
pub const TERRA_400: u32 = 0xe07a5f;
pub const TERRA_300: u32 = 0xf0a982;

// Sage: grass, success, power health.
pub const SAGE_700: u32 = 0x5b6d47;
pub const SAGE_600: u32 = 0x6c8158;
pub const SAGE_500: u32 = 0x7f9466;
pub const SAGE_400: u32 = 0x93a576;
pub const SAGE_300: u32 = 0xa7b98b;
pub const SAGE_200: u32 = 0xb9c48f;
pub const SAGE_100: u32 = 0xd2d8ae;

// Teal: science and fluids.
pub const TEAL_700: u32 = 0x355c59;
pub const TEAL_600: u32 = 0x4b7d7a;
pub const TEAL_500: u32 = 0x6fa6a0;
pub const TEAL_400: u32 = 0x8cc7bf;
pub const TEAL_300: u32 = 0xb8e0da;

// Warm steel: the metal family. Deliberately less blue than the old grey.
pub const STEEL_LIGHT: u32 = 0xc9c4ba;
pub const STEEL_400: u32 = 0xa39a8e;
pub const STEEL_500: u32 = 0x9b938a;
pub const STEEL_600: u32 = 0x7c746a;
pub const STEEL_700: u32 = 0x655d55;

// Lacquer: a soft dusty rose-vermilion, used as a thin accent (titles, the
// selected slot ring, primary buttons) and later for lanterns and machine
// roofs. 500 is the key colour and stays below ~0.55 HSV saturation.
pub const LACQUER_700: u32 = 0x8a463c;
pub const LACQUER_600: u32 = 0x9c4a40;
pub const LACQUER_500: u32 = 0xc9705f;
pub const LACQUER_400: u32 = 0xd98b76;
pub const LACQUER_300: u32 = 0xecb4a4;

// Verdigris teal: roof trim and metal accents.
pub const VERDIGRIS_600: u32 = 0x468c82;
pub const VERDIGRIS_500: u32 = 0x5fa89d;
pub const VERDIGRIS_400: u32 = 0x83c2b8;

// Cream paper: UI surfaces. 300 is the darker slot "well".
pub const PAPER_100: u32 = 0xf6ead2;
pub const PAPER_200: u32 = 0xecd9b0;
pub const PAPER_300: u32 = 0xb39868;

// Dark plum-brown ink for text.
pub const INKBROWN_900: u32 = 0x3b2a2e;
pub const INKBROWN_700: u32 = 0x5a4447;
pub const INKBROWN_500: u32 = 0x6f5a55;

// Lush soft meadow greens, replacing the olive sage ground.
pub const MEADOW_700: u32 = 0x4c7a45;
pub const MEADOW_600: u32 = 0x5b8c4d;
pub const MEADOW_500: u32 = 0x6ca057;
pub const MEADOW_400: u32 = 0x83b469;
pub const MEADOW_300: u32 = 0x9cc87d;
pub const MEADOW_200: u32 = 0xb8da9a;

// Lavender-blue dusk.
pub const DUSK_700: u32 = 0x4a4a78;
pub const DUSK_500: u32 = 0x6e6a9c;
pub const DUSK_300: u32 = 0xa8a3c8;

// ---------------------------------------------------------------- named palette

pub const INK: u32 = PLUM_700;
pub const INK_DEEP: u32 = PLUM_950;
pub const CREAM: u32 = 0xf6e7cb;
pub const PAPER: u32 = 0xe8d1a8;
pub const DIM: u32 = 0xb8a696;
pub const PEACH: u32 = TERRA_300;
pub const TERRA: u32 = TERRA_500;
pub const RUST: u32 = TERRA_600;
pub const MUSTARD: u32 = HONEY_500;
pub const HONEY: u32 = HONEY_400;
pub const LAMP: u32 = 0xffb27a;
pub const SAGE: u32 = SAGE_300;
pub const SAGE_D: u32 = SAGE_500;
pub const OLIVE: u32 = SAGE_700;
pub const TEAL: u32 = TEAL_500;
pub const TEAL_D: u32 = TEAL_600;
pub const SKY: u32 = 0xa9c9cf;
pub const PLUM: u32 = 0x6c4a5e;
pub const ROSE: u32 = 0xd99a9a;
pub const BROWN: u32 = 0x8a624a;
pub const BROWN_D: u32 = 0x5e4234;
pub const STEEL: u32 = STEEL_500;
pub const STEEL_D: u32 = STEEL_700;
pub const CHAR: u32 = 0x3f3739;
pub const WHITE: u32 = 0xffffff;

pub const BG: u32 = PAPER_100;
pub const BG2: u32 = PAPER_200;
/// Warm mid wood-brown for panel edges: cream paper does most of the work now.
pub const WOOD_TRIM: u32 = 0x9a6b4a;
pub const EDGE: u32 = WOOD_TRIM;
pub const SHADE: u32 = 0x140d12;
pub const SHADOW: u32 = 0x2a1c24;
pub const BEVEL: u32 = 0xffd9a8;

// Ink-on-paper text and UI tokens (Task 2). `CREAM` and `DIM` above stay as
// the light paint/overlay colours used by sprites and the world.
pub const TEXT: u32 = INKBROWN_900;
pub const TEXT_DIM: u32 = INKBROWN_500;
pub const TITLE: u32 = LACQUER_600;
/// Deep gold/amber for key hints and accent text that sits on paper. `HONEY`
/// is too pale to read on cream; this keeps the gold accent while passing WCAG.
pub const KEY: u32 = 0x8a5a1e;
/// Darker wood, for the panel's hard under-edge that reads as paper thickness.
pub const TRIM_SHADOW: u32 = 0x6b4a34;
pub const SLOT_WELL: u32 = PAPER_300;

// ---------------------------------------------------------------- terrain & ores

pub const GRASS_A: u32 = MEADOW_400;
pub const GRASS_B: u32 = MEADOW_500;
pub const GRASS_WARM: u32 = MEADOW_300;
pub const BLADE_LIGHT: u32 = MEADOW_200;
pub const BLADE_DARK: u32 = MEADOW_600;
pub const MOSS: u32 = MEADOW_500;
pub const MOSS_DARK: u32 = MEADOW_700;
pub const STEM_LIGHT: u32 = 0x9fb07f;
pub const PEBBLE_LIGHT: u32 = 0xd3c4ab;
pub const PEBBLE_DARK: u32 = 0xa6977f;

pub const ORE_IRON: u32 = 0x6e7886;
pub const ORE_IRON_FACET: u32 = 0x9aa6b8;
pub const ORE_IRON_GLINT: u32 = 0xdfe7f0;
pub const ORE_COPPER: u32 = 0xc0713f;
pub const ORE_COPPER_FACET: u32 = 0xefa05e;
pub const ORE_COPPER_PATINA: u32 = 0x6fa89a;
pub const ORE_COAL: u32 = 0x2c2930;
pub const ORE_COAL_FACET: u32 = 0x4a4550;
pub const ORE_COAL_SHEEN: u32 = 0x8a76a8;
pub const ORE_STONE: u32 = 0xd8c8a8;
pub const ORE_STONE_FACET: u32 = 0xf0e4cc;
pub const QUARTZ: u32 = 0xd8d2e2;
pub const QUARTZ_LIGHT: u32 = 0xf3effa;
pub const QUARTZ_SHADE: u32 = 0xb9aed0;

// Pastel decor flowers: pink, butter-yellow and lilac, sitting on the meadow.
pub const FLOWER_PINK: u32 = 0xe8a7b8;
pub const FLOWER_BUTTER: u32 = 0xf2d06b;
pub const FLOWER_LILAC: u32 = 0xc4b3e0;

pub const TAR_LIGHT: u32 = 0x3a2b36;
pub const TAR_DARK: u32 = 0x1e161c;
pub const TAR_SHEEN: u32 = 0x9d7fa0;
pub const TAR_BUBBLE: u32 = 0x8a7090;

// ---------------------------------------------------------------- logistics

pub const BELT_BAND: u32 = 0x4b3328;
pub const BELT_RIDGE: u32 = 0x62442f;
pub const BELT_RAIL_HI: u32 = 0xd6b45f;
pub const BELT_RAIL_LO: u32 = 0x94702f;
pub const TUNNEL_STONE: u32 = 0xb7a58a;
pub const TUNNEL_MOUTH: u32 = 0x1f171b;
pub const WOOD: u32 = 0xb07d55;
pub const WOOD_DARK: u32 = 0x9a6c48;

// ---------------------------------------------------------------- machines

// Market-stall roofs (Task 7): soft dusty-rose lacquer over cream/wood walls,
// with verdigris ridge and eave trim. Kept as machine tokens so sprites and any
// later UI accent can share the exact same rose.
pub const ROOF: u32 = LACQUER_500;
pub const ROOF_SHADE: u32 = LACQUER_600;
pub const ROOF_LIGHT: u32 = LACQUER_400;
/// Verdigris teal ridge cap and eave trim on the roofs.
pub const TRIM: u32 = VERDIGRIS_500;
pub const TRIM_DARK: u32 = VERDIGRIS_600;
/// Boiler-room fittings: warm brass and copper for pipes, collars and frames.
pub const BRASS: u32 = 0xc99a42;
pub const COPPER: u32 = 0xb8744c;

pub const DRILL_HOLE: u32 = 0x4a3a30;
pub const PIT: u32 = 0x2c2226;
pub const FIREBOX: u32 = 0x2a1a1a;
pub const VENT_HI: u32 = 0x34484a;
pub const VENT_LO: u32 = 0x223234;
pub const GLASS_HI: u32 = 0xcfe4e2;
pub const LAMP_INNER: u32 = 0xfff0c0;
pub const SOLAR_HI: u32 = 0x5578a0;
pub const SOLAR_LO: u32 = 0x2f4462;
pub const WELLHEAD_STONE: u32 = 0xa39684;
pub const WELLHEAD_STONE_D: u32 = 0x6f6557;
pub const WELLHEAD_DARK: u32 = 0x2a1f25;
pub const REFINERY_STONE: u32 = 0xb5aa98;
pub const REFINERY_STONE_D: u32 = 0x7d7364;
pub const WALL_STONE: u32 = 0xbcae98;
pub const ENGINE_INNER: u32 = 0x2c1e2a;
pub const HIVE_SHELL: u32 = 0xa0567e;
pub const HIVE_SHELL_D: u32 = 0x4e2a44;
pub const HIVE_CELL: u32 = 0x2a1424;
pub const HIVE_GLOW_A: u32 = 0xff8fb8;
pub const HIVE_GLOW_B: u32 = 0xe86aa0;
pub const VISOR: u32 = 0x3a2d3a;
pub const MITE_SHELL: u32 = 0xb8574f;
pub const MITE_DARK: u32 = 0x6a2a32;
pub const BRUTE_SHELL: u32 = 0x8e5aa0;
pub const BRUTE_DARK: u32 = 0x4a2a58;

// ---------------------------------------------------------------- items & effects

pub const VIAL_GLASS: u32 = 0xe8f1f0;
pub const IRON_PLATE: u32 = 0xc9c4ba;
pub const COPPER_PLATE: u32 = 0xe8945e;
pub const STEEL_PLATE_HI: u32 = 0xb8b4aa;
pub const STEEL_PLATE_LO: u32 = 0x7c766c;
pub const GLASS: u32 = 0xbfe6ec;
pub const GLASS_LIGHT: u32 = 0xd6f1f4;
pub const WIRE: u32 = 0xe9a04f;
pub const ROD: u32 = 0x9198a0;
pub const CIRCUIT_HI: u32 = 0x6fb27a;
pub const CIRCUIT_LO: u32 = 0x3f7a4c;
pub const MOTOR_HI: u32 = 0x7f9ad6;
pub const MOTOR_LO: u32 = 0x48609a;
pub const POLYMER_A: u32 = 0xf2ecd2;
pub const POLYMER_B: u32 = 0xe4dbb2;
pub const PROCESSOR_HI: u32 = 0x4d4450;
pub const PROCESSOR_LO: u32 = 0x2c2530;
pub const PROCESSOR_LED: u32 = 0x8fe0a8;
pub const FUEL_HI: u32 = 0xf6a05a;
pub const FUEL_LO: u32 = 0xc85a33;
pub const AETHER_GLOW: u32 = 0xc89cff;
pub const AETHER_HI: u32 = 0xe8d4ff;
pub const AETHER_LO: u32 = 0x8a5ad0;
pub const DATA_RED: u32 = 0xe0736a;
pub const DATA_GREEN: u32 = 0x8cbf6e;
pub const DATA_BLUE: u32 = 0x6fb0d8;
pub const DATA_PURPLE: u32 = 0xb98ae0;
pub const SMOKE: u32 = 0xf1e6d6;
pub const CLOUD: u32 = 0x2a1c30;
pub const HEART: u32 = 0xf08c84;

// ---------------------------------------------------------------- item colours

// One colour per item, all drawn from the palette above. Re-tuned in Task 2 so
// every icon stands out on the paper `SLOT_WELL`: metals, ores and building
// items read as dark ink, while glass, gems and glow items stay bright pastels.
pub const ITEM_IRON_ORE: u32 = 0x5a554e;
pub const ITEM_COPPER_ORE: u32 = 0x9a5231;
pub const ITEM_COAL: u32 = ORE_COAL;
pub const ITEM_STONE: u32 = 0x7d6748;
pub const ITEM_QUARTZ: u32 = QUARTZ;
pub const ITEM_IRON_PLATE: u32 = 0x6a625a;
pub const ITEM_COPPER_PLATE: u32 = 0xa05a34;
pub const ITEM_BRICK: u32 = TERRA_700;
pub const ITEM_STEEL: u32 = 0x554f48;
pub const ITEM_GLASS: u32 = GLASS;
pub const ITEM_GEAR: u32 = 0x554f48;
pub const ITEM_WIRE: u32 = 0x8a5e38;
pub const ITEM_ROD: u32 = 0x4f4843;
pub const ITEM_CIRCUIT: u32 = CIRCUIT_LO;
pub const ITEM_MOTOR: u32 = 0x3f4d78;
pub const ITEM_LENS: u32 = GLASS_LIGHT;
pub const ITEM_POLYMER: u32 = POLYMER_A;
pub const ITEM_PROCESSOR: u32 = PROCESSOR_LO;
pub const ITEM_FUEL_CELL: u32 = 0x8a3a20;
pub const ITEM_SLUG_CLIP: u32 = 0x8a5e38;
pub const ITEM_HEAVY_CLIP: u32 = 0x8a3f2c;
pub const ITEM_AETHER_CORE: u32 = AETHER_HI;
pub const ITEM_SURVEY_DATA: u32 = 0x9a4438;
pub const ITEM_MECH_DATA: u32 = 0x4f7a3e;
pub const ITEM_OPTIC_DATA: u32 = 0x3f6a8c;
pub const ITEM_QUANTUM_DATA: u32 = 0x5f4285;

pub const ITEM_BELT: u32 = BELT_BAND;
pub const ITEM_TUNNEL: u32 = 0x5a4a40;
pub const ITEM_SPLITTER: u32 = 0x5f5a62;
pub const ITEM_ARM: u32 = 0x8a5e38;
pub const ITEM_CHEST: u32 = 0x7d5a38;
pub const ITEM_DRILL: u32 = 0x8a5e38;
pub const ITEM_SMELTER: u32 = 0x824b3a;
pub const ITEM_ASSEMBLER: u32 = 0x48627f;
pub const ITEM_LAB: u32 = TEAL_700;
pub const ITEM_GENERATOR: u32 = 0x6e5f55;
pub const ITEM_POLE: u32 = 0x6b4f36;
pub const ITEM_SOLAR: u32 = SOLAR_LO;
pub const ITEM_PIPE: u32 = 0x4f4843;
pub const ITEM_WELLHEAD: u32 = 0x50463c;
pub const ITEM_REFINERY: u32 = 0x6a5439;
pub const ITEM_MIXER: u32 = 0x3f5a4d;
pub const ITEM_WALL: u32 = 0x66605a;
pub const ITEM_SENTRY: u32 = OLIVE;
pub const ITEM_GRAND_ENGINE: u32 = 0x5a3a86;

// ---------------------------------------------------------------- minimap & world

pub const MAP_GRASS: u32 = MEADOW_500;
pub const MAP_BUILDING: u32 = HONEY_400;
pub const MAP_HIVE: u32 = 0xd65a96;
pub const MAP_ENEMY: u32 = 0xe05050;
pub const MAP_TAR: u32 = TAR_DARK;
pub const WORLD_CLEAR: u32 = MEADOW_500;
pub const DIRT: u32 = 0x6b543d;
pub const POLLUTION: u32 = 0x8c522e;
pub const BADGE: u32 = CREAM;
pub const HP_SAGE: u32 = SAGE_300;
pub const HP_TERRA: u32 = TERRA_400;

// ---------------------------------------------------------------- tests

#[cfg(test)]
mod tests {
    use super::*;

    /// Body, secondary and title text must stay readable on the paper surfaces.
    #[test]
    fn ui_text_contrast_is_readable() {
        assert!(contrast(TEXT, BG) >= 7.0, "TEXT on BG = {:.2}", contrast(TEXT, BG));
        assert!(contrast(TEXT_DIM, BG) >= 4.5, "TEXT_DIM on BG = {:.2}", contrast(TEXT_DIM, BG));
        assert!(contrast(TITLE, BG) >= 4.5, "TITLE on BG = {:.2}", contrast(TITLE, BG));
        assert!(contrast(TEXT, BG2) >= 4.5, "TEXT on BG2 = {:.2}", contrast(TEXT, BG2));
        assert!(contrast(KEY, BG) >= 4.5, "KEY on BG = {:.2}", contrast(KEY, BG));
    }

    /// Every item icon must stand out against the slot well, and the slot
    /// count (drawn in TEXT) must stay readable on it too.
    #[test]
    fn items_stand_out_in_slot_well() {
        let items = [
            ITEM_IRON_ORE,
            ITEM_COPPER_ORE,
            ITEM_COAL,
            ITEM_STONE,
            ITEM_QUARTZ,
            ITEM_IRON_PLATE,
            ITEM_COPPER_PLATE,
            ITEM_BRICK,
            ITEM_STEEL,
            ITEM_GLASS,
            ITEM_GEAR,
            ITEM_WIRE,
            ITEM_ROD,
            ITEM_CIRCUIT,
            ITEM_MOTOR,
            ITEM_LENS,
            ITEM_POLYMER,
            ITEM_PROCESSOR,
            ITEM_FUEL_CELL,
            ITEM_SLUG_CLIP,
            ITEM_HEAVY_CLIP,
            ITEM_AETHER_CORE,
            ITEM_SURVEY_DATA,
            ITEM_MECH_DATA,
            ITEM_OPTIC_DATA,
            ITEM_QUANTUM_DATA,
            ITEM_BELT,
            ITEM_TUNNEL,
            ITEM_SPLITTER,
            ITEM_ARM,
            ITEM_CHEST,
            ITEM_DRILL,
            ITEM_SMELTER,
            ITEM_ASSEMBLER,
            ITEM_LAB,
            ITEM_GENERATOR,
            ITEM_POLE,
            ITEM_SOLAR,
            ITEM_PIPE,
            ITEM_WELLHEAD,
            ITEM_REFINERY,
            ITEM_MIXER,
            ITEM_WALL,
            ITEM_SENTRY,
            ITEM_GRAND_ENGINE,
        ];
        for item in items {
            assert!(
                contrast(item, SLOT_WELL) >= 1.6,
                "item {item:#x} on SLOT_WELL = {:.2}",
                contrast(item, SLOT_WELL)
            );
        }
        assert!(contrast(TEXT, SLOT_WELL) >= 4.5, "TEXT on SLOT_WELL = {:.2}", contrast(TEXT, SLOT_WELL));
    }

    /// The lacquer accent reads as a soft dusty rose, not an aggressive red,
    /// titles stay readable on paper, and panel edges are warm wood trim.
    #[test]
    fn soft_red_is_not_aggressive() {
        assert!(
            saturation(LACQUER_500) <= 0.55,
            "LACQUER_500 saturation = {:.2}",
            saturation(LACQUER_500)
        );
        assert!(contrast(TITLE, BG) >= 4.5, "TITLE on BG = {:.2}", contrast(TITLE, BG));
        assert_eq!(EDGE, WOOD_TRIM, "EDGE must be the warm wood trim");
    }
}

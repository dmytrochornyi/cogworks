//! A tiny anti-aliased software painter built on signed distance functions.

use macroquad::math::{Vec2, vec2};
use macroquad::texture::Image;

use crate::theme;

/// Straight (non-premultiplied) RGBA, 0..=1.
pub type C = [f32; 4];

pub const fn hex(h: u32) -> C {
    theme::rgb(h)
}

pub fn mix(a: C, b: C, t: f32) -> C {
    let t = t.clamp(0.0, 1.0);
    [
        a[0] + (b[0] - a[0]) * t,
        a[1] + (b[1] - a[1]) * t,
        a[2] + (b[2] - a[2]) * t,
        a[3] + (b[3] - a[3]) * t,
    ]
}

/// Multiplies brightness.
pub fn lit(c: C, f: f32) -> C {
    [(c[0] * f).min(1.0), (c[1] * f).min(1.0), (c[2] * f).min(1.0), c[3]]
}

pub fn with_alpha(c: C, a: f32) -> C {
    [c[0], c[1], c[2], a]
}

// The warm lo-fi palette, sourced from the shared design tokens.
pub const INK: C = hex(theme::INK);
pub const CREAM: C = hex(theme::CREAM);
pub const PAPER: C = hex(theme::PAPER);
pub const PEACH: C = hex(theme::PEACH);
pub const TERRA: C = hex(theme::TERRA);
pub const RUST: C = hex(theme::RUST);
pub const MUSTARD: C = hex(theme::MUSTARD);
pub const HONEY: C = hex(theme::HONEY);
pub const SAGE: C = hex(theme::SAGE);
pub const SAGE_D: C = hex(theme::SAGE_D);
pub const OLIVE: C = hex(theme::OLIVE);
pub const TEAL: C = hex(theme::TEAL);
pub const TEAL_D: C = hex(theme::TEAL_D);
pub const SKY: C = hex(theme::SKY);
pub const PLUM: C = hex(theme::PLUM);
pub const ROSE: C = hex(theme::ROSE);
pub const BROWN: C = hex(theme::BROWN);
pub const BROWN_D: C = hex(theme::BROWN_D);
pub const STEEL: C = hex(theme::STEEL);
pub const STEEL_D: C = hex(theme::STEEL_D);
pub const CHAR: C = hex(theme::CHAR);
pub const LAMP: C = hex(theme::LAMP);
pub const WHITE: C = [1.0, 1.0, 1.0, 1.0];

// Terrain and ore materials.
pub const GRASS_A: C = hex(theme::GRASS_A);
pub const GRASS_B: C = hex(theme::GRASS_B);
pub const GRASS_WARM: C = hex(theme::GRASS_WARM);
pub const BLADE_LIGHT: C = hex(theme::BLADE_LIGHT);
pub const BLADE_DARK: C = hex(theme::BLADE_DARK);
pub const MOSS: C = hex(theme::MOSS);
pub const MOSS_DARK: C = hex(theme::MOSS_DARK);
pub const STEM_LIGHT: C = hex(theme::STEM_LIGHT);
pub const PEBBLE_LIGHT: C = hex(theme::PEBBLE_LIGHT);
pub const PEBBLE_DARK: C = hex(theme::PEBBLE_DARK);
pub const ORE_IRON: C = hex(theme::ORE_IRON);
pub const ORE_IRON_FACET: C = hex(theme::ORE_IRON_FACET);
pub const ORE_IRON_GLINT: C = hex(theme::ORE_IRON_GLINT);
pub const ORE_COPPER: C = hex(theme::ORE_COPPER);
pub const ORE_COPPER_FACET: C = hex(theme::ORE_COPPER_FACET);
pub const ORE_COPPER_PATINA: C = hex(theme::ORE_COPPER_PATINA);
pub const ORE_COAL: C = hex(theme::ORE_COAL);
pub const ORE_COAL_FACET: C = hex(theme::ORE_COAL_FACET);
pub const ORE_COAL_SHEEN: C = hex(theme::ORE_COAL_SHEEN);
pub const ORE_STONE: C = hex(theme::ORE_STONE);
pub const ORE_STONE_FACET: C = hex(theme::ORE_STONE_FACET);
pub const QUARTZ_LIGHT: C = hex(theme::QUARTZ_LIGHT);
pub const QUARTZ_SHADE: C = hex(theme::QUARTZ_SHADE);
pub const FLOWER_PINK: C = hex(theme::FLOWER_PINK);
pub const FLOWER_BUTTER: C = hex(theme::FLOWER_BUTTER);
pub const FLOWER_LILAC: C = hex(theme::FLOWER_LILAC);
pub const TAR_LIGHT: C = hex(theme::TAR_LIGHT);
pub const TAR_DARK: C = hex(theme::TAR_DARK);
pub const TAR_SHEEN: C = hex(theme::TAR_SHEEN);
pub const TAR_BUBBLE: C = hex(theme::TAR_BUBBLE);

// Logistics and machine materials.
pub const BELT_BAND: C = hex(theme::BELT_BAND);
pub const BELT_RIDGE: C = hex(theme::BELT_RIDGE);
pub const BELT_RAIL_HI: C = hex(theme::BELT_RAIL_HI);
pub const BELT_RAIL_LO: C = hex(theme::BELT_RAIL_LO);
pub const TUNNEL_STONE: C = hex(theme::TUNNEL_STONE);
pub const TUNNEL_MOUTH: C = hex(theme::TUNNEL_MOUTH);
pub const WOOD: C = hex(theme::WOOD);
pub const WOOD_DARK: C = hex(theme::WOOD_DARK);
pub const DRILL_HOLE: C = hex(theme::DRILL_HOLE);
pub const PIT: C = hex(theme::PIT);
pub const FIREBOX: C = hex(theme::FIREBOX);
pub const VENT_HI: C = hex(theme::VENT_HI);
pub const VENT_LO: C = hex(theme::VENT_LO);
pub const GLASS_HI: C = hex(theme::GLASS_HI);
pub const LAMP_INNER: C = hex(theme::LAMP_INNER);
pub const SOLAR_HI: C = hex(theme::SOLAR_HI);
pub const SOLAR_LO: C = hex(theme::SOLAR_LO);
pub const WELLHEAD_STONE: C = hex(theme::WELLHEAD_STONE);
pub const WELLHEAD_STONE_D: C = hex(theme::WELLHEAD_STONE_D);
pub const WELLHEAD_DARK: C = hex(theme::WELLHEAD_DARK);
pub const REFINERY_STONE: C = hex(theme::REFINERY_STONE);
pub const REFINERY_STONE_D: C = hex(theme::REFINERY_STONE_D);
pub const WALL_STONE: C = hex(theme::WALL_STONE);
pub const ENGINE_INNER: C = hex(theme::ENGINE_INNER);
pub const HIVE_SHELL: C = hex(theme::HIVE_SHELL);
pub const HIVE_SHELL_D: C = hex(theme::HIVE_SHELL_D);
pub const HIVE_CELL: C = hex(theme::HIVE_CELL);
pub const HIVE_GLOW_A: C = hex(theme::HIVE_GLOW_A);
pub const HIVE_GLOW_B: C = hex(theme::HIVE_GLOW_B);
pub const VISOR: C = hex(theme::VISOR);
pub const MITE_SHELL: C = hex(theme::MITE_SHELL);
pub const MITE_DARK: C = hex(theme::MITE_DARK);
pub const BRUTE_SHELL: C = hex(theme::BRUTE_SHELL);
pub const BRUTE_DARK: C = hex(theme::BRUTE_DARK);

// Items and effects.
pub const VIAL_GLASS: C = hex(theme::VIAL_GLASS);
pub const IRON_PLATE: C = hex(theme::IRON_PLATE);
pub const COPPER_PLATE: C = hex(theme::COPPER_PLATE);
pub const STEEL_LIGHT: C = hex(theme::STEEL_LIGHT);
pub const STEEL_PLATE_HI: C = hex(theme::STEEL_PLATE_HI);
pub const STEEL_PLATE_LO: C = hex(theme::STEEL_PLATE_LO);
pub const GLASS: C = hex(theme::GLASS);
pub const GLASS_LIGHT: C = hex(theme::GLASS_LIGHT);
pub const WIRE: C = hex(theme::WIRE);
pub const ROD: C = hex(theme::ROD);
pub const CIRCUIT_HI: C = hex(theme::CIRCUIT_HI);
pub const CIRCUIT_LO: C = hex(theme::CIRCUIT_LO);
pub const MOTOR_HI: C = hex(theme::MOTOR_HI);
pub const MOTOR_LO: C = hex(theme::MOTOR_LO);
pub const POLYMER_A: C = hex(theme::POLYMER_A);
pub const POLYMER_B: C = hex(theme::POLYMER_B);
pub const PROCESSOR_HI: C = hex(theme::PROCESSOR_HI);
pub const PROCESSOR_LO: C = hex(theme::PROCESSOR_LO);
pub const PROCESSOR_LED: C = hex(theme::PROCESSOR_LED);
pub const FUEL_HI: C = hex(theme::FUEL_HI);
pub const FUEL_LO: C = hex(theme::FUEL_LO);
pub const AETHER_GLOW: C = hex(theme::AETHER_GLOW);
pub const AETHER_HI: C = hex(theme::AETHER_HI);
pub const AETHER_LO: C = hex(theme::AETHER_LO);
pub const DATA_RED: C = hex(theme::DATA_RED);
pub const DATA_GREEN: C = hex(theme::DATA_GREEN);
pub const DATA_BLUE: C = hex(theme::DATA_BLUE);
pub const DATA_PURPLE: C = hex(theme::DATA_PURPLE);
pub const SMOKE: C = hex(theme::SMOKE);
pub const CLOUD: C = hex(theme::CLOUD);
pub const HEART: C = hex(theme::HEART);

pub struct Canvas {
    pub w: usize,
    pub h: usize,
    px: Vec<C>,
}

impl Canvas {
    pub fn new(w: usize, h: usize) -> Self {
        Canvas { w, h, px: vec![[0.0; 4]; w * h] }
    }

    fn blend(&mut self, x: usize, y: usize, c: C) {
        let d = &mut self.px[y * self.w + x];
        let a = c[3] + d[3] * (1.0 - c[3]);
        if a <= 0.0 {
            return;
        }
        for k in 0..3 {
            d[k] = (c[k] * c[3] + d[k] * d[3] * (1.0 - c[3])) / a;
        }
        d[3] = a;
    }

    /// Adds light: brightens existing pixels without changing coverage much.
    fn add(&mut self, x: usize, y: usize, c: C) {
        let d = &mut self.px[y * self.w + x];
        for k in 0..3 {
            d[k] = (d[k] + c[k] * c[3]).min(1.0);
        }
        d[3] = (d[3] + c[3] * 0.5).min(1.0);
    }

    /// Paints every pixel inside `sdf` (negative = inside) with `paint`, anti-aliased.
    pub fn fill(&mut self, sdf: impl Fn(Vec2) -> f32, paint: impl Fn(Vec2) -> C) {
        for y in 0..self.h {
            for x in 0..self.w {
                let p = vec2(x as f32 + 0.5, y as f32 + 0.5);
                let d = sdf(p);
                let cov = (0.5 - d).clamp(0.0, 1.0);
                if cov > 0.0 {
                    let mut c = paint(p);
                    c[3] *= cov;
                    self.blend(x, y, c);
                }
            }
        }
    }

    /// A filled shape with an inner outline band of width `ow`.
    pub fn shape(&mut self, sdf: impl Fn(Vec2) -> f32 + Copy, paint: impl Fn(Vec2) -> C, ow: f32) {
        self.fill(sdf, paint);
        if ow > 0.0 {
            self.fill(move |p| { let d = sdf(p); d.max(-(d + ow)) }, |_| INK);
        }
    }

    /// Solid colour shape with outline.
    pub fn solid(&mut self, sdf: impl Fn(Vec2) -> f32 + Copy, c: C, ow: f32) {
        self.shape(sdf, move |_| c, ow);
    }

    /// Soft drop shadow of a shape.
    pub fn shadow(&mut self, sdf: impl Fn(Vec2) -> f32, offset: Vec2, blur: f32, a: f32) {
        for y in 0..self.h {
            for x in 0..self.w {
                let p = vec2(x as f32 + 0.5, y as f32 + 0.5) - offset;
                let d = sdf(p);
                let cov = smooth(blur, -blur, d);
                if cov > 0.0 {
                    self.blend(x, y, with_alpha(hex(theme::SHADOW), a * cov));
                }
            }
        }
    }

    /// Additive soft glow.
    pub fn glow(&mut self, c: Vec2, r: f32, col: C) {
        for y in 0..self.h {
            for x in 0..self.w {
                let p = vec2(x as f32 + 0.5, y as f32 + 0.5);
                let t = 1.0 - (p.distance(c) / r).min(1.0);
                if t > 0.0 {
                    self.add(x, y, with_alpha(col, col[3] * t * t));
                }
            }
        }
    }

    /// Raw pixel colour at (x, y), straight RGBA in 0..=1. Used by the
    /// gen_assets tests to inspect seams between tiles.
    #[cfg(test)]
    pub fn pixel(&self, x: usize, y: usize) -> C {
        self.px[y * self.w + x]
    }

    /// Copies the `w`x`h` region starting at (x, y) into a new canvas.
    pub fn crop(&self, x: usize, y: usize, w: usize, h: usize) -> Canvas {
        let mut out = Canvas::new(w, h);
        for yy in 0..h {
            for xx in 0..w {
                out.px[yy * w + xx] = self.px[(y + yy) * self.w + (x + xx)];
            }
        }
        out
    }

    pub fn save(&self, path: &str) {
        let mut img = Image::gen_image_color(self.w as u16, self.h as u16, macroquad::color::BLANK);
        for y in 0..self.h {
            for x in 0..self.w {
                let c = self.px[y * self.w + x];
                // export_png writes rows bottom-up, so flip here.
                let row = (self.h - 1 - y) as u32;
                img.set_pixel(x as u32, row, theme::rgba(c[0], c[1], c[2], c[3]));
            }
        }
        img.export_png(path);
    }
}

pub fn smooth(e0: f32, e1: f32, x: f32) -> f32 {
    let t = ((x - e0) / (e1 - e0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

// --- Signed distance functions (pixels) ---

pub fn rrect(p: Vec2, c: Vec2, half: Vec2, r: f32) -> f32 {
    let q = (p - c).abs() - half + Vec2::splat(r);
    q.max(Vec2::ZERO).length() + q.x.max(q.y).min(0.0) - r
}

/// Rounded rect from its top-left corner and size.
pub fn rbox(p: Vec2, x: f32, y: f32, w: f32, h: f32, r: f32) -> f32 {
    rrect(p, vec2(x + w / 2.0, y + h / 2.0), vec2(w / 2.0, h / 2.0), r)
}

pub fn circle(p: Vec2, c: Vec2, r: f32) -> f32 {
    p.distance(c) - r
}

pub fn capsule(p: Vec2, a: Vec2, b: Vec2, r: f32) -> f32 {
    let pa = p - a;
    let ba = b - a;
    let h = (pa.dot(ba) / ba.dot(ba)).clamp(0.0, 1.0);
    (pa - ba * h).length() - r
}

pub fn ring(p: Vec2, c: Vec2, r: f32, w: f32) -> f32 {
    (p.distance(c) - r).abs() - w / 2.0
}

pub fn ellipse(p: Vec2, c: Vec2, r: Vec2) -> f32 {
    // Approximation, fine for soft shapes.
    let q = (p - c) / r;
    (q.length() - 1.0) * r.min_element()
}

/// Gear: a circle with `teeth` rounded teeth.
pub fn gear(p: Vec2, c: Vec2, r: f32, teeth: u32, depth: f32, rot: f32) -> f32 {
    let d = p - c;
    let a = d.y.atan2(d.x) + rot;
    let seg = std::f32::consts::TAU / teeth as f32;
    let k = ((a / seg).rem_euclid(1.0) - 0.5).abs() * 2.0;
    let tooth = smooth(0.35, 0.65, k);
    d.length() - (r + depth * (1.0 - tooth))
}

/// Convex polygon (vertices in order).
pub fn poly(p: Vec2, pts: &[Vec2]) -> f32 {
    let n = pts.len();
    let mut d = (p - pts[0]).dot(p - pts[0]);
    let mut s = 1.0f32;
    let mut j = n - 1;
    for i in 0..n {
        let e = pts[j] - pts[i];
        let w = p - pts[i];
        let b = w - e * (w.dot(e) / e.dot(e)).clamp(0.0, 1.0);
        d = d.min(b.dot(b));
        let c1 = p.y >= pts[i].y;
        let c2 = p.y < pts[j].y;
        let c3 = e.x * w.y > e.y * w.x;
        if (c1 && c2 && c3) || (!c1 && !c2 && !c3) {
            s = -s;
        }
        j = i;
    }
    s * d.sqrt()
}

// --- Noise ---

fn hash2(x: i32, y: i32, seed: u32) -> f32 {
    let mut h = (x as u32).wrapping_mul(374_761_393)
        ^ (y as u32).wrapping_mul(668_265_263)
        ^ seed.wrapping_mul(2_246_822_519);
    h = (h ^ (h >> 13)).wrapping_mul(1_274_126_177);
    ((h ^ (h >> 16)) & 0xffff) as f32 / 65535.0
}

/// Tileable value noise: `period` lattice cells across the tile.
pub fn noise(p: Vec2, cell: f32, period: i32, seed: u32) -> f32 {
    let q = p / cell;
    let (ix, iy) = (q.x.floor() as i32, q.y.floor() as i32);
    let f = q - vec2(ix as f32, iy as f32);
    let u = f * f * (Vec2::splat(3.0) - 2.0 * f);
    let w = |x: i32, y: i32| hash2(x.rem_euclid(period), y.rem_euclid(period), seed);
    let a = w(ix, iy);
    let b = w(ix + 1, iy);
    let c = w(ix, iy + 1);
    let d = w(ix + 1, iy + 1);
    let top = a + (b - a) * u.x;
    let bot = c + (d - c) * u.x;
    top + (bot - top) * u.y
}

/// Fractal noise that tiles every `size` pixels.
pub fn fbm(p: Vec2, size: f32, seed: u32) -> f32 {
    let mut v = 0.0;
    let mut amp = 0.5;
    let mut period = 4;
    for o in 0..4 {
        v += amp * noise(p, size / period as f32, period, seed + o);
        amp *= 0.5;
        period *= 2;
    }
    v / 0.9375
}

/// Deterministic random in 0..1 for decoration placement.
pub fn rand(i: u32, seed: u32) -> f32 {
    hash2(i as i32, 7, seed)
}

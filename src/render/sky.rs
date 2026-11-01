use macroquad::prelude::*;

use crate::theme;

/// Seconds per full day.
pub const DAY_LENGTH: f32 = 480.0;

pub struct Sky {
    /// Ambient light multiplier.
    pub ambient: Color,
    /// Darkness: 0 at noon, 1 at midnight. Drives particles and clouds.
    pub night: f32,
    /// How strongly lamps, windows and lanterns shine: 0 by day, 1 at night.
    pub glow: f32,
    /// Mood tint for the composite grade: rgb is the tint colour, `a` the
    /// strength. Keeps grass from going olive/teal under the ambient multiply.
    pub tint: Color,
    pub label: &'static str,
}

/// First three channels of a theme token, as floats.
const fn c3(hex: u32) -> [f32; 3] {
    let c = theme::rgb(hex);
    [c[0], c[1], c[2]]
}

/// Smoothly interpolate a value keyed by day fraction (0 = midnight).
fn lerp_keys(keys: &[(f32, f32)], p: f32) -> f32 {
    let k = keys.iter().rposition(|(t, _)| *t <= p).unwrap_or(0).min(keys.len() - 2);
    let (t0, v0) = keys[k];
    let (t1, v1) = keys[k + 1];
    let f = ((p - t0) / (t1 - t0)).clamp(0.0, 1.0);
    let f = f * f * (3.0 - 2.0 * f);
    v0 + (v1 - v0) * f
}

/// Smoothly interpolate an RGBA value (tint) keyed by day fraction.
fn lerp_keys4(keys: &[(f32, [f32; 4])], p: f32) -> [f32; 4] {
    let k = keys.iter().rposition(|(t, _)| *t <= p).unwrap_or(0).min(keys.len() - 2);
    let (t0, c0) = keys[k];
    let (t1, c1) = keys[k + 1];
    let f = ((p - t0) / (t1 - t0)).clamp(0.0, 1.0);
    let f = f * f * (3.0 - 2.0 * f);
    let mut out = [0.0; 4];
    for i in 0..4 {
        out[i] = c0[i] + (c1[i] - c0[i]) * f;
    }
    out
}

/// The pure core of `sky()`: `p` is the day fraction with 0 = midnight.
pub fn sky_at_phase(p: f32) -> Sky {
    let p = p.rem_euclid(1.0);
    const KEYS: [(f32, [f32; 3], &str); 10] = [
        (0.00, c3(theme::DUSK_500), "Night"),
        (0.22, c3(theme::DUSK_500), "Night"),
        (0.26, [0.78, 0.62, 0.70], "Dawn"),
        (0.30, [0.96, 0.90, 0.80], "Morning"),
        (0.50, [0.98, 1.00, 0.95], "Midday"),
        (0.62, [1.00, 0.80, 0.55], "Golden hour"),
        (0.70, [0.88, 0.56, 0.56], "Sunset"),
        (0.80, c3(theme::DUSK_300), "Dusk"),
        (0.88, c3(theme::DUSK_500), "Night"),
        (1.00, c3(theme::DUSK_500), "Night"),
    ];
    let k = KEYS.iter().rposition(|(t, _, _)| *t <= p).unwrap_or(0).min(KEYS.len() - 2);
    let (t0, c0, label) = KEYS[k];
    let (t1, c1, _) = KEYS[k + 1];
    let f = ((p - t0) / (t1 - t0)).clamp(0.0, 1.0);
    let f = f * f * (3.0 - 2.0 * f);
    let c: Vec<f32> = (0..3).map(|i| c0[i] + (c1[i] - c0[i]) * f).collect();
    // Darkness follows its own curve, so particles and clouds stay in step with
    // the phase even though the night ambient is now brighter (never pitch dark).
    const NIGHT: [(f32, f32); 9] = [
        (0.00, 1.0),
        (0.22, 1.0),
        (0.30, 0.0),
        (0.50, 0.0),
        (0.62, 0.2),
        (0.70, 0.55),
        (0.80, 0.9),
        (0.88, 1.0),
        (1.00, 1.0),
    ];
    // Lamps come on at golden hour and stay lit through the long dusk and night.
    const GLOW: [(f32, f32); 9] = [
        (0.00, 1.0),
        (0.22, 1.0),
        (0.26, 0.0),
        (0.60, 0.0),
        (0.62, 0.15),
        (0.70, 0.55),
        (0.80, 0.9),
        (0.88, 1.0),
        (1.00, 1.0),
    ];
    // Mood tint applied by COMPOSITE (and mirrored by theme::grade): rgb is the
    // tint colour, `a` the strength. The ambient multiplies green grass, which
    // would otherwise read olive at golden hour and teal at night; the tint
    // counteracts that. COMPOSITE scales the tint by (1 - lamp_share) so
    // lamp-lit areas keep their warm light instead of being repainted, while the
    // additive warm glow (see composite_rgb) carries the lantern's own colour
    // over the green grass. Unlit areas get the full strength: ~0.6 at golden
    // hour, ~0.65 at dusk and ~0.75 at night.
    const TINT: [(f32, [f32; 4]); 10] = [
        (0.00, [0.66, 0.42, 0.92, 0.75]), // violet-blue night
        (0.22, [0.66, 0.42, 0.92, 0.75]), // night
        (0.30, [1.00, 1.00, 1.00, 0.00]), // morning: no tint
        (0.50, [1.00, 1.00, 1.00, 0.00]), // midday: no tint
        (0.62, [1.00, 0.76, 0.42, 0.60]), // golden hour: peach-gold
        (0.68, [1.00, 0.76, 0.42, 0.60]), // hold the peach through golden hour
        (0.74, [0.86, 0.50, 0.86, 0.62]), // sunset: shift to lavender-pink
        (0.80, [0.80, 0.45, 0.90, 0.65]), // dusk: lavender-pink
        (0.88, [0.66, 0.42, 0.92, 0.75]), // night: violet-blue
        (1.00, [0.66, 0.42, 0.92, 0.75]), // night
    ];
    let t = lerp_keys4(&TINT, p);
    Sky {
        ambient: theme::rgba(c[0], c[1], c[2], 1.0),
        night: lerp_keys(&NIGHT, p),
        glow: lerp_keys(&GLOW, p),
        tint: theme::rgba(t[0], t[1], t[2], t[3]),
        label,
    }
}

pub fn sky(time: f32) -> Sky {
    // Start the first day in the morning.
    sky_at_phase((time / DAY_LENGTH + 0.3).fract())
}

/// Perceptual-luminance weights used by the composite maths. Keep in sync with
/// the `COMPOSITE` shader in `src/assets.rs` and `theme::grade`.
const LUMA: [f32; 3] = [0.299, 0.587, 0.114];

/// How strongly light beyond the ambient adds its own colour on top of the
/// surface (a warm "haze"), so lamp pools read peach over green grass. Keep in
/// sync with `COMPOSITE` in `src/assets.rs`.
#[allow(dead_code)]
const LAMP_HAZE: f32 = 0.30;

#[allow(dead_code)]
fn lum3(c: [f32; 3]) -> f32 {
    c[0] * LUMA[0] + c[1] * LUMA[1] + c[2] * LUMA[2]
}

/// THE reference for the world's lighting + mood-tint composite. `light` is the
/// light-map value as stored in the light render target (ambient·0.5 plus the
/// lamps), i.e. *before* the ×2; `ambient` is `Sky::ambient` (full strength);
/// `tint` is `Sky::tint` (rgb colour, strength in alpha).
///
/// This mirrors the lighting + tint block of the `COMPOSITE` shader in
/// `src/assets.rs` exactly — same operations and constants; the shader then
/// applies the highlight roll-off, warm grade and vignette, which stay there.
/// `theme::grade` mirrors the unlit (lamp_share = 0) case for flat images such
/// as the minimap.
///
/// The maths: a multiplicative base, a mood tint that applies at full strength
/// in unlit areas and fades to zero deep inside a lamp pool (so warm lantern
/// light is not repainted lavender/gold), and an additive warm glow so light
/// beyond the ambient carries its own colour over the green grass. The
/// highlight roll-off, warm grade and vignette are applied afterwards by the
/// shader and are *not* part of this function.
#[allow(dead_code)]
pub fn composite_rgb(surface: [f32; 3], light: [f32; 3], ambient: [f32; 3], tint: Color) -> [f32; 3] {
    // (a) Multiplicative base: the light map is stored at half intensity.
    let full = [light[0] * 2.0, light[1] * 2.0, light[2] * 2.0];
    let mut col = [surface[0] * full[0], surface[1] * full[1], surface[2] * full[2]];
    // How deep this pixel sits inside a lamp pool: 0 unlit, →1 deep in a pool.
    let lamp_share = ((lum3(full) - lum3(ambient)) / lum3(full).max(1e-4)).clamp(0.0, 1.0);
    // (b) Mood tint, full strength in unlit areas, none inside a lamp pool.
    let strength = tint.a * (1.0 - lamp_share);
    let tinted = [
        col[0] * tint.r * 1.6 + tint.r * 0.08,
        col[1] * tint.g * 1.6 + tint.g * 0.08,
        col[2] * tint.b * 1.6 + tint.b * 0.08,
    ];
    col = [
        col[0] + (tinted[0] - col[0]) * strength,
        col[1] + (tinted[1] - col[1]) * strength,
        col[2] + (tinted[2] - col[2]) * strength,
    ];
    // (c) Additive warm glow: light exceeding the ambient adds its own colour.
    for i in 0..3 {
        col[i] += (full[i] - ambient[i]).max(0.0) * LAMP_HAZE;
    }
    col
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn night_is_never_pitch_dark() {
        let mut min_avg = f32::INFINITY;
        let mut darkest = sky_at_phase(0.0);
        for i in 0..1000 {
            let s = sky_at_phase(i as f32 / 1000.0);
            let avg = (s.ambient.r + s.ambient.g + s.ambient.b) / 3.0;
            if avg < min_avg {
                min_avg = avg;
                darkest = s;
            }
        }
        assert!(min_avg >= 0.45, "night too dark: min ambient avg = {min_avg:.3}");
        assert!(darkest.ambient.b > darkest.ambient.r, "darkest ambient should lean blue (lavender)");
    }

    #[test]
    fn days_are_bright_and_sunny() {
        let s = sky_at_phase(0.5);
        let avg = (s.ambient.r + s.ambient.g + s.ambient.b) / 3.0;
        assert!(avg >= 0.95, "midday ambient avg = {avg:.3}");
        assert!(s.glow < 0.1, "midday glow = {:.3}", s.glow);
    }

    #[test]
    fn lanterns_glow_from_golden_hour() {
        assert!(sky_at_phase(0.68).glow >= 0.3, "glow at golden hour = {:.3}", sky_at_phase(0.68).glow);
        assert!(sky_at_phase(0.85).glow >= 0.8, "glow at dusk = {:.3}", sky_at_phase(0.85).glow);
    }

    #[test]
    fn dusk_phase_is_long() {
        let lit = (0..1000).filter(|i| sky_at_phase(*i as f32 / 1000.0).glow >= 0.5).count();
        assert!(lit as f32 / 1000.0 >= 0.40, "lanterns lit for only {lit}/1000 of the day");
    }

    /// `MEADOW_400` grass as the reference surface for the graded-output tests.
    fn grass() -> [f32; 3] {
        c3(theme::MEADOW_400)
    }

    /// Full ambient colour at a day phase.
    fn ambient(p: f32) -> [f32; 3] {
        let a = sky_at_phase(p).ambient;
        [a.r, a.g, a.b]
    }

    /// The light-map value in an unlit area: ambient stored at half intensity.
    fn unlit(p: f32) -> [f32; 3] {
        let a = ambient(p);
        [a[0] * 0.5, a[1] * 0.5, a[2] * 0.5]
    }

    /// The light-map value inside a lamp pool: ambient plus a full `LAMP`.
    fn lamp_pool(p: f32) -> [f32; 3] {
        let a = ambient(p);
        let l = c3(theme::LAMP);
        [a[0] * 0.5 + l[0] * 0.5, a[1] * 0.5 + l[1] * 0.5, a[2] * 0.5 + l[2] * 0.5]
    }

    #[test]
    fn golden_hour_reads_gold() {
        let p = 0.68;
        let c = composite_rgb(grass(), unlit(p), ambient(p), sky_at_phase(p).tint);
        let (r, g, b) = (c[0], c[1], c[2]);
        assert!(r > g && g > b, "golden hour grass should read gold, not olive: r={r:.3} g={g:.3} b={b:.3}");
        assert!(r - b >= 0.18, "golden hour needs a warm r-b split: {:.3}", r - b);
        assert!(g >= 0.75 * r, "golden hour keeps some green: r={r:.3} g={g:.3}");
    }

    #[test]
    fn dusk_reads_lavender() {
        let p = 0.8;
        let c = composite_rgb(grass(), unlit(p), ambient(p), sky_at_phase(p).tint);
        let (r, g, b) = (c[0], c[1], c[2]);
        assert!(
            b >= g * 0.95 && r >= g * 0.95,
            "dusk grass should read lavender, not teal: r={r:.3} g={g:.3} b={b:.3}"
        );
        assert!(g >= 0.6 * r, "dusk grass should not go magenta: r={r:.3} g={g:.3}");
    }

    #[test]
    fn night_reads_violet() {
        let p = 0.95;
        let c = composite_rgb(grass(), unlit(p), ambient(p), sky_at_phase(p).tint);
        let (r, g, b) = (c[0], c[1], c[2]);
        assert!(b > g, "night grass should read violet, not teal: r={r:.3} g={g:.3} b={b:.3}");
        assert!(b > r * 0.9, "night grass should lean violet-blue: r={r:.3} b={b:.3}");
    }

    #[test]
    fn lamp_pool_reads_peach() {
        for p in [0.8, 0.95] {
            let c = composite_rgb(grass(), lamp_pool(p), ambient(p), sky_at_phase(p).tint);
            let (r, g, b) = (c[0], c[1], c[2]);
            assert!(r > g && g > b, "lamp pool at p={p} should read warm peach: r={r:.3} g={g:.3} b={b:.3}");
            assert!(r - b >= 0.15, "lamp pool at p={p} needs a warm r-b split: {:.3}", r - b);
            assert!(g <= r * 0.9, "lamp pool at p={p} should not be yellow-green: r={r:.3} g={g:.3}");
        }
    }

    #[test]
    fn midday_untouched() {
        let p = 0.5;
        let c = composite_rgb(grass(), unlit(p), ambient(p), sky_at_phase(p).tint);
        let amb = ambient(p);
        let reference = [grass()[0] * amb[0], grass()[1] * amb[1], grass()[2] * amb[2]];
        for i in 0..3 {
            assert!(
                (c[i] - reference[i]).abs() < 0.03,
                "midday should have no tint: channel {i} is {:.4} vs {:.4}",
                c[i],
                reference[i]
            );
        }
    }
}

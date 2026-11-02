//! Loads the generated sprites (packed into one atlas), font and shaders.

use std::collections::HashMap;

use macroquad::miniquad::{BlendFactor, BlendState, BlendValue, Equation, MipmapFilterMode, PipelineParams};
use macroquad::prelude::*;

use crate::assets_list::SPRITES;

const ATLAS: u32 = 2048;
/// Each sprite's edge pixels are repeated into this border so filtering never
/// bleeds. 8 px keeps the smallest mip level clear of neighbouring sprites.
const PAD: u32 = 8;

pub struct Assets {
    pub atlas: Texture2D,
    rects: HashMap<&'static str, Rect>,
    pub font: Font,
    pub font_bold: Font,
    /// Combines the world with the light map and applies colour grading and a vignette.
    pub composite: Material,
    /// Plain textured drawing with additive blending, for lights.
    pub additive: Material,
    /// Ore fields: cuts each tile's edge from which neighbours are the same ore,
    /// so a deposit reads as one continuous patch (see `ORE_FIELD`).
    pub ore_field: Material,
    /// Keeps only the bright parts of the graded frame, for the bloom buffer.
    pub bright: Material,
    /// The final screen pass: bloom, ink, halftone, grain, motion blur (see `render::post`).
    pub post: Material,
}

impl Assets {
    pub async fn load() -> Self {
        let mut images: Vec<(&'static str, Image)> = SPRITES
            .iter()
            .map(|(name, bytes)| {
                let img = Image::from_file_with_format(bytes, Some(ImageFormat::Png))
                    .unwrap_or_else(|e| panic!("bad sprite {name}: {e}"));
                (*name, img)
            })
            .collect();
        images.sort_by_key(|(_, i)| std::cmp::Reverse(i.height));

        let mut atlas = Image::gen_image_color(ATLAS as u16, ATLAS as u16, BLANK);
        let mut rects = HashMap::new();
        let (mut x, mut y, mut row) = (0u32, 0u32, 0u32);
        for (name, img) in &images {
            let (w, h) = (img.width as u32, img.height as u32);
            if x + w + 2 * PAD > ATLAS {
                x = 0;
                y += row;
                row = 0;
            }
            assert!(y + h + 2 * PAD <= ATLAS, "sprite atlas is full");
            for yy in 0..h + 2 * PAD {
                let sy = yy.saturating_sub(PAD).min(h - 1);
                for xx in 0..w + 2 * PAD {
                    let sx = xx.saturating_sub(PAD).min(w - 1);
                    let s = ((sy * w + sx) * 4) as usize;
                    let d = (((y + yy) * ATLAS + x + xx) * 4) as usize;
                    atlas.bytes[d..d + 4].copy_from_slice(&img.bytes[s..s + 4]);
                }
            }
            rects.insert(*name, Rect::new((x + PAD) as f32, (y + PAD) as f32, w as f32, h as f32));
            x += w + 2 * PAD;
            row = row.max(h + 2 * PAD);
        }
        let atlas = Texture2D::from_image(&atlas);
        // Linear filtering plus mipmaps keep the packed sprites clean when zoomed
        // out. miniquad 0.4 exposes mipmap generation, so build them and use a
        // mipmapped min filter on the raw miniquad texture.
        unsafe {
            let id = atlas.raw_miniquad_id();
            let ctx = get_internal_gl().quad_context;
            ctx.texture_set_filter(id, FilterMode::Linear, MipmapFilterMode::Linear);
            ctx.texture_generate_mipmaps(id);
        }

        let font = load_ttf_font_from_bytes(include_bytes!("../assets/fonts/MPLUSRounded1c-Medium.ttf"))
            .expect("font");
        let font_bold = load_ttf_font_from_bytes(include_bytes!("../assets/fonts/MPLUSRounded1c-Bold.ttf"))
            .expect("font bold");

        let composite = load_material(
            ShaderSource::Glsl { vertex: VERTEX, fragment: COMPOSITE },
            MaterialParams {
                uniforms: vec![
                    UniformDesc::new("Params", UniformType::Float4),
                    UniformDesc::new("Tint", UniformType::Float4),
                    UniformDesc::new("Ambient", UniformType::Float4),
                    UniformDesc::new("Screen", UniformType::Float2),
                ],
                textures: vec!["Light".to_string()],
                ..Default::default()
            },
        )
        .expect("composite shader");
        let additive = load_material(
            ShaderSource::Glsl { vertex: VERTEX, fragment: TEXTURED },
            MaterialParams {
                pipeline_params: PipelineParams {
                    color_blend: Some(BlendState::new(
                        Equation::Add,
                        BlendFactor::Value(BlendValue::SourceAlpha),
                        BlendFactor::One,
                    )),
                    ..Default::default()
                },
                ..Default::default()
            },
        )
        .expect("additive shader");

        let ore_field = load_material(
            ShaderSource::Glsl { vertex: VERTEX_EXTRA, fragment: ORE_FIELD },
            MaterialParams {
                pipeline_params: PipelineParams {
                    color_blend: Some(BlendState::new(
                        Equation::Add,
                        BlendFactor::Value(BlendValue::SourceAlpha),
                        BlendFactor::OneMinusValue(BlendValue::SourceAlpha),
                    )),
                    ..Default::default()
                },
                ..Default::default()
            },
        )
        .expect("ore field shader");

        let bright = load_material(
            ShaderSource::Glsl { vertex: VERTEX, fragment: BRIGHT },
            MaterialParams { uniforms: vec![UniformDesc::new("Src", UniformType::Float2)], ..Default::default() },
        )
        .expect("bright-pass shader");
        let post = load_material(
            ShaderSource::Glsl { vertex: VERTEX, fragment: POST },
            MaterialParams {
                uniforms: vec![
                    UniformDesc::new("Screen", UniformType::Float4),
                    UniformDesc::new("LookA", UniformType::Float4),
                    UniformDesc::new("LookB", UniformType::Float4),
                    UniformDesc::new("Blur", UniformType::Float4),
                ],
                textures: vec!["Bloom".to_string()],
                ..Default::default()
            },
        )
        .expect("post shader");

        Assets { atlas, rects, font, font_bold, composite, additive, ore_field, bright, post }
    }

    pub fn rect(&self, name: &str) -> Rect {
        *self.rects.get(name).unwrap_or_else(|| panic!("missing sprite {name}"))
    }

    /// Draws a sprite with its top-left corner at `pos`, scaled to `size`.
    pub fn draw(&self, name: &str, pos: Vec2, size: Vec2, params: SpriteParams) {
        draw_texture_ex(&self.atlas, pos.x, pos.y, params.tint, DrawTextureParams {
            dest_size: Some(size),
            source: Some(self.rect(name)),
            rotation: params.rotation,
            flip_x: params.flip_x,
            flip_y: params.flip_y,
            pivot: params.pivot,
        });
    }

    /// Draws a sprite centred on `center`, rotated around its centre.
    pub fn draw_centered(&self, name: &str, center: Vec2, size: Vec2, rotation: f32, tint: Color) {
        self.draw(name, center - size * 0.5, size, SpriteParams { rotation, tint, ..Default::default() });
    }

    /// Draws a sprite standing on `base` (its bottom centre) and bent sideways
    /// like a plant in the wind: the bottom stays put and each row shifts by
    /// `bend * (height above the base)²`, so the top moves `bend` pixels and the
    /// trunk curves rather than tilting rigidly.
    pub fn draw_bent(&self, name: &str, base: Vec2, size: Vec2, bend: f32, flip_x: bool, tint: Color) {
        const ROWS: usize = 6;
        let uv = self.uv_rect(name);
        let (u0, u1) = if flip_x { (uv.right(), uv.x) } else { (uv.x, uv.right()) };
        let mut vertices = Vec::with_capacity((ROWS + 1) * 2);
        let mut indices = Vec::with_capacity(ROWS * 6);
        for r in 0..=ROWS {
            let f = r as f32 / ROWS as f32; // 0 at the top, 1 at the base
            let up = 1.0 - f;
            let y = base.y - size.y * up;
            let dx = bend * up * up;
            let v = uv.y + uv.h * f;
            vertices.push(Vertex::new(base.x - size.x * 0.5 + dx, y, 0.0, u0, v, tint));
            vertices.push(Vertex::new(base.x + size.x * 0.5 + dx, y, 0.0, u1, v, tint));
            if r > 0 {
                let i = (r * 2) as u16;
                indices.extend_from_slice(&[i - 2, i - 1, i + 1, i - 2, i + 1, i]);
            }
        }
        draw_mesh(&Mesh { vertices, indices, texture: Some(self.atlas.clone()) });
    }

    /// A sprite's rectangle in the atlas, in 0..1 texture coordinates.
    pub fn uv_rect(&self, name: &str) -> Rect {
        let r = self.rect(name);
        let s = self.atlas.size();
        Rect::new(r.x / s.x, r.y / s.y, r.w / s.x, r.h / s.y)
    }

    /// Native pixel size of a sprite.
    pub fn size(&self, name: &str) -> Vec2 {
        self.rect(name).size()
    }
}

#[derive(Clone, Copy)]
pub struct SpriteParams {
    pub rotation: f32,
    pub tint: Color,
    pub flip_x: bool,
    pub flip_y: bool,
    pub pivot: Option<Vec2>,
}

impl Default for SpriteParams {
    fn default() -> Self {
        SpriteParams { rotation: 0.0, tint: WHITE, flip_x: false, flip_y: false, pivot: None }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Width and height from the IHDR chunk of a PNG, per the big-endian u32s at
    /// bytes 16..24 (after the 8-byte signature and the 8-byte IHDR prefix).
    fn png_size(bytes: &[u8]) -> (u32, u32) {
        assert!(bytes.len() >= 24, "sprite is too small to be a PNG");
        assert_eq!(&bytes[0..8], b"\x89PNG\r\n\x1a\n", "sprite is not a PNG");
        let w = u32::from_be_bytes(bytes[16..20].try_into().unwrap());
        let h = u32::from_be_bytes(bytes[20..24].try_into().unwrap());
        (w, h)
    }

    /// The grade shader must not reintroduce film grain or a per-frame noise term.
    #[test]
    fn composite_has_no_grain() {
        assert!(!COMPOSITE.contains("fract(sin("), "COMPOSITE must not add film grain");
        assert!(!COMPOSITE.contains("Params.x"), "COMPOSITE must not read a noise param");
    }

    #[test]
    fn sprite_sizes_match_baseline() {
        let mut baseline: Vec<(String, (u32, u32))> = include_str!("sprite_sizes.txt")
            .lines()
            .map(|l| {
                let mut it = l.split_whitespace();
                let name = it.next().expect("name").to_string();
                let w: u32 = it.next().expect("width").parse().expect("width is an int");
                let h: u32 = it.next().expect("height").parse().expect("height is an int");
                assert!(it.next().is_none(), "extra fields on the line for {name}");
                (name, (w, h))
            })
            .collect();

        let mut actual: Vec<(String, (u32, u32))> = SPRITES
            .iter()
            .map(|(name, bytes)| (name.to_string(), png_size(bytes)))
            .collect();

        baseline.sort();
        actual.sort();

        assert_eq!(baseline, actual, "sprite sizes drifted from the baseline");
    }
}

const VERTEX: &str = r#"#version 100
attribute vec3 position;
attribute vec2 texcoord;
attribute vec4 color0;
varying lowp vec2 uv;
varying lowp vec4 color;
uniform mat4 Model;
uniform mat4 Projection;
void main() {
    gl_Position = Projection * Model * vec4(position, 1.0);
    uv = texcoord;
    color = color0 / 255.0;
}
"#;

/// Like `VERTEX`, but also passes the per-vertex `normal` through as `extra`,
/// which macroquad leaves free for custom data.
const VERTEX_EXTRA: &str = r#"#version 100
attribute vec3 position;
attribute vec2 texcoord;
attribute vec4 color0;
attribute vec4 normal;
varying lowp vec2 uv;
varying lowp vec4 color;
varying mediump vec4 extra;
uniform mat4 Model;
uniform mat4 Projection;
void main() {
    gl_Position = Projection * Model * vec4(position, 1.0);
    uv = texcoord;
    color = color0 / 255.0;
    extra = normal;
}
"#;

/// One ore-field tile. `extra.xy` is the position inside the tile (0..1, y
/// down) and `extra.z` an 8-bit mask of which neighbours hold the same ore,
/// clockwise from north: N, NE, E, SE, S, SW, W, NW. The field ends a little
/// short of every other neighbour, measured by a smooth distance to it (a soft
/// minimum, so corners round off), and the texture's alpha, which holds rock
/// height, lets rocks reach past that line for a rocky outline. The outline is
/// inked like the painted art, with a soft inner shade inside it.
const ORE_FIELD: &str = r#"#version 100
#ifdef GL_FRAGMENT_PRECISION_HIGH
precision highp float;
#else
precision mediump float;
#endif
varying lowp vec2 uv;
varying lowp vec4 color;
varying mediump vec4 extra;
uniform sampler2D Texture;

// Softness of the minimum: larger rounds outer corners more.
const float K = 0.26;
// Where the edge sits (distance from the open neighbour, in tiles), and how far
// the rock height pushes it in or out.
const float EDGE = 0.16;
const float ROCKY = 0.34;

float cell_dist(vec2 p, vec2 lo) {
    vec2 q = max(max(lo - p, p - (lo + 1.0)), 0.0);
    return length(q);
}

float open_term(float mask, float bit, vec2 p, vec2 lo) {
    // 1 when this neighbour is NOT ore, i.e. the field must fade toward it.
    float on = mod(floor(mask / bit), 2.0);
    return (1.0 - on) * exp(-cell_dist(p, lo) / K);
}

void main() {
    vec4 t = texture2D(Texture, uv);
    vec2 p = extra.xy;
    float m = floor(extra.z + 0.5);
    float acc = 0.0;
    acc += open_term(m, 1.0, p, vec2(0.0, -1.0));
    acc += open_term(m, 2.0, p, vec2(1.0, -1.0));
    acc += open_term(m, 4.0, p, vec2(1.0, 0.0));
    acc += open_term(m, 8.0, p, vec2(1.0, 1.0));
    acc += open_term(m, 16.0, p, vec2(0.0, 1.0));
    acc += open_term(m, 32.0, p, vec2(-1.0, 1.0));
    acc += open_term(m, 64.0, p, vec2(-1.0, 0.0));
    acc += open_term(m, 128.0, p, vec2(-1.0, -1.0));
    float d = acc > 1e-6 ? -K * log(acc) : 4.0;
    // Rocks (high alpha) reach past the edge, bare earth stops short of it.
    float e = d + (t.a - 0.6) * ROCKY - EDGE;
    float a = smoothstep(0.0, 0.025, e);
    // A dark ink line along the outline, like the painted art, then a gentle
    // inner shade so the deposit sits a little into the ground.
    vec3 ink = vec3(0.17, 0.10, 0.06);
    vec3 c = mix(ink, t.rgb, smoothstep(0.02, 0.06, e));
    c *= mix(0.82, 1.0, smoothstep(0.0, 0.45, e));
    gl_FragColor = vec4(c, a) * color;
}
"#;

const TEXTURED: &str = r#"#version 100
precision mediump float;
varying lowp vec2 uv;
varying lowp vec4 color;
uniform sampler2D Texture;
void main() {
    gl_FragColor = texture2D(Texture, uv) * color;
}
"#;

/// The light map is stored at half intensity so lamps can push the image brighter than ambient.
///
/// The lighting + tint block mirrors `render::sky::composite_rgb` exactly (same
/// operations and constants) — keep the two in sync. After it come the
/// unchanged grade steps: highlight roll-off, warm grade and vignette.
const COMPOSITE: &str = r#"#version 100
#ifdef GL_FRAGMENT_PRECISION_HIGH
precision highp float;
#else
precision mediump float;
#endif
varying lowp vec2 uv;
uniform sampler2D Texture;
uniform sampler2D Light;
uniform vec4 Params;   // unused, unused, vignette, warmth
uniform vec4 Tint;     // mood tint: rgb colour, a strength
uniform vec4 Ambient;  // ambient light (rgb), a unused — see render::sky::composite_rgb
uniform vec2 Screen;   // framebuffer size (currently unused)
void main() {
    vec3 c = texture2D(Texture, uv).rgb;
    vec3 l = texture2D(Light, uv).rgb;
    // ---- composite_rgb (mirrors render::sky::composite_rgb — keep in sync) ----
    vec3 full = l * 2.0;
    vec3 col = c * full;
    float llum = dot(full, vec3(0.299, 0.587, 0.114));
    float alum = dot(Ambient.rgb, vec3(0.299, 0.587, 0.114));
    float lamp_share = clamp((llum - alum) / max(llum, 0.0001), 0.0, 1.0);
    float strength = Tint.a * (1.0 - lamp_share);
    col = mix(col, col * Tint.rgb * 1.6 + Tint.rgb * 0.08, strength);
    col += max(full - Ambient.rgb, 0.0) * 0.30;
    // ---- unchanged grade: highlight roll-off, warm grade, vignette ----
    // Soft highlight roll-off so bright lamps bloom into cream instead of clipping.
    col = col / (1.0 + max(col - 0.85, 0.0) * 0.8);
    // Warm grade: lavender-lifted shadows, honeyed highlights.
    float luma = dot(col, vec3(0.299, 0.587, 0.114));
    col = mix(col, col * vec3(1.06, 1.0, 0.88), Params.w);
    col += vec3(0.03, 0.025, 0.06) * (1.0 - luma);
    col = mix(vec3(luma), col, 0.90);
    vec2 d = uv - 0.5;
    col *= 1.0 - dot(d, d) * Params.z;
    gl_FragColor = vec4(col, 1.0);
}
"#;

/// Bloom bright pass: a 4-tap box filter (the bloom buffer is a quarter of the
/// screen) that keeps only light above a soft threshold.
const BRIGHT: &str = r#"#version 100
#ifdef GL_FRAGMENT_PRECISION_HIGH
precision highp float;
#else
precision mediump float;
#endif
varying lowp vec2 uv;
uniform sampler2D Texture;
uniform vec2 Src; // source size in pixels
void main() {
    vec2 t = 1.0 / Src;
    vec3 c = texture2D(Texture, uv + vec2(-t.x, -t.y)).rgb
           + texture2D(Texture, uv + vec2( t.x, -t.y)).rgb
           + texture2D(Texture, uv + vec2(-t.x,  t.y)).rgb
           + texture2D(Texture, uv + vec2( t.x,  t.y)).rgb;
    c *= 0.25;
    float l = dot(c, vec3(0.299, 0.587, 0.114));
    gl_FragColor = vec4(c * smoothstep(0.80, 1.05, l), 1.0);
}
"#;

/// Final screen pass (see `render::post`). The graded frame is stored upside
/// down, so screen-space vectors arrive with y already flipped.
const POST: &str = r#"#version 100
#ifdef GL_FRAGMENT_PRECISION_HIGH
precision highp float;
#else
precision mediump float;
#endif
varying lowp vec2 uv;
uniform sampler2D Texture; // graded frame
uniform sampler2D Bloom;   // quarter-size bright pass
uniform vec4 Screen; // width, height, time, unused
uniform vec4 LookA;  // ink, halftone, unused, chroma
uniform vec4 LookB;  // bloom, unused, posterize, unused
uniform vec4 Blur;   // streak (uv), focus centre (uv)

float luma(vec3 c) { return dot(c, vec3(0.299, 0.587, 0.114)); }

// Colour plates printed slightly out of register: red nudged one way, blue
// the other, more toward the screen edges.
vec3 plates(vec2 p) {
    vec2 px = 1.0 / Screen.xy;
    vec2 off = ((p - 0.5) * 0.0035 + vec2(0.8, 0.4) * px) * LookA.w;
    return vec3(texture2D(Texture, p + off).r, texture2D(Texture, p).g, texture2D(Texture, p - off).b);
}

void main() {
    vec2 px = 1.0 / Screen.xy;
    vec2 frag = uv * Screen.xy;

    // ---- camera motion blur, faded out around the robot so it stays crisp ----
    vec2 toFocus = (uv - Blur.zw) * Screen.xy;
    float keep = smoothstep(40.0, 140.0, length(toFocus) / max(1.0, Screen.y / 900.0));
    vec2 streak = Blur.xy * keep;
    vec3 col;
    if (dot(streak, streak) * Screen.x * Screen.x > 1.0) {
        col = vec3(0.0);
        for (int i = 0; i < 8; i++) {
            float k = (float(i) + 0.5) / 8.0 - 0.5;
            col += plates(uv + streak * k);
        }
        col /= 8.0;
    } else {
        col = plates(uv);
    }

    // ---- ink: dark lines where luminance changes sharply (Sobel) ----
    if (LookA.x > 0.0) {
        float tl = luma(texture2D(Texture, uv + px * vec2(-1.0, -1.0)).rgb);
        float tc = luma(texture2D(Texture, uv + px * vec2( 0.0, -1.0)).rgb);
        float tr = luma(texture2D(Texture, uv + px * vec2( 1.0, -1.0)).rgb);
        float ml = luma(texture2D(Texture, uv + px * vec2(-1.0,  0.0)).rgb);
        float mr = luma(texture2D(Texture, uv + px * vec2( 1.0,  0.0)).rgb);
        float bl = luma(texture2D(Texture, uv + px * vec2(-1.0,  1.0)).rgb);
        float bc = luma(texture2D(Texture, uv + px * vec2( 0.0,  1.0)).rgb);
        float br = luma(texture2D(Texture, uv + px * vec2( 1.0,  1.0)).rgb);
        float gx = (tr + 2.0 * mr + br) - (tl + 2.0 * ml + bl);
        float gy = (bl + 2.0 * bc + br) - (tl + 2.0 * tc + tr);
        float edge = smoothstep(0.32, 0.75, length(vec2(gx, gy)));
        col = mix(col, col * vec3(0.30, 0.20, 0.24), edge * LookA.x * 0.75);
    }

    // ---- posterize toward a few printed tones ----
    if (LookB.z > 0.0) {
        vec3 q = floor(col * 6.0 + 0.5) / 6.0;
        col = mix(col, q, LookB.z);
    }

    // ---- halftone dots in the shadows, on a 45-degree screen ----
    if (LookA.y > 0.0) {
        float cell = max(4.0, Screen.y / 190.0);
        vec2 r = mat2(0.7071, -0.7071, 0.7071, 0.7071) * frag / cell;
        float d = length(fract(r) - 0.5);
        float dark = 1.0 - luma(col);
        float rad = sqrt(clamp((dark - 0.42) * 1.6, 0.0, 1.0)) * 0.56;
        float dotm = 1.0 - smoothstep(rad - 0.07, rad + 0.07, d);
        col = mix(col, col * vec3(0.58, 0.52, 0.80), dotm * LookA.y);
    }

    // ---- bloom: a soft glow around bright things, screened on top ----
    if (LookB.x > 0.0) {
        vec2 bt = px * 4.0;
        vec3 b = texture2D(Bloom, uv).rgb * 0.20;
        b += (texture2D(Bloom, uv + bt * vec2( 2.0,  0.0)).rgb + texture2D(Bloom, uv + bt * vec2(-2.0,  0.0)).rgb
            + texture2D(Bloom, uv + bt * vec2( 0.0,  2.0)).rgb + texture2D(Bloom, uv + bt * vec2( 0.0, -2.0)).rgb) * 0.12;
        b += (texture2D(Bloom, uv + bt * vec2( 3.0,  3.0)).rgb + texture2D(Bloom, uv + bt * vec2(-3.0,  3.0)).rgb
            + texture2D(Bloom, uv + bt * vec2( 3.0, -3.0)).rgb + texture2D(Bloom, uv + bt * vec2(-3.0, -3.0)).rgb) * 0.08;
        b *= vec3(1.0, 0.9, 0.75) * LookB.x;
        col = 1.0 - (1.0 - col) * (1.0 - clamp(b, 0.0, 1.0));
    }

    gl_FragColor = vec4(clamp(col, 0.0, 1.0), 1.0);
}
"#;

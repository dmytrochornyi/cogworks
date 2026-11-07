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
    /// Records where light pools overlap (see `OVERLAP`).
    pub overlap: Material,
    /// Ore fields: cuts each tile's edge from which neighbours are the same ore,
    /// so a deposit reads as one continuous patch (see `ORE_FIELD`).
    pub ore_field: Material,
    /// Keeps only the bright parts of the graded frame, for the bloom buffer.
    pub bright: Material,
    /// Blurs the bright pass at quarter resolution.
    pub bloom_blur: Material,
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
                textures: vec!["Light".to_string(), "Overlap".to_string()],
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
        // Red adds up each light's reach; alpha multiplies (1 - reach), so the
        // composite can tell two pools meeting from one bright pool.
        let overlap = load_material(
            ShaderSource::Glsl { vertex: VERTEX, fragment: OVERLAP },
            MaterialParams {
                pipeline_params: PipelineParams {
                    color_blend: Some(BlendState::new(Equation::Add, BlendFactor::One, BlendFactor::One)),
                    alpha_blend: Some(BlendState::new(
                        Equation::Add,
                        BlendFactor::Zero,
                        BlendFactor::Value(BlendValue::SourceAlpha),
                    )),
                    ..Default::default()
                },
                ..Default::default()
            },
        )
        .expect("overlap shader");

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
        let bloom_blur = load_material(
            ShaderSource::Glsl { vertex: VERTEX, fragment: BLOOM_BLUR },
            MaterialParams { uniforms: vec![UniformDesc::new("Src", UniformType::Float2)], ..Default::default() },
        )
        .expect("bloom blur shader");
        let post = load_material(
            ShaderSource::Glsl { vertex: VERTEX, fragment: POST },
            MaterialParams {
                uniforms: vec![
                    UniformDesc::new("Screen", UniformType::Float4),
                    UniformDesc::new("Fx", UniformType::Float4),
                    UniformDesc::new("Blur", UniformType::Float4),
                    UniformDesc::new("Sun", UniformType::Float4),
                    UniformDesc::new("View", UniformType::Float4),
                ],
                textures: vec!["Bloom".to_string()],
                ..Default::default()
            },
        )
        .expect("post shader");

        Assets { atlas, rects, font, font_bold, composite, additive, overlap, ore_field, bright, bloom_blur, post }
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

/// One light's reach `f` (glow falloff times strength): red gets `f` added,
/// alpha gets multiplied by `1 - f`. After all lights, `alpha - 1 + red` is the
/// sum of pairwise products of reaches: zero under a single light, growing
/// where pools overlap.
const OVERLAP: &str = r#"#version 100
precision mediump float;
varying lowp vec2 uv;
varying lowp vec4 color;
uniform sampler2D Texture;
void main() {
    float f = texture2D(Texture, uv).a * color.a;
    gl_FragColor = vec4(f, 0.0, 0.0, 1.0 - f);
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
uniform sampler2D Overlap; // r: summed reach, a: product of (1 - reach)
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
    // ---- where light pools meet: no white glare, a deeper rose-amber glow ----
    vec4 o = texture2D(Overlap, uv);
    float ov = clamp((o.a - 1.0 + o.r) * 2.5, 0.0, 1.0);
    float ol = dot(col, vec3(0.299, 0.587, 0.114));
    vec3 rose = ol * vec3(1.32, 0.80, 0.52) * 0.95;
    col = mix(col, rose, ov * 0.6);
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
    gl_FragColor = vec4(c * smoothstep(0.82, 1.08, l), 1.0);
}
"#;

/// Blurs the quarter-size bright pass, so the full-screen post pass can read
/// its soft bloom with a single tap. 13 bilinear taps cover about 9x9 texels.
const BLOOM_BLUR: &str = r#"#version 100
#ifdef GL_FRAGMENT_PRECISION_HIGH
precision highp float;
#else
precision mediump float;
#endif
varying lowp vec2 uv;
uniform sampler2D Texture;
uniform vec2 Src; // bloom buffer size in pixels
void main() {
    vec2 t = 1.0 / Src;
    vec3 c = texture2D(Texture, uv).rgb * 0.16;
    c += (texture2D(Texture, uv + t * vec2( 1.5,  1.5)).rgb + texture2D(Texture, uv + t * vec2(-1.5,  1.5)).rgb
        + texture2D(Texture, uv + t * vec2( 1.5, -1.5)).rgb + texture2D(Texture, uv + t * vec2(-1.5, -1.5)).rgb) * 0.11;
    c += (texture2D(Texture, uv + t * vec2( 3.5,  0.0)).rgb + texture2D(Texture, uv + t * vec2(-3.5,  0.0)).rgb
        + texture2D(Texture, uv + t * vec2( 0.0,  3.5)).rgb + texture2D(Texture, uv + t * vec2( 0.0, -3.5)).rgb) * 0.07;
    c += (texture2D(Texture, uv + t * vec2( 3.0,  3.0)).rgb + texture2D(Texture, uv + t * vec2(-3.0,  3.0)).rgb
        + texture2D(Texture, uv + t * vec2( 3.0, -3.0)).rgb + texture2D(Texture, uv + t * vec2(-3.0, -3.0)).rgb) * 0.04;
    gl_FragColor = vec4(c, 1.0);
}
"#;

/// Final screen pass (see `render::post`). The graded frame is stored upside
/// down, so screen-space vectors arrive with y already flipped. Texture reads:
/// one for the frame (six while the camera moves) and one for the bloom; the
/// rest is arithmetic.
const POST: &str = r#"#version 100
#ifdef GL_FRAGMENT_PRECISION_HIGH
precision highp float;
#else
precision mediump float;
#endif
varying lowp vec2 uv;
uniform sampler2D Texture; // graded frame
uniform sampler2D Bloom;   // quarter-size, already blurred
uniform vec4 Screen; // width, height, time, unused
uniform vec4 Fx;     // bloom, vibrance, cloud shadows, unused
uniform vec4 Blur;   // streak (uv), focus centre (uv)
uniform vec4 Sun;    // shaft strength, sun x (-1 left .. 1 right), warmth, mood amount
uniform vec4 View;   // camera top-left in world pixels (x, y), pixels per tile, daylight

float luma(vec3 c) { return dot(c, vec3(0.299, 0.587, 0.114)); }
float hash(vec2 p) {
    p = fract(p * vec2(443.897, 441.423));
    p += dot(p, p.yx + 19.19);
    return fract((p.x + p.y) * p.x);
}
float vnoise(vec2 p) {
    vec2 i = floor(p);
    vec2 f = fract(p);
    f = f * f * (3.0 - 2.0 * f);
    return mix(mix(hash(i), hash(i + vec2(1.0, 0.0)), f.x),
               mix(hash(i + vec2(0.0, 1.0)), hash(i + vec2(1.0, 1.0)), f.x), f.y);
}

void main() {
    vec2 frag = uv * Screen.xy;
    vec2 sp = vec2(frag.x, Screen.y - frag.y);   // screen pixels, y down
    vec2 wp = (sp + View.xy) / View.z;            // world tiles
    float t = Screen.z;
    float mood = Sun.w;
    float day = View.w;

    // ---- camera motion blur, faded out around the robot so it stays crisp ----
    vec2 toFocus = (uv - Blur.zw) * Screen.xy;
    float keep = smoothstep(40.0, 140.0, length(toFocus) / max(1.0, Screen.y / 900.0));
    vec2 streak = Blur.xy * keep;
    vec3 col;
    if (dot(streak, streak) * Screen.x * Screen.x > 1.0) {
        col = vec3(0.0);
        for (int i = 0; i < 6; i++) {
            col += texture2D(Texture, uv + streak * ((float(i) + 0.5) / 6.0 - 0.5)).rgb;
        }
        col /= 6.0;
    } else {
        col = texture2D(Texture, uv).rgb;
    }

    // ---- vibrance: lush greens and skies, vivid colours left alone ----
    float l = luma(col);
    if (Fx.y > 0.0) {
        float sat = max(col.r, max(col.g, col.b)) - min(col.r, min(col.g, col.b));
        col = mix(vec3(l), col, 1.0 + Fx.y * 0.3 * (1.0 - sat));
    }

    // ---- big soft cloud shadows drifting over the meadows by day ----
    float cloudy = Fx.z * day;
    if (cloudy > 0.0) {
        vec2 cp = (wp - vec2(0.35, -0.12) * t) / 16.0;
        float n = vnoise(cp) * 0.65 + vnoise(cp * 2.3 + 7.1) * 0.35;
        float sh = smoothstep(0.55, 0.75, n) * cloudy;
        col *= mix(vec3(1.0), vec3(0.80, 0.86, 0.96), sh * 0.75);
    }

    // ---- grade: soft contrast; airy cool shadows by day, warm brown ones and
    // golden highlights as the sun goes down ----
    if (mood > 0.0) {
        float w = clamp(Sun.z, 0.0, 1.0);
        vec3 s = clamp(col, 0.0, 1.0);
        col = mix(col, s * s * (3.0 - 2.0 * s), 0.18 * mood);
        l = luma(col);
        vec3 shadowTone = mix(vec3(0.90, 0.95, 1.06), vec3(0.80, 0.66, 0.62), w);
        vec3 lightTone = mix(vec3(1.04, 1.02, 0.96), vec3(1.06, 0.93, 0.82), w);
        col *= mix(vec3(1.0), lightTone, mood * smoothstep(0.25, 0.9, l));
        col = mix(col, col * shadowTone, mood * (0.45 + 0.55 * w) * (1.0 - smoothstep(0.05, 0.55, l)));
    }

    // ---- bloom: a gentle halation around bright things, screened on top ----
    if (Fx.x > 0.0) {
        vec3 b = texture2D(Bloom, uv).rgb * vec3(1.0, 0.92, 0.80) * Fx.x;
        col = 1.0 - (1.0 - col) * (1.0 - clamp(b, 0.0, 1.0));
    }

    // ---- sun: warm haze from the sun's corner and slanting shafts of light,
    // anchored to the world so they stay put as the camera moves ----
    float sun = Sun.x * mood;
    if (sun > 0.0) {
        vec2 corner = vec2(0.5 + 0.55 * Sun.y, -0.15);
        vec2 q = (sp / Screen.xy - corner) * vec2(Screen.x / Screen.y, 1.0);
        float near = clamp(1.0 - length(q) * 0.85, 0.0, 1.0);
        col = 1.0 - (1.0 - col) * (1.0 - vec3(1.0, 0.72, 0.38) * near * near * 0.22 * sun);

        vec2 dir = normalize(vec2(-0.65 * Sun.y, 1.0));
        float s = dot(wp, vec2(dir.y, -dir.x)) / 1.7;
        float b = (0.5 + 0.5 * sin(s * 1.00 + t * 0.07))
                * (0.5 + 0.5 * sin(s * 2.31 + 1.7 - t * 0.11))
                * (0.5 + 0.5 * sin(s * 0.47 + 4.2 + t * 0.05));
        b = smoothstep(0.14, 0.34, b);
        // Out of the beams the land sits in warm shade; inside, it is sunlit.
        col *= mix(vec3(1.0), mix(vec3(0.70, 0.62, 0.64), vec3(1.15, 0.97, 0.82), b), sun * 0.75);
        vec3 shaft = vec3(1.0, 0.70, 0.40) * b * (0.3 + 0.7 * near) * 0.16 * sun;
        col = 1.0 - (1.0 - col) * (1.0 - shaft);
    }

    // ---- aerial haze: the top of the view fades toward the sky colour ----
    if (mood > 0.0) {
        float top = 1.0 - sp.y / Screen.y;
        vec3 sky = mix(vec3(0.86, 0.93, 1.0), vec3(1.0, 0.84, 0.62), clamp(Sun.z, 0.0, 1.0));
        col = mix(col, sky, top * top * 0.12 * mood * day);
    }

    gl_FragColor = vec4(clamp(col, 0.0, 1.0), 1.0);
}
"#;

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

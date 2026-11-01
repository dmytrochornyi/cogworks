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

        Assets { atlas, rects, font, font_bold, composite, additive }
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
/// The lighting + tint block mirrors `render::composite_rgb` exactly (same
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
uniform vec4 Ambient;  // ambient light (rgb), a unused — see render::composite_rgb
uniform vec2 Screen;   // framebuffer size (currently unused)
void main() {
    vec3 c = texture2D(Texture, uv).rgb;
    vec3 l = texture2D(Light, uv).rgb;
    // ---- composite_rgb (mirrors render::composite_rgb — keep in sync) ----
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

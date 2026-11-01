//! Loads the generated sprites (packed into one atlas), sounds, font and shaders.

use std::collections::HashMap;

use macroquad::audio::{Sound, load_sound_from_bytes};
use macroquad::miniquad::{BlendFactor, BlendState, BlendValue, Equation, PipelineParams};
use macroquad::prelude::*;

use crate::assets_list::{SOUNDS, SPRITES};

const ATLAS: u32 = 2048;
/// Each sprite's edge pixels are repeated into this border so filtering never bleeds.
const PAD: u32 = 2;

pub struct Assets {
    pub atlas: Texture2D,
    rects: HashMap<&'static str, Rect>,
    pub font: Font,
    pub sounds: HashMap<&'static str, Sound>,
    /// Combines the world with the light map and applies colour grading, vignette and grain.
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
        atlas.set_filter(FilterMode::Linear);

        let mut sounds = HashMap::new();
        for (name, bytes) in SOUNDS {
            if let Ok(s) = load_sound_from_bytes(bytes).await {
                sounds.insert(*name, s);
            }
        }

        let font = load_ttf_font_from_bytes(include_bytes!("../assets/fonts/Nunito.ttf"))
            .expect("font");

        let composite = load_material(
            ShaderSource::Glsl { vertex: VERTEX, fragment: COMPOSITE },
            MaterialParams {
                uniforms: vec![
                    UniformDesc::new("Params", UniformType::Float4),
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

        Assets { atlas, rects, font, sounds, composite, additive }
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
const COMPOSITE: &str = r#"#version 100
#ifdef GL_FRAGMENT_PRECISION_HIGH
precision highp float;
#else
precision mediump float;
#endif
varying lowp vec2 uv;
uniform sampler2D Texture;
uniform sampler2D Light;
uniform vec4 Params; // time, grain, vignette, warmth
uniform vec2 Screen; // framebuffer size, so grain is the same physical size at every resolution
void main() {
    vec3 c = texture2D(Texture, uv).rgb;
    vec3 l = texture2D(Light, uv).rgb * 2.0;
    vec3 col = c * l;
    // Soft highlight roll-off so bright lamps bloom into cream instead of clipping.
    col = col / (1.0 + max(col - 0.85, 0.0) * 0.8);
    // Warm grade: plum-lifted shadows, honeyed highlights.
    float luma = dot(col, vec3(0.299, 0.587, 0.114));
    col = mix(col, col * vec3(1.06, 1.0, 0.88), Params.w);
    col += vec3(0.045, 0.02, 0.06) * (1.0 - luma);
    col = mix(vec3(luma), col, 0.92);
    vec2 d = uv - 0.5;
    col *= 1.0 - dot(d, d) * Params.z;
    // Keep the hash input small: big sin() arguments overflow mediump and turn black.
    // Hash on the physical pixel grid (2 px cells) so grain stays the same size on any screen.
    vec2 q = floor(uv * Screen / 2.0);
    float n = fract(sin(dot(mod(q, 97.0) + Params.x, vec2(1.2989, 7.8233))) * 4375.85);
    col += (n - 0.5) * Params.y;
    gl_FragColor = vec4(col, 1.0);
}
"#;

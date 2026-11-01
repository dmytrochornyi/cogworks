//! The minimap: a periodic raster of the world plus the HUD frame around it.

use macroquad::prelude::*;

use crate::render::*;
use crate::theme::{self, line, radius, space};
use crate::world::*;
use crate::Game;

use super::metrics::ui_width;
use super::primitives::{Elevation, circle, circle_lines, rrect_lines, surface};
use super::style::{BG, EDGE, MINIMAP, TEXT};

pub struct Minimap {
    img: Image,
    tex: Option<Texture2D>,
    timer: f32,
}

impl Minimap {
    pub fn new() -> Self {
        Minimap { img: Image::gen_image_color(MAP_W as u16, MAP_H as u16, BLANK), tex: None, timer: 0.0 }
    }

    fn refresh(&mut self, world: &World) {
        let tint = sky(world.time).tint;
        for y in 0..MAP_H {
            for x in 0..MAP_W {
                let p = ivec2(x, y);
                let col = match (world.entity_ref_at(p), world.res_at(p)) {
                    (Some(e), _) if e.kind == Kind::Hive => theme::color(theme::MAP_HIVE),
                    (Some(_), _) => theme::color(theme::MAP_BUILDING),
                    (None, Some(Resource::Ore(i))) => i.color(),
                    (None, Some(Resource::Tar)) => theme::color(theme::MAP_TAR),
                    (None, None) => theme::color(theme::MAP_GRASS),
                };
                // Same warm grade as the world shader, so the map reads as the same place.
                self.img.set_pixel(x as u32, y as u32, theme::grade(col, tint));
            }
        }
        for en in &world.enemies {
            let p = en.pos.floor().as_ivec2();
            if in_bounds(p) {
                self.img.set_pixel(p.x as u32, p.y as u32, theme::grade(theme::color(theme::MAP_ENEMY), tint));
            }
        }
        match &self.tex {
            Some(t) => t.update(&self.img),
            None => {
                let t = Texture2D::from_image(&self.img);
                t.set_filter(FilterMode::Linear);
                self.tex = Some(t);
            }
        }
    }
}

pub(super) fn minimap(g: &mut Game) {
    g.minimap.timer -= get_frame_time();
    if g.minimap.timer <= 0.0 || g.minimap.tex.is_none() {
        g.minimap.timer = 1.0;
        g.minimap.refresh(&g.world);
    }
    let outer = Rect::new(ui_width() - MINIMAP - space::XL - 6.0, space::L, MINIMAP + space::L, MINIMAP + space::L);
    surface(outer, Elevation::Panel);
    let r = Rect::new(outer.x + space::S, outer.y + space::S, MINIMAP, MINIMAP);
    if let Some(t) = &g.minimap.tex {
        draw_texture_ex(t, r.x, r.y, WHITE, DrawTextureParams { dest_size: Some(r.size()), ..Default::default() });
    }
    rrect_lines(r, radius::SM, line::STRONG, EDGE);
    let scale = MINIMAP / MAP_W as f32;
    let p = g.world.player.pos * scale + r.point();
    circle(p, 4.0, TEXT);
    circle_lines(p, 4.0, line::BASE, BG);
    let lo = g.cam.s2w(Vec2::ZERO) * scale + r.point();
    let hi = g.cam.s2w(vec2(screen_width(), screen_height())) * scale + r.point();
    rrect_lines(Rect::new(lo.x, lo.y, hi.x - lo.x, hi.y - lo.y), 2.0, line::HAIR, theme::alpha(theme::CREAM, 0.6));
    g.ui_rects.push(outer);
}

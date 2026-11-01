use std::f32::consts::{FRAC_PI_2, PI};

use macroquad::prelude::*;

use crate::assets::Assets;
use crate::items::*;
use crate::render::*;
use crate::theme::{self, TextSize, line, radius, space};
use crate::world::*;
use crate::{Game, Panel as OpenPanel};

const SLOT: f32 = 48.0;
const GAP: f32 = 6.0;
const QUICK_SLOT: f32 = 54.0;
const MINIMAP: f32 = 190.0;
const LINE: f32 = 21.0;
const HEADER: f32 = 54.0;

// Tokens, expressed once for the whole UI.
const BG: Color = theme::color(theme::BG);
const BG2: Color = theme::color(theme::BG2);
const EDGE: Color = theme::color(theme::EDGE);
pub const TEXT: Color = theme::color(theme::TEXT);
const TEXT_DIM: Color = theme::color(theme::TEXT_DIM);
const TITLE: Color = theme::color(theme::TITLE);
const KEY: Color = theme::color(theme::KEY);
const SLOT_WELL: Color = theme::color(theme::SLOT_WELL);
const HONEY: Color = theme::color(theme::HONEY);
// Rust accent for warnings: darker than the light `PEACH` so it reads on paper.
const PEACH: Color = theme::color(theme::TERRA_600);
const TERRA: Color = theme::color(theme::TERRA);
const TEAL: Color = theme::color(theme::TEAL);
const SHADE: Color = theme::color(theme::SHADE);

// ------------------------------------------------------------------ ui camera

/// Pure scale function so the layout maths can be tested without a window.
fn scale_for_height(h: f32) -> f32 {
    (h / 900.0).clamp(0.8, 1.6)
}

/// Uniform UI scale: ~1.0 at 900p, clamped so tiny windows stay usable and
/// 4K screens get a proportionally larger interface instead of a speck.
pub fn ui_scale() -> f32 {
    scale_for_height(screen_height())
}

/// Logical UI space, in which every layout below is written.
pub fn ui_size() -> Vec2 {
    vec2(screen_width(), screen_height()) / ui_scale()
}

pub fn ui_width() -> f32 {
    ui_size().x
}

pub fn ui_height() -> f32 {
    ui_size().y
}

/// Mouse position in logical UI space.
pub fn ui_mouse() -> Vec2 {
    let m: Vec2 = mouse_position().into();
    m / ui_scale()
}

/// The UI camera, laid out in logical UI space over the given size.
///
/// macroquad 0.4 inverts Y again for cameras without a render target, so
/// `from_display_rect` alone draws the UI upside down. Flip it back to y-down.
pub fn ui_camera(w: f32, h: f32) -> Camera2D {
    let mut cam = Camera2D::from_display_rect(Rect::new(0.0, 0.0, w, h));
    cam.zoom.y = -cam.zoom.y;
    cam
}

/// Restrict drawing to `r` while the research grid scrolls.
fn clip(r: Rect) {
    let s = ui_scale();
    unsafe {
        get_internal_gl().quad_gl.scissor(Some((
            (r.x * s) as i32,
            (r.y * s) as i32,
            (r.w * s) as i32,
            (r.h * s) as i32,
        )));
    }
}

fn unclip() {
    unsafe {
        get_internal_gl().quad_gl.scissor(None);
    }
}

// ------------------------------------------------------------------ primitives

const CORNERS: [f32; 4] = [PI, -FRAC_PI_2, 0.0, FRAC_PI_2];

fn corner_centers(r: Rect, rad: f32) -> [Vec2; 4] {
    [
        vec2(r.x + rad, r.y + rad),
        vec2(r.right() - rad, r.y + rad),
        vec2(r.right() - rad, r.bottom() - rad),
        vec2(r.x + rad, r.bottom() - rad),
    ]
}

/// Filled rounded rectangle with no overlapping parts, so translucent colours stay even.
pub fn rrect(r: Rect, rad: f32, col: Color) {
    let rad = rad.min(r.w / 2.0).min(r.h / 2.0).max(0.0);
    draw_rectangle(r.x, r.y + rad, r.w, r.h - 2.0 * rad, col);
    draw_rectangle(r.x + rad, r.y, r.w - 2.0 * rad, rad, col);
    draw_rectangle(r.x + rad, r.bottom() - rad, r.w - 2.0 * rad, rad, col);
    for (c, start) in corner_centers(r, rad).into_iter().zip(CORNERS) {
        for k in 0..12 {
            let a0 = start + k as f32 / 12.0 * FRAC_PI_2;
            let a1 = start + (k + 1) as f32 / 12.0 * FRAC_PI_2;
            draw_triangle(c, c + Vec2::from_angle(a0) * rad, c + Vec2::from_angle(a1) * rad, col);
        }
    }
}

pub fn rrect_lines(r: Rect, rad: f32, thick: f32, col: Color) {
    let rad = rad.min(r.w / 2.0).min(r.h / 2.0);
    draw_line(r.x + rad, r.y, r.right() - rad, r.y, thick, col);
    draw_line(r.x + rad, r.bottom(), r.right() - rad, r.bottom(), thick, col);
    draw_line(r.x, r.y + rad, r.x, r.bottom() - rad, thick, col);
    draw_line(r.right(), r.y + rad, r.right(), r.bottom() - rad, thick, col);
    for (c, start) in corner_centers(r, rad).into_iter().zip(CORNERS) {
        for k in 0..12 {
            let p0 = c + Vec2::from_angle(start + k as f32 / 12.0 * FRAC_PI_2) * rad;
            let p1 = c + Vec2::from_angle(start + (k + 1) as f32 / 12.0 * FRAC_PI_2) * rad;
            draw_line(p0.x, p0.y, p1.x, p1.y, thick, col);
        }
    }
}

/// Dashed rounded outline, used by the placement preview.
pub fn rrect_dashed_lines(r: Rect, rad: f32, thick: f32, col: Color) {
    let rad = rad.min(r.w / 2.0).min(r.h / 2.0).max(0.0);
    let mut path: Vec<Vec2> = Vec::new();
    let edge = |pts: &mut Vec<Vec2>, a: Vec2, b: Vec2| {
        let n = ((b - a).length() / 4.0).ceil().max(1.0) as usize;
        for k in 0..=n {
            pts.push(a.lerp(b, k as f32 / n as f32));
        }
    };
    edge(&mut path, vec2(r.x + rad, r.y), vec2(r.right() - rad, r.y));
    for k in 0..=12 {
        let a = -FRAC_PI_2 + k as f32 / 12.0 * FRAC_PI_2;
        path.push(vec2(r.right() - rad, r.y + rad) + Vec2::from_angle(a) * rad);
    }
    edge(&mut path, vec2(r.right(), r.y + rad), vec2(r.right(), r.bottom() - rad));
    for k in 0..=12 {
        let a = k as f32 / 12.0 * FRAC_PI_2;
        path.push(vec2(r.right() - rad, r.bottom() - rad) + Vec2::from_angle(a) * rad);
    }
    edge(&mut path, vec2(r.right() - rad, r.bottom()), vec2(r.x + rad, r.bottom()));
    for k in 0..=12 {
        let a = FRAC_PI_2 + k as f32 / 12.0 * FRAC_PI_2;
        path.push(vec2(r.x + rad, r.bottom() - rad) + Vec2::from_angle(a) * rad);
    }
    edge(&mut path, vec2(r.x, r.bottom() - rad), vec2(r.x, r.y + rad));
    for k in 0..=12 {
        let a = PI + k as f32 / 12.0 * FRAC_PI_2;
        path.push(vec2(r.x + rad, r.y + rad) + Vec2::from_angle(a) * rad);
    }
    let mut dist = 0.0;
    for w in path.windows(2) {
        let seg = (w[1] - w[0]).length();
        if (dist % 14.0) < 7.0 {
            draw_line(w[0].x, w[0].y, w[1].x, w[1].y, thick, col);
        }
        dist += seg;
    }
}

/// Smoother circles than macroquad's default 20-gon, for badges and rings.
pub fn circle(c: Vec2, r: f32, col: Color) {
    draw_poly(c.x, c.y, 48, r, 0.0, col);
}

pub fn circle_lines(c: Vec2, r: f32, thick: f32, col: Color) {
    draw_poly_lines(c.x, c.y, 48, r, 0.0, thick, col);
}

fn hovered(r: Rect) -> bool {
    r.contains(ui_mouse())
}

fn clicked(r: Rect) -> bool {
    is_mouse_button_pressed(MouseButton::Left) && hovered(r)
}

fn ease_out(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    1.0 - (1.0 - t).powi(3)
}

/// Pixel offset of the open modal panel during its drop-in animation.
fn slide_px(g: &Game) -> f32 {
    (1.0 - ease_out(g.panel_slide)) * 28.0
}

// ------------------------------------------------------------------ type

/// Physical raster size and the draw-time scale that undoes `ui_scale()`.
///
/// Text is rasterised at the screen's pixel density (font size × `ui_scale`)
/// and then drawn at `font_scale = 1 / ui_scale`, so it stays sharp on 1440p
/// and 4K instead of being drawn small and scaled up. Measuring must use the
/// same pair so drawn and measured widths agree.
pub fn raster_params(px: f32, scale: f32) -> (u16, f32) {
    ((px * scale).round() as u16, 1.0 / scale)
}

/// The font cut for a text size: titles and headings are Bold, the rest Medium.
fn font_for(a: &Assets, size: TextSize) -> &Font {
    if size.bold() { &a.font_bold } else { &a.font }
}

/// Full line metrics (ascent + descent) for a text size, measured from a fixed
/// reference glyph pair so every segment of one line shares the same baseline.
fn line_metrics(a: &Assets, size: TextSize) -> TextDimensions {
    let (font_size, font_scale) = raster_params(size.px(), ui_scale());
    measure_text("Hg", Some(font_for(a, size)), font_size, font_scale)
}

/// Draws one line of text. `y` is the top of the line box; the baseline comes
/// from a fixed reference string rather than the string's own tallest glyph, so
/// segments without ascenders ("zoom", "save", "pause") sit on the same line
/// as their neighbours instead of floating higher.
pub fn text(a: &Assets, t: &str, x: f32, y: f32, size: TextSize, col: Color) {
    let (font_size, font_scale) = raster_params(size.px(), ui_scale());
    let font = font_for(a, size);
    let baseline = line_metrics(a, size).offset_y;
    draw_text_ex(t, x.round(), (y + baseline).round(), TextParams {
        font: Some(font),
        font_size,
        font_scale,
        color: col,
        ..Default::default()
    });
}

pub fn text_w(a: &Assets, t: &str, size: TextSize) -> f32 {
    let (font_size, font_scale) = raster_params(size.px(), ui_scale());
    measure_text(t, Some(font_for(a, size)), font_size, font_scale).width
}

/// Text centred in `r`, using the string's width and the fixed line height so
/// vertical centring doesn't shift for strings without ascenders or descenders.
fn text_in(a: &Assets, t: &str, r: Rect, size: TextSize, col: Color) {
    let (font_size, font_scale) = raster_params(size.px(), ui_scale());
    let font = font_for(a, size);
    let w = measure_text(t, Some(font), font_size, font_scale).width;
    let h = line_metrics(a, size).height;
    text(a, t, r.x + (r.w - w) / 2.0, r.y + (r.h - h) / 2.0, size, col);
}

fn text_right(a: &Assets, t: &str, right: f32, y: f32, size: TextSize, col: Color) {
    text(a, t, right - text_w(a, t, size), y, size, col);
}

/// Renders a help line with the key/action before each " · " in the deep-gold
/// key colour and the description in the base colour, preserving the original
/// column spacing.
fn text_keys(a: &Assets, t: &str, x: f32, y: f32, size: TextSize, base: Color) {
    let key_col = KEY;
    let mut cx = x;
    let mut rest = t;
    let span = |s: &str, cx: &mut f32, col: Color| {
        text(a, s, *cx, y, size, col);
        *cx += text_w(a, s, size);
    };
    while !rest.is_empty() {
        let (seg, tail) = match rest.find("  ") {
            Some(i) => rest.split_at(i),
            None => (rest, ""),
        };
        if let Some((key, after)) = seg.split_once(" · ") {
            span(key, &mut cx, key_col);
            span(" · ", &mut cx, base);
            span(after, &mut cx, base);
        } else {
            span(seg, &mut cx, base);
        }
        let n = tail.chars().take_while(|c| *c == ' ').count();
        if n > 0 {
            let spaces = &tail[..n];
            span(spaces, &mut cx, base);
            rest = &tail[n..];
        } else {
            rest = tail;
        }
    }
}

// ------------------------------------------------------------------ surfaces

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Elevation {
    /// Full window with a soft two-layer shadow.
    Panel,
    /// Inset card (research, recipe rows).
    Card,
    /// Floating tooltip.
    Tooltip,
    /// Toast pill.
    Toast,
    /// Centre-screen banner.
    Banner,
}

pub fn surface(r: Rect, elev: Elevation) {
    surface_alpha(r, elev, 1.0);
}

fn surface_alpha(r: Rect, elev: Elevation, alpha: f32) {
    let (rad, fill, border, border_w, shadow) = match elev {
        Elevation::Panel => (radius::LG, theme::alpha(theme::BG, 0.96), EDGE, line::BASE, true),
        Elevation::Card => (radius::MD, BG2, theme::alpha(theme::EDGE, 0.7), line::BASE, false),
        Elevation::Tooltip => (radius::MD, theme::alpha(theme::BG, 0.98), EDGE, line::BASE, true),
        Elevation::Toast => (radius::MD, theme::alpha(theme::BG, 0.94), EDGE, line::BASE, true),
        Elevation::Banner => (radius::LG, theme::alpha(theme::BG, 0.92), EDGE, line::BASE, true),
    };
    let mix_a = |c: Color, a: f32| theme::with_alpha(c, c.a * a * alpha);
    if shadow {
        // Soft drop shadow, kept subtle under the cream surface.
        rrect(Rect::new(r.x + 4.0, r.y + 6.0, r.w, r.h), rad, mix_a(SHADE, 0.22));
        rrect(Rect::new(r.x + 8.0, r.y + 10.0, r.w, r.h), rad, mix_a(SHADE, 0.14));
        // Hard wood under-edge, offset straight down so it reads as paper thickness.
        rrect(Rect::new(r.x, r.y + 3.0, r.w, r.h), rad, mix_a(theme::color(theme::TRIM_SHADOW), 1.0));
    }
    // Border as two filled rounded rects: an outer one in the border colour and
    // an inner one in the fill colour, inset by the border width. This gives a
    // clean smooth edge band instead of an anti-aliased outline line.
    rrect(r, rad, mix_a(border, 1.0));
    rrect(
        Rect::new(r.x + border_w, r.y + border_w, r.w - 2.0 * border_w, r.h - 2.0 * border_w),
        (rad - border_w).max(0.0),
        mix_a(fill, 1.0),
    );
}

/// A window with a title, a close button and an Esc hint.
pub struct Panel<'a> {
    r: Rect,
    title: Option<&'a str>,
    subtitle: Option<&'a str>,
    closable: bool,
}

impl<'a> Panel<'a> {
    pub fn new(r: Rect) -> Self {
        Panel { r, title: None, subtitle: None, closable: false }
    }

    pub fn title(mut self, t: &'a str) -> Self {
        self.title = Some(t);
        self
    }

    pub fn subtitle(mut self, t: &'a str) -> Self {
        self.subtitle = Some(t);
        self
    }

    pub fn closable(mut self) -> Self {
        self.closable = true;
        self
    }

    /// Draws the window and returns its content area plus whether close was clicked.
    pub fn show(self, g: &mut Game) -> (Rect, bool) {
        surface(self.r, Elevation::Panel);
        g.ui_rects.push(self.r);
        let a = g.assets.clone();
        let mut closed = false;
        if let Some(t) = self.title {
            text(&a, t, self.r.x + space::XL, self.r.y + space::L, TextSize::Xl, TITLE);
        }
        if let Some(s) = self.subtitle {
            let w = self.title.map(|t| text_w(&a, t, TextSize::Xl)).unwrap_or(0.0);
            text(&a, s, self.r.x + space::XL + w + space::L, self.r.y + space::L + 8.0, TextSize::S, TEXT_DIM);
        }
        if self.closable {
            let cb = Rect::new(self.r.right() - space::L - 28.0, self.r.y + space::M, 28.0, 28.0);
            let hint = text_w(&a, "Esc", TextSize::Xs);
            text(&a, "Esc", cb.x - hint - space::S, self.r.y + space::L - 2.0, TextSize::Xs, KEY);
            if button_kind(&a, cb, "x", true, ButtonKind::Secondary) {
                closed = true;
            }
            g.ui_rects.push(cb);
        }
        let top = if self.title.is_some() { self.r.y + HEADER } else { self.r.y + space::L };
        let content = Rect::new(
            self.r.x + space::XL,
            top,
            self.r.w - 2.0 * space::XL,
            (self.r.bottom() - space::L - top).max(0.0),
        );
        (content, closed)
    }
}

// ------------------------------------------------------------------ components

fn bar(r: Rect, frac: f32, col: Color) {
    rrect(r, r.h / 2.0, SLOT_WELL);
    if frac > 0.0 {
        let w = (r.w * frac.clamp(0.0, 1.0)).max(r.h);
        rrect(Rect::new(r.x, r.y, w, r.h), r.h / 2.0, col);
    }
    rrect_lines(r, r.h / 2.0, line::HAIR, theme::alpha(theme::TEXT, 0.22));
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ButtonKind {
    Primary,
    Secondary,
}

fn button_kind(a: &Assets, r: Rect, label: &str, enabled: bool, kind: ButtonKind) -> bool {
    let hot = enabled && hovered(r);
    let down = hot && is_mouse_button_down(MouseButton::Left);
    let lacquer = theme::color(theme::LACQUER_500);
    let lacquer_hot = theme::color(theme::LACQUER_400);
    let (fill, border, text_col) = match (kind, enabled, hot) {
        (ButtonKind::Primary, true, false) => (lacquer, lacquer, BG),
        (ButtonKind::Primary, true, true) => (lacquer_hot, lacquer_hot, BG),
        (ButtonKind::Primary, false, _) => (theme::alpha(theme::SLOT_WELL, 0.5), theme::alpha(theme::EDGE, 0.35), theme::alpha(theme::TEXT_DIM, 0.5)),
        (ButtonKind::Secondary, true, false) => (BG, EDGE, TEXT),
        (ButtonKind::Secondary, true, true) => (BG2, EDGE, TEXT),
        (ButtonKind::Secondary, false, _) => (theme::alpha(theme::BG2, 0.5), theme::alpha(theme::EDGE, 0.35), theme::alpha(theme::TEXT_DIM, 0.45)),
    };
    let off = if down { 1.0 } else { 0.0 };
    rrect(Rect::new(r.x, r.y + off, r.w, r.h), radius::MD, fill);
    rrect_lines(Rect::new(r.x, r.y + off, r.w, r.h), radius::MD, line::BASE, border);
    text_in(a, label, Rect::new(r.x, r.y + off, r.w, r.h), TextSize::M, text_col);
    enabled && clicked(r)
}

fn button(a: &Assets, r: Rect, label: &str, enabled: bool) -> bool {
    button_kind(a, r, label, enabled, ButtonKind::Secondary)
}

fn primary_button(a: &Assets, r: Rect, label: &str, enabled: bool) -> bool {
    button_kind(a, r, label, enabled, ButtonKind::Primary)
}

fn slot(a: &Assets, r: Rect, item: Option<Item>, count: Option<u32>, selected: bool, unavailable: bool) -> bool {
    let hot = hovered(r);
    // A warm paper "well" under the icon; hovering lightens it to the panel.
    let fill = if hot && !selected { BG } else { SLOT_WELL };
    rrect(r, radius::MD, fill);
    // Selection is the only lacquer-ringed state; hover keeps the normal wood edge.
    let (thick, ring) = if selected {
        (line::STRONG, theme::color(theme::LACQUER_500))
    } else {
        (line::BASE, EDGE)
    };
    rrect_lines(r, radius::MD, thick, ring);
    if let Some(item) = item {
        let tint = if unavailable { theme::alpha(theme::WHITE, 0.4) } else { WHITE };
        draw_item_tint(a, r.center(), r.w * 0.7, item, tint);
    }
    if let Some(n) = count {
        let s = n.to_string();
        let x = r.right() - text_w(a, &s, TextSize::S) - 4.0;
        let y = r.bottom() - 22.0;
        text(a, &s, x, y, TextSize::S, if unavailable { TEXT_DIM } else { TEXT });
    }
    if selected {
        // Small corner tick that survives even when the icon is bright.
        let p = vec2(r.right() - 3.0, r.y + 3.0);
        draw_triangle(p, p + vec2(-7.0, 0.0), p + vec2(0.0, 7.0), KEY);
    }
    clicked(r)
}

/// Tooltip text for a recipe, with what the player has of each ingredient.
fn recipe_lines(world: &World, r: usize) -> Vec<String> {
    let rec = &RECIPES[r];
    let mut lines = vec![rec.name.to_string()];
    if let Some((f, n)) = rec.fluid_in {
        lines.push(format!("{n} {}", f.name()));
    }
    for &(i, n) in rec.inputs {
        lines.push(format!("{n} {} · Have {}", i.name(), world.player.inv.get(i)));
    }
    let outs: Vec<String> = rec
        .outputs
        .iter()
        .map(|(i, n)| format!("{n} {}", i.name()))
        .chain(rec.fluid_out.map(|(f, n)| format!("{n} {}", f.name())))
        .collect();
    lines.push(format!("Makes {} in {}s", outs.join(" + "), rec.time));
    lines
}

fn recipe_icon(a: &Assets, r: Rect, recipe: usize, selected: bool, unavailable: bool) {
    let rec = &RECIPES[recipe];
    slot(a, r, rec.main_output(), None, selected, unavailable);
    if rec.main_output().is_none()
        && let Some((f, _)) = rec.fluid_out
    {
        let tint = if unavailable { theme::alpha(theme::WHITE, 0.4) } else { f.color() };
        circle(r.center(), r.w * 0.26, tint);
        circle_lines(r.center(), r.w * 0.26, line::STRONG, TEXT);
    }
}

// ------------------------------------------------------------------ minimap

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
                self.img.set_pixel(x as u32, y as u32, theme::grade(col));
            }
        }
        for en in &world.enemies {
            let p = en.pos.floor().as_ivec2();
            if in_bounds(p) {
                self.img.set_pixel(p.x as u32, p.y as u32, theme::grade(theme::color(theme::MAP_ENEMY)));
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

// ------------------------------------------------------------------ frame

pub fn draw(g: &mut Game) {
    g.ui_rects.clear();
    g.tooltip.clear();
    set_camera(&ui_camera(ui_width(), ui_height()));

    hud(g);
    quickbar(g);
    craft_queue(g);
    minimap(g);
    match g.panel {
        OpenPanel::Inventory => inventory_panel(g),
        OpenPanel::Research => research_panel(g),
        OpenPanel::Building(id, _) => building_panel(g, id),
        OpenPanel::None => {}
    }
    if g.tooltip.is_empty() && !g.mouse_over_ui() && g.tool.is_none() {
        g.tooltip = g.world.describe(g.mouse_tile());
    }
    let a = g.assets.clone();
    radio(g, &a);
    toasts(g, &a);
    if g.show_help {
        help(&a);
    }
    overlays(g, &a);
    draw_tooltip(&a, &g.tooltip);
    set_default_camera();
}

fn hud(g: &mut Game) {
    let a = g.assets.clone();
    let t = g.time;
    let w = &g.world;
    let r = Rect::new(space::L, space::L, 306.0, 112.0);
    surface(r, Elevation::Panel);
    a.draw_centered("icon_heart", vec2(r.x + 26.0, r.y + 26.0), Vec2::splat(24.0), 0.0, WHITE);
    let hp_frac = w.player.hp.max(0.0) / PLAYER_HP;
    bar(Rect::new(r.x + 44.0, r.y + 18.0, 244.0, 14.0), hp_frac, TERRA);
    if hp_frac > 0.0 && hp_frac < 0.3 {
        let p = 0.5 + 0.5 * (t * 5.0).sin();
        rrect_lines(
            Rect::new(r.x + 42.0, r.y + 16.0, 248.0, 18.0),
            9.0,
            line::STRONG,
            theme::alpha(theme::TERRA_400, 0.3 + 0.5 * p),
        );
    }
    a.draw_centered("icon_flask", vec2(r.x + 26.0, r.y + 58.0), Vec2::splat(24.0), 0.0, WHITE);
    match w.research.current {
        Some(t) => {
            let tech = &TECHS[t];
            text(&a, tech.name, r.x + 44.0, r.y + 46.0, TextSize::M, TEXT);
            bar(Rect::new(r.x + 44.0, r.y + 66.0, 244.0, 8.0), w.research.progress as f32 / tech.units as f32, TEAL);
        }
        None => text(&a, "Pick a research · press T", r.x + 44.0, r.y + 51.0, TextSize::M, KEY),
    }
    let sky = sky(w.time);
    let threat = if w.peaceful { "Peaceful".to_string() } else { format!("Hives {:.0}%", w.evolution * 100.0) };
    text(&a, &format!("{} · {threat}", sky.label), r.x + space::L, r.y + 88.0, TextSize::S, TEXT_DIM);
    g.ui_rects.push(r);
}

fn quickbar_rects() -> Vec<Rect> {
    let total = 10.0 * QUICK_SLOT + 9.0 * GAP;
    let x0 = (ui_width() - total) / 2.0;
    let y = ui_height() - QUICK_SLOT - space::L;
    (0..10).map(|k| Rect::new(x0 + k as f32 * (QUICK_SLOT + GAP), y, QUICK_SLOT, QUICK_SLOT)).collect()
}

fn quickbar(g: &mut Game) {
    let a = g.assets.clone();
    let rects = quickbar_rects();
    let back = Rect::new(
        rects[0].x - space::S,
        rects[0].y - space::S,
        rects[9].right() - rects[0].x + 2.0 * space::S,
        QUICK_SLOT + 2.0 * space::S,
    );
    surface(back, Elevation::Panel);
    g.ui_rects.push(back);
    for (k, r) in rects.into_iter().enumerate() {
        let item = g.world.player.quickbar[k];
        let count = item.map(|i| g.world.player.inv.get(i));
        let selected = item.is_some() && g.tool == item;
        slot(&a, r, item, count, selected, count == Some(0));
        text(&a, &((k + 1) % 10).to_string(), r.x + 5.0, r.y + 3.0, TextSize::Xs, TEXT_DIM);
        if hovered(r)
            && let Some(i) = item
        {
            g.tooltip = vec![i.name().to_string(), format!("Key {}", (k + 1) % 10)];
        }
        if clicked(r) {
            g.select_slot(k);
        }
    }
}

fn craft_queue(g: &mut Game) {
    if g.world.player.queue.is_empty() {
        return;
    }
    let a = g.assets.clone();
    let y = ui_height() - QUICK_SLOT - space::L - SLOT - 28.0;
    let total = 10.0 * QUICK_SLOT + 9.0 * GAP;
    let x0 = (ui_width() - total) / 2.0;
    text(&a, "Crafting · click to cancel", x0, y - 20.0, TextSize::S, TEXT_DIM);
    let mut cancel = None;
    for (k, c) in g.world.player.queue.iter().take(12).enumerate() {
        let r = Rect::new(x0 + k as f32 * (SLOT + GAP), y, SLOT, SLOT);
        let rec = &RECIPES[c.recipe];
        slot(&a, r, rec.main_output(), None, k == 0, false);
        if k == 0 {
            bar(Rect::new(r.x + 5.0, r.bottom() - 8.0, r.w - 10.0, 4.0), c.progress / rec.time, HONEY);
        }
        if hovered(r) {
            g.tooltip = vec![rec.name.to_string()];
        }
        if clicked(r) {
            cancel = Some(k);
        }
        g.ui_rects.push(r);
    }
    if g.world.player.queue.len() > 12 {
        let more = format!("+{}", g.world.player.queue.len() - 12);
        text(&a, &more, x0 + 12.0 * (SLOT + GAP), y + 13.0, TextSize::L, TEXT);
    }
    if let Some(k) = cancel {
        g.world.cancel_craft(k);
    }
}

fn minimap(g: &mut Game) {
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

fn centered(w: f32, h: f32) -> Rect {
    let (sw, sh) = (ui_width(), ui_height());
    let w = w.min(sw - 2.0 * space::L);
    let h = h.min(sh - 2.0 * space::L);
    Rect::new(((sw - w) / 2.0).max(space::L), ((sh - h) / 2.0).max(space::L), w, h)
}

const INV_W: f32 = 1020.0;
const INV_H: f32 = 580.0;
const INV_COLS: usize = 9;

fn grid_rect(origin: Vec2, k: usize, cols: usize, size: f32) -> Rect {
    let (col, row) = (k % cols, k / cols);
    Rect::new(origin.x + col as f32 * (size + GAP), origin.y + row as f32 * (size + GAP), size, size)
}

fn inv_slots(g: &Game) -> Vec<(Item, Rect)> {
    let r = centered(INV_W, INV_H);
    let origin = vec2(r.x + space::XL, r.y + HEADER - slide_px(g));
    g.world
        .player
        .inv
        .iter()
        .enumerate()
        .map(|(k, (item, _))| (item, grid_rect(origin, k, INV_COLS, SLOT)))
        .collect()
}

/// The inventory item under the mouse, when the inventory is open.
pub fn hovered_inventory_item(g: &Game) -> Option<Item> {
    if g.panel != OpenPanel::Inventory {
        return None;
    }
    inv_slots(g).into_iter().find(|(_, r)| hovered(*r)).map(|(i, _)| i)
}

fn inventory_panel(g: &mut Game) {
    let a = g.assets.clone();
    let r = centered(INV_W, INV_H);
    let r = Rect::new(r.x, r.y - slide_px(g), r.w, r.h);
    let (content, closed) = Panel::new(r)
        .title("Backpack")
        .subtitle("Click an item to hold it")
        .closable()
        .show(g);
    if closed {
        g.panel = OpenPanel::None;
        return;
    }
    for (item, sr) in inv_slots(g) {
        let n = g.world.player.inv.get(item);
        slot(&a, sr, Some(item), Some(n), g.tool == Some(item), false);
        if hovered(sr) {
            let mut tip = vec![item.name().to_string(), format!("{n} carried")];
            if item.place_kind().is_some() {
                tip.push("Click to hold".into());
            }
            tip.push("Press 1–0 to put it on the quickbar".into());
            g.tooltip = tip;
        }
        if clicked(sr) && item.place_kind().is_some() {
            g.tool = Some(item);
            g.panel = OpenPanel::None;
        }
    }

    let cx = content.x + INV_COLS as f32 * (SLOT + GAP) + space::XL;
    draw_line(cx - space::M, content.y, cx - space::M, r.bottom() - 44.0, line::HAIR, EDGE);
    text(&a, "Workbench", cx, content.y, TextSize::Xl, TITLE);
    text(&a, "Shift-click to craft five", content.right() - 200.0, content.y + 8.0, TextSize::Xs, TEXT_DIM);
    let origin = vec2(cx, content.y + 38.0);
    let recipes: Vec<usize> = (0..RECIPES.len()).filter(|&id| RECIPES[id].hand && g.world.recipe_unlocked(id)).collect();
    let shift = is_key_down(KeyCode::LeftShift) || is_key_down(KeyCode::RightShift);
    let cols = ((content.right() - cx + GAP) / (SLOT + GAP)).floor().max(1.0) as usize;
    for (k, &id) in recipes.iter().enumerate() {
        let sr = grid_rect(origin, k, cols.min(8), SLOT);
        let can = g.world.can_craft(id);
        recipe_icon(&a, sr, id, false, !can);
        if hovered(sr) {
            let mut tip = recipe_lines(&g.world, id);
            tip.push("Click · craft 1   Shift-click · craft 5".into());
            g.tooltip = tip;
        }
        if clicked(sr)
            && let Err(e) = g.world.queue_craft(id, if shift { 5 } else { 1 })
        {
            g.toast(e);
        }
    }
    let foot = "Missing parts are crafted for you · machine-only recipes are set inside Assemblers.";
    text(&a, foot, content.x, r.bottom() - 26.0, TextSize::S, TEXT_DIM);
}

#[derive(Clone, Copy, Debug)]
struct ResearchLayout {
    width: f32,
    height: f32,
    cols: usize,
    card_w: f32,
    card_h: f32,
    grid_h: f32,
    view_h: f32,
    max_scroll: f32,
}

/// Research panel geometry for a logical screen size. Columns come from the
/// available width, and any overflow becomes a scroll range so no card is lost.
fn research_layout(sw: f32, sh: f32) -> ResearchLayout {
    let card_h = 106.0;
    let min_card_w = 300.0;
    let width = (sw - 2.0 * space::L).min(1100.0);
    let inner_w = width - 2.0 * space::XL;
    let cols = (((inner_w + GAP) / (min_card_w + GAP)).floor() as usize).max(1);
    let rows = TECHS.len().div_ceil(cols);
    let grid_h = rows as f32 * (card_h + GAP) - GAP;
    let want_h = HEADER + grid_h + 2.0 * space::L;
    let height = want_h.min(sh - 2.0 * space::L);
    let view_h = ((height - HEADER - 2.0 * space::L) - 8.0).max(1.0);
    let card_w = (inner_w - (cols as f32 - 1.0) * GAP) / cols as f32;
    ResearchLayout {
        width,
        height,
        cols,
        card_w,
        card_h,
        grid_h,
        view_h,
        max_scroll: (grid_h - view_h).max(0.0),
    }
}

fn research_panel(g: &mut Game) {
    let a = g.assets.clone();
    let layout = research_layout(ui_width(), ui_height());
    let (cols, card_w, card_h, grid_h, max_scroll) =
        (layout.cols, layout.card_w, layout.card_h, layout.grid_h, layout.max_scroll);
    let r = centered(layout.width, layout.height);    let r = Rect::new(r.x, r.y - slide_px(g), r.w, r.h);
    let (content, closed) = Panel::new(r)
        .title("Research")
        .subtitle("Labs slowly use up the data vials shown · pick something to study")
        .closable()
        .show(g);
    if closed {
        g.panel = OpenPanel::None;
        return;
    }

    let view = Rect::new(content.x, content.y, content.w, layout.view_h);
    if hovered(view) && max_scroll > 0.0 {
        g.research_scroll = (g.research_scroll - mouse_wheel().1 * 44.0).clamp(0.0, max_scroll);
    } else {
        g.research_scroll = g.research_scroll.clamp(0.0, max_scroll);
    }
    let scroll = g.research_scroll;

    clip(view);
    for (k, tech) in TECHS.iter().enumerate() {
        let col = k % cols;
        let row = k / cols;
        let cr = Rect::new(
            content.x + col as f32 * (card_w + GAP),
            content.y + row as f32 * (card_h + GAP) - scroll,
            card_w,
            card_h,
        );
        if cr.bottom() < view.y - 100.0 || cr.y > view.bottom() + 100.0 {
            continue;
        }
        let done = g.world.research.done[k];
        let current = g.world.research.current == Some(k);
        let available = g.world.tech_available(k);
        let hot = available && hovered(cr);
        let tint = if done {
            Some(theme::alpha(theme::SAGE_700, 0.22))
        } else if current {
            Some(theme::alpha(theme::LACQUER_500, 0.16))
        } else if hot {
            Some(theme::alpha(theme::WHITE, 0.5))
        } else {
            None
        };
        surface(cr, Elevation::Card);
        if let Some(t) = tint {
            rrect(cr, radius::MD, t);
        }
        rrect_lines(
            cr,
            radius::MD,
            if current { line::STRONG } else { line::BASE },
            EDGE,
        );
        let name_col = if done {
            theme::color(theme::SAGE_700)
        } else if current {
            TITLE
        } else if available {
            TEXT
        } else {
            TEXT_DIM
        };
        text(&a, tech.name, cr.x + space::L, cr.y + space::M, TextSize::L, name_col);
        if done {
            text_right(&a, "Done", cr.right() - space::L, cr.y + space::M + 2.0, TextSize::S, theme::color(theme::SAGE_700));
        }
        let mut x = cr.x + space::L;
        for &s in tech.science {
            draw_item(&a, vec2(x + 10.0, cr.y + 44.0), 22.0, s);
            x += 22.0;
        }
        text(&a, &format!("x{} · {}s each", tech.units, tech.time), x + space::S, cr.y + 40.0, TextSize::S, TEXT_DIM);
        let unlocks = match tech.bonus {
            Some(Bonus::ArmSpeed) => "Arms move 50% faster".to_string(),
            Some(Bonus::DrillSpeed) => "Drills mine 50% faster".to_string(),
            None => tech.recipes.join(", "),
        };
        text(&a, &unlocks, cr.x + space::L, cr.y + 70.0, TextSize::S, TEAL);
        if !done && !available {
            let req: Vec<&str> = tech.prereqs.iter().map(|&p| TECHS[p].name).collect();
            text(&a, &format!("Needs {}", req.join(", ")), cr.x + space::L, cr.y + 90.0, TextSize::Xs, PEACH);
        }
        if current {
            bar(
                Rect::new(cr.x + space::L, cr.y + 88.0, cr.w - 2.0 * space::L, 8.0),
                g.world.research.progress as f32 / tech.units as f32,
                HONEY,
            );
        }
        if available && clicked(cr) {
            g.world.set_research(k);
            g.toast(format!("Researching {}", tech.name));
        }
    }
    unclip();

    if max_scroll > 0.0 {
        let track = Rect::new(view.right() + 3.0, view.y, 3.0, view.h);
        rrect(track, 1.5, theme::alpha(theme::EDGE, 0.5));
        let frac = view.h / grid_h;
        let thumb_h = (view.h * frac).max(24.0);
        let thumb_y = view.y + (view.h - thumb_h) * (scroll / max_scroll);
        rrect(Rect::new(track.x, thumb_y, track.w, thumb_h), 1.5, theme::alpha(theme::HONEY, 0.8));
    }
}

fn machine_cols(w: f32) -> usize {
    ((w + GAP) / (SLOT + GAP)).floor().max(1.0) as usize
}

fn building_body_height(g: &Game, id: usize, body_w: f32) -> f32 {
    let e = g.world.entity(id);
    match &e.state {
        State::Crafter(c) => {
            let recipes = g.world.machine_recipes(e.kind);
            let cols = machine_cols(body_w);
            let mut h = space::XL + recipes.len().div_ceil(cols) as f32 * (SLOT + GAP) + space::M;
            if let Some(rid) = c.recipe {
                let rec = &RECIPES[rid];
                h += 22.0 + 12.0 + 12.0;
                if rec.fluid_in.is_some() {
                    h += 24.0;
                }
                h += rec.inputs.len() as f32 * 24.0;
                h += c.out.iter().count() as f32 * 24.0;
                if rec.fluid_out.is_some() {
                    h += 24.0;
                }
                if rec.fluid_in.is_some() || rec.fluid_out.is_some() {
                    h += 22.0;
                }
            } else {
                h += 26.0;
            }
            h
        }
        State::Lab { input, .. } => 24.0 + 12.0 + 14.0 + input.iter().count() as f32 * 24.0 + space::M,
        State::Generator { fuel, .. } => 24.0 + 14.0 + fuel.iter().count() as f32 * 24.0 + 26.0,
        State::Chest { stock } => {
            let cols = machine_cols(body_w);
            let rows = stock.iter().count().div_ceil(cols).max(1);
            24.0 + 14.0 + rows as f32 * (SLOT + GAP) + space::M
        }
        State::Sentry { ammo, .. } => 24.0 + 14.0 + ammo.iter().count() as f32 * 24.0 + space::M,
        State::Engine { .. } => 24.0 + 14.0 + 12.0 + 14.0,
        State::Drill { .. } => g.world.describe(e.pos).len().saturating_sub(1) as f32 * 24.0 + space::S,
        _ => g.world.describe(e.pos).len().saturating_sub(1) as f32 * 24.0 + space::S,
    }
}

fn building_panel(g: &mut Game, id: usize) {
    let a = g.assets.clone();
    let width = 390.0_f32.min(ui_width() - 2.0 * space::L);
    let body_w = width - 2.0 * space::XL;
    let body_h = building_body_height(g, id, body_w);
    let height = (HEADER + 34.0 + body_h + 64.0).max(220.0).min(ui_height() - 2.0 * space::L);
    let x = (ui_width() - width - space::XL).max(space::L);
    let y = (space::L + MINIMAP + space::XL + space::M).min((ui_height() - height - space::L).max(space::L));
    let r = Rect::new(x, y - slide_px(g), width, height);
    let e = g.world.entity(id).clone();
    let (content, closed) = Panel::new(r).title(e.kind.name()).closable().show(g);
    if closed {
        g.panel = OpenPanel::None;
        return;
    }
    let x = content.x;
    let mut y = content.y;
    let btn_y = r.bottom() - space::L - 36.0;
    let half = (content.w - space::S) / 2.0;
    let left_btn = Rect::new(x, btn_y, half, 36.0);
    let right_btn = Rect::new(x + half + space::S, btn_y, half, 36.0);

    bar(Rect::new(x, y - 4.0, 90.0, 8.0), e.hp / e.kind.max_hp(), theme::color(theme::SAGE_500));
    if e.kind.draw_kw() > 0.0 {
        let (p, col) = match e.net.and_then(|n| g.world.power.nets.get(n)) {
            None => ("No power · put a pole nearby".to_string(), PEACH),
            Some(n) => (format!("Power {:.0}% · {:.0}/{:.0} kW", n.sat * 100.0, n.demand.min(n.supply), n.supply), TEXT_DIM),
        };
        text(&a, &p, x + 102.0, y - 6.0, TextSize::S, col);
    }
    y += 22.0;

    match &e.state {
        State::Crafter(c) => {
            text(&a, "Recipe", x, y, TextSize::M, TEXT);
            y += 14.0;
            let recipes = g.world.machine_recipes(e.kind);
            let cols = machine_cols(content.w);
            for (k, &rid) in recipes.iter().enumerate() {
                let sr = grid_rect(vec2(x, y), k, cols, SLOT);
                recipe_icon(&a, sr, rid, c.recipe == Some(rid), false);
                if hovered(sr) {
                    g.tooltip = recipe_lines(&g.world, rid);
                }
                if clicked(sr) {
                    g.world.set_recipe(id, rid);
                }
            }
            y += recipes.len().div_ceil(cols) as f32 * (SLOT + GAP) + space::M;
            if let Some(rid) = c.recipe {
                let rec = &RECIPES[rid];
                text(&a, &format!("Making {}", rec.name), x, y, TextSize::M, TEAL);
                y += 22.0;
                bar(Rect::new(x, y, content.w, 10.0), c.progress, HONEY);
                y += 22.0;
                if let Some((f, n)) = rec.fluid_in {
                    text(&a, &format!("{} · {:.0}/{n}", f.name(), c.fluid_in), x, y, TextSize::S, TEXT_DIM);
                    y += 24.0;
                }
                for &(i, n) in rec.inputs {
                    draw_item(&a, vec2(x + 10.0, y + 2.0), 20.0, i);
                    text(&a, &format!("{} · {}/{n}", i.name(), c.input.get(i)), x + 26.0, y, TextSize::S, TEXT_DIM);
                    y += 24.0;
                }
                for (i, n) in c.out.iter() {
                    draw_item(&a, vec2(x + 10.0, y + 2.0), 20.0, i);
                    text(&a, &format!("Ready · {} x{n}", i.name()), x + 26.0, y, TextSize::S, TEXT);
                    y += 24.0;
                }
                if let Some((f, _)) = rec.fluid_out {
                    text(&a, &format!("Ready · {} {:.0}", f.name(), c.fluid_out), x, y, TextSize::S, TEXT);
                    y += 24.0;
                }
                if rec.fluid_in.is_some() || rec.fluid_out.is_some() {
                    text(&a, "Pipes join at the teal in and peach out valves", x, y, TextSize::Xs, TEXT_DIM);
                }
            } else if e.kind == Kind::Smelter {
                text(&a, "Picks its recipe from the first ore it gets", x, y, TextSize::S, TEXT_DIM);
            }
            if primary_button(&a, left_btn, "Insert ingredients", c.recipe.is_some()) {
                let n = g.world.insert_from_player(id);
                g.toast(format!("Inserted {n} items"));
            }
            if button(&a, right_btn, "Take output", c.out.total() > 0) {
                g.world.take_to_player(id);
            }
        }
        State::Lab { input, progress } => {
            match g.world.research.current {
                Some(t) => text(&a, &format!("Studying {}", TECHS[t].name), x, y, TextSize::M, TEAL),
                None => {
                    let lead = "Nothing to study · press ";
                    text(&a, lead, x, y, TextSize::M, PEACH);
                    text(&a, "T", x + text_w(&a, lead, TextSize::M), y, TextSize::M, KEY);
                }
            }
            y += 24.0;
            bar(Rect::new(x, y, content.w, 10.0), *progress, TEAL);
            y += 24.0;
            stock_lines(&a, input, x, &mut y);
            if primary_button(&a, left_btn, "Insert data", true) {
                let n = g.world.insert_from_player(id);
                g.toast(format!("Inserted {n} items"));
            }
        }
        State::Generator { fuel, load, .. } => {
            text(&a, &format!("Working at {:.0}% of 600 kW", load * 100.0), x, y, TextSize::M, TEXT);
            y += 26.0;
            stock_lines(&a, fuel, x, &mut y);
            text(&a, "Burns coal or fuel cells", x, y + 4.0, TextSize::S, TEXT_DIM);
            if primary_button(&a, left_btn, "Add fuel", true) {
                let n = g.world.insert_from_player(id);
                g.toast(format!("Inserted {n} items"));
            }
        }
        State::Chest { stock } => {
            text(&a, &format!("{}/{CHEST_CAP} stored", stock.total()), x, y, TextSize::M, TEXT);
            y += 26.0;
            let cols = machine_cols(content.w);
            let mut take = None;
            for (k, (item, n)) in stock.iter().enumerate() {
                let sr = grid_rect(vec2(x, y), k, cols, SLOT);
                slot(&a, sr, Some(item), Some(n), false, false);
                if hovered(sr) {
                    g.tooltip = vec![item.name().to_string(), "Click to take".into()];
                }
                if clicked(sr) {
                    take = Some((item, n));
                }
            }
            if let Some((item, n)) = take
                && let State::Chest { stock } = &mut g.world.entity_mut(id).state
            {
                stock.take(item, n);
                g.world.player.inv.add(item, n);
            }
            if primary_button(&a, left_btn, "Store items", true) {
                let n = g.world.insert_from_player(id);
                g.toast(format!("Stored {n} items"));
            }
            if button(&a, right_btn, "Take all", stock.total() > 0) {
                g.world.take_to_player(id);
            }
        }
        State::Sentry { ammo, shots, .. } => {
            text(&a, &format!("{shots} shots loaded"), x, y, TextSize::M, TEXT);
            y += 26.0;
            stock_lines(&a, ammo, x, &mut y);
            if primary_button(&a, left_btn, "Load ammo", true) {
                let n = g.world.insert_from_player(id);
                g.toast(format!("Inserted {n} items"));
            }
        }
        State::Engine { cores } => {
            text(&a, &format!("Aether Cores · {cores}/{ENGINE_CORES}"), x, y, TextSize::M, TEXT);
            y += 26.0;
            bar(
                Rect::new(x, y, content.w, 12.0),
                *cores as f32 / ENGINE_CORES as f32,
                theme::color(theme::AETHER_HI),
            );
            if primary_button(&a, left_btn, "Feed cores", true) {
                g.world.insert_from_player(id);
            }
        }
        State::Drill { out, .. } => {
            for line in g.world.describe(e.pos).iter().skip(1) {
                text(&a, line, x, y, TextSize::S, TEXT_DIM);
                y += 24.0;
            }
            if primary_button(&a, left_btn, "Take ore", !out.is_empty()) {
                g.world.take_to_player(id);
            }
        }
        _ => {
            for line in g.world.describe(e.pos).iter().skip(1) {
                text(&a, line, x, y, TextSize::S, TEXT_DIM);
                y += 24.0;
            }
        }
    }
}

fn stock_lines(a: &Assets, stock: &Stock, x: f32, y: &mut f32) {
    for (i, n) in stock.iter() {
        draw_item(a, vec2(x + 10.0, *y + 2.0), 20.0, i);
        text(a, &format!("{} x{n}", i.name()), x + 26.0, *y, TextSize::S, TEXT_DIM);
        *y += 24.0;
    }
}

fn radio(g: &Game, a: &Assets) {
    let (prefix, suffix) = if g.music_on { ("Lo-fi radio · ", " to mute") } else { ("Radio off · ", " to play") };
    let w = text_w(a, prefix, TextSize::S) + text_w(a, "M", TextSize::S) + text_w(a, suffix, TextSize::S);
    let x = ui_width() - w - 46.0;
    let total = 10.0 * QUICK_SLOT + 9.0 * GAP;
    let qx = (ui_width() - total) / 2.0;
    // On narrow screens the quickbar reaches the bottom-right corner; lift the
    // radio above it instead of letting the two panels overlap.
    let overlap = x + w > qx && x < qx + total;
    let y = if overlap { ui_height() - QUICK_SLOT - space::L - 26.0 } else { ui_height() - 30.0 };
    // A small paper pill keeps the label readable over the grass.
    surface(Rect::new(x - 36.0, y - 20.0, w + 50.0, 44.0), Elevation::Toast);
    let bob = if g.music_on { (g.time * 2.4).sin() * 2.0 } else { 0.0 };
    let tint = if g.music_on { WHITE } else { theme::alpha(theme::WHITE, 0.4) };
    a.draw_centered("icon_note", vec2(x - 14.0, y - 5.0 + bob), Vec2::splat(20.0), 0.0, tint);
    text(a, prefix, x, y, TextSize::S, TEXT_DIM);
    let mx = x + text_w(a, prefix, TextSize::S);
    text(a, "M", mx, y, TextSize::S, KEY);
    text(a, suffix, mx + text_w(a, "M", TextSize::S), y, TextSize::S, TEXT_DIM);
}

fn toasts(g: &Game, a: &Assets) {
    for (k, (msg, t)) in g.toasts.iter().enumerate() {
        let alpha = t.min(1.0);
        let age = 3.0 - t;
        let slide = if age < 0.25 { (1.0 - age / 0.25) * -22.0 } else { 0.0 };
        let w = text_w(a, msg, TextSize::L);
        let x = (ui_width() - w) / 2.0;
        let y = 140.0 + k as f32 * 42.0 + slide;
        let r = Rect::new(x - 18.0, y - 6.0, w + 36.0, 38.0);
        surface_alpha(r, Elevation::Toast, alpha);
        rrect(
            Rect::new(r.x, r.y + 9.0, 4.0, r.h - 18.0),
            2.0,
            theme::alpha(theme::EDGE, alpha),
        );
        text(a, msg, x, y + 4.0, TextSize::L, theme::alpha(theme::TEXT, alpha));
    }
}

fn help(a: &Assets) {
    let lines = [
        ("Welcome to Cogworks", TextSize::Xl, TITLE),
        ("", TextSize::M, TEXT),
        ("WASD · walk          Wheel · zoom          E · backpack & workbench          T · research", TextSize::M, TEXT),
        ("1–0 · quickbar      R · rotate      Q · copy building or empty hands      Esc · close", TextSize::M, TEXT),
        ("Left click · build (drag belts, pipes, walls) or open a building", TextSize::M, TEXT),
        ("Right hold · mine ore by hand, or pick a building back up", TextSize::M, TEXT),
        ("F5 · save   F9 · load   O · pollution   P · peaceful   M · music   Space · pause", TextSize::M, TEXT),
        ("", TextSize::M, TEXT),
        ("Arms carry things from behind them to the front. Drills drop ore at their chute.", TextSize::M, TEXT),
        ("Belts have two lanes; feeding from the side fills the near one.", TextSize::M, TEXT),
        ("Machines need power: burn coal in a Generator, spread it with lamp poles.", TextSize::M, TEXT),
        ("Smoke drifts out to the hives at the map's edge, and they come to visit.", TextSize::M, TEXT),
        ("Fill Labs with data, research upward, and wake the Grand Engine.", TextSize::M, TEXT),
        ("", TextSize::M, TEXT),
        ("H · hide this note", TextSize::S, TEXT_DIM),
    ];
    let w = lines.iter().map(|(l, s, _)| text_w(a, l, *s)).fold(0.0, f32::max) + 2.0 * space::XL;
    let h = lines.len() as f32 * LINE + 2.0 * space::L;
    let r = Rect::new(space::L, 142.0, w, h);
    surface(r, Elevation::Panel);
    for (k, (l, size, col)) in lines.iter().enumerate() {
        text_keys(a, l, r.x + space::XL, r.y + space::L + k as f32 * LINE, *size, *col);
    }
}

fn overlays(g: &Game, a: &Assets) {
    let banner = |msg: &str, col: Color, y: f32| {
        let w = text_w(a, msg, TextSize::Xxl);
        let x = (ui_width() - w) / 2.0;
        let r = Rect::new(x - 28.0, y, w + 56.0, 54.0);
        surface(r, Elevation::Banner);
        text(a, msg, x, y + 12.0, TextSize::Xxl, col);
    };
    if !g.world.player.alive() {
        let msg = format!("Cog took a tumble · back in {:.0}…", g.world.player.respawn.ceil());
        banner(&msg, PEACH, ui_height() / 2.0 - 40.0);
    }
    if g.world.won {
        banner("The Grand Engine hums awake · you did it", TITLE, 92.0);
    }
    if g.paused {
        banner("Paused", TEXT, ui_height() / 2.0 - 110.0);
    }
}

fn draw_tooltip(a: &Assets, lines: &[String]) {
    if lines.is_empty() {
        return;
    }
    let width = lines.iter().map(|l| text_w(a, l, TextSize::M)).fold(0.0, f32::max);
    let m = ui_mouse();
    let h = lines.len() as f32 * LINE + 16.0;
    let x = (m.x + 18.0).min(ui_width() - width - 34.0).max(space::S);
    let y = (m.y + 18.0).min(ui_height() - h - space::S).max(space::S);
    let r = Rect::new(x, y, width + 28.0, h);
    surface(r, Elevation::Tooltip);
    for (k, l) in lines.iter().enumerate() {
        let col = if k == 0 { TITLE } else { TEXT };
        text(a, l, x + 14.0, y + 8.0 + k as f32 * LINE, TextSize::M, col);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn raster_params_render_at_physical_size() {
        assert_eq!(raster_params(17.0, 1.6), (27, 0.625));
        assert_eq!(raster_params(17.0, 1.0), (17, 1.0));
        assert_eq!(raster_params(13.0, 0.8).0, 10);
    }

    #[test]
    fn ui_scale_stays_readable() {
        assert!((scale_for_height(900.0) - 1.0).abs() < 1e-6);
        assert!((scale_for_height(768.0) - 768.0 / 900.0).abs() < 1e-6);
        assert_eq!(scale_for_height(600.0), 0.8);
        assert_eq!(scale_for_height(2160.0), 1.6);
    }

    #[test]
    fn ui_camera_is_y_down() {
        let cam = ui_camera(1000.0, 500.0);
        let m = cam.matrix();
        let origin = m.project_point3(vec3(0.0, 0.0, 0.0));
        let corner = m.project_point3(vec3(1000.0, 500.0, 0.0));
        let close = |a: f32, b: f32| (a - b).abs() < 1e-4;
        assert!(close(origin.x, -1.0) && close(origin.y, 1.0), "origin projected to {origin:?}");
        assert!(close(corner.x, 1.0) && close(corner.y, -1.0), "corner projected to {corner:?}");
    }

    #[test]
    fn research_grid_never_loses_a_card() {
        for (w, h) in [
            (1440.0, 900.0),
            (1600.0, 900.0),
            (1920.0, 1080.0),
            (1280.0, 750.0),
            (1000.0, 750.0),
            (700.0, 750.0),
        ] {
            let l = research_layout(w, h);
            assert!(l.width <= w - 2.0 * space::L + 0.01);
            assert!(l.height <= h - 2.0 * space::L + 0.01);
            assert!(l.cols >= 1);
            assert!(l.cols * TECHS.len().div_ceil(l.cols) >= TECHS.len());
            assert!(
                l.grid_h - l.max_scroll <= l.view_h + 0.01,
                "{w}x{h}: grid {} scroll {} view {}",
                l.grid_h,
                l.max_scroll,
                l.view_h
            );
        }
    }
}

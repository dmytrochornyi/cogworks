//! The full-window world map: the minimap's raster drawn large, with the
//! player, the camera's view rectangle, hives and enemies over it, plus a
//! legend. Hovering the image shows the same tile tooltip as the world.

use macroquad::prelude::*;

use crate::assets::Assets;
use crate::items::Item;
use crate::theme::{self, TextSize, line, radius, space};
use crate::world::*;
use crate::{Game, Panel as OpenPanel};

use super::super::metrics::{clip, ui_height, ui_mouse, ui_width, unclip};
use super::super::minimap::map_player_and_view;
use super::super::primitives::{circle, rrect, rrect_lines};
use super::super::style::{EDGE, HEADER, TEXT};
use super::super::text::text;
use super::super::tooltip::Tip;
use super::super::widgets::Panel;
use super::slide_px;

/// Width of the fixed legend column, wide enough for its longest row.
const LEGEND_W: f32 = 220.0;
/// Vertical pitch of one legend row.
const ROW_H: f32 = 22.0;
/// Side of a legend colour swatch.
const SWATCH: f32 = 14.0;

/// Where the map panel's pieces sit for a logical screen size. All rects are
/// in logical UI coordinates and are computed purely so they can be tested.
#[derive(Clone, Copy, Debug)]
struct MapLayout {
    panel: Rect,
    map: Rect,
    legend: Rect,
}

/// The content area inside a titled panel, mirroring `Panel::show`.
fn map_content(panel: Rect) -> Rect {
    let top = panel.y + HEADER;
    Rect::new(panel.x + space::XL, top, panel.w - 2.0 * space::XL, (panel.bottom() - space::L - top).max(0.0))
}

/// The largest square map that fits the window: `space::L` margins all round,
/// the titled panel's `HEADER` and padding (`space::XL` sides, `space::L`
/// bottom), plus the fixed legend column and its gap. The panel is then sized
/// to wrap exactly the map and legend and centred in the window. Computed from
/// the given size only, so it can be unit-tested without a window.
fn map_layout(sw: f32, sh: f32) -> MapLayout {
    // Height available to the map inside the panel's content area, and width
    // after the fixed legend column and its gap are taken out of the content.
    let max_h = (sh - 2.0 * space::L - HEADER - space::L).max(0.0);
    let max_w = (sw - 2.0 * space::L - 2.0 * space::XL - LEGEND_W - space::L).max(0.0);
    let map_size = max_h.min(max_w);
    let panel = Rect::new(
        ((sw - (2.0 * space::XL + map_size + space::L + LEGEND_W)) / 2.0).max(space::L),
        ((sh - (HEADER + map_size + space::L)) / 2.0).max(space::L),
        2.0 * space::XL + map_size + space::L + LEGEND_W,
        HEADER + map_size + space::L,
    );
    let content = map_content(panel);
    let map = Rect::new(content.x, content.y, map_size, map_size);
    let legend = Rect::new(map.right() + space::L, content.y, LEGEND_W, content.h);
    MapLayout { panel, map, legend }
}

/// The map tile under a UI point on the map image, or `None` outside it.
fn tile_at(map: Rect, p: Vec2) -> Option<IVec2> {
    if !map.contains(p) {
        return None;
    }
    let scale = map.w / MAP_W as f32;
    let t = ((p - map.point()) / scale).floor().as_ivec2();
    in_bounds(t).then_some(t)
}

/// One legend row: a colour swatch and its name.
fn legend_row(a: &Assets, x: f32, y: f32, col: Color, name: &str) {
    let sw = Rect::new(x, y + (ROW_H - SWATCH) * 0.5, SWATCH, SWATCH);
    rrect(sw, radius::SM, col);
    rrect_lines(sw, radius::SM, line::HAIR, EDGE);
    text(a, name, x + SWATCH + space::S, y, TextSize::M, TEXT);
}

pub(crate) fn map_panel(g: &mut Game) {
    let a = g.assets.clone();
    let layout = map_layout(ui_width(), ui_height());
    let slide = slide_px(g);
    let panel = Rect::new(layout.panel.x, layout.panel.y - slide, layout.panel.w, layout.panel.h);
    let map = Rect::new(layout.map.x, layout.map.y - slide, layout.map.w, layout.map.h);
    let legend = Rect::new(layout.legend.x, layout.legend.y - slide, layout.legend.w, layout.legend.h);
    let (_content, closed) = Panel::new(panel).title("Map").closable().show(g);
    if closed {
        g.panel = OpenPanel::None;
        return;
    }

    // The one raster, shared with the minimap so nothing is refreshed twice.
    if let Some(t) = g.minimap.tex() {
        draw_texture_ex(t, map.x, map.y, WHITE, DrawTextureParams { dest_size: Some(map.size()), ..Default::default() });
    }
    rrect_lines(map, radius::SM, line::STRONG, EDGE);

    // Markers drawn over the image, clipped to it.
    clip(map);
    map_player_and_view(g, map);
    let scale = map.w / MAP_W as f32;
    let hive = theme::color(theme::MAP_HIVE);
    let enemy = theme::color(theme::MAP_ENEMY);
    for e in g.world.entities.iter().flatten().filter(|e| e.kind == Kind::Hive) {
        // A hive is a 3x3 footprint, only a few pixels on the raster, so it
        // gets a larger circle than the single-tile enemy dots.
        circle(e.center() * scale + map.point(), 1.6 * scale, hive);
    }
    for en in &g.world.enemies {
        circle(en.pos * scale + map.point(), 0.7 * scale, enemy);
    }
    unclip();

    // Legend: the colours the map really uses, then the two counts.
    let ores = [Item::IronOre, Item::CopperOre, Item::Coal, Item::Stone, Item::Quartz];
    let mut y = legend.y + space::S;
    for item in ores {
        legend_row(&a, legend.x, y, item.color(), item.name());
        y += ROW_H;
    }
    legend_row(&a, legend.x, y, theme::color(theme::MAP_TAR), "Tar");
    y += ROW_H;
    legend_row(&a, legend.x, y, theme::color(theme::MAP_BUILDING), "Buildings");
    y += ROW_H;
    legend_row(&a, legend.x, y, theme::color(theme::MAP_HIVE), "Hives");
    y += ROW_H;
    legend_row(&a, legend.x, y, theme::color(theme::MAP_ENEMY), "Enemies");
    y += ROW_H;
    legend_row(&a, legend.x, y, TEXT, "Player");
    y += ROW_H + space::S;
    let buildings = g.world.entities.iter().flatten().filter(|e| e.kind != Kind::Hive).count();
    let hives = g.world.entities.iter().flatten().filter(|e| e.kind == Kind::Hive).count();
    text(&a, &format!("Buildings: {buildings}"), legend.x, y, TextSize::M, TEXT);
    y += ROW_H;
    text(&a, &format!("Hives: {hives}"), legend.x, y, TextSize::M, TEXT);

    // Hovering the map shows the tile tooltip for the tile underneath; a click
    // does nothing (the panel itself already swallows the click).
    if let Some(tile) = tile_at(map, ui_mouse()) {
        // Action hints (mine, place, choose) don't apply from the map, so they
        // are dropped here; the world tooltip still shows them.
        let rows = g.world.describe_rows_for_map(tile);
        if !rows.is_empty() {
            g.tooltip = Some(Tip::Card { icon: g.world.describe_icon(tile), rows });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn map_stays_square_and_inside_the_window() {
        for (w, h) in [(1440.0, 900.0), (1920.0, 1080.0), (1250.0, 775.0), (820.0, 900.0)] {
            let l = map_layout(w, h);
            assert!(l.panel.x >= space::L, "{w}x{h}: panel off the left");
            assert!(l.panel.y >= space::L, "{w}x{h}: panel off the top");
            assert!(l.panel.right() <= w - space::L + 0.01, "{w}x{h}: panel off the right");
            assert!(l.panel.bottom() <= h - space::L + 0.01, "{w}x{h}: panel off the bottom");
            assert!(l.map.w > 0.0, "{w}x{h}: map collapsed");
            assert!((l.map.w - l.map.h).abs() < 0.01, "{w}x{h}: map not square: {l:?}");
            assert!(l.map.x >= l.panel.x && l.map.right() <= l.panel.right() + 0.01, "{w}x{h}: map outside panel");
            assert!(l.map.y >= l.panel.y && l.map.bottom() <= l.panel.bottom() + 0.01, "{w}x{h}: map outside panel");
            assert!(l.legend.right() <= l.panel.right() + 0.01, "{w}x{h}: legend outside panel");
            assert!(l.legend.y >= l.panel.y && l.legend.bottom() <= l.panel.bottom() + 0.01, "{w}x{h}: legend outside panel");
            assert!(l.legend.x >= l.map.right(), "{w}x{h}: legend overlaps map");
        }
    }

    #[test]
    fn map_panel_is_sized_to_content_and_centred() {
        // At 16:9 the panel wraps the map and legend instead of filling the
        // window, and sits centred horizontally.
        let wide = map_layout(1600.0, 900.0);
        assert!(wide.panel.w < 1600.0, "16:9: panel should not fill the window: {wide:?}");
        assert!((wide.panel.center().x - 800.0).abs() < 0.01, "16:9: panel not centred: {wide:?}");
        assert!((wide.map.w - wide.map.h).abs() < 0.01, "16:9: map not square: {wide:?}");

        // At a narrow size the panel still fits inside the window margins.
        let narrow = map_layout(820.0, 900.0);
        assert!(narrow.panel.x >= space::L, "narrow: panel off the left: {narrow:?}");
        assert!(narrow.panel.right() <= 820.0 - space::L + 0.01, "narrow: panel off the right: {narrow:?}");
        assert!(narrow.panel.y >= space::L, "narrow: panel off the top: {narrow:?}");
        assert!(narrow.panel.bottom() <= 900.0 - space::L + 0.01, "narrow: panel off the bottom: {narrow:?}");
        assert!((narrow.map.w - narrow.map.h).abs() < 0.01, "narrow: map not square: {narrow:?}");
    }

    #[test]
    fn screen_to_tile_maps_corners() {
        let l = map_layout(1440.0, 900.0);
        let map = l.map;
        assert_eq!(tile_at(map, map.point()), Some(ivec2(0, 0)), "top-left");
        assert_eq!(
            tile_at(map, vec2(map.right() - 0.5, map.bottom() - 0.5)),
            Some(ivec2(MAP_W - 1, MAP_H - 1)),
            "just inside the bottom-right"
        );
        assert_eq!(tile_at(map, vec2(map.right() + 1.0, map.center().y)), None, "right edge");
        assert_eq!(tile_at(map, vec2(map.center().x, map.bottom() + 1.0)), None, "bottom edge");
        assert_eq!(tile_at(map, vec2(map.x - 1.0, map.y)), None, "left of the map");
    }
}

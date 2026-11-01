//! The user interface: HUD, minimap, tooltips and the window panels.

use macroquad::prelude::*;

use crate::{Game, Panel as OpenPanel};

mod hud;
mod metrics;
mod minimap;
mod panels;
mod primitives;
mod style;
mod text;
mod tooltip;
mod widgets;

use metrics::{ui_camera, ui_height, ui_width};

pub use metrics::ui_mouse;
pub use minimap::Minimap;
pub use panels::inventory::hovered_inventory_item;
pub use primitives::{circle, circle_lines, rrect, rrect_dashed_lines, rrect_lines};

pub fn draw(g: &mut Game) {
    g.ui_rects.clear();
    g.tooltip.clear();
    set_camera(&ui_camera(ui_width(), ui_height()));

    hud::hud(g);
    hud::quickbar(g);
    hud::craft_queue(g);
    minimap::minimap(g);
    match g.panel {
        OpenPanel::Inventory => panels::inventory::inventory_panel(g),
        OpenPanel::Research => panels::research::research_panel(g),
        OpenPanel::Building(id, _) => panels::building::building_panel(g, id),
        OpenPanel::None => {}
    }
    if g.tooltip.is_empty() && !g.mouse_over_ui() && g.tool.is_none() {
        g.tooltip = g.world.describe(g.mouse_tile());
    }
    let a = g.assets.clone();
    hud::radio(g, &a);
    hud::toasts(g, &a);
    if g.show_help {
        hud::help(&a);
    }
    hud::overlays(g, &a);
    tooltip::draw_tooltip(&a, &g.tooltip);
    set_default_camera();
}

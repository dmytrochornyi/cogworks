//! The user interface: HUD, minimap, tooltips and the window panels.

use macroquad::prelude::*;

use crate::{Game, Panel as OpenPanel};

mod hud;
mod metrics;
mod menus;
mod minimap;
mod panels;
mod primitives;
mod style;
mod text;
mod tooltip;
mod widgets;

use metrics::{ui_camera, ui_height, ui_width};

pub use metrics::{mouse_override, screen_mouse, screenshot_mode, ui_mouse};
pub use minimap::Minimap;
pub use panels::inventory::hovered_inventory_item;
pub use primitives::{circle, circle_lines, rrect, rrect_dashed_lines, rrect_lines};
pub use tooltip::Tip;

pub fn draw(g: &mut Game) {
    g.ui_rects.clear();
    g.clickables.clear();
    g.tooltip = None;
    set_camera(&ui_camera(ui_width(), ui_height()));
    primitives::set_paper(g.assets.atlas.clone(), g.assets.rect("paper"));

    if let crate::game::Screen::Title(page) = g.screen {
        menus::title(g, page);
        let a = g.assets.clone();
        hud::toasts(g, &a);
        g.draw_ui_cursor();
        set_default_camera();
        return;
    }

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
    if g.tooltip.is_none() && !g.mouse_over_ui() && g.tool.is_none() {
        let tile = g.mouse_tile();
        let rows = g.world.describe_rows(tile);
        if !rows.is_empty() {
            g.tooltip = Some(Tip::Card { icon: g.world.describe_icon(tile), rows });
        }
    }
    let a = g.assets.clone();
    hud::radio(g, &a);
    hud::toasts(g, &a);
    if g.show_help {
        hud::help(g, &a);
    }
    hud::overlays(g, &a);
    if let crate::game::Screen::Paused(page) = g.screen {
        g.tooltip = None;
        menus::pause(g, page);
    }
    tooltip::draw_tooltip(&a, &g.world, &g.tooltip);
    g.draw_ui_cursor();
    set_default_camera();
}

//! The inventory panel: the backpack grid and the hand-crafting workbench.

use macroquad::prelude::*;

use crate::items::*;
use crate::theme::{TextSize, line, space};
use crate::{Game, Panel as OpenPanel};

use super::super::primitives::{clicked, hovered};
use super::super::style::{EDGE, GAP, HEADER, SLOT, TEXT_DIM, TITLE};
use super::super::text::{text, text_h, text_w};
use super::super::tooltip::Tip;
use super::super::widgets::{Panel, recipe_icon, slot};
use super::{centered, grid_rect, slide_px};

const INV_W: f32 = 1020.0;
const INV_H: f32 = 580.0;
const INV_COLS: usize = 9;

/// The workbench header: where the "Shift-click to craft five" hint goes and
/// where the recipe grid starts, as offsets from the workbench column's top.
/// The hint sits beside the title when it fits; otherwise it drops to its own
/// line and the grid moves down to stay clear of it.
fn workbench_header(x: f32, right: f32, title_w: f32, title_h: f32, hint_w: f32) -> (f32, f32, f32) {
    if x + title_w + space::M + hint_w <= right {
        (x + title_w + space::M, 8.0, title_h + 4.0)
    } else {
        (x, title_h + 2.0, title_h + 2.0 + 20.0)
    }
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

pub(crate) fn inventory_panel(g: &mut Game) {
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
        slot(&a, sr, Some(item), Some(n), g.tool == Some(item), false, &mut g.clickables);
        if hovered(sr) {
            let carried = format!("{n} carried");
            let mut tip = vec![carried.as_str()];
            if item.place_kind().is_some() {
                tip.push("Click to hold");
            }
            tip.push("Press 1–0 to put it on the quickbar");
            g.tooltip = Some(Tip::item(item, &tip));
        }
        if clicked(sr) && item.place_kind().is_some() {
            g.tool = Some(item);
            g.panel = OpenPanel::None;
        }
    }

    let cx = content.x + INV_COLS as f32 * (SLOT + GAP) + space::XL;
    draw_line(cx - space::M, content.y, cx - space::M, r.bottom() - 44.0, line::HAIR, EDGE);
    let title = "Workbench";
    text(&a, title, cx, content.y, TextSize::Xl, TITLE);
    let title_w = text_w(&a, title, TextSize::Xl);
    let title_h = text_h(&a, TextSize::Xl);
    let hint = "Shift-click to craft five";
    let (hint_x, hint_off, grid_off) =
        workbench_header(cx, content.right(), title_w, title_h, text_w(&a, hint, TextSize::Xs));
    text(&a, hint, hint_x, content.y + hint_off, TextSize::Xs, TEXT_DIM);
    let origin = vec2(cx, content.y + grid_off);
    let recipes: Vec<usize> = (0..RECIPES.len()).filter(|&id| RECIPES[id].hand && g.world.recipe_unlocked(id)).collect();
    let shift = is_key_down(KeyCode::LeftShift) || is_key_down(KeyCode::RightShift);
    let cols = ((content.right() - cx + GAP) / (SLOT + GAP)).floor().max(1.0) as usize;
    for (k, &id) in recipes.iter().enumerate() {
        let sr = grid_rect(origin, k, cols.min(8), SLOT);
        let can = g.world.can_craft(id);
        recipe_icon(&a, sr, id, false, !can, &mut g.clickables);
        if hovered(sr) {
            g.tooltip = Some(Tip::Recipe(id));
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn workbench_hint_never_overprints_the_title() {
        // Plenty of room: the hint sits beside the title and the grid stays high.
        let (hx, _hy, gy) = workbench_header(744.0, 1206.0, 170.0, 34.0, 150.0);
        assert!(hx >= 744.0 + 170.0 + space::M - 0.01, "hint overlaps the title");
        assert!(gy < 40.0, "grid should not move down when the hint fits");
        // No room: the hint drops below the title and the grid moves down too.
        let (hx, hy, gy) = workbench_header(744.0, 760.0, 170.0, 34.0, 150.0);
        assert_eq!(hx, 744.0);
        assert!(hy > 34.0, "hint must sit below the title");
        assert!(gy > hy, "grid must sit below the hint");
    }
}

//! The building panel: machine state, recipe pickers and stock read-outs.

use macroquad::prelude::*;

use crate::items::*;
use crate::render::*;
use crate::theme::{self, TextSize, space};
use crate::world::*;
use crate::{Game, Panel as OpenPanel};

use super::super::metrics::{ui_height, ui_width};
use super::super::primitives::{clicked, hovered};
use super::super::style::{GAP, HEADER, HONEY, KEY, MINIMAP, PEACH, SLOT, TEAL, TEXT, TEXT_DIM};
use super::super::text::{text, text_w};
use super::super::tooltip::Tip;
use super::super::widgets::{Panel, bar, button, primary_button, recipe_icon, slot, stock_lines};
use super::{grid_rect, slide_px};

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

pub(crate) fn building_panel(g: &mut Game, id: usize) {
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
                recipe_icon(&a, sr, rid, c.recipe == Some(rid), false, &mut g.clickables);
                if hovered(sr) {
                    g.tooltip = Some(Tip::Recipe(rid));
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
            if primary_button(&a, left_btn, "Insert ingredients", c.recipe.is_some(), &mut g.clickables) {
                let n = g.world.insert_from_player(id);
                g.toast(format!("Inserted {n} items"));
            }
            if button(&a, right_btn, "Take output", c.out.total() > 0, &mut g.clickables) {
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
            if primary_button(&a, left_btn, "Insert data", true, &mut g.clickables) {
                let n = g.world.insert_from_player(id);
                g.toast(format!("Inserted {n} items"));
            }
        }
        State::Generator { fuel, load, .. } => {
            text(&a, &format!("Working at {:.0}% of 600 kW", load * 100.0), x, y, TextSize::M, TEXT);
            y += 26.0;
            stock_lines(&a, fuel, x, &mut y);
            text(&a, "Burns coal or fuel cells", x, y + 4.0, TextSize::S, TEXT_DIM);
            if primary_button(&a, left_btn, "Add fuel", true, &mut g.clickables) {
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
                slot(&a, sr, Some(item), Some(n), false, false, &mut g.clickables);
                if hovered(sr) {
                    g.tooltip = Some(Tip::item(item, &[&format!("{n} stored"), "Click to take"]));
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
            if primary_button(&a, left_btn, "Store items", true, &mut g.clickables) {
                let n = g.world.insert_from_player(id);
                g.toast(format!("Stored {n} items"));
            }
            if button(&a, right_btn, "Take all", stock.total() > 0, &mut g.clickables) {
                g.world.take_to_player(id);
            }
        }
        State::Sentry { ammo, shots, .. } => {
            text(&a, &format!("{shots} shots loaded"), x, y, TextSize::M, TEXT);
            y += 26.0;
            stock_lines(&a, ammo, x, &mut y);
            if primary_button(&a, left_btn, "Load ammo", true, &mut g.clickables) {
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
            if primary_button(&a, left_btn, "Feed cores", true, &mut g.clickables) {
                g.world.insert_from_player(id);
            }
        }
        State::Drill { out, .. } => {
            for line in g.world.describe(e.pos).iter().skip(1) {
                text(&a, line, x, y, TextSize::S, TEXT_DIM);
                y += 24.0;
            }
            if primary_button(&a, left_btn, "Take ore", !out.is_empty(), &mut g.clickables) {
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

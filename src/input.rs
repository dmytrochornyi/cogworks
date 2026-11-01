use macroquad::prelude::{IVec2, KeyCode, MouseButton, Vec2, is_key_down, is_key_pressed, is_mouse_button_down, is_mouse_button_pressed, ivec2, mouse_position, mouse_wheel};

use crate::game::{Game, Panel};
use crate::items::Item;
use crate::save;
use crate::ui;
use crate::world::{Dir, Kind, dims};

/// Seconds of holding right click to pick up a building.
const DECONSTRUCT_TIME: f32 = 0.3;

impl Game {
    pub(crate) fn input(&mut self, dt: f32) {
        let mut walk = Vec2::ZERO;
        if is_key_down(KeyCode::W) || is_key_down(KeyCode::Up) {
            walk.y -= 1.0;
        }
        if is_key_down(KeyCode::S) || is_key_down(KeyCode::Down) {
            walk.y += 1.0;
        }
        if is_key_down(KeyCode::A) || is_key_down(KeyCode::Left) {
            walk.x -= 1.0;
        }
        if is_key_down(KeyCode::D) || is_key_down(KeyCode::Right) {
            walk.x += 1.0;
        }
        self.world.player.walk = walk;

        // Camera follows the player.
        let target = self.world.player.pos;
        self.cam.pos += (target - self.cam.pos) * (1.0 - (-dt * 10.0).exp());
        let wheel = mouse_wheel().1;
        if wheel != 0.0 && !self.mouse_over_ui() {
            let factor = if wheel > 0.0 { 1.12 } else { 1.0 / 1.12 };
            self.cam.zoom = (self.cam.zoom * factor).clamp(18.0, 110.0);
        }

        self.keys();
        if !self.world.player.alive() {
            self.world.player.mining = None;
            return;
        }
        self.mouse(dt);
    }

    fn keys(&mut self) {
        let numbers = [
            KeyCode::Key1,
            KeyCode::Key2,
            KeyCode::Key3,
            KeyCode::Key4,
            KeyCode::Key5,
            KeyCode::Key6,
            KeyCode::Key7,
            KeyCode::Key8,
            KeyCode::Key9,
            KeyCode::Key0,
        ];
        for (slot, key) in numbers.into_iter().enumerate() {
            if !is_key_pressed(key) {
                continue;
            }
            // While hovering an inventory item, number keys bind it to the quickbar.
            match ui::hovered_inventory_item(self) {
                Some(item) => {
                    self.world.player.quickbar[slot] = Some(item);
                    self.toast(format!("Bound {} to slot {}", item.name(), (slot + 1) % 10));
                }
                None => self.select_slot(slot),
            }
        }
        let tile = self.mouse_tile();
        if is_key_pressed(KeyCode::R) {
            let ccw = is_key_down(KeyCode::LeftShift);
            if self.tool.is_some() {
                self.rot = if ccw { self.rot.ccw() } else { self.rot.cw() };
            } else {
                self.world.rotate(tile, !ccw);
            }
        }
        if is_key_pressed(KeyCode::Q) {
            match self.world.entity_ref_at(tile).and_then(|e| e.kind.item().map(|i| (i, e.dir))) {
                Some((item, dir)) if self.tool != Some(item) => {
                    self.tool = Some(item);
                    self.rot = dir;
                }
                _ => self.tool = None,
            }
        }
        if is_key_pressed(KeyCode::Escape) {
            if self.panel != Panel::None {
                self.panel = Panel::None;
            } else {
                self.tool = None;
            }
        }
        if is_key_pressed(KeyCode::E) || is_key_pressed(KeyCode::Tab) {
            self.panel = if self.panel == Panel::Inventory { Panel::None } else { Panel::Inventory };
        }
        if is_key_pressed(KeyCode::T) {
            self.panel = if self.panel == Panel::Research { Panel::None } else { Panel::Research };
        }
        if is_key_pressed(KeyCode::H) || is_key_pressed(KeyCode::F1) {
            self.show_help = !self.show_help;
        }
        if is_key_pressed(KeyCode::O) {
            self.show_pollution = !self.show_pollution;
        }
        if is_key_pressed(KeyCode::Space) {
            self.paused = !self.paused;
        }
        if is_key_pressed(KeyCode::M) {
            self.audio.toggle_music();
        }
        if is_key_pressed(KeyCode::P) {
            self.world.peaceful = !self.world.peaceful;
            let msg = if self.world.peaceful { "Peaceful mode on" } else { "Peaceful mode off" };
            self.toast(msg);
        }
        if is_key_pressed(KeyCode::F5) {
            match save::save(&mut self.world, save::SAVE_PATH) {
                Ok(()) => self.toast(format!("Saved to {}", save::SAVE_PATH)),
                Err(e) => self.toast(e),
            }
        }
        if is_key_pressed(KeyCode::F9) {
            match save::newest_save().ok_or_else(|| "No save yet: press F5 to save".to_string()).and_then(save::load) {
                Ok(world) => {
                    self.world = world;
                    self.cam.pos = self.world.player.pos;
                    self.panel = Panel::None;
                    self.tool = None;
                    self.show_help = false;
                    self.toast("Game loaded");
                }
                Err(e) => self.toast(e),
            }
        }
    }

    fn mouse(&mut self, dt: f32) {
        let over_ui = self.mouse_over_ui();
        let tile = self.mouse_tile();

        // Left: build, or open the building under the cursor.
        if !is_mouse_button_down(MouseButton::Left) {
            self.last_drag = None;
        } else if !over_ui || self.last_drag.is_some() {
            match self.tool.and_then(Item::place_kind) {
                Some(kind) => self.paint(kind),
                None if is_mouse_button_pressed(MouseButton::Left) => {
                    self.panel = match self.world.entity_at(tile) {
                        Some(id) if self.world.entity(id).kind != Kind::Hive => {
                            Panel::Building(id, self.world.entity(id).pos)
                        }
                        _ => Panel::None,
                    };
                }
                None => {}
            }
        }

        // Right: pick up buildings or mine ore.
        let right = is_mouse_button_down(MouseButton::Right) && !over_ui;
        let in_reach = self.world.player.in_reach(tile);
        let building = self.world.entity_ref_at(tile).filter(|e| e.kind != Kind::Hive).map(|e| e.pos);
        match (right, building) {
            (true, Some(pos)) if in_reach => {
                self.world.player.mining = None;
                let progress = match self.deconstruct {
                    Some((p, t)) if p == pos => t + dt / DECONSTRUCT_TIME,
                    _ => 0.0,
                };
                if progress >= 1.0 {
                    self.deconstruct = None;
                    if let Some(kind) = self.world.deconstruct(pos) {
                        self.toast(format!("+1 {}", kind.name()));
                    }
                } else {
                    self.deconstruct = Some((pos, progress));
                }
            }
            (true, None) if in_reach && self.world.ore_at(tile).is_some() => {
                self.deconstruct = None;
                if self.world.player.mining.is_none_or(|(t, _)| t != tile) {
                    self.world.player.mining = Some((tile, 0.0));
                }
            }
            (true, _) if is_mouse_button_pressed(MouseButton::Right) && !in_reach => {
                self.toast("Out of reach");
            }
            _ => {
                self.deconstruct = None;
                self.world.player.mining = None;
            }
        }
    }

    /// Places the held building, following the drag for conveyors, pipes and walls.
    fn paint(&mut self, kind: Kind) {
        let pos = self.ghost_pos(kind);
        let draggable = matches!(kind, Kind::Belt | Kind::Pipe | Kind::Wall);
        let Some(mut prev) = self.last_drag else {
            self.try_place(kind, pos, true);
            self.last_drag = Some(pos);
            return;
        };
        if !draggable {
            return;
        }
        while prev != pos {
            let delta = pos - prev;
            let step = if delta.x != 0 { ivec2(delta.x.signum(), 0) } else { ivec2(0, delta.y.signum()) };
            let next = prev + step;
            if kind == Kind::Belt {
                let d = Dir::from_delta(step).unwrap_or(self.rot);
                self.rot = d;
                self.world.set_dir(prev, d);
            }
            self.try_place(kind, next, false);
            prev = next;
        }
        self.last_drag = Some(pos);
    }

    fn try_place(&mut self, kind: Kind, pos: IVec2, report_all: bool) {
        let center = pos + (dims(kind, self.rot) - IVec2::ONE) / 2;
        if !self.world.player.in_reach(center) {
            if report_all {
                self.toast("Out of reach");
            }
            return;
        }
        if let Err(msg) = self.world.place(kind, pos, self.rot) {
            // While dragging, running over existing buildings is expected.
            if report_all || self.world.entity_at(pos).is_none() {
                self.toast(msg);
            }
        }
    }

    pub fn mouse_world(&self) -> Vec2 {
        self.cam.s2w(mouse_position().into())
    }

    pub fn mouse_tile(&self) -> IVec2 {
        self.mouse_world().floor().as_ivec2()
    }

    pub fn mouse_over_ui(&self) -> bool {
        let m = ui::ui_mouse();
        self.ui_rects.iter().any(|r| r.contains(m))
    }

    /// Top-left tile for the held building so that it's centred on the cursor.
    pub fn ghost_pos(&self, kind: Kind) -> IVec2 {
        let d = dims(kind, self.rot);
        (self.mouse_world() - (d - IVec2::ONE).as_vec2() * 0.5).floor().as_ivec2()
    }

    pub fn select_slot(&mut self, slot: usize) {
        match self.world.player.quickbar[slot] {
            Some(item) if self.tool == Some(item) => self.tool = None,
            Some(item) if item.place_kind().is_some() => self.tool = Some(item),
            _ => {}
        }
    }
}

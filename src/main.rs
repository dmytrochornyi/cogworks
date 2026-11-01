mod assets;
mod assets_list;
mod items;
mod render;
mod save;
mod theme;
mod ui;
mod world;

use std::rc::Rc;

use macroquad::audio::{PlaySoundParams, play_sound, set_sound_volume};
use macroquad::prelude::*;

use assets::Assets;
use items::*;
use render::*;
use world::*;

const TICK: f32 = 1.0 / 60.0;
/// Seconds of holding right click to pick up a building.
const DECONSTRUCT_TIME: f32 = 0.3;
const MUSIC_VOLUME: f32 = 0.38;

fn conf() -> Conf {
    let dim = |key: &str, default: i32| {
        std::env::var(key).ok().and_then(|v| v.parse().ok()).unwrap_or(default)
    };
    Conf {
        window_title: "Cogworks".to_owned(),
        window_width: dim("COGWORKS_WIDTH", 1440),
        window_height: dim("COGWORKS_HEIGHT", 900),
        window_resizable: true,
        sample_count: 4,
        ..Default::default()
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Panel {
    None,
    Inventory,
    Research,
    /// Entity id and its position, so a reused id isn't mistaken for the old building.
    Building(usize, IVec2),
}

pub struct Game {
    pub world: World,
    pub cam: Cam,
    /// Building item in hand.
    pub tool: Option<Item>,
    pub rot: Dir,
    /// Last tile painted during the current left-drag.
    last_drag: Option<IVec2>,
    pub panel: Panel,
    /// 0→1 open animation for the current modal panel; 1 = settled.
    panel_slide: f32,
    last_panel: Panel,
    pub toasts: Vec<(String, f32)>,
    pub show_help: bool,
    pub show_pollution: bool,
    pub paused: bool,
    /// Screen areas covered by UI last frame; clicks there don't reach the map.
    pub ui_rects: Vec<Rect>,
    /// Tooltip lines collected while drawing UI, drawn last so they sit on top.
    pub tooltip: Vec<String>,
    /// Scroll offset of the research grid, in logical UI pixels.
    pub research_scroll: f32,
    deconstruct: Option<(IVec2, f32)>,
    pub minimap: ui::Minimap,
    pub time: f32,
    pub assets: Rc<Assets>,
    renderer: Renderer,
    pub music_on: bool,
    /// Seconds since a gunshot was last heard, to keep sentries from drowning everything.
    shot_cooldown: f32,
    /// Seconds of unpaused play since the last autosave.
    autosave_timer: f32,
    /// Off in the screenshot dev mode, so it never writes save files.
    pub autosave: bool,
}

impl Game {
    fn new(world: World, assets: Rc<Assets>) -> Self {
        let cam = Cam { pos: world.player.pos, zoom: 52.0 };
        Game {
            world,
            cam,
            tool: None,
            rot: Dir::E,
            last_drag: None,
            panel: Panel::None,
            panel_slide: 1.0,
            last_panel: Panel::None,
            toasts: Vec::new(),
            show_help: true,
            show_pollution: false,
            paused: false,
            ui_rects: Vec::new(),
            tooltip: Vec::new(),
            research_scroll: 0.0,
            deconstruct: None,
            minimap: ui::Minimap::new(),
            time: 0.0,
            assets,
            renderer: Renderer::new(),
            music_on: true,
            shot_cooldown: 0.0,
            autosave_timer: 0.0,
            autosave: true,
        }
    }

    pub fn toast(&mut self, msg: impl Into<String>) {
        let msg = msg.into();
        if let Some(t) = self.toasts.iter_mut().find(|(m, _)| *m == msg) {
            t.1 = 3.0;
            return;
        }
        self.toasts.push((msg, 3.0));
        if self.toasts.len() > 5 {
            self.toasts.remove(0);
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

    fn input(&mut self, dt: f32) {
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
            self.music_on = !self.music_on;
            if let Some(music) = self.assets.sounds.get("music") {
                set_sound_volume(music, if self.music_on { MUSIC_VOLUME } else { 0.0 });
            }
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

    fn update(&mut self, dt: f32) {
        self.time += dt;
        if self.autosave && !self.paused && self.world.player.alive() && save::autosave_tick(&mut self.autosave_timer, dt) {
            match save::save(&mut self.world, save::AUTOSAVE_PATH) {
                Ok(()) => self.toast("Autosaved"),
                Err(e) => self.toast(e),
            }
        }
        for (_, t) in &mut self.toasts {
            *t -= dt;
        }
        self.toasts.retain(|(_, t)| *t > 0.0);
        let events: Vec<String> = self.world.events.drain(..).collect();
        for e in events {
            self.toast(e);
        }
        self.play_sounds(dt);
        if let Panel::Building(id, pos) = self.panel
            && self.world.entities.get(id).and_then(Option::as_ref).is_none_or(|e| e.pos != pos)
        {
            self.panel = Panel::None;
        }
        if self.panel != self.last_panel {
            self.last_panel = self.panel;
            self.panel_slide = if self.panel == Panel::None { 1.0 } else { 0.0 };
        }
        self.panel_slide = (self.panel_slide + dt / 0.16).min(1.0);
    }

    fn play_sounds(&mut self, dt: f32) {
        self.shot_cooldown += dt;
        if is_mouse_button_pressed(MouseButton::Left) && self.mouse_over_ui() {
            self.play("click", 0.5);
        }
        let sounds: Vec<Sfx> = self.world.sounds.drain(..).collect();
        for s in sounds {
            match s {
                Sfx::Place => self.play("place", 0.8),
                Sfx::Pickup => self.play("pickup", 0.7),
                Sfx::Craft => self.play("craft", 0.7),
                Sfx::Research => self.play("research", 0.8),
                Sfx::Shot(at) => {
                    let d = at.distance(self.world.player.pos);
                    if self.shot_cooldown > 0.07 && d < 30.0 {
                        self.shot_cooldown = 0.0;
                        self.play("shot", 0.6 * (1.0 - d / 30.0));
                    }
                }
            }
        }
    }

    fn play(&self, name: &str, volume: f32) {
        if let Some(s) = self.assets.sounds.get(name) {
            play_sound(s, PlaySoundParams { looped: false, volume });
        }
    }

    fn draw(&mut self) {
        clear_background(BLACK);
        let dt = get_frame_time().min(0.1);
        self.renderer.frame(&self.assets, &self.world, &self.cam, self.time, dt, self.show_pollution);
        self.draw_cursor();
        ui::draw(self);
    }

    fn draw_cursor(&self) {
        if self.mouse_over_ui() || !self.world.player.alive() {
            return;
        }
        let z = self.cam.zoom;
        match self.tool.and_then(Item::place_kind) {
            Some(kind) => {
                let pos = self.ghost_pos(kind);
                let center = pos + (dims(kind, self.rot) - IVec2::ONE) / 2;
                let ok = self.world.player.inv.get(kind.item().unwrap_or(Item::Belt)) > 0
                    && self.world.player.in_reach(center)
                    && self.world.check_place(kind, pos, self.rot).is_ok();
                draw_ghost(&self.assets, &self.cam, kind, pos, self.rot, ok, self.time);
            }
            None => {
                let tile = self.mouse_tile();
                let (pos, size) = match self.world.entity_ref_at(tile) {
                    Some(e) => (e.pos, e.dims()),
                    None => (tile, IVec2::ONE),
                };
                let s = self.cam.w2s(pos.as_vec2());
                let w = size.as_vec2() * z;
                let col = if self.world.player.in_reach(tile) {
                    theme::alpha(theme::CREAM, 0.75)
                } else {
                    theme::alpha(theme::TERRA_400, 0.6)
                };
                ui::rrect_lines(Rect::new(s.x, s.y, w.x, w.y), z * 0.12, 2.0, col);
            }
        }
        // Progress ring for mining / picking up.
        let progress = self
            .deconstruct
            .map(|(_, p)| p)
            .or(self.world.player.mining.map(|(_, p)| p));
        if let Some(p) = progress {
            let (mx, my) = mouse_position();
            draw_circle(mx + 20.0, my + 20.0, 12.0, theme::alpha(theme::BG, 0.85));
            let steps = (p * 24.0) as i32;
            for k in 0..steps {
                let a = -std::f32::consts::FRAC_PI_2 + k as f32 / 24.0 * std::f32::consts::TAU;
                draw_circle(mx + 20.0 + a.cos() * 8.0, my + 20.0 + a.sin() * 8.0, 2.2, theme::color(theme::KEY));
            }
        }
    }
}

#[macroquad::main(conf)]
async fn main() {
    let seed = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0x9E37_79B9);
    // `cogworks <save file>` resumes that save.
    let world = match std::env::args().nth(1) {
        Some(path) => save::load(&path).unwrap_or_else(|e| panic!("{e}")),
        None => World::new(seed),
    };
    let assets = Rc::new(Assets::load().await);
    if let Some(music) = assets.sounds.get("music") {
        play_sound(music, PlaySoundParams { looped: true, volume: MUSIC_VOLUME });
    }
    let mut game = Game::new(world, assets);
    // Dev aid: start the day at a chosen phase (0 = midnight, as in `sky_at_phase`).
    if let Some(p) = std::env::var("COGWORKS_DAY_PHASE").ok().and_then(|v| v.parse::<f32>().ok()) {
        game.world.time = DAY_LENGTH * (p.clamp(0.0, 1.0) + 0.7);
    }
    // Dev aid: render a few seconds, write a screenshot and quit.
    let screenshot = std::env::var("COGWORKS_SCREENSHOT").ok();
    if screenshot.is_some() {
        game.autosave = false;
        game.show_help = std::env::var("COGWORKS_HELP").is_ok();
        game.panel = match std::env::var("COGWORKS_PANEL").as_deref() {
            Ok("research") => Panel::Research,
            Ok("inventory") => Panel::Inventory,
            Ok("building") => {
                let crafters = || {
                    game.world
                        .entities
                        .iter()
                        .enumerate()
                        .filter_map(|(i, e)| e.as_ref().map(|e| (i, e)))
                        .filter(|(_, e)| matches!(e.state, State::Crafter(_)))
                        .map(|(i, e)| (i, e.pos))
                };
                let pick = crafters().find(|(i, _)| game.world.entity(*i).kind == Kind::Assembler).or_else(|| crafters().next());
                pick.map(|(i, p)| Panel::Building(i, p)).unwrap_or(Panel::None)
            }
            _ => Panel::None,
        };
    }
    let mut frames = 0;
    let mut acc = 0.0;
    loop {
        frames += 1;
        let dt = get_frame_time().min(0.1);
        game.input(dt);
        if !game.paused {
            acc += dt;
            while acc >= TICK {
                game.world.update(TICK);
                acc -= TICK;
            }
        }
        game.update(dt);
        game.draw();
        if let Some(path) = &screenshot
            && frames == 240
        {
            get_screen_data().export_png(path);
            break;
        }
        next_frame().await;
    }
}

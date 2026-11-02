use std::rc::Rc;

use macroquad::prelude::{BLACK, IVec2, MouseButton, Rect, clear_background, get_frame_time, is_mouse_button_pressed, vec2};

use crate::assets::Assets;
use crate::audio::Audio;
use crate::input::Gesture;
use crate::items::Item;
use crate::render::{Cam, Look, Renderer, sky};
use crate::save;
use crate::settings::Settings;
use crate::ui;
use crate::world::{Dir, Fx, Sfx, World};

/// Which page of a full-screen menu is showing.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum MenuPage {
    Main,
    NewGame,
    Settings,
}

/// What the game is showing: the title screen, play, or the pause menu. The
/// world only simulates while playing.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Screen {
    Title(MenuPage),
    Playing,
    Paused(MenuPage),
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
    pub(crate) last_drag: Option<IVec2>,
    /// Which surface the left button's current press began on.
    pub(crate) gesture_left: Gesture,
    /// Which surface the right button's current press began on.
    pub(crate) gesture_right: Gesture,
    pub panel: Panel,
    /// 0→1 open animation for the current modal panel; 1 = settled.
    pub(crate) panel_slide: f32,
    last_panel: Panel,
    pub toasts: Vec<(String, f32)>,
    pub show_help: bool,
    pub show_pollution: bool,
    /// Post-processing preset (V cycles it).
    pub look: Look,
    pub paused: bool,
    /// Screen areas covered by UI last frame; clicks there don't reach the map.
    pub ui_rects: Vec<Rect>,
    /// Clickable UI controls (slots, buttons, recipe icons, cards); hovering
    /// them shows the Hand cursor. Cleared each frame alongside `ui_rects`.
    pub clickables: Vec<Rect>,
    /// Tooltip collected while drawing UI, drawn last so it sits on top.
    pub tooltip: Option<ui::Tip>,
    /// Scroll offset of the research grid, in logical UI pixels.
    pub research_scroll: f32,
    pub(crate) deconstruct: Option<(IVec2, f32)>,
    pub minimap: ui::Minimap,
    pub time: f32,
    pub assets: Rc<Assets>,
    renderer: Renderer,
    pub(crate) audio: Audio,
    pub music_on: bool,
    /// Seconds since a gunshot was last heard, to keep sentries from drowning everything.
    shot_cooldown: f32,
    /// Seconds of unpaused play since the last autosave.
    autosave_timer: f32,
    /// Off in the screenshot dev mode, so it never writes save files.
    pub autosave: bool,
    pub screen: Screen,
    /// Whether `world` is a real game (loaded or started), so the title screen
    /// can offer Continue. False while it's only a fresh backdrop.
    pub has_game: bool,
    pub settings: Settings,
    /// Set by the menus' Quit; the main loop exits when it sees it.
    pub quit: bool,
    /// 0→1 fade-in of the current menu screen.
    pub(crate) menu_fade: f32,
}

impl Game {
    pub(crate) fn new(world: World, assets: Rc<Assets>, mut audio: Audio, settings: Settings) -> Self {
        let cam = Cam { pos: world.player.pos, zoom: 52.0 };
        audio.set_levels(settings.levels());
        Game {
            world,
            cam,
            tool: None,
            rot: Dir::E,
            last_drag: None,
            gesture_left: Gesture::Idle,
            gesture_right: Gesture::Idle,
            panel: Panel::None,
            panel_slide: 1.0,
            last_panel: Panel::None,
            toasts: Vec::new(),
            show_help: true,
            show_pollution: false,
            look: Look::default(),
            paused: false,
            ui_rects: Vec::new(),
            clickables: Vec::new(),
            tooltip: None,
            research_scroll: 0.0,
            deconstruct: None,
            minimap: ui::Minimap::new(),
            time: 0.0,
            assets,
            renderer: Renderer::new(),
            audio,
            music_on: true,
            shot_cooldown: 0.0,
            autosave_timer: 0.0,
            autosave: true,
            screen: Screen::Title(MenuPage::Main),
            has_game: false,
            settings,
            quit: false,
            menu_fade: 0.0,
        }
    }

    /// Back to play from a menu.
    pub(crate) fn resume(&mut self) {
        if matches!(self.screen, Screen::Title(_)) {
            self.cam.pos = self.world.player.draw_pos;
            self.cam.zoom = 52.0;
        }
        self.screen = Screen::Playing;
        self.has_game = true;
    }

    /// Opens the pause menu, closing any window and putting the tool away.
    pub(crate) fn pause_menu(&mut self) {
        self.panel = Panel::None;
        self.screen = Screen::Paused(MenuPage::Main);
        self.menu_fade = 0.0;
    }

    /// Starts a fresh world and drops straight into it.
    pub(crate) fn new_game(&mut self, peaceful: bool) {
        let seed = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0x9E37_79B9);
        self.world = World::new(seed);
        self.world.peaceful = peaceful;
        self.panel = Panel::None;
        self.tool = None;
        self.show_help = true;
        self.paused = false;
        self.autosave_timer = 0.0;
        self.resume();
    }

    /// Saves to the manual save slot (F5), with a toast either way.
    pub(crate) fn save_game(&mut self) {
        match save::save(&mut self.world, save::SAVE_PATH) {
            Ok(()) => self.toast(format!("Saved to {}", save::SAVE_PATH)),
            Err(e) => self.toast(e),
        }
    }

    /// Leaves play for the title screen, autosaving so Continue picks up here.
    pub(crate) fn to_title(&mut self) {
        if self.autosave
            && self.world.player.alive()
            && let Err(e) = save::save(&mut self.world, save::AUTOSAVE_PATH)
        {
            self.toast(e);
        }
        self.panel = Panel::None;
        self.tool = None;
        self.screen = Screen::Title(MenuPage::Main);
        self.menu_fade = 0.0;
    }

    /// Applies changed settings straight away (volumes); fullscreen is switched
    /// by the settings page itself.
    pub(crate) fn apply_settings(&mut self, s: Settings) {
        self.settings = s;
        self.audio.set_levels(s.levels());
    }

    /// On the title screen the camera drifts slowly over the backdrop world.
    pub(crate) fn drift_camera(&mut self, dt: f32) {
        let t = self.time;
        let target = self.world.player.pos + vec2((t * 0.05).sin() * 7.0, (t * 0.037).cos() * 3.5);
        self.cam.pos += (target - self.cam.pos) * (1.0 - (-dt * 1.5).exp());
        self.cam.zoom += (44.0 - self.cam.zoom) * (1.0 - (-dt * 2.0).exp());
    }

    /// Eases the camera toward where the robot is drawn this frame.
    pub(crate) fn follow_camera(&mut self, dt: f32) {
        let target = self.world.player.draw_pos;
        self.cam.pos += (target - self.cam.pos) * (1.0 - (-dt * 10.0).exp());
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

    pub(crate) fn update(&mut self, dt: f32) {
        self.time += dt;
        self.menu_fade = (self.menu_fade + dt / 0.35).min(1.0);
        if self.autosave && self.screen == Screen::Playing && !self.paused && self.world.player.alive() && save::autosave_tick(&mut self.autosave_timer, dt) {
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
        self.audio.update(dt, sky(self.world.time).night);
        self.music_on = self.audio.music_on();
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
            self.audio.sfx("click", 0.5);
        }
        let fx: Vec<Fx> = self.world.fx.drain(..).collect();
        self.renderer.effects(&fx);
        let sounds: Vec<Sfx> = self.world.sounds.drain(..).collect();
        for s in sounds {
            match s {
                Sfx::Place => self.audio.sfx("place", 0.8),
                Sfx::Pickup => self.audio.sfx("pickup", 0.7),
                Sfx::Craft => self.audio.sfx("craft", 0.7),
                Sfx::Research => self.audio.sfx("research", 0.8),
                Sfx::Shot(at) => {
                    let d = at.distance(self.world.player.pos);
                    if self.shot_cooldown > 0.07 && d < 30.0 {
                        self.shot_cooldown = 0.0;
                        self.audio.sfx("shot", 0.6 * (1.0 - d / 30.0));
                    }
                }
            }
        }
    }

    pub(crate) fn draw(&mut self) {
        clear_background(BLACK);
        let dt = get_frame_time().min(0.1);
        self.renderer.frame(&self.assets, &self.world, &self.cam, self.time, dt, self.show_pollution, self.look);
        self.draw_cursor();
        ui::draw(self);
    }
}

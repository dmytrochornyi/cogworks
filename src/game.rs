use std::rc::Rc;

use macroquad::prelude::{BLACK, IVec2, MouseButton, Rect, clear_background, get_frame_time, is_mouse_button_pressed};

use crate::assets::Assets;
use crate::audio::Audio;
use crate::input::Gesture;
use crate::items::Item;
use crate::render::{Cam, Renderer, sky};
use crate::save;
use crate::ui;
use crate::world::{Dir, Fx, Sfx, World};

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
}

impl Game {
    pub(crate) fn new(world: World, assets: Rc<Assets>, audio: Audio) -> Self {
        let cam = Cam { pos: world.player.pos, zoom: 52.0 };
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
        }
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
        self.renderer.frame(&self.assets, &self.world, &self.cam, self.time, dt, self.show_pollution);
        self.draw_cursor();
        ui::draw(self);
    }
}

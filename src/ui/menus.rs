//! Full-screen menus: the title screen (with new-game and settings pages) and
//! the pause menu. They draw over the world, which stays visible behind them.

use macroquad::prelude::*;

use crate::game::{MenuPage, Screen};
use crate::items::TECHS;
use crate::render::DAY_LENGTH;
use crate::theme::{self, TextSize, line, space};
use crate::Game;

use super::metrics::{ui_height, ui_mouse, ui_width};
use super::primitives::{Elevation, ease_out, hovered, rrect, rrect_lines, surface};
use super::style::{EDGE, KEY, SLOT_WELL, TEXT, TEXT_DIM};
use super::text::{text, text_in, text_w};
use super::widgets::{Panel, button, primary_button};

const BTN_H: f32 = 46.0;
const CARD_W: f32 = 380.0;

/// Darkens the world behind a menu: a flat veil plus heavier top and bottom bands.
fn veil(strength: f32) {
    let (w, h) = (ui_width(), ui_height());
    draw_rectangle(0.0, 0.0, w, h, theme::rgba(0.10, 0.06, 0.04, 0.28 * strength));
    for k in 0..24 {
        let f = k as f32 / 24.0;
        let a = (1.0 - f).powi(2) * 0.35 * strength;
        let band = h * 0.3 / 24.0;
        draw_rectangle(0.0, f * h * 0.3, w, band + 1.0, theme::rgba(0.10, 0.06, 0.04, a));
        draw_rectangle(0.0, h - (f + 1.0 / 24.0) * h * 0.3, w, band + 1.0, theme::rgba(0.10, 0.06, 0.04, a));
    }
}

/// A short line about the game in memory, for under the Continue button.
fn save_summary(g: &Game) -> String {
    let day = (g.world.time / DAY_LENGTH).floor() as i32 + 1;
    let done = g.world.research.done.iter().filter(|d| **d).count();
    let mut s = format!("Day {day} · {done}/{} researched", TECHS.len());
    if g.world.peaceful {
        s.push_str(" · peaceful");
    }
    s
}

/// Stacks full-width buttons down a card, returning each one's rect.
struct Stack {
    x: f32,
    y: f32,
    w: f32,
}

impl Stack {
    fn next(&mut self, h: f32) -> Rect {
        let r = Rect::new(self.x, self.y, self.w, h);
        self.y += h + space::S + 2.0;
        r
    }
}

pub(super) fn title(g: &mut Game, page: MenuPage) {
    let a = g.assets.clone();
    veil(1.0);
    let (w, h) = (ui_width(), ui_height());
    let t = ease_out(g.menu_fade);

    // Logo, dropping in as the menu opens.
    let lw = (w * 0.5).clamp(320.0, 640.0);
    let lh = lw * 208.0 / 640.0;
    let ly = (h * 0.1).max(space::L) - (1.0 - t) * 30.0;
    a.draw("logo", vec2((w - lw) / 2.0, ly), vec2(lw, lh), Default::default());
    let tagline = "A cozy automation game: mine, build, research, and wake the Grand Engine";
    let tw = text_w(&a, tagline, TextSize::S);
    text(&a, tagline, (w - tw) / 2.0 + 1.0, ly + lh + space::S + 1.0, TextSize::S, theme::rgba(0.1, 0.06, 0.04, 0.6));
    text(&a, tagline, (w - tw) / 2.0, ly + lh + space::S, TextSize::S, theme::color(theme::CREAM));

    let top = ly + lh + space::XL + space::L;
    match page {
        MenuPage::Main => title_main(g, top),
        MenuPage::NewGame => new_game_page(g, top),
        MenuPage::Settings => settings_page(g, top, Screen::Title(MenuPage::Main)),
    }

    let version = format!("v{}", env!("CARGO_PKG_VERSION"));
    text(&a, &version, space::L, h - space::L - 18.0, TextSize::S, theme::alpha(theme::CREAM, 0.75));
    let credit = "Made in Rust with macroquad";
    let cw = text_w(&a, credit, TextSize::S);
    text(&a, credit, w - cw - space::L, h - space::L - 18.0, TextSize::S, theme::alpha(theme::CREAM, 0.75));
}

fn title_main(g: &mut Game, top: f32) {
    let a = g.assets.clone();
    let has_game = g.has_game;
    let rows = 4.0;
    let card_h = space::L * 2.0 + rows * (BTN_H + space::S + 2.0) + if has_game { 22.0 } else { 0.0 };
    let r = Rect::new((ui_width() - CARD_W) / 2.0, top, CARD_W, card_h);
    surface(r, Elevation::Panel);
    g.ui_rects.push(r);
    let mut s = Stack { x: r.x + space::XL, y: r.y + space::L + 4.0, w: r.w - 2.0 * space::XL };

    let cont = s.next(BTN_H);
    let pressed_continue = if has_game {
        primary_button(&a, cont, "Continue", true, &mut g.clickables)
    } else {
        button(&a, cont, "Continue", false, &mut g.clickables)
    };
    if has_game {
        let summary = save_summary(g);
        text_in(&a, &summary, Rect::new(cont.x, cont.bottom() + 1.0, cont.w, 20.0), TextSize::Xs, TEXT_DIM);
        s.y += 22.0;
    }
    let new_btn = s.next(BTN_H);
    let pressed_new = if has_game {
        button(&a, new_btn, "New game", true, &mut g.clickables)
    } else {
        primary_button(&a, new_btn, "New game", true, &mut g.clickables)
    };
    let pressed_settings = button(&a, s.next(BTN_H), "Settings", true, &mut g.clickables);
    let pressed_quit = button(&a, s.next(BTN_H), "Quit", true, &mut g.clickables);

    let enter = is_key_pressed(KeyCode::Enter) || is_key_pressed(KeyCode::KpEnter);
    if pressed_continue || (enter && has_game) {
        g.resume();
    } else if pressed_new || (enter && !has_game) {
        g.screen = Screen::Title(MenuPage::NewGame);
    } else if pressed_settings {
        g.screen = Screen::Title(MenuPage::Settings);
    } else if pressed_quit {
        g.quit = true;
    }
}

fn new_game_page(g: &mut Game, top: f32) {
    let a = g.assets.clone();
    let r = Rect::new((ui_width() - CARD_W - 40.0) / 2.0, top, CARD_W + 40.0, 250.0);
    let (content, closed) = Panel::new(r).title("New game").closable().show(g);
    let mut s = Stack { x: content.x, y: content.y, w: content.w };
    let normal = s.next(58.0);
    let peaceful = s.next(58.0);
    let start_normal = primary_button(&a, normal, "", true, &mut g.clickables);
    text(&a, "Normal", normal.x + space::L, normal.y + 8.0, TextSize::M, theme::color(theme::BG));
    text(&a, "Hives wake up when smoke reaches them", normal.x + space::L, normal.y + 31.0, TextSize::Xs, theme::alpha(theme::BG, 0.85));
    let start_peaceful = button(&a, peaceful, "", true, &mut g.clickables);
    text(&a, "Peaceful", peaceful.x + space::L, peaceful.y + 8.0, TextSize::M, TEXT);
    text(&a, "No attacks: just build (toggle any time with P)", peaceful.x + space::L, peaceful.y + 31.0, TextSize::Xs, TEXT_DIM);
    s.y += space::XS;
    let note = if g.has_game {
        "Your save files stay until you save over them."
    } else {
        "A fresh meadow, a few tools and a long way to go."
    };
    text(&a, note, content.x, s.y, TextSize::S, TEXT_DIM);
    if start_normal {
        g.new_game(false);
    } else if start_peaceful {
        g.new_game(true);
    } else if closed || is_key_pressed(KeyCode::Escape) {
        g.screen = Screen::Title(MenuPage::Main);
    }
}

/// Settings, shared by the title screen and the pause menu; `back` is where
/// Close and Esc go.
fn settings_page(g: &mut Game, top: f32, back: Screen) {
    let a = g.assets.clone();
    let r = Rect::new((ui_width() - 460.0) / 2.0, top, 460.0, 300.0);
    let (content, closed) = Panel::new(r).title("Settings").closable().show(g);
    let mut y = content.y;
    let mut s = g.settings;
    for (label, v) in [("Music", &mut s.music), ("Ambience", &mut s.ambience), ("Sound effects", &mut s.sfx)] {
        text(&a, label, content.x, y + 4.0, TextSize::M, TEXT);
        let pct = format!("{:.0}%", *v * 100.0);
        let pw = text_w(&a, &pct, TextSize::S);
        text(&a, &pct, content.right() - pw, y + 6.0, TextSize::S, KEY);
        let track = Rect::new(content.x + 140.0, y + 10.0, content.w - 140.0 - 56.0, 10.0);
        slider(g, track, v);
        y += 44.0;
    }
    let fs = Rect::new(content.x, y + 4.0, content.w, 40.0);
    let label = if s.fullscreen { "Fullscreen: on" } else { "Fullscreen: off" };
    if button(&a, fs, label, true, &mut g.clickables) {
        s.fullscreen = !s.fullscreen;
        set_fullscreen(s.fullscreen);
    }
    y += 56.0;
    text(&a, "M mutes the music at any time", content.x, y, TextSize::S, TEXT_DIM);
    if s != g.settings {
        g.apply_settings(s);
    }
    if closed || is_key_pressed(KeyCode::Escape) {
        if let Err(e) = g.settings.save() {
            g.toast(e);
        }
        g.screen = back;
    }
}

/// A horizontal slider for a 0..=1 value: drag or click anywhere on its track.
fn slider(g: &mut Game, track: Rect, v: &mut f32) {
    let hit = Rect::new(track.x - 8.0, track.y - 12.0, track.w + 16.0, track.h + 24.0);
    g.clickables.push(hit);
    if is_mouse_button_down(MouseButton::Left) && hovered(hit) {
        *v = ((ui_mouse().x - track.x) / track.w).clamp(0.0, 1.0);
    }
    rrect(track, track.h / 2.0, SLOT_WELL);
    let fill = Rect::new(track.x, track.y, (track.w * *v).max(track.h), track.h);
    rrect(fill, track.h / 2.0, theme::color(theme::LACQUER_500));
    rrect_lines(track, track.h / 2.0, line::HAIR, EDGE);
    let knob = vec2(track.x + track.w * *v, track.y + track.h / 2.0);
    let kr = if hovered(hit) { 10.0 } else { 9.0 };
    draw_circle(knob.x, knob.y + 1.5, kr, theme::alpha(theme::SHADE, 0.35));
    draw_circle(knob.x, knob.y, kr, theme::color(theme::BG));
    draw_circle_lines(knob.x, knob.y, kr, line::BASE, EDGE);
}

pub(super) fn pause(g: &mut Game, page: MenuPage) {
    let a = g.assets.clone();
    veil(0.8);
    let top = (ui_height() * 0.22).max(space::L);
    if page == MenuPage::Settings {
        settings_page(g, top, Screen::Paused(MenuPage::Main));
        return;
    }
    let r = Rect::new((ui_width() - CARD_W) / 2.0, top, CARD_W, 400.0);
    let (content, closed) = Panel::new(r).title("Paused").closable().show(g);
    let mut s = Stack { x: content.x, y: content.y, w: content.w };
    let resume = primary_button(&a, s.next(BTN_H), "Resume", true, &mut g.clickables);
    let save = button(&a, s.next(BTN_H), "Save game", true, &mut g.clickables);
    let settings = button(&a, s.next(BTN_H), "Settings", true, &mut g.clickables);
    let menu = button(&a, s.next(BTN_H), "Main menu", true, &mut g.clickables);
    let quit = button(&a, s.next(BTN_H), "Save and quit", true, &mut g.clickables);
    let hint = "Esc to resume · F5 saves at any time";
    let hw = text_w(&a, hint, TextSize::Xs);
    text(&a, hint, r.x + (r.w - hw) / 2.0, s.y + 2.0, TextSize::Xs, TEXT_DIM);

    if resume || closed || is_key_pressed(KeyCode::Escape) {
        g.resume();
    } else if save {
        g.save_game();
    } else if settings {
        g.screen = Screen::Paused(MenuPage::Settings);
    } else if menu {
        g.to_title();
    } else if quit {
        g.save_game();
        g.quit = true;
    }
}

// Release builds on Windows open no console window behind the game.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod assets;
mod assets_list;
mod audio;
mod audio_list;
mod cursor;
mod game;
mod input;
mod items;
mod render;
mod save;
mod settings;
mod theme;
mod ui;
mod world;

pub use game::{Game, Panel};
use game::{MenuPage, Screen};
use settings::Settings;

use std::rc::Rc;

use macroquad::prelude::*;

use assets::Assets;
use render::DAY_LENGTH;
use world::{Kind, State, World};

const TICK: f32 = 1.0 / 60.0;

fn conf() -> Conf {
    let dim = |key: &str, default: i32| {
        std::env::var(key).ok().and_then(|v| v.parse().ok()).unwrap_or(default)
    };
    Conf {
        window_title: "Cogworks".to_owned(),
        window_width: dim("COGWORKS_WIDTH", 1440),
        window_height: dim("COGWORKS_HEIGHT", 900),
        window_resizable: true,
        // Dev aid: `COGWORKS_FULLSCREEN=1` forces fullscreen (e.g. for README screenshots).
        fullscreen: std::env::var("COGWORKS_FULLSCREEN").is_ok()
            || (Settings::load().fullscreen && std::env::var("COGWORKS_SCREENSHOT").is_err()),
        sample_count: 4,
        icon: Some(window_icon()),
        ..Default::default()
    }
}

/// The cog from the logo (see `tools/art/logo.py`), as the window icon.
fn window_icon() -> macroquad::miniquad::conf::Icon {
    fn rgba<const N: usize>(png: &[u8]) -> [u8; N] {
        let img = Image::from_file_with_format(png, Some(ImageFormat::Png)).expect("icon png");
        img.bytes.try_into().expect("icon size")
    }
    macroquad::miniquad::conf::Icon {
        small: rgba(include_bytes!("../assets/icon/icon_16.png")),
        medium: rgba(include_bytes!("../assets/icon/icon_32.png")),
        big: rgba(include_bytes!("../assets/icon/icon_64.png")),
    }
}

#[macroquad::main(conf)]
async fn main() {
    let seed = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0x9E37_79B9);
    let screenshot_mode = std::env::var("COGWORKS_SCREENSHOT").is_ok();
    // `cogworks <save file>` drops straight into that save. Otherwise the title
    // screen opens over the newest save (so Continue resumes it), or over a
    // fresh world when there is none yet.
    let mut load_error = None;
    let (world, has_game, screen) = match std::env::args().nth(1) {
        Some(path) => (save::load(&path).unwrap_or_else(|e| panic!("{e}")), true, Screen::Playing),
        None if screenshot_mode => (World::new(seed), false, Screen::Playing),
        None => match save::newest_save().map(save::load) {
            Some(Ok(w)) => (w, true, Screen::Title(MenuPage::Main)),
            Some(Err(e)) => {
                load_error = Some(e);
                (World::new(seed), false, Screen::Title(MenuPage::Main))
            }
            None => (World::new(seed), false, Screen::Title(MenuPage::Main)),
        },
    };
    let assets = Rc::new(Assets::load().await);
    let audio = audio::Audio::load().await;
    let mut game = Game::new(world, assets, audio, Settings::load());
    game.has_game = has_game;
    game.screen = screen;
    // A loaded game doesn't need the welcome note again.
    game.show_help = !has_game;
    if let Some(e) = load_error {
        game.toast(e);
    }
    // Dev aid: open a menu in screenshot mode.
    game.screen = match std::env::var("COGWORKS_SCREEN").as_deref() {
        Ok("title") => Screen::Title(MenuPage::Main),
        Ok("newgame") => Screen::Title(MenuPage::NewGame),
        Ok("settings") => Screen::Title(MenuPage::Settings),
        Ok("pause") => Screen::Paused(MenuPage::Main),
        _ => game.screen,
    };
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
    // Hide the OS cursor once, after the window opens; the cozy custom cursor
    // (Task 9) replaces it.
    show_mouse(false);
    // Dev aid (screenshot mode only): `COGWORKS_MOUSE=x,y` places the mouse at
    // logical UI coordinates to capture hover states, and `COGWORKS_MOUSE_DOWN=1`
    // fakes the left press for the cursor squash. Read in `src/ui/metrics.rs`
    // (`mouse_override`) and `src/cursor.rs` (`left_down_for_squash`); neither is
    // fed into gameplay input.
    let mut frames = 0;
    let mut acc = 0.0;
    loop {
        frames += 1;
        let dt = get_frame_time().min(0.1);
        match game.screen {
            Screen::Playing => game.input(dt),
            Screen::Title(_) => game.drift_camera(dt),
            Screen::Paused(_) => {}
        }
        if game.screen == Screen::Playing && !game.paused {
            acc += dt;
            while acc >= TICK {
                game.world.update(TICK);
                acc -= TICK;
            }
        }
        // Draw the robot between its last two ticks, so it glides instead of
        // stepping whenever a frame gets zero or two ticks.
        game.world.player.interpolate(acc / TICK);
        if !matches!(game.screen, Screen::Title(_)) {
            game.follow_camera(dt);
        }
        game.update(dt);
        game.draw();
        if let Some(path) = &screenshot
            && frames == 240
        {
            get_screen_data().export_png(path);
            break;
        }
        if game.quit {
            break;
        }
        next_frame().await;
    }
}

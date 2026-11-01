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
mod theme;
mod ui;
mod world;

pub use game::{Game, Panel};

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
        sample_count: 4,
        ..Default::default()
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
    let audio = audio::Audio::load().await;
    let mut game = Game::new(world, assets, audio);
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

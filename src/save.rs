use std::fs::{self, File};
use std::io::{BufReader, BufWriter, Write};
use std::time::SystemTime;

use crate::world::World;

pub const SAVE_PATH: &str = "cogworks_save.json";
/// Written automatically every [`AUTOSAVE_EVERY`] seconds of unpaused play, so a
/// manual F5 save is never overwritten behind the player's back.
pub const AUTOSAVE_PATH: &str = "cogworks_autosave.json";
pub const AUTOSAVE_EVERY: f32 = 180.0;

/// Writes to a temporary file first and renames it into place, so a crash or a
/// full disk mid-write can never leave a half-written save behind.
pub fn save(world: &mut World, path: &str) -> Result<(), String> {
    world.sync_pipes();
    let tmp = format!("{path}.tmp");
    let fail = |e: &dyn std::fmt::Display| format!("Save failed: {e}");
    let file = File::create(&tmp).map_err(|e| fail(&e))?;
    let mut out = BufWriter::new(file);
    serde_json::to_writer(&mut out, world).map_err(|e| fail(&e))?;
    out.flush().map_err(|e| fail(&e))?;
    drop(out);
    fs::rename(&tmp, path).map_err(|e| fail(&e))
}

/// The most recently written of the manual save and the autosave, if any exist.
pub fn newest_save() -> Option<&'static str> {
    let modified = |p: &str| fs::metadata(p).and_then(|m| m.modified()).ok();
    pick_newer(modified(SAVE_PATH), modified(AUTOSAVE_PATH))
}

fn pick_newer(manual: Option<SystemTime>, auto: Option<SystemTime>) -> Option<&'static str> {
    match (manual, auto) {
        (Some(m), Some(a)) => Some(if a > m { AUTOSAVE_PATH } else { SAVE_PATH }),
        (Some(_), None) => Some(SAVE_PATH),
        (None, Some(_)) => Some(AUTOSAVE_PATH),
        (None, None) => None,
    }
}

/// Advances the autosave timer by `dt` and reports whether a save is due now.
pub fn autosave_tick(timer: &mut f32, dt: f32) -> bool {
    *timer += dt;
    if *timer >= AUTOSAVE_EVERY {
        *timer = 0.0;
        true
    } else {
        false
    }
}

pub fn load(path: &str) -> Result<World, String> {
    let file = File::open(path).map_err(|e| format!("Load failed: {e}"))?;
    let mut world: World =
        serde_json::from_reader(BufReader::new(file)).map_err(|e| format!("Load failed: {e}"))?;
    world.after_load();
    Ok(world)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn loads_whichever_save_is_newer() {
        let t = SystemTime::UNIX_EPOCH + Duration::from_secs(1_000);
        let later = t + Duration::from_secs(5);
        assert_eq!(pick_newer(Some(t), Some(later)), Some(AUTOSAVE_PATH));
        assert_eq!(pick_newer(Some(later), Some(t)), Some(SAVE_PATH));
        assert_eq!(pick_newer(Some(t), Some(t)), Some(SAVE_PATH));
        assert_eq!(pick_newer(None, Some(t)), Some(AUTOSAVE_PATH));
        assert_eq!(pick_newer(Some(t), None), Some(SAVE_PATH));
        assert_eq!(pick_newer(None, None), None);
    }

    #[test]
    fn autosave_fires_once_per_interval() {
        let mut timer = 0.0;
        let fired = (0..(AUTOSAVE_EVERY as usize * 10 * 3)).filter(|_| autosave_tick(&mut timer, 0.1)).count();
        assert_eq!(fired, 3);
    }
}

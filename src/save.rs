use std::fs::{self, File};
use std::io::{BufReader, BufWriter, Write};
use std::time::SystemTime;

use crate::world::World;

pub const SAVE_PATH: &str = "cogworks_save.json";
/// Written automatically every [`AUTOSAVE_EVERY`] seconds of unpaused play, so a
/// manual F5 save is never overwritten behind the player's back.
pub const AUTOSAVE_PATH: &str = "cogworks_autosave.json";
/// The previous autosave, kept aside when a new game would overwrite it.
pub const PREV_AUTOSAVE_PATH: &str = "cogworks_autosave.prev.json";
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

/// The other save slot to try when `failed` (the newest save) won't load.
pub fn fallback_save(failed: &str) -> Option<&'static str> {
    match failed {
        SAVE_PATH => Some(AUTOSAVE_PATH),
        AUTOSAVE_PATH => Some(SAVE_PATH),
        _ => None,
    }
}

/// Renames any existing autosave aside so a new game doesn't destroy the only
/// copy of that progress. A previous `.prev.json` is replaced, so at most the
/// last-but-one autosave survives.
pub fn preserve_autosave() -> Result<(), String> {
    if fs::metadata(AUTOSAVE_PATH).is_err() {
        return Ok(());
    }
    // `fs::rename` won't replace an existing destination on Windows.
    let _ = fs::remove_file(PREV_AUTOSAVE_PATH);
    fs::rename(AUTOSAVE_PATH, PREV_AUTOSAVE_PATH)
        .map_err(|e| format!("Couldn't keep the previous autosave: {e}"))
}

/// A save that was loaded, and which slot it came from.
pub struct LoadedSave {
    pub world: World,
    /// The slot actually loaded.
    pub slot: &'static str,
    /// The newest slot, when it failed to load and `slot` is the fallback.
    pub failed: Option<&'static str>,
}

/// Loads the newest save; when it fails to load, tries the other slot. Callers
/// toast [`LoadedSave::slot`] (and can mention [`LoadedSave::failed`]) so the
/// player knows which game they got.
pub fn load_newest_or_fallback() -> Result<LoadedSave, String> {
    let newest = newest_save().ok_or_else(|| "No save yet: press F5 to save".to_string())?;
    match load(newest) {
        Ok(world) => Ok(LoadedSave { world, slot: newest, failed: None }),
        Err(err) => {
            let Some(other) = fallback_save(newest) else {
                return Err(err);
            };
            match load(other) {
                Ok(world) => Ok(LoadedSave { world, slot: other, failed: Some(newest) }),
                Err(other_err) => Err(format!("{err} · {other} also failed: {other_err}")),
            }
        }
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
    fn fallback_swaps_to_the_other_slot() {
        assert_eq!(fallback_save(SAVE_PATH), Some(AUTOSAVE_PATH));
        assert_eq!(fallback_save(AUTOSAVE_PATH), Some(SAVE_PATH));
        assert_eq!(fallback_save("anything_else.json"), None);
    }

    #[test]
    fn autosave_fires_once_per_interval() {
        let mut timer = 0.0;
        let fired = (0..(AUTOSAVE_EVERY as usize * 10 * 3)).filter(|_| autosave_tick(&mut timer, 0.1)).count();
        assert_eq!(fired, 3);
    }
}

use std::fs::{self, File};
use std::io::{BufWriter, Write};
use std::time::SystemTime;

use crate::world::World;

pub const SAVE_PATH: &str = "cogworks_save.json";
/// Written automatically every [`AUTOSAVE_EVERY`] seconds of unpaused play, so a
/// manual F5 save is never overwritten behind the player's back.
pub const AUTOSAVE_PATH: &str = "cogworks_autosave.json";
/// The previous autosave, kept aside when a new game would overwrite it.
pub const PREV_AUTOSAVE_PATH: &str = "cogworks_autosave.prev.json";
/// Where the emergency save lands when the simulation panics. Deliberately not
/// one of the normal slots, so a possibly broken state can't overwrite a good save.
pub const CRASH_SAVE_PATH: &str = "cogworks_crash_save.json";
pub const AUTOSAVE_EVERY: f32 = 180.0;

/// Writes to a temporary file first and renames it into place, so a crash or a
/// full disk mid-write can never leave a half-written save behind.
pub fn save(world: &mut World, path: &str) -> Result<(), String> {
    world.sync_pipes();
    let bytes = serde_json::to_vec(world).map_err(|e| format!("Save failed: {e}"))?;
    write_atomic(path, &bytes).map_err(|e| format!("Save failed: {e}"))
}

/// Writes `bytes` to `path` atomically: the bytes go to a `path.tmp`, which is
/// flushed and synced to disk before being renamed over `path`. A crash or a
/// full disk mid-write can never leave a half-written file behind, and no
/// `.tmp` is left on success.
pub fn write_atomic(path: &str, bytes: &[u8]) -> Result<(), String> {
    let tmp = format!("{path}.tmp");
    let file = File::create(&tmp).map_err(|e| e.to_string())?;
    let mut out = BufWriter::new(file);
    out.write_all(bytes).map_err(|e| e.to_string())?;
    out.flush().map_err(|e| e.to_string())?;
    out.get_ref().sync_all().map_err(|e| e.to_string())?;
    drop(out);
    fs::rename(&tmp, path).map_err(|e| e.to_string())
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
    // `fs::rename` replaces an existing destination on every platform, so a
    // failed rename leaves the previous backup untouched rather than deleted.
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
    /// Problems [`World::repair`] fixed on load, if any.
    pub notes: Vec<String>,
}

/// Loads the newest save; when it fails to load, tries the other slot. `Ok(None)`
/// means no save exists yet (callers decide what to toast, if anything). Callers
/// toast [`LoadedSave::slot`] (and can mention [`LoadedSave::failed`]) so the
/// player knows which game they got.
pub fn load_newest_or_fallback() -> Result<Option<LoadedSave>, String> {
    let Some(newest) = newest_save() else {
        return Ok(None);
    };
    match load(newest) {
        Ok((world, notes)) => Ok(Some(LoadedSave { world, notes, slot: newest, failed: None })),
        Err(err) => {
            let Some(other) = fallback_save(newest) else {
                return Err(err);
            };
            match load(other) {
                Ok((world, notes)) => {
                    Ok(Some(LoadedSave { world, notes, slot: other, failed: Some(newest) }))
                }
                // If the fallback slot is simply absent, the original error is
                // the whole story; don't pile an "also failed" note on top.
                Err(other_err) if fs::metadata(other).is_ok() => {
                    Err(format!("{err} · {other} also failed: {other_err}"))
                }
                Err(_) => Err(err),
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

pub fn load(path: &str) -> Result<(World, Vec<String>), String> {
    let json = fs::read_to_string(path).map_err(|e| format!("Load failed: {e}"))?;
    load_from_str(&json)
}

/// Deserializes, validates and repairs a save from `json`. Used by [`load`] and
/// by tests that don't want to touch the filesystem.
pub fn load_from_str(json: &str) -> Result<(World, Vec<String>), String> {
    let mut world: World = serde_json::from_str(json).map_err(|e| format!("Load failed: {e}"))?;
    let notes = world.repair().map_err(|e| format!("Load failed: {e}"))?;
    world.after_load();
    Ok((world, notes))
}

/// A one-line summary of what [`World::repair`] fixed, for a startup or F9
/// toast, or `None` when nothing was repaired.
pub fn repair_toast(notes: &[String]) -> Option<String> {
    let first = notes.first()?;
    let mut msg = format!("Save repaired: {first}");
    if notes.len() > 1 {
        msg.push_str(&format!(" and {} more", notes.len() - 1));
    }
    Some(msg)
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

    #[test]
    fn write_atomic_writes_replaces_and_leaves_no_tmp() {
        let dir = temp_dir("write_atomic");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("save.json");
        let path = path.to_str().unwrap();

        write_atomic(path, b"first").unwrap();
        assert_eq!(std::fs::read(path).unwrap(), b"first");
        assert!(!std::path::Path::new(&format!("{path}.tmp")).exists());

        // A second write replaces the file in place, still leaving no temp file.
        write_atomic(path, b"second, longer").unwrap();
        assert_eq!(std::fs::read(path).unwrap(), b"second, longer");
        assert!(!std::path::Path::new(&format!("{path}.tmp")).exists());

        std::fs::remove_dir_all(&dir).ok();
    }

    fn temp_dir(name: &str) -> std::path::PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        std::env::temp_dir().join(format!("cogworks_{name}_{}_{nanos}", std::process::id()))
    }
}

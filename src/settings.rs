//! Player settings (volumes, fullscreen), kept in `cogworks_settings.json` next
//! to the saves so they survive between sessions.

use std::fs;

use serde::{Deserialize, Serialize};

use crate::audio::Levels;
use crate::render::Look;

pub const SETTINGS_PATH: &str = "cogworks_settings.json";

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub music: f32,
    pub ambience: f32,
    pub sfx: f32,
    pub fullscreen: bool,
    pub look: Look,
    pub music_muted: bool,
}

impl Default for Settings {
    fn default() -> Self {
        let l = Levels::default();
        Settings {
            music: l.music,
            ambience: l.ambience,
            sfx: l.sfx,
            fullscreen: false,
            look: Look::default(),
            music_muted: false,
        }
    }
}

impl Settings {
    /// The saved settings, or the defaults when there are none (or they can't be read).
    pub fn load() -> Settings {
        Self::load_from(SETTINGS_PATH)
    }

    /// Loads settings from `path`, or returns the defaults when it's absent or
    /// unreadable. Split out from [`Settings::load`] so tests can use a temp dir.
    pub fn load_from(path: &str) -> Settings {
        fs::read_to_string(path)
            .ok()
            .and_then(|s| serde_json::from_str::<Settings>(&s).ok())
            .map(Settings::clamped)
            .unwrap_or_default()
    }

    pub fn save(&self) -> Result<(), String> {
        self.save_to(SETTINGS_PATH)
    }

    /// Writes settings atomically to `path`. Split out from [`Settings::save`] so
    /// tests can use a temp dir.
    pub fn save_to(&self, path: &str) -> Result<(), String> {
        let json = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
        crate::save::write_atomic(path, json.as_bytes())
            .map_err(|e| format!("Couldn't save settings: {e}"))
    }

    pub fn levels(&self) -> Levels {
        Levels { music: self.music, ambience: self.ambience, sfx: self.sfx }
    }

    fn clamped(mut self) -> Settings {
        for v in [&mut self.music, &mut self.ambience, &mut self.sfx] {
            *v = if v.is_finite() { v.clamp(0.0, 1.0) } else { 0.5 };
        }
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::Look;

    #[test]
    fn settings_round_trip_and_fill_missing_fields() {
        let s = Settings {
            music: 0.2,
            ambience: 0.0,
            sfx: 1.0,
            fullscreen: true,
            look: Look::Clean,
            music_muted: true,
        };
        let back: Settings = serde_json::from_str(&serde_json::to_string(&s).unwrap()).unwrap();
        assert_eq!(back, s);
        // An older file with only some fields keeps the defaults for the rest,
        // including the fields added later (look, music_muted).
        let partial: Settings = serde_json::from_str(r#"{"music": 0.1}"#).unwrap();
        assert_eq!(partial.music, 0.1);
        assert_eq!(partial.sfx, Settings::default().sfx);
        assert_eq!(partial.look, Look::default());
        assert!(!partial.music_muted);
        // Out-of-range values are pulled back into 0..=1.
        assert_eq!(Settings { music: 3.0, ..s }.clamped().music, 1.0);
    }

    #[test]
    fn save_to_and_load_from_round_trip_through_a_path() {
        let dir = std::env::temp_dir().join(format!(
            "cogworks_settings_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("settings.json");
        let path = path.to_str().unwrap();

        let s = Settings {
            music: 0.2,
            ambience: 0.0,
            sfx: 1.0,
            fullscreen: true,
            look: Look::Clean,
            music_muted: true,
        };
        s.save_to(path).unwrap();
        assert_eq!(Settings::load_from(path), s);
        assert!(!std::path::Path::new(&format!("{path}.tmp")).exists());

        std::fs::remove_dir_all(&dir).ok();
    }
}

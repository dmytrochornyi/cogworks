//! Player settings (volumes, fullscreen), kept in `cogworks_settings.json` next
//! to the saves so they survive between sessions.

use std::fs;

use serde::{Deserialize, Serialize};

use crate::audio::Levels;

pub const SETTINGS_PATH: &str = "cogworks_settings.json";

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub music: f32,
    pub ambience: f32,
    pub sfx: f32,
    pub fullscreen: bool,
}

impl Default for Settings {
    fn default() -> Self {
        let l = Levels::default();
        Settings { music: l.music, ambience: l.ambience, sfx: l.sfx, fullscreen: false }
    }
}

impl Settings {
    /// The saved settings, or the defaults when there are none (or they can't be read).
    pub fn load() -> Settings {
        fs::read_to_string(SETTINGS_PATH)
            .ok()
            .and_then(|s| serde_json::from_str::<Settings>(&s).ok())
            .map(Settings::clamped)
            .unwrap_or_default()
    }

    pub fn save(&self) -> Result<(), String> {
        let json = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
        fs::write(SETTINGS_PATH, json).map_err(|e| format!("Couldn't save settings: {e}"))
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

    #[test]
    fn settings_round_trip_and_fill_missing_fields() {
        let s = Settings { music: 0.2, ambience: 0.0, sfx: 1.0, fullscreen: true };
        let back: Settings = serde_json::from_str(&serde_json::to_string(&s).unwrap()).unwrap();
        assert_eq!(back, s);
        // An older file with only some fields keeps the defaults for the rest.
        let partial: Settings = serde_json::from_str(r#"{"music": 0.1}"#).unwrap();
        assert_eq!(partial.music, 0.1);
        assert_eq!(partial.sfx, Settings::default().sfx);
        // Out-of-range values are pulled back into 0..=1.
        assert_eq!(Settings { music: 3.0, ..s }.clamped().music, 1.0);
    }
}

//! Global (not per-project) user settings stored in `Documents/Nova/settings.json`.
//!
//! Only ever touched from the GUI thread / plugin construction, never from `process()`.
//! The file is versioned, written atomically and a missing or corrupt file simply yields the
//! defaults. Theme, glow and reduce-motion are placeholders until the redesign wires them up;
//! theme is deliberately NOT part of presets or projects.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

pub const SETTINGS_VERSION: u32 = 1;
pub const MIN_WINDOW: (u32, u32) = (960, 600);
pub const MAX_WINDOW: (u32, u32) = (7680, 4320);
pub const SETTINGS_DIR_ENV: &str = "NOVA_SETTINGS_DIR";

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub settings_version: u32,
    /// Default GUI scale for new instances (0.75..=2.0).
    pub gui_scale: f32,
    /// Last-used window size in logical points; used for new instances.
    pub window_width: u32,
    pub window_height: u32,
    /// Placeholder: "aurora", "midnight_neon" or "arctic" once the redesign lands.
    pub theme: String,
    /// Placeholder: glow intensity, 0.0..=1.5.
    pub glow: f32,
    /// Placeholder: disables decorative animation.
    pub reduce_motion: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            settings_version: SETTINGS_VERSION,
            gui_scale: 1.0,
            window_width: 1240,
            window_height: 780,
            theme: String::from("aurora"),
            glow: 1.0,
            reduce_motion: false,
        }
    }
}

impl Settings {
    /// Clamps everything into valid ranges (also repairs NaN and absurd values).
    pub fn sanitize(&mut self) {
        let finite = |v: f32, default: f32| if v.is_finite() { v } else { default };
        self.gui_scale = finite(self.gui_scale, 1.0).clamp(0.75, 2.0);
        self.glow = finite(self.glow, 1.0).clamp(0.0, 1.5);
        self.window_width = self.window_width.clamp(MIN_WINDOW.0, MAX_WINDOW.0);
        self.window_height = self.window_height.clamp(MIN_WINDOW.1, MAX_WINDOW.1);
        if self.theme.is_empty() || self.theme.len() > 32 {
            self.theme = String::from("aurora");
        }
        self.settings_version = SETTINGS_VERSION;
    }

    /// Parses settings text; any error gives defaults.
    pub fn from_json(text: &str) -> Self {
        let mut settings: Settings = serde_json::from_str(text).unwrap_or_default();
        settings.sanitize();
        settings
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_else(|_| String::from("{}"))
    }
}

/// `Documents/Nova` (override with the NOVA_SETTINGS_DIR environment variable).
pub fn settings_dir() -> Option<PathBuf> {
    if let Some(dir) = std::env::var_os(SETTINGS_DIR_ENV) {
        return Some(PathBuf::from(dir));
    }
    let home = std::env::var_os("USERPROFILE").or_else(|| std::env::var_os("HOME"))?;
    Some(PathBuf::from(home).join("Documents").join("Nova"))
}

pub fn settings_path() -> Option<PathBuf> {
    settings_dir().map(|dir| dir.join("settings.json"))
}

/// Loads the settings file; creates it with defaults when missing. Never fails.
pub fn load_from(path: &Path) -> Settings {
    match std::fs::read_to_string(path) {
        Ok(text) => {
            let settings = Settings::from_json(&text);
            if serde_json::from_str::<Settings>(&text).is_err() {
                // Keep the broken file for inspection, then start fresh.
                let _ = std::fs::rename(path, path.with_extension("corrupt.json"));
                let _ = save_to(path, &settings);
            }
            settings
        }
        Err(_) => {
            let settings = Settings::default();
            let _ = save_to(path, &settings);
            settings
        }
    }
}

/// Writes atomically: temp file in the same folder, then rename over the target.
pub fn save_to(path: &Path, settings: &Settings) -> std::io::Result<()> {
    let dir = path.parent().unwrap_or_else(|| Path::new("."));
    std::fs::create_dir_all(dir)?;
    let temp = path.with_extension("json.tmp");
    std::fs::write(&temp, settings.to_json())?;
    std::fs::rename(&temp, path)
}

pub fn load() -> Settings {
    settings_path().map_or_else(Settings::default, |path| load_from(&path))
}

pub fn save(settings: &Settings) {
    if let Some(path) = settings_path() {
        let _ = save_to(&path, settings);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_path(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("nova-settings-test-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        dir.join("Nova").join("settings.json")
    }

    #[test]
    fn missing_file_is_created_with_defaults() {
        let path = temp_path("missing");
        let settings = load_from(&path);
        assert_eq!(settings, Settings::default());
        assert!(path.exists());
    }

    #[test]
    fn save_and_load_round_trip() {
        let path = temp_path("roundtrip");
        let settings = Settings {
            gui_scale: 1.5,
            window_width: 1400,
            window_height: 900,
            ..Settings::default()
        };
        save_to(&path, &settings).unwrap();
        assert_eq!(load_from(&path), settings);
        assert!(!path.with_extension("json.tmp").exists());
    }

    #[test]
    fn corrupt_file_gives_defaults_and_is_repaired() {
        let path = temp_path("corrupt");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, "{ not json at all").unwrap();
        assert_eq!(load_from(&path), Settings::default());
        assert!(serde_json::from_str::<Settings>(&std::fs::read_to_string(&path).unwrap()).is_ok());
        assert!(path.with_extension("corrupt.json").exists());
    }

    #[test]
    fn out_of_range_values_are_clamped() {
        let settings = Settings::from_json(
            r#"{"gui_scale": 9.0, "window_width": 10, "window_height": 99999, "glow": -3.0, "future_field": 1}"#,
        );
        assert_eq!(settings.gui_scale, 2.0);
        assert_eq!(settings.window_width, MIN_WINDOW.0);
        assert_eq!(settings.window_height, MAX_WINDOW.1);
        assert_eq!(settings.glow, 0.0);
    }
}

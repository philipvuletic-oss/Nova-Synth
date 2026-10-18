//! Versioned GUI state used by M0; future project data will extend this state structure.

use serde::{Deserialize, Serialize};

pub const CURRENT_STATE_VERSION: u32 = 2;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct VersionedState {
    pub state_version: u32,
    pub gui_scale: f32,
    pub selected_wavetable: String,
    pub imported_wavetable_name: Option<String>,
    pub imported_wavetable_source: Option<String>,
    pub imported_wavetable_frames: Option<Vec<Vec<i16>>>,
}

impl Default for VersionedState {
    fn default() -> Self {
        Self {
            state_version: 0,
            gui_scale: 1.0,
            selected_wavetable: String::from("Basic Shapes"),
            imported_wavetable_name: None,
            imported_wavetable_source: None,
            imported_wavetable_frames: None,
        }
    }
}

impl VersionedState {
    pub fn migrate(&mut self) {
        if !self.gui_scale.is_finite() {
            self.gui_scale = 1.0;
        }
        self.gui_scale = self.gui_scale.clamp(0.75, 2.0);
        if self.selected_wavetable.is_empty() {
            self.selected_wavetable = String::from("Basic Shapes");
        }
        if let Some(frames) = self.imported_wavetable_frames.as_mut() {
            frames.truncate(256);
            for frame in frames.iter_mut() {
                frame.truncate(2048);
            }
            if frames.is_empty() || frames.iter().any(|frame| frame.len() != 2048) {
                self.imported_wavetable_frames = None;
                self.imported_wavetable_name = None;
                self.imported_wavetable_source = None;
            }
        }
        if self.state_version < CURRENT_STATE_VERSION {
            self.state_version = CURRENT_STATE_VERSION;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migration_adds_current_version_and_clamps_scale() {
        let mut state = VersionedState {
            state_version: 0,
            gui_scale: 4.0,
            ..VersionedState::default()
        };
        state.migrate();
        assert_eq!(state.state_version, CURRENT_STATE_VERSION);
        assert_eq!(state.gui_scale, 2.0);
    }

    #[test]
    fn older_serialized_state_uses_defaults_for_new_fields() {
        let mut state: VersionedState =
            serde_json::from_str(r#"{"state_version":0}"#).expect("valid old state");
        state.migrate();
        assert_eq!(state.state_version, CURRENT_STATE_VERSION);
        assert_eq!(state.gui_scale, 1.0);
    }
}

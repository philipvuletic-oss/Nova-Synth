//! Versioned GUI state used by M0; future project data will extend this state structure.

use serde::{Deserialize, Serialize};

pub const CURRENT_STATE_VERSION: u32 = 1;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct VersionedState {
    pub state_version: u32,
    pub gui_scale: f32,
}

impl Default for VersionedState {
    fn default() -> Self {
        Self {
            state_version: 0,
            gui_scale: 1.0,
        }
    }
}

impl VersionedState {
    pub fn migrate(&mut self) {
        if !self.gui_scale.is_finite() {
            self.gui_scale = 1.0;
        }
        self.gui_scale = self.gui_scale.clamp(0.75, 2.0);
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

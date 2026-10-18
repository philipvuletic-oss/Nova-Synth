use nih_plug::prelude::*;
use nih_plug_egui::EguiState;
use std::sync::Arc;

pub mod envelope;
pub mod global;

use crate::state::persisted::VersionedState;
use envelope::Env1Params;
use global::GlobalParams;

#[derive(Params)]
pub struct NovaParams {
    #[persist = "editor-state"]
    pub editor_state: Arc<EguiState>,

    #[id = "master_volume"]
    pub master_volume: FloatParam,

    #[nested(group = "env1")]
    pub env1: Env1Params,

    #[nested(group = "global")]
    pub global: GlobalParams,

    #[persist = "nova-state"]
    pub persisted: Arc<std::sync::Mutex<VersionedState>>,
}

impl NovaParams {
    pub fn new() -> Self {
        Self {
            editor_state: EguiState::from_size(1240, 780),
            master_volume: FloatParam::new(
                "Master Volume",
                util::db_to_gain(-6.0),
                FloatRange::Skewed {
                    min: util::db_to_gain(-60.0),
                    max: util::db_to_gain(6.0),
                    factor: FloatRange::gain_skew_factor(-60.0, 6.0),
                },
            )
            .with_smoother(SmoothingStyle::Linear(20.0))
            .with_step_size(0.01)
            .with_unit(" dB")
            .with_value_to_string(Arc::new(|gain| {
                if gain <= util::db_to_gain(-59.9) {
                    String::from("-inf")
                } else {
                    format!("{:.1}", util::gain_to_db(gain))
                }
            }))
            .with_string_to_value(Arc::new(|value| {
                if value.trim().starts_with("-inf") {
                    Some(util::db_to_gain(-60.0))
                } else {
                    value
                        .trim_end_matches([' ', 'd', 'D', 'b', 'B'])
                        .parse::<f32>()
                        .ok()
                        .map(util::db_to_gain)
                }
            })),
            env1: Env1Params::default(),
            global: GlobalParams::default(),
            persisted: Arc::new(std::sync::Mutex::new(VersionedState::default())),
        }
    }
}

impl Default for NovaParams {
    fn default() -> Self {
        Self::new()
    }
}

impl NovaParams {
    pub fn reset_smoothers(&self) {
        self.master_volume
            .smoothed
            .reset(self.master_volume.value());
        self.env1.reset_smoothers();
        self.global.reset_smoothers();
    }
}

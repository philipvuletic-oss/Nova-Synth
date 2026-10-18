use nih_plug::prelude::*;
use nih_plug_egui::EguiState;
use std::sync::Arc;

pub mod envelope;
pub mod global;

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
            .with_unit(" dB")
            .with_value_to_string(formatters::v2s_f32_gain_to_db(1))
            .with_string_to_value(formatters::s2v_f32_gain_to_db()),
            env1: Env1Params::default(),
            global: GlobalParams::default(),
        }
    }
}

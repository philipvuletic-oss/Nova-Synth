use nih_plug::prelude::*;
use std::sync::Arc;

#[derive(Params)]
pub struct OscillatorParams {
    #[id = "osc_a_enable"]
    pub enable: BoolParam,
    #[id = "osc_a_wt_position"]
    pub wt_position: FloatParam,
    #[id = "osc_a_interp_mode"]
    pub interp_mode: IntParam,
    #[id = "osc_a_level"]
    pub level: FloatParam,
    #[id = "osc_a_pan"]
    pub pan: FloatParam,
    #[id = "osc_a_phase"]
    pub phase: FloatParam,
    #[id = "osc_a_phase_rand"]
    pub phase_rand: FloatParam,
    #[id = "osc_a_phase_memory"]
    pub phase_memory: BoolParam,
}

impl Default for OscillatorParams {
    fn default() -> Self {
        Self {
            enable: BoolParam::new("Osc A Enable", true),
            wt_position: FloatParam::new(
                "WT Position",
                0.0,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_smoother(SmoothingStyle::Linear(20.0))
            .with_unit("%")
            .with_value_to_string(formatters::v2s_f32_percentage(0))
            .with_string_to_value(formatters::s2v_f32_percentage()),
            interp_mode: IntParam::new("WT Interpolation", 0, IntRange::Linear { min: 0, max: 2 })
                .with_value_to_string(Arc::new(|value| {
                    match value {
                        0 => "Frames",
                        1 => "Smooth",
                        _ => "Step",
                    }
                    .to_string()
                })),
            level: FloatParam::new(
                "Osc A Level",
                0.75,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_smoother(SmoothingStyle::Linear(20.0))
            .with_unit("%")
            .with_value_to_string(formatters::v2s_f32_percentage(0))
            .with_string_to_value(formatters::s2v_f32_percentage()),
            pan: FloatParam::new(
                "Osc A Pan",
                0.0,
                FloatRange::Linear {
                    min: -1.0,
                    max: 1.0,
                },
            )
            .with_smoother(SmoothingStyle::Linear(20.0))
            .with_unit("%")
            .with_value_to_string(formatters::v2s_f32_percentage(0))
            .with_string_to_value(formatters::s2v_f32_percentage()),
            phase: FloatParam::new(
                "Phase",
                0.0,
                FloatRange::Linear {
                    min: 0.0,
                    max: 360.0,
                },
            )
            .with_unit("°")
            .with_value_to_string(formatters::v2s_f32_rounded(0)),
            phase_rand: FloatParam::new(
                "Phase Randomization",
                0.0,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_unit("%")
            .with_value_to_string(formatters::v2s_f32_percentage(0))
            .with_string_to_value(formatters::s2v_f32_percentage()),
            phase_memory: BoolParam::new("Phase Memory", false),
        }
    }
}

impl OscillatorParams {
    pub fn reset_smoothers(&self) {
        self.wt_position.smoothed.reset(self.wt_position.value());
        self.level.smoothed.reset(self.level.value());
        self.pan.smoothed.reset(self.pan.value());
    }
}

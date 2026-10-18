use nih_plug::prelude::*;
use std::sync::Arc;

#[derive(Params)]
pub struct GlobalParams {
    #[id = "velocity_sensitivity"]
    pub velocity_sensitivity: FloatParam,
    #[id = "polyphony"]
    pub polyphony: IntParam,
    #[id = "pitchbend_range"]
    pub pitchbend_range: FloatParam,
    #[id = "a4_tuning"]
    pub a4_tuning: FloatParam,
    #[id = "safety_clipper"]
    pub safety_clipper: BoolParam,
    #[id = "gui_scale"]
    pub gui_scale: FloatParam,
}

impl Default for GlobalParams {
    fn default() -> Self {
        Self::with_gui_scale(1.0)
    }
}

impl GlobalParams {
    /// `default_gui_scale` comes from the global settings file for new plugin instances.
    pub fn with_gui_scale(default_gui_scale: f32) -> Self {
        Self {
            velocity_sensitivity: FloatParam::new(
                "Velocity Sensitivity",
                0.7,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_smoother(SmoothingStyle::Linear(20.0))
            .with_unit("%")
            .with_value_to_string(formatters::v2s_f32_percentage(0))
            .with_string_to_value(formatters::s2v_f32_percentage()),
            polyphony: IntParam::new("Polyphony", 16, IntRange::Linear { min: 1, max: 32 }),
            pitchbend_range: FloatParam::new(
                "Pitch Bend Range",
                2.0,
                FloatRange::Linear {
                    min: 0.0,
                    max: 48.0,
                },
            )
            .with_unit(" st")
            .with_value_to_string(formatters::v2s_f32_rounded(0)),
            a4_tuning: FloatParam::new(
                "A4 Tuning",
                440.0,
                FloatRange::Linear {
                    min: 415.0,
                    max: 466.0,
                },
            )
            .with_unit(" Hz")
            .with_value_to_string(formatters::v2s_f32_rounded(1))
            .with_string_to_value(formatters::s2v_f32_hz_then_khz()),
            safety_clipper: BoolParam::new("Safety Clipper", true),
            gui_scale: FloatParam::new(
                "GUI Scale",
                default_gui_scale.clamp(0.75, 2.0),
                FloatRange::Linear {
                    min: 0.75,
                    max: 2.0,
                },
            )
            .with_step_size(0.25)
            .with_unit("%")
            .with_value_to_string(Arc::new(|value| format!("{:.0}%", value * 100.0)))
            .with_string_to_value(Arc::new(|value| {
                value
                    .trim_end_matches('%')
                    .trim()
                    .parse::<f32>()
                    .ok()
                    .map(|value| value / 100.0)
            })),
        }
    }
}

impl GlobalParams {
    pub fn reset_smoothers(&self) {
        self.velocity_sensitivity
            .smoothed
            .reset(self.velocity_sensitivity.value());
    }
}

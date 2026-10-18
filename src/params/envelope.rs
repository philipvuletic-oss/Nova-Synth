use nih_plug::prelude::*;
use std::sync::Arc;

#[derive(Params)]
pub struct Env1Params {
    #[id = "env1_delay"]
    pub delay: FloatParam,
    #[id = "env1_attack"]
    pub attack: FloatParam,
    #[id = "env1_hold"]
    pub hold: FloatParam,
    #[id = "env1_decay"]
    pub decay: FloatParam,
    #[id = "env1_sustain"]
    pub sustain: FloatParam,
    #[id = "env1_release"]
    pub release: FloatParam,
    #[id = "env1_attack_curve"]
    pub attack_curve: FloatParam,
    #[id = "env1_decay_curve"]
    pub decay_curve: FloatParam,
    #[id = "env1_release_curve"]
    pub release_curve: FloatParam,
}

impl Default for Env1Params {
    fn default() -> Self {
        Self {
            delay: seconds("Delay", "env1_delay", 0.0, 0.0, 5.0),
            attack: seconds("Attack", "env1_attack", 0.005, 0.001, 10.0),
            hold: seconds("Hold", "env1_hold", 0.0, 0.0, 5.0),
            decay: seconds("Decay", "env1_decay", 0.25, 0.001, 10.0),
            sustain: FloatParam::new("Sustain", 0.7, FloatRange::Linear { min: 0.0, max: 1.0 })
                .with_unit("%")
                .with_value_to_string(formatters::v2s_f32_percentage(0))
                .with_string_to_value(formatters::s2v_f32_percentage()),
            release: seconds("Release", "env1_release", 0.3, 0.001, 10.0),
            attack_curve: curve("Attack Curve", "env1_attack_curve"),
            decay_curve: curve("Decay Curve", "env1_decay_curve"),
            release_curve: curve("Release Curve", "env1_release_curve"),
        }
    }
}

fn seconds(name: &'static str, _id: &'static str, default: f32, min: f32, max: f32) -> FloatParam {
    FloatParam::new(
        name,
        default,
        FloatRange::Skewed {
            min,
            max,
            factor: FloatRange::skew_factor(-2.0),
        },
    )
    .with_unit(" s")
    .with_value_to_string(formatters::v2s_f32_rounded(3))
    .with_string_to_value(Arc::new(|value| {
        value
            .trim()
            .trim_end_matches('s')
            .trim()
            .parse::<f32>()
            .ok()
    }))
}

fn curve(name: &'static str, _id: &'static str) -> FloatParam {
    FloatParam::new(
        name,
        0.0,
        FloatRange::Linear {
            min: -1.0,
            max: 1.0,
        },
    )
}

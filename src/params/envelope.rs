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

#[derive(Clone, Copy, Debug)]
pub struct EnvelopeValues {
    pub delay: f32,
    pub attack: f32,
    pub hold: f32,
    pub decay: f32,
    pub sustain: f32,
    pub release: f32,
    pub attack_curve: f32,
    pub decay_curve: f32,
    pub release_curve: f32,
}

impl Env1Params {
    pub fn next_values(&self) -> EnvelopeValues {
        EnvelopeValues {
            delay: self.delay.smoothed.next(),
            attack: self.attack.smoothed.next(),
            hold: self.hold.smoothed.next(),
            decay: self.decay.smoothed.next(),
            sustain: self.sustain.smoothed.next(),
            release: self.release.smoothed.next(),
            attack_curve: self.attack_curve.smoothed.next(),
            decay_curve: self.decay_curve.smoothed.next(),
            release_curve: self.release_curve.smoothed.next(),
        }
    }

    pub fn reset_smoothers(&self) {
        self.delay.smoothed.reset(self.delay.value());
        self.attack.smoothed.reset(self.attack.value());
        self.hold.smoothed.reset(self.hold.value());
        self.decay.smoothed.reset(self.decay.value());
        self.sustain.smoothed.reset(self.sustain.value());
        self.release.smoothed.reset(self.release.value());
        self.attack_curve.smoothed.reset(self.attack_curve.value());
        self.decay_curve.smoothed.reset(self.decay_curve.value());
        self.release_curve
            .smoothed
            .reset(self.release_curve.value());
    }
}

impl Default for Env1Params {
    fn default() -> Self {
        Self {
            delay: seconds("Delay", 0.0, 0.0, 5.0),
            attack: seconds("Attack", 0.005, 0.001, 10.0),
            hold: seconds("Hold", 0.0, 0.0, 5.0),
            decay: seconds("Decay", 0.25, 0.001, 10.0),
            sustain: FloatParam::new("Sustain", 0.7, FloatRange::Linear { min: 0.0, max: 1.0 })
                .with_smoother(SmoothingStyle::Linear(20.0))
                .with_unit("%")
                .with_value_to_string(formatters::v2s_f32_percentage(0))
                .with_string_to_value(formatters::s2v_f32_percentage()),
            release: seconds("Release", 0.3, 0.001, 10.0),
            attack_curve: curve("Attack Curve"),
            decay_curve: curve("Decay Curve"),
            release_curve: curve("Release Curve"),
        }
    }
}

fn seconds(name: &'static str, default: f32, min: f32, max: f32) -> FloatParam {
    let range = if min == 0.0 {
        FloatRange::Linear { min, max }
    } else {
        FloatRange::Skewed {
            min,
            max,
            factor: FloatRange::skew_factor(-2.0),
        }
    };
    FloatParam::new(name, default, range)
        .with_smoother(SmoothingStyle::Linear(20.0))
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

fn curve(name: &'static str) -> FloatParam {
    FloatParam::new(
        name,
        0.0,
        FloatRange::Linear {
            min: -1.0,
            max: 1.0,
        },
    )
    .with_smoother(SmoothingStyle::Linear(20.0))
}

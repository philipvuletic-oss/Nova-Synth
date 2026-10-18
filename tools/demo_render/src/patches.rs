//! Named demo patches: a Rust builder for parameter values plus the persisted wavetable choice.
//! Only features that exist today are used; every new feature must add or extend a patch here.

use nih_plug::prelude::*;
use nova::params::NovaParams;

pub struct Patch {
    pub name: &'static str,
    pub description: &'static str,
    build: fn(&mut PatchBuilder),
}

pub struct PatchBuilder {
    params: NovaParams,
}

impl PatchBuilder {
    pub fn wavetable(&mut self, name: &str) -> &mut Self {
        if let Ok(mut state) = self.params.persisted.lock() {
            state.selected_wavetable = name.to_string();
        }
        self
    }

    pub fn float(
        &mut self,
        select: fn(&mut NovaParams) -> &mut FloatParam,
        plain: f32,
    ) -> &mut Self {
        let param = select(&mut self.params);
        let range = param.range();
        let (min, max) = match range {
            FloatRange::Linear { min, max }
            | FloatRange::Skewed { min, max, .. }
            | FloatRange::SymmetricalSkewed { min, max, .. } => (min, max),
            _ => (f32::MIN, f32::MAX),
        };
        let name = param.name().to_string();
        let unit = param.unit();
        *param = FloatParam::new(name, plain.clamp(min, max), range)
            .with_smoother(SmoothingStyle::None)
            .with_unit(unit);
        self
    }

    pub fn interpolation(&mut self, mode: i32) -> &mut Self {
        let name = self.params.osc_a.interp_mode.name().to_string();
        self.params.osc_a.interp_mode =
            IntParam::new(name, mode, IntRange::Linear { min: 0, max: 2 });
        self
    }
}

macro_rules! f {
    ($b:expr, $($path:ident).+, $v:expr) => {
        $b.float(|p| &mut p.$($path).+, $v)
    };
}

pub const PATCHES: &[Patch] = &[
    Patch {
        name: "Init",
        description: "Factory default patch (Basic Shapes)",
        build: |_| {},
    },
    Patch {
        name: "Supersaw",
        description:
            "Analog Saw Stack table, fast attack, long release (single oscillator, no unison yet)",
        build: |b| {
            b.wavetable("Analog Saw Stack");
            f!(b, env1.attack, 0.01);
            f!(b, env1.release, 0.8);
            f!(b, master_volume, util::db_to_gain(-9.0));
        },
    },
    Patch {
        name: "Reese Bass",
        description: "Supersaw Morph table, sustained, mid-table position",
        build: |b| {
            b.wavetable("Supersaw Morph");
            f!(b, osc_a.wt_position, 0.45);
            f!(b, env1.attack, 0.005);
            f!(b, env1.release, 0.15);
            f!(b, master_volume, util::db_to_gain(-9.0));
        },
    },
    Patch {
        name: "Pluck",
        description: "Pluck Spectrum Decay table with zero sustain",
        build: |b| {
            b.wavetable("Pluck Spectrum Decay");
            f!(b, env1.attack, 0.001);
            f!(b, env1.decay, 0.35);
            f!(b, env1.sustain, 0.0);
            f!(b, env1.release, 0.2);
        },
    },
    Patch {
        name: "Pad",
        description: "Spectral Noise Pad table, slow attack and release",
        build: |b| {
            b.wavetable("Spectral Noise Pad");
            f!(b, env1.attack, 1.2);
            f!(b, env1.release, 2.0);
            f!(b, osc_a.wt_position, 0.3);
            b.interpolation(1);
        },
    },
    Patch {
        name: "Lead",
        description: "PWM Square table, smooth interpolation, short release",
        build: |b| {
            b.wavetable("PWM Square");
            f!(b, env1.attack, 0.004);
            f!(b, env1.release, 0.12);
            f!(b, osc_a.wt_position, 0.5);
            b.interpolation(1);
        },
    },
    Patch {
        name: "FM Bell",
        description: "FM Bell table, percussive envelope",
        build: |b| {
            b.wavetable("FM Bell");
            f!(b, env1.attack, 0.001);
            f!(b, env1.decay, 1.5);
            f!(b, env1.sustain, 0.0);
            f!(b, env1.release, 1.0);
        },
    },
    Patch {
        name: "Acid",
        description: "Sync Sweep table, short decay (no filter yet - M5)",
        build: |b| {
            b.wavetable("Sync Sweep");
            f!(b, osc_a.wt_position, 0.6);
            f!(b, env1.attack, 0.001);
            f!(b, env1.decay, 0.18);
            f!(b, env1.sustain, 0.35);
            f!(b, env1.release, 0.05);
        },
    },
    Patch {
        name: "Growl",
        description: "FM Growl table, medium sustain",
        build: |b| {
            b.wavetable("FM Growl");
            f!(b, osc_a.wt_position, 0.7);
            f!(b, env1.attack, 0.003);
            f!(b, env1.release, 0.2);
            f!(b, master_volume, util::db_to_gain(-9.0));
        },
    },
    Patch {
        name: "Wide Chord",
        description: "Vocal Choir table panned off-centre (no unison/stereo FX yet)",
        build: |b| {
            b.wavetable("Vocal Choir");
            f!(b, osc_a.pan, 0.35);
            f!(b, env1.attack, 0.05);
            f!(b, env1.release, 0.6);
            f!(b, master_volume, util::db_to_gain(-9.0));
        },
    },
];

impl Patch {
    pub fn instantiate(&self) -> nova::Nova {
        let mut builder = PatchBuilder {
            params: NovaParams::new(),
        };
        (self.build)(&mut builder);
        builder.params.reset_smoothers();
        nova::Nova::with_params(builder.params)
    }
}

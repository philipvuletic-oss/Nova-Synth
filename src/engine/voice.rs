use crate::params::envelope::EnvelopeValues;
use std::sync::atomic::AtomicU32;

pub const MAX_VOICES: usize = 32;

pub struct VoiceRenderSettings<'a> {
    pub polyphony: usize,
    pub a4_tuning: f32,
    pub pitch_bend_range: f32,
    pub pitch_bend: &'a [f32; 16],
    pub envelope: EnvelopeValues,
    pub velocity_sensitivity: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum KeyboardCommand {
    NoteOn { note: u8, velocity: f32 },
    NoteOff { note: u8 },
    PitchBend { value: f32 },
    ModWheel { value: f32 },
}

#[derive(Default)]
pub struct AudioTelemetry {
    pub peak_left: AtomicU32,
    pub peak_right: AtomicU32,
    pub active_voices: AtomicU32,
    pub cpu_load: AtomicU32,
}

impl AudioTelemetry {
    pub fn cpu_load(&self) -> f32 {
        f32::from_bits(self.cpu_load.load(std::sync::atomic::Ordering::Relaxed))
    }
    pub fn peak_left(&self) -> f32 {
        f32::from_bits(self.peak_left.load(std::sync::atomic::Ordering::Relaxed))
    }
    pub fn peak_right(&self) -> f32 {
        f32::from_bits(self.peak_right.load(std::sync::atomic::Ordering::Relaxed))
    }
    pub fn active_voices(&self) -> u32 {
        self.active_voices
            .load(std::sync::atomic::Ordering::Relaxed)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Stage {
    Off,
    Delay,
    Attack,
    Hold,
    Decay,
    Sustain,
    Release,
}

#[derive(Clone, Copy)]
struct Envelope {
    stage: Stage,
    level: f32,
    start_level: f32,
    elapsed: f32,
}

impl Default for Envelope {
    fn default() -> Self {
        Self {
            stage: Stage::Off,
            level: 0.0,
            start_level: 0.0,
            elapsed: 0.0,
        }
    }
}

impl Envelope {
    fn trigger(&mut self, delay: f32) {
        self.start_level = self.level;
        self.elapsed = 0.0;
        self.stage = if delay > 0.0 {
            Stage::Delay
        } else {
            Stage::Attack
        };
    }

    fn release(&mut self) {
        if self.stage != Stage::Off && self.stage != Stage::Release {
            self.start_level = self.level;
            self.elapsed = 0.0;
            self.stage = Stage::Release;
        }
    }

    fn next(&mut self, sample_rate: f32, params: EnvelopeValues) -> f32 {
        let dt = sample_rate.recip();
        let delay = params.delay.max(0.0);
        let attack = params.attack.max(dt);
        let hold = params.hold.max(0.0);
        let decay = params.decay.max(dt);
        let sustain = params.sustain.clamp(0.0, 1.0);
        let release = params.release.max(dt);

        match self.stage {
            Stage::Off => self.level = 0.0,
            Stage::Delay => {
                self.elapsed += dt;
                if self.elapsed >= delay {
                    self.elapsed = 0.0;
                    self.start_level = self.level;
                    self.stage = Stage::Attack;
                }
            }
            Stage::Attack => {
                self.elapsed += dt;
                let position = (self.elapsed / attack).clamp(0.0, 1.0);
                self.level = self.start_level
                    + (1.0 - self.start_level) * shape(position, params.attack_curve);
                if position >= 1.0 {
                    self.level = 1.0;
                    self.elapsed = 0.0;
                    self.stage = if hold > 0.0 {
                        Stage::Hold
                    } else {
                        Stage::Decay
                    };
                }
            }
            Stage::Hold => {
                self.elapsed += dt;
                if self.elapsed >= hold {
                    self.elapsed = 0.0;
                    self.stage = Stage::Decay;
                }
            }
            Stage::Decay => {
                self.elapsed += dt;
                let position = (self.elapsed / decay).clamp(0.0, 1.0);
                self.level = 1.0 + (sustain - 1.0) * shape(position, params.decay_curve);
                if position >= 1.0 {
                    self.level = sustain;
                    self.stage = Stage::Sustain;
                }
            }
            Stage::Sustain => self.level = sustain,
            Stage::Release => {
                self.elapsed += dt;
                let position = (self.elapsed / release).clamp(0.0, 1.0);
                self.level = self.start_level * (1.0 - shape(position, params.release_curve));
                if position >= 1.0 {
                    self.level = 0.0;
                    self.stage = Stage::Off;
                }
            }
        }
        if !self.level.is_finite() {
            *self = Self::default();
        }
        self.level.clamp(0.0, 1.0)
    }
}

fn shape(position: f32, curve: f32) -> f32 {
    let x = position.clamp(0.0, 1.0);
    let amount = curve.clamp(-1.0, 1.0);
    if amount.abs() < 0.001 {
        x
    } else if amount > 0.0 {
        x.powf(1.0 + amount * 4.0)
    } else {
        1.0 - (1.0 - x).powf(1.0 + amount.abs() * 4.0)
    }
}

#[derive(Clone, Copy)]
struct Voice {
    active: bool,
    note: u8,
    channel: u8,
    key_down: bool,
    sustained: bool,
    velocity: f32,
    pressure: f32,
    phase: f64,
    age: u64,
    envelope: Envelope,
}

impl Default for Voice {
    fn default() -> Self {
        Self {
            active: false,
            note: 0,
            channel: 0,
            key_down: false,
            sustained: false,
            velocity: 0.0,
            pressure: 1.0,
            phase: 0.0,
            age: 0,
            envelope: Envelope::default(),
        }
    }
}

pub struct VoiceManager {
    voices: [Voice; MAX_VOICES],
    sample_rate: f32,
    age: u64,
}

impl VoiceManager {
    pub fn new(sample_rate: f32) -> Self {
        Self {
            voices: [Voice::default(); MAX_VOICES],
            sample_rate: sample_rate.max(1.0),
            age: 0,
        }
    }

    pub fn set_sample_rate(&mut self, sample_rate: f32) {
        if sample_rate.is_finite() && sample_rate > 0.0 {
            self.sample_rate = sample_rate;
        }
    }

    pub fn reset(&mut self) {
        self.voices.fill(Voice::default());
        self.age = 0;
    }

    pub fn set_polyphony(&mut self, polyphony: usize) {
        let limit = polyphony.clamp(1, MAX_VOICES);
        self.voices[limit..].fill(Voice::default());
    }

    pub fn note_on(
        &mut self,
        channel: u8,
        note: u8,
        velocity: f32,
        pressure: f32,
        delay: f32,
        polyphony: usize,
    ) -> usize {
        let limit = polyphony.clamp(1, MAX_VOICES);
        self.age = self.age.wrapping_add(1);
        let same_note = self.voices[..limit]
            .iter()
            .position(|voice| voice.active && voice.channel == channel && voice.note == note);
        let free = self.voices[..limit].iter().position(|voice| !voice.active);
        let selected = same_note
            .or(free)
            .unwrap_or_else(|| self.steal_candidate(limit));
        let voice = &mut self.voices[selected];
        voice.active = true;
        voice.note = note.min(127);
        voice.channel = channel.min(15);
        voice.key_down = true;
        voice.sustained = false;
        voice.velocity = velocity.clamp(0.0, 1.0);
        voice.pressure = pressure.clamp(0.0, 1.0);
        voice.age = self.age;
        voice.envelope.trigger(delay.max(0.0));
        selected
    }

    fn steal_candidate(&self, limit: usize) -> usize {
        let mut candidate = 0;
        let mut found_release = false;
        for index in 0..limit {
            let voice = &self.voices[index];
            let is_release = voice.envelope.stage == Stage::Release;
            if is_release && !found_release {
                candidate = index;
                found_release = true;
            } else if is_release == found_release {
                let current = &self.voices[candidate];
                let replace = if found_release {
                    voice.envelope.level < current.envelope.level
                        || (voice.envelope.level == current.envelope.level
                            && voice.age < current.age)
                } else {
                    voice.age < current.age
                };
                if replace {
                    candidate = index;
                }
            }
        }
        candidate
    }

    pub fn note_off(&mut self, channel: u8, note: u8, sustain: bool, _release_velocity: f32) {
        for voice in &mut self.voices {
            if voice.active && voice.channel == channel && voice.note == note && voice.key_down {
                voice.key_down = false;
                if sustain {
                    voice.sustained = true;
                } else {
                    voice.envelope.release();
                }
            }
        }
    }

    pub fn release_sustained(&mut self, channel: u8) {
        for voice in &mut self.voices {
            if voice.active && voice.channel == channel && voice.sustained {
                voice.sustained = false;
                voice.envelope.release();
            }
        }
    }

    pub fn choke(&mut self, channel: u8, note: u8) {
        for voice in &mut self.voices {
            if voice.active && voice.channel == channel && voice.note == note {
                *voice = Voice::default();
            }
        }
    }

    pub fn set_pressure(&mut self, channel: u8, note: u8, pressure: f32) {
        for voice in &mut self.voices {
            if voice.active && voice.channel == channel && voice.note == note {
                voice.pressure = pressure.clamp(0.0, 1.0);
            }
        }
    }

    pub fn render_sample(&mut self, settings: &VoiceRenderSettings<'_>) -> f32 {
        let limit = settings.polyphony.clamp(1, MAX_VOICES);
        let sensitivity = settings.velocity_sensitivity.clamp(0.0, 1.0);
        let mut output = 0.0f32;
        for voice in &mut self.voices[..limit] {
            if !voice.active {
                continue;
            }
            let bend = settings.pitch_bend[voice.channel as usize].clamp(-1.0, 1.0)
                * settings.pitch_bend_range.clamp(0.0, 48.0);
            let frequency = settings.a4_tuning.clamp(415.0, 466.0)
                * 2.0f32.powf((voice.note as f32 - 69.0 + bend) / 12.0);
            let amplitude = voice.envelope.next(self.sample_rate, settings.envelope);
            let velocity_gain = 1.0 - sensitivity + sensitivity * voice.velocity;
            let pressure_gain = 0.5 + voice.pressure * 0.5;
            let contribution = (voice.phase * std::f64::consts::TAU).sin() as f32
                * amplitude
                * velocity_gain
                * pressure_gain;
            voice.phase += frequency as f64 / self.sample_rate as f64;
            voice.phase -= voice.phase.floor();
            if !contribution.is_finite() || !voice.phase.is_finite() {
                *voice = Voice::default();
            } else if voice.envelope.stage == Stage::Off {
                voice.active = false;
            } else {
                output += contribution;
            }
        }
        output
    }

    pub fn active_voice_count(&self) -> usize {
        self.voices.iter().filter(|voice| voice.active).count()
    }

    pub fn note_is_active(&self, channel: u8, note: u8) -> bool {
        self.voices
            .iter()
            .any(|voice| voice.active && voice.channel == channel && voice.note == note)
    }

    #[cfg(test)]
    pub fn release_level(&self, index: usize) -> f32 {
        self.voices[index].envelope.level
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn manager() -> VoiceManager {
        VoiceManager::new(1_000.0)
    }

    #[test]
    fn release_phase_voice_is_stolen_before_held_voice() {
        let mut manager = manager();
        manager.note_on(0, 60, 1.0, 1.0, 0.0, 2);
        manager.note_on(0, 61, 1.0, 1.0, 0.0, 2);
        manager.note_off(0, 60, false, 0.5);
        manager.note_on(0, 62, 1.0, 1.0, 0.0, 2);

        assert!(!manager.note_is_active(0, 60));
        assert!(manager.note_is_active(0, 61));
        assert!(manager.note_is_active(0, 62));
    }

    #[test]
    fn quietest_release_voice_is_stolen_first() {
        let mut manager = manager();
        manager.note_on(0, 60, 1.0, 1.0, 0.0, 2);
        manager.note_on(0, 61, 1.0, 1.0, 0.0, 2);
        manager.note_off(0, 60, false, 0.0);
        manager.note_off(0, 61, false, 0.0);
        manager.voices[0].envelope.level = 0.2;
        manager.voices[1].envelope.level = 0.8;
        manager.note_on(0, 62, 1.0, 1.0, 0.0, 2);

        assert!(!manager.note_is_active(0, 60));
        assert!(manager.note_is_active(0, 61));
    }

    #[test]
    fn sustain_holds_note_until_pedal_releases() {
        let mut manager = manager();
        manager.note_on(2, 60, 1.0, 1.0, 0.0, 4);
        manager.note_off(2, 60, true, 0.0);
        assert!(manager.note_is_active(2, 60));
        assert_ne!(manager.voices[0].envelope.stage, Stage::Release);

        manager.release_sustained(2);
        assert_eq!(manager.voices[0].envelope.stage, Stage::Release);
    }

    #[test]
    fn same_note_retrigger_reuses_voice() {
        let mut manager = manager();
        let first = manager.note_on(0, 60, 0.5, 1.0, 0.0, 8);
        let second = manager.note_on(0, 60, 1.0, 1.0, 0.0, 8);
        assert_eq!(first, second);
        assert_eq!(manager.active_voice_count(), 1);
        assert!(manager.voices[first].key_down);
    }

    #[test]
    fn note_off_for_stolen_voice_is_ignored() {
        let mut manager = manager();
        manager.note_on(0, 60, 1.0, 1.0, 0.0, 1);
        manager.note_on(0, 62, 1.0, 1.0, 0.0, 1);
        manager.note_off(0, 60, false, 0.0);
        assert!(manager.note_is_active(0, 62));
        assert_eq!(manager.voices[0].envelope.stage, Stage::Attack);
    }

    #[test]
    fn lowering_polyphony_clears_voices_outside_the_new_limit() {
        let mut manager = manager();
        manager.note_on(0, 60, 1.0, 1.0, 0.0, 4);
        manager.note_on(0, 61, 1.0, 1.0, 0.0, 4);
        manager.note_on(0, 62, 1.0, 1.0, 0.0, 4);
        manager.note_on(0, 63, 1.0, 1.0, 0.0, 4);

        manager.set_polyphony(2);

        assert_eq!(manager.active_voice_count(), 2);
        assert!(!manager.note_is_active(0, 62));
        assert!(!manager.note_is_active(0, 63));
    }

    #[test]
    fn random_note_stream_never_panics_or_exceeds_voice_limit() {
        let mut manager = manager();
        let mut state = 0x9E37_79B9_u32;
        for _ in 0..1_000 {
            state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            let note = 36 + ((state >> 16) % 60) as u8;
            if state & 1 == 0 {
                manager.note_on(0, note, (state & 0xffff) as f32 / 65_535.0, 1.0, 0.0, 32);
            } else {
                manager.note_off(0, note, state & 2 != 0, 0.0);
            }
            assert!(manager.active_voice_count() <= MAX_VOICES);
        }
    }
}

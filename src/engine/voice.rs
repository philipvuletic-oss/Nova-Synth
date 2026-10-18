use crate::content::wavetables::{FrameInterpolation, Wavetable};
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
    pub wavetable: Option<&'a Wavetable>,
    pub wt_position: f32,
    pub interpolation: FrameInterpolation,
    pub oscillator_level: f32,
    pub oscillator_enabled: bool,
    pub unison: UnisonSettings,
}

pub const MAX_UNISON: usize = 16;

/// Unison parameters for one oscillator. `Default` is a single plain voice (no unison).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct UnisonSettings {
    pub voices: usize,
    /// 0..1 perceptual detune amount.
    pub detune: f32,
    /// Detune range in semitones at detune = 1.
    pub range_semitones: f32,
    /// 0 = centre voice only, 1 = side voices only (equal power).
    pub blend: f32,
    /// 0..1 stereo width.
    pub width: f32,
    /// 0..1 amount of random start phase per unison voice.
    pub random_phase: f32,
}

impl Default for UnisonSettings {
    fn default() -> Self {
        Self {
            voices: 1,
            detune: 0.25,
            range_semitones: 2.0,
            blend: 0.75,
            width: 1.0,
            random_phase: 1.0,
        }
    }
}

/// Per-voice detune (semitones), gain and pan for unison voice `index` of `count`.
/// Detune is non-linear: outer voices spread much further than inner ones.
pub fn unison_voice_layout(index: usize, count: usize, s: &UnisonSettings) -> (f32, f32, f32) {
    if count <= 1 {
        return (0.0, 1.0, 0.0);
    }
    let position = 2.0 * index as f32 / (count - 1) as f32 - 1.0;
    let curved = position.signum() * position.abs().powf(1.6);
    let amount = s.detune.clamp(0.0, 1.0).powf(1.5) * s.range_semitones.clamp(0.0, 48.0);
    let detune = curved * amount;
    let blend = s.blend.clamp(0.0, 1.0) * std::f32::consts::FRAC_PI_2;
    let odd = count % 2 == 1;
    let is_centre = odd && index == count / 2;
    let gain = if !odd {
        1.0 / (count as f32).sqrt()
    } else if is_centre {
        blend.cos()
    } else {
        blend.sin() / ((count - 1) as f32).sqrt()
    };
    let pan = if is_centre {
        0.0
    } else {
        let side = if index % 2 == 0 { -1.0 } else { 1.0 };
        side * s.width.clamp(0.0, 1.0) * (0.35 + 0.65 * position.abs())
    };
    (detune, gain, pan)
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
    phase_offset: f32,
    uni_phase: [f64; MAX_UNISON],
    uni_start: [f32; MAX_UNISON],
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
            phase_offset: 0.0,
            uni_phase: [0.0; MAX_UNISON],
            uni_start: [0.0; MAX_UNISON],
            age: 0,
            envelope: Envelope::default(),
        }
    }
}

pub struct VoiceManager {
    voices: [Voice; MAX_VOICES],
    sample_rate: f32,
    age: u64,
    phase_memory: bool,
    phase_randomization: f32,
    phase_start: f64,
}

impl VoiceManager {
    pub fn new(sample_rate: f32) -> Self {
        Self {
            voices: [Voice::default(); MAX_VOICES],
            sample_rate: sample_rate.max(1.0),
            age: 0,
            phase_memory: false,
            phase_randomization: 0.0,
            phase_start: 0.0,
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

    pub fn configure_phase(
        &mut self,
        phase_memory: bool,
        phase_randomization: f32,
        phase_start_degrees: f32,
    ) {
        self.phase_memory = phase_memory;
        self.phase_randomization = phase_randomization.clamp(0.0, 1.0);
        self.phase_start = phase_start_degrees.clamp(0.0, 360.0) as f64 / 360.0;
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
        if !self.phase_memory {
            voice.phase = self.phase_start;
        }
        voice.active = true;
        voice.note = note.min(127);
        voice.channel = channel.min(15);
        voice.key_down = true;
        voice.sustained = false;
        voice.velocity = velocity.clamp(0.0, 1.0);
        voice.pressure = pressure.clamp(0.0, 1.0);
        voice.age = self.age;
        let random = self.age.wrapping_mul(0x9E37_79B9_7F4A_7C15);
        voice.phase_offset = ((random >> 40) as u32 as f32 / (1u32 << 24) as f32) * 2.0 - 1.0;
        let mut state = random ^ 0xD1B5_4A32_D192_ED03;
        for start in &mut voice.uni_start {
            state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
            let mut z = state;
            z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
            z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
            *start = ((z >> 40) as u32 as f32) / (1u32 << 24) as f32;
        }
        voice.uni_phase = [voice.phase; MAX_UNISON];
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

    /// Mono mix (left channel); identical to the old output when unison is off.
    pub fn render_sample(&mut self, settings: &VoiceRenderSettings<'_>) -> f32 {
        self.render_stereo(settings).0
    }

    pub fn render_stereo(&mut self, settings: &VoiceRenderSettings<'_>) -> (f32, f32) {
        let limit = settings.polyphony.clamp(1, MAX_VOICES);
        let sensitivity = settings.velocity_sensitivity.clamp(0.0, 1.0);
        let phase_randomization = self.phase_randomization;
        let uni = settings.unison;
        let count = uni.voices.clamp(1, MAX_UNISON);
        let mut layout = [(0.0f32, 1.0f32, 0.0f32); MAX_UNISON];
        for (index, entry) in layout.iter_mut().enumerate().take(count) {
            *entry = unison_voice_layout(index, count, &uni);
        }
        let (mut out_l, mut out_r) = (0.0f32, 0.0f32);
        for voice in &mut self.voices[..limit] {
            if !voice.active {
                continue;
            }
            let bend = settings.pitch_bend[voice.channel as usize].clamp(-1.0, 1.0)
                * settings.pitch_bend_range.clamp(0.0, 48.0);
            let base_semitones = voice.note as f32 - 69.0 + bend;
            let frequency =
                settings.a4_tuning.clamp(415.0, 466.0) * 2.0f32.powf(base_semitones / 12.0);
            let amplitude = voice.envelope.next(self.sample_rate, settings.envelope);
            let velocity_gain = 1.0 - sensitivity + sensitivity * voice.velocity;
            let pressure_gain = 0.5 + voice.pressure * 0.5;
            let scale = amplitude
                * velocity_gain
                * pressure_gain
                * settings.oscillator_level.clamp(0.0, 1.0);
            let (mut left, mut right) = (0.0f32, 0.0f32);
            if settings.oscillator_enabled {
                let read = |phase: f64, frequency: f32| -> f32 {
                    settings.wavetable.map_or_else(
                        || (phase * std::f64::consts::TAU).sin() as f32,
                        |wavetable| {
                            wavetable.sample(
                                phase,
                                frequency,
                                settings.wt_position,
                                settings.interpolation,
                            )
                        },
                    )
                };
                if count == 1 {
                    let phase = (voice.phase + (voice.phase_offset * phase_randomization) as f64)
                        .rem_euclid(1.0);
                    left = read(phase, frequency);
                    right = left;
                } else {
                    for (index, (detune, gain, pan)) in
                        layout.iter().copied().enumerate().take(count)
                    {
                        let voice_frequency = frequency * 2.0f32.powf(detune / 12.0);
                        let phase = (voice.uni_phase[index]
                            + (voice.uni_start[index] * uni.random_phase.clamp(0.0, 1.0)) as f64)
                            .rem_euclid(1.0);
                        let sample = read(phase, voice_frequency) * gain;
                        let angle = (pan + 1.0) * std::f32::consts::FRAC_PI_4;
                        left += sample * angle.cos() * std::f32::consts::SQRT_2;
                        right += sample * angle.sin() * std::f32::consts::SQRT_2;
                        voice.uni_phase[index] = (voice.uni_phase[index]
                            + voice_frequency as f64 / self.sample_rate as f64)
                            .rem_euclid(1.0);
                    }
                }
            }
            let (left, right) = (left * scale, right * scale);
            voice.phase += frequency as f64 / self.sample_rate as f64;
            voice.phase -= voice.phase.floor();
            if !left.is_finite() || !right.is_finite() || !voice.phase.is_finite() {
                *voice = Voice::default();
            } else if voice.envelope.stage == Stage::Off {
                voice.active = false;
            } else {
                out_l += left;
                out_r += right;
            }
        }
        (out_l, out_r)
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

#[cfg(test)]
mod unison_tests {
    use super::*;
    use crate::params::envelope::Env1Params;

    fn render(unison: UnisonSettings, samples: usize) -> (Vec<f32>, Vec<f32>) {
        let table = Wavetable::fallback_sine(48_000.0);
        let envelope = Env1Params::default();
        envelope.reset_smoothers();
        let mut voices = VoiceManager::new(48_000.0);
        voices.note_on(0, 57, 1.0, 1.0, 0.0, 8);
        let bend = [0.0; 16];
        let (mut l, mut r) = (Vec::new(), Vec::new());
        for _ in 0..samples {
            let settings = VoiceRenderSettings {
                polyphony: 8,
                a4_tuning: 440.0,
                pitch_bend_range: 2.0,
                pitch_bend: &bend,
                envelope: envelope.next_values(),
                velocity_sensitivity: 0.0,
                wavetable: Some(&table),
                wt_position: 0.0,
                interpolation: FrameInterpolation::Frames,
                oscillator_level: 1.0,
                oscillator_enabled: true,
                unison,
            };
            let (a, b) = voices.render_stereo(&settings);
            l.push(a);
            r.push(b);
        }
        (l, r)
    }

    fn rms(x: &[f32]) -> f32 {
        (x.iter().map(|v| v * v).sum::<f32>() / x.len() as f32).sqrt()
    }

    #[test]
    fn layout_is_symmetric_and_outer_voices_detune_more() {
        let s = UnisonSettings {
            voices: 7,
            detune: 1.0,
            ..UnisonSettings::default()
        };
        let d: Vec<f32> = (0..7).map(|i| unison_voice_layout(i, 7, &s).0).collect();
        assert!((d[0] + d[6]).abs() < 1.0e-5);
        assert!(d[3].abs() < 1.0e-6);
        assert!((d[1] - d[0]).abs() < d[0].abs());
        assert!((d[0].abs() - 2.0).abs() < 1.0e-4);
    }

    #[test]
    fn loudness_stays_similar_when_adding_voices() {
        let one = rms(&render(UnisonSettings::default(), 24_000).0[8_000..]);
        for n in [3, 8, 16] {
            let uni = UnisonSettings {
                voices: n,
                ..UnisonSettings::default()
            };
            let (l, r) = render(uni, 24_000);
            let both = (rms(&l[8_000..]).powi(2) + rms(&r[8_000..]).powi(2)).sqrt() / 2.0f32.sqrt();
            let ratio_db = 20.0 * (both / one).log10();
            assert!(ratio_db.abs() < 4.0, "{n} voices: {ratio_db} dB");
        }
    }

    #[test]
    fn width_widens_and_zero_width_is_mono() {
        let wide = UnisonSettings {
            voices: 8,
            width: 1.0,
            ..UnisonSettings::default()
        };
        let (l, r) = render(wide, 24_000);
        assert!(l.iter().zip(&r).any(|(a, b)| (a - b).abs() > 0.01));
        let mono = UnisonSettings { width: 0.0, ..wide };
        let (l, r) = render(mono, 4_000);
        assert!(l.iter().zip(&r).all(|(a, b)| (a - b).abs() < 1.0e-5));
        assert!(l.iter().chain(&r).all(|v| v.is_finite()));
    }
}

#![allow(clippy::type_complexity)]

pub mod content;
pub mod dsp;
pub mod engine;
pub mod params;
pub mod state;
pub mod ui;

use nih_plug::prelude::*;
use std::sync::Arc;

use crate::content::wavetables::{FrameInterpolation, Wavetable};
use crate::engine::{AudioTelemetry, KeyboardCommand, VoiceManager, VoiceRenderSettings};
use crate::params::NovaParams;
use crate::ui::UiShared;
use arc_swap::ArcSwap;

pub struct Nova {
    params: Arc<NovaParams>,
    voices: VoiceManager,
    sample_rate: f32,
    pitch_bend: [f32; 16],
    sustain: [bool; 16],
    mod_wheel: [f32; 16],
    channel_pressure: [f32; 16],
    keyboard_rx: rtrb::Consumer<KeyboardCommand>,
    telemetry: Arc<AudioTelemetry>,
    ui_shared: Arc<UiShared>,
    wavetable: Arc<ArcSwap<Wavetable>>,
}

impl Default for Nova {
    fn default() -> Self {
        let (keyboard_tx, keyboard_rx) = rtrb::RingBuffer::new(256);
        let telemetry = Arc::new(AudioTelemetry::default());
        let wavetable = Arc::new(ArcSwap::from_pointee(Wavetable::fallback_sine(48_000.0)));
        let ui_shared = Arc::new(UiShared::new(
            keyboard_tx,
            telemetry.clone(),
            wavetable.clone(),
        ));
        Self {
            params: Arc::new(NovaParams::new()),
            voices: VoiceManager::new(48_000.0),
            sample_rate: 48_000.0,
            pitch_bend: [0.0; 16],
            sustain: [false; 16],
            mod_wheel: [0.0; 16],
            channel_pressure: [1.0; 16],
            keyboard_rx,
            telemetry,
            ui_shared,
            wavetable,
        }
    }
}

impl Plugin for Nova {
    const NAME: &'static str = "Nova";
    const VENDOR: &'static str = "Nova Audio";
    const URL: &'static str = "https://example.com/nova";
    const EMAIL: &'static str = "support@example.com";
    const VERSION: &'static str = env!("CARGO_PKG_VERSION");

    const AUDIO_IO_LAYOUTS: &'static [AudioIOLayout] = &[AudioIOLayout {
        main_input_channels: None,
        main_output_channels: NonZeroU32::new(2),
        ..AudioIOLayout::const_default()
    }];

    const MIDI_INPUT: MidiConfig = MidiConfig::MidiCCs;
    const SAMPLE_ACCURATE_AUTOMATION: bool = true;
    type SysExMessage = ();
    type BackgroundTask = ();

    fn params(&self) -> Arc<dyn Params> {
        self.params.clone()
    }

    fn editor(&mut self, _async_executor: AsyncExecutor<Self>) -> Option<Box<dyn Editor>> {
        ui::editor(self.params.clone(), self.ui_shared.clone())
    }

    fn initialize(
        &mut self,
        _audio_io_layout: &AudioIOLayout,
        buffer_config: &BufferConfig,
        _context: &mut impl InitContext<Self>,
    ) -> bool {
        self.sample_rate = buffer_config.sample_rate.max(1.0);
        self.ui_shared.wavetable_sample_rate.store(
            self.sample_rate.to_bits(),
            std::sync::atomic::Ordering::Relaxed,
        );
        self.voices.set_sample_rate(self.sample_rate);
        self.params.reset_smoothers();
        let generation = self
            .ui_shared
            .wavetable_generation
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed)
            .wrapping_add(1);
        let persisted = if let Ok(mut state) = self.params.persisted.lock() {
            crate::state::migrations::migrate(&mut state);
            Some(state.clone())
        } else {
            None
        };
        let current_table = self.wavetable.load_full();
        let wavetable = self.wavetable.clone();
        let generation_counter = self.ui_shared.wavetable_generation.clone();
        let loading = self.ui_shared.wavetable_loading.clone();
        let sample_rate = self.sample_rate;
        loading.store(true, std::sync::atomic::Ordering::Relaxed);
        std::thread::spawn(move || {
            let restored = restore_wavetable(persisted, sample_rate)
                .or_else(|| current_table.rebuild_for_sample_rate(sample_rate).ok());
            if generation_counter.load(std::sync::atomic::Ordering::Relaxed) == generation {
                if let Some(table) = restored {
                    wavetable.store(Arc::new(table));
                }
                loading.store(false, std::sync::atomic::Ordering::Relaxed);
            }
        });
        true
    }

    fn reset(&mut self) {
        self.voices.reset();
        self.params.reset_smoothers();
        self.pitch_bend = [0.0; 16];
        self.sustain = [false; 16];
    }

    fn process(
        &mut self,
        buffer: &mut Buffer,
        _aux: &mut AuxiliaryBuffers,
        context: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        let _denormal_guard = crate::dsp::denormal::DenormalGuard::new();
        let started = std::time::Instant::now();
        let mut next_event = context.next_event();
        let max_voices = self.params.global.polyphony.value().clamp(1, 32) as usize;
        self.voices.set_polyphony(max_voices);
        self.voices.configure_phase(
            self.params.osc_a.phase_memory.value(),
            self.params.osc_a.phase_rand.value(),
            self.params.osc_a.phase.value(),
        );
        let wavetable = self.wavetable.load();
        let mut peak_left = 0.0f32;
        let mut peak_right = 0.0f32;
        let mut rendered_samples = 0usize;

        for (sample_index, mut channels) in buffer.iter_samples().enumerate() {
            rendered_samples = sample_index + 1;
            while let Ok(command) = self.keyboard_rx.pop() {
                match command {
                    KeyboardCommand::NoteOn { note, velocity } => {
                        self.voices.note_on(
                            0,
                            note,
                            velocity,
                            1.0,
                            self.params.env1.delay.value(),
                            max_voices,
                        );
                    }
                    KeyboardCommand::NoteOff { note } => {
                        self.voices.note_off(0, note, self.sustain[0], 0.0);
                    }
                    KeyboardCommand::PitchBend { value } => {
                        self.pitch_bend[0] = value.clamp(-1.0, 1.0);
                    }
                    KeyboardCommand::ModWheel { value } => {
                        self.mod_wheel[0] = value.clamp(0.0, 1.0);
                    }
                }
            }

            while let Some(event) = next_event {
                if event.timing() > sample_index as u32 {
                    break;
                }
                match event {
                    NoteEvent::NoteOn {
                        channel,
                        note,
                        velocity,
                        ..
                    } => {
                        let channel = channel.min(15);
                        let _ = self.voices.note_on(
                            channel,
                            note,
                            velocity,
                            self.channel_pressure[channel as usize],
                            self.params.env1.delay.value(),
                            max_voices,
                        );
                    }
                    NoteEvent::NoteOff {
                        channel,
                        note,
                        velocity,
                        ..
                    } => {
                        let channel = channel.min(15);
                        self.voices.note_off(
                            channel,
                            note,
                            self.sustain[channel as usize],
                            velocity,
                        );
                    }
                    NoteEvent::Choke { channel, note, .. } => {
                        let channel = channel.min(15);
                        self.voices.choke(channel, note);
                    }
                    NoteEvent::MidiPitchBend { channel, value, .. } => {
                        let channel = channel.min(15);
                        self.pitch_bend[channel as usize] = (value.clamp(0.0, 1.0) - 0.5) * 2.0;
                    }
                    NoteEvent::MidiChannelPressure {
                        channel, pressure, ..
                    } => {
                        let channel = channel.min(15);
                        self.channel_pressure[channel as usize] = pressure.clamp(0.0, 1.0);
                    }
                    NoteEvent::PolyPressure {
                        channel,
                        note,
                        pressure,
                        ..
                    } => self.voices.set_pressure(channel.min(15), note, pressure),
                    NoteEvent::MidiCC {
                        channel, cc, value, ..
                    } => {
                        let channel = channel.min(15);
                        match cc {
                            1 => self.mod_wheel[channel as usize] = value.clamp(0.0, 1.0),
                            64 => {
                                let is_down = value >= 0.5;
                                if self.sustain[channel as usize] && !is_down {
                                    self.voices.release_sustained(channel);
                                }
                                self.sustain[channel as usize] = is_down;
                            }
                            _ => {}
                        }
                    }
                    _ => {}
                }
                next_event = context.next_event();
            }

            let bend_range = self.params.global.pitchbend_range.value().clamp(0.0, 48.0);
            let tuning = self.params.global.a4_tuning.value().clamp(415.0, 466.0);
            let volume = self.params.master_volume.smoothed.next();
            let volume = if volume <= util::db_to_gain(-59.9) {
                0.0
            } else {
                volume
            };
            let settings = VoiceRenderSettings {
                polyphony: max_voices,
                a4_tuning: tuning,
                pitch_bend_range: bend_range,
                pitch_bend: &self.pitch_bend,
                envelope: self.params.env1.next_values(),
                velocity_sensitivity: self.params.global.velocity_sensitivity.smoothed.next(),
                wavetable: Some(&wavetable),
                wt_position: self.params.osc_a.wt_position.smoothed.next(),
                interpolation: match self.params.osc_a.interp_mode.value() {
                    1 => FrameInterpolation::Smooth,
                    2 => FrameInterpolation::Step,
                    _ => FrameInterpolation::Frames,
                },
                oscillator_level: self.params.osc_a.level.smoothed.next(),
                oscillator_enabled: self.params.osc_a.enable.value(),
            };
            let mut left = self.voices.render_sample(&settings);
            let mut right = left;
            let pan = self.params.osc_a.pan.smoothed.next().clamp(-1.0, 1.0);
            let pan_angle = (pan + 1.0) * std::f32::consts::FRAC_PI_4;
            left *= volume * pan_angle.cos() * std::f32::consts::SQRT_2;
            right *= volume * pan_angle.sin() * std::f32::consts::SQRT_2;
            if self.params.global.safety_clipper.value() {
                left = left.tanh();
                right = right.tanh();
            }
            if !left.is_finite() {
                left = 0.0;
            }
            if !right.is_finite() {
                right = 0.0;
            }
            peak_left = peak_left.max(left.abs());
            peak_right = peak_right.max(right.abs());

            if let Some(channel) = channels.get_mut(0) {
                *channel = left;
            }
            if let Some(channel) = channels.get_mut(1) {
                *channel = right;
            }
        }

        self.telemetry
            .peak_left
            .store(peak_left.to_bits(), std::sync::atomic::Ordering::Relaxed);
        self.telemetry
            .peak_right
            .store(peak_right.to_bits(), std::sync::atomic::Ordering::Relaxed);
        self.telemetry.active_voices.store(
            self.voices.active_voice_count() as u32,
            std::sync::atomic::Ordering::Relaxed,
        );
        let duration = rendered_samples as f32 / self.sample_rate;
        let cpu = if duration > 0.0 {
            (started.elapsed().as_secs_f32() / duration).clamp(0.0, 1.0)
        } else {
            0.0
        };
        self.telemetry
            .cpu_load
            .store(cpu.to_bits(), std::sync::atomic::Ordering::Relaxed);
        ProcessStatus::KeepAlive
    }
}

impl Nova {
    pub fn mod_wheel_value(&self, channel: u8) -> f32 {
        self.mod_wheel[channel.min(15) as usize]
    }
}

fn restore_wavetable(
    state: Option<crate::state::persisted::VersionedState>,
    sample_rate: f32,
) -> Option<Wavetable> {
    let state = state?;
    let imported = state.imported_wavetable_frames.as_ref().and_then(|frames| {
        let frames = frames
            .iter()
            .map(|frame| {
                frame
                    .iter()
                    .map(|sample| *sample as f32 / i16::MAX as f32)
                    .collect()
            })
            .collect();
        Wavetable::from_frames(
            state
                .imported_wavetable_name
                .clone()
                .unwrap_or_else(|| String::from("Imported wavetable")),
            state
                .imported_wavetable_source
                .clone()
                .unwrap_or_else(|| String::from("Embedded project data")),
            frames,
            sample_rate,
        )
        .ok()
    });
    imported.or_else(|| {
        crate::content::wavetables::FACTORY_TABLES
            .iter()
            .position(|table| table.name == state.selected_wavetable)
            .and_then(|index| {
                crate::content::factory::generate_factory_wavetable(index, sample_rate).ok()
            })
    })
}

impl Vst3Plugin for Nova {
    const VST3_CLASS_ID: [u8; 16] = *b"NovaSynthPlugin1";
    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] =
        &[Vst3SubCategory::Instrument, Vst3SubCategory::Synth];
}

// nih-plug's VST3 macro expands a dependency macro that emits a trailing-semicolon warning.
#[allow(semicolon_in_expressions_from_non_local_macros)]
mod plugin_exports {
    use super::Nova;
    use nih_plug::prelude::nih_export_vst3;

    nih_export_vst3!(Nova);
}

#![allow(clippy::type_complexity)]

pub mod content;
pub mod dsp;
pub mod engine;
pub mod params;
pub mod state;
pub mod ui;

use nih_plug::prelude::*;
use std::sync::Arc;

use crate::engine::{AudioTelemetry, KeyboardCommand, VoiceManager};
use crate::params::NovaParams;
use crate::ui::UiShared;

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
}

impl Default for Nova {
    fn default() -> Self {
        let (keyboard_tx, keyboard_rx) = rtrb::RingBuffer::new(256);
        let telemetry = Arc::new(AudioTelemetry::default());
        let ui_shared = Arc::new(UiShared::new(keyboard_tx));
        Self {
            params: Arc::new(NovaParams::new()),
            voices: VoiceManager::new(48_000.0),
            sample_rate: 48_000.0,
            pitch_bend: [0.0; 16],
            sustain: [false; 16],
            mod_wheel: [0.0; 16],
            channel_pressure: [0.0; 16],
            keyboard_rx,
            telemetry,
            ui_shared,
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
        Some(ui::editor(self.params.clone(), self.ui_shared.clone()))
    }

    fn initialize(
        &mut self,
        _audio_io_layout: &AudioIOLayout,
        buffer_config: &BufferConfig,
        _context: &mut impl InitContext<Self>,
    ) -> bool {
        self.sample_rate = buffer_config.sample_rate.max(1.0);
        self.voices.set_sample_rate(self.sample_rate);
        true
    }

    fn reset(&mut self) {
        self.voices.reset();
        self.pitch_bend = [0.0; 16];
        self.sustain = [false; 16];
    }

    fn process(
        &mut self,
        buffer: &mut Buffer,
        _aux: &mut AuxiliaryBuffers,
        context: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        let started = std::time::Instant::now();
        let mut next_event = context.next_event();
        let max_voices = self.params.global.polyphony.value().clamp(1, 32) as usize;
        let mut peak_left = 0.0f32;
        let mut peak_right = 0.0f32;

        for sample_index in 0..buffer.samples() {
            while let Some(command) = self.keyboard_rx.pop().ok() {
                match command {
                    KeyboardCommand::NoteOn { note, velocity } => {
                        self.voices.note_on(0, note, velocity, 0.0, max_voices);
                    }
                    KeyboardCommand::NoteOff { note } => {
                        self.voices.note_off(0, note, self.sustain[0], 0.0);
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
                    } => self.voices.note_on(
                        channel,
                        note,
                        velocity,
                        self.channel_pressure[channel as usize],
                        max_voices,
                    ),
                    NoteEvent::NoteOff {
                        channel, note, ..
                    } => self.voices.note_off(
                        channel,
                        note,
                        self.sustain[channel as usize],
                        0.0,
                    ),
                    NoteEvent::Choke { channel, note, .. } => {
                        self.voices.choke(channel, note);
                    }
                    NoteEvent::MidiPitchBend { channel, value, .. } => {
                        self.pitch_bend[channel as usize] = (value.clamp(0.0, 1.0) - 0.5) * 2.0;
                    }
                    NoteEvent::MidiChannelPressure {
                        channel, pressure, ..
                    } => self.channel_pressure[channel as usize] = pressure.clamp(0.0, 1.0),
                    NoteEvent::PolyPressure {
                        channel,
                        note,
                        pressure,
                        ..
                    } => self.voices.set_pressure(channel, note, pressure),
                    NoteEvent::MidiCC {
                        channel, cc, value, ..
                    } => match cc {
                        1 => self.mod_wheel[channel as usize] = value.clamp(0.0, 1.0),
                        64 => {
                            let is_down = value >= 0.5;
                            if self.sustain[channel as usize] && !is_down {
                                self.voices.release_sustained(channel);
                            }
                            self.sustain[channel as usize] = is_down;
                        }
                        _ => {}
                    },
                    _ => {}
                }
                next_event = context.next_event();
            }

            let bend_range = self.params.global.pitchbend_range.value().clamp(0.0, 48.0);
            let tuning = self.params.global.a4_tuning.value().clamp(415.0, 466.0);
            let volume = self.params.master_volume.smoothed.next();
            let mut left = 0.0;
            let mut right = 0.0;
            self.voices.render_sample(
                max_voices,
                tuning,
                bend_range,
                &self.pitch_bend,
                &self.params.env1,
                self.params.global.velocity_sensitivity.value(),
                &mut left,
                &mut right,
            );
            left *= volume;
            right *= volume;
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

            let mut channels = buffer.iter_samples().nth(sample_index).unwrap_or_default();
            if let Some(channel) = channels.next() {
                *channel = left;
            }
            if let Some(channel) = channels.next() {
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
        let duration = buffer.samples() as f32 / self.sample_rate;
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

impl Vst3Plugin for Nova {
    const VST3_CLASS_ID: [u8; 16] = *b"NovaSynthPlugin1";
    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] =
        &[Vst3SubCategory::Instrument, Vst3SubCategory::Synth];
}

nih_export_vst3!(Nova);

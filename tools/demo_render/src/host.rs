//! A minimal offline "host": initialises the real `Nova` plugin and calls `Plugin::process`.

use nih_plug::prelude::*;
use nova::Nova;
use std::collections::VecDeque;
use std::time::Instant;

struct OfflineInit;

impl InitContext<Nova> for OfflineInit {
    fn plugin_api(&self) -> PluginApi {
        PluginApi::Standalone
    }
    fn execute(&self, _task: ()) {}
    fn set_latency_samples(&self, _samples: u32) {}
    fn set_current_voice_capacity(&self, _capacity: u32) {}
}

struct OfflineProcess {
    events: VecDeque<NoteEvent<()>>,
    transport: Transport,
}

impl ProcessContext<Nova> for OfflineProcess {
    fn plugin_api(&self) -> PluginApi {
        PluginApi::Standalone
    }
    fn execute_background(&self, _task: ()) {}
    fn execute_gui(&self, _task: ()) {}
    fn transport(&self) -> &Transport {
        &self.transport
    }
    fn next_event(&mut self) -> Option<NoteEvent<()>> {
        self.events.pop_front()
    }
    fn send_event(&mut self, _event: NoteEvent<()>) {}
    fn set_latency_samples(&self, _samples: u32) {}
    fn set_current_voice_capacity(&self, _capacity: u32) {}
}

/// One MIDI note of a phrase, in seconds.
#[derive(Clone, Copy, Debug)]
pub struct Note {
    pub start: f64,
    pub length: f64,
    pub note: u8,
    pub velocity: f32,
}

pub struct Rendered {
    pub left: Vec<f32>,
    pub right: Vec<f32>,
    /// Wall-clock time spent inside `Plugin::process` divided by audio duration.
    pub real_time_factor: f64,
}

pub const SAMPLE_RATE: f32 = 48_000.0;
const BLOCK: usize = 512;

/// Renders `notes` through `plugin` for `seconds` of audio.
pub fn render(mut plugin: Nova, notes: &[Note], seconds: f64) -> Rendered {
    let layout = Nova::AUDIO_IO_LAYOUTS[0];
    let config = BufferConfig {
        sample_rate: SAMPLE_RATE,
        min_buffer_size: None,
        max_buffer_size: BLOCK as u32,
        process_mode: ProcessMode::Offline,
    };
    assert!(plugin.initialize(&layout, &config, &mut OfflineInit));
    // Wait for the background wavetable thread so the patch's table is the one that sounds.
    while plugin.wavetable_loading() {
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    plugin.reset();

    let total = (seconds * SAMPLE_RATE as f64).ceil() as usize;
    let mut timeline: Vec<(usize, NoteEvent<()>)> = Vec::new();
    for note in notes {
        let on = (note.start * SAMPLE_RATE as f64).round() as usize;
        let off = ((note.start + note.length) * SAMPLE_RATE as f64).round() as usize;
        timeline.push((
            on,
            NoteEvent::NoteOn {
                timing: 0,
                voice_id: None,
                channel: 0,
                note: note.note,
                velocity: note.velocity,
            },
        ));
        timeline.push((
            off,
            NoteEvent::NoteOff {
                timing: 0,
                voice_id: None,
                channel: 0,
                note: note.note,
                velocity: 0.0,
            },
        ));
    }
    timeline.sort_by_key(|(at, event)| (*at, matches!(event, NoteEvent::NoteOn { .. })));

    // SAFETY: `Transport` only holds plain data (bool, f32 and `Option`s of numbers) for which the
    // all-zero bit pattern is valid (`None`/0/false), and Nova never reads the transport.
    let transport: Transport = unsafe { std::mem::zeroed() };
    let mut context = OfflineProcess {
        events: VecDeque::new(),
        transport,
    };
    let mut left = vec![0.0f32; total];
    let mut right = vec![0.0f32; total];
    let mut cursor = 0;
    let mut next = 0;
    let mut spent = 0.0f64;
    while cursor < total {
        let len = BLOCK.min(total - cursor);
        context.events.clear();
        while next < timeline.len() && timeline[next].0 < cursor + len {
            let (at, mut event) = timeline[next];
            let timing = at.saturating_sub(cursor) as u32;
            match &mut event {
                NoteEvent::NoteOn { timing: t, .. } | NoteEvent::NoteOff { timing: t, .. } => {
                    *t = timing;
                }
                _ => {}
            }
            context.events.push_back(event);
            next += 1;
        }
        let (block_left, block_right) = (
            &mut left[cursor..cursor + len],
            &mut right[cursor..cursor + len],
        );
        let mut buffer = Buffer::default();
        // SAFETY: both slices live until after `process` returns and have equal length.
        unsafe {
            buffer.set_slices(len, |slices| {
                *slices = vec![
                    std::slice::from_raw_parts_mut(block_left.as_mut_ptr(), len),
                    std::slice::from_raw_parts_mut(block_right.as_mut_ptr(), len),
                ];
            });
        }
        let mut aux = AuxiliaryBuffers {
            inputs: &mut [],
            outputs: &mut [],
        };
        let started = Instant::now();
        plugin.process(&mut buffer, &mut aux, &mut context);
        spent += started.elapsed().as_secs_f64();
        cursor += len;
    }
    Rendered {
        left,
        right,
        real_time_factor: spent / (total as f64 / SAMPLE_RATE as f64),
    }
}

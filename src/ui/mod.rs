//! The egui application shell and controls for Nova.
pub mod editor;
pub mod pages;
pub mod theme;
pub mod widgets;

use crate::content::wavetables::Wavetable;
use crate::engine::{AudioTelemetry, KeyboardCommand};
use arc_swap::ArcSwap;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64};
use std::sync::{Arc, Mutex};

pub struct UiShared {
    pub keyboard_tx: Mutex<rtrb::Producer<KeyboardCommand>>,
    pub telemetry: Arc<AudioTelemetry>,
    pub wavetable: Arc<ArcSwap<Wavetable>>,
    pub wavetable_loading: Arc<AtomicBool>,
    pub wavetable_generation: Arc<AtomicU64>,
    pub wavetable_sample_rate: Arc<AtomicU32>,
}

impl UiShared {
    pub fn new(
        keyboard_tx: rtrb::Producer<KeyboardCommand>,
        telemetry: Arc<AudioTelemetry>,
        wavetable: Arc<ArcSwap<Wavetable>>,
    ) -> Self {
        Self {
            keyboard_tx: Mutex::new(keyboard_tx),
            telemetry,
            wavetable,
            wavetable_loading: Arc::new(AtomicBool::new(false)),
            wavetable_generation: Arc::new(AtomicU64::new(0)),
            wavetable_sample_rate: Arc::new(AtomicU32::new(48_000.0f32.to_bits())),
        }
    }
}

pub use editor::editor;

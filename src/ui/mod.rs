//! The egui application shell and controls for Nova.
pub mod editor;
pub mod pages;
pub mod theme;
pub mod widgets;

use crate::engine::{AudioTelemetry, KeyboardCommand};
use std::sync::{Arc, Mutex};

pub struct UiShared {
    pub keyboard_tx: Mutex<rtrb::Producer<KeyboardCommand>>,
    pub telemetry: Arc<AudioTelemetry>,
}

impl UiShared {
    pub fn new(
        keyboard_tx: rtrb::Producer<KeyboardCommand>,
        telemetry: Arc<AudioTelemetry>,
    ) -> Self {
        Self {
            keyboard_tx: Mutex::new(keyboard_tx),
            telemetry,
        }
    }
}

pub use editor::editor;

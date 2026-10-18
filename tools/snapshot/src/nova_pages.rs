//! Renders the real Nova GUI (`nova::ui::app::draw_app`) headlessly.

use crate::{Image, Renderer};
use nih_plug::prelude::{GuiContext, ParamPtr, ParamSetter, PluginApi, PluginState};
use nova::content::factory::generate_factory_wavetable;
use nova::content::wavetables::{Wavetable, WavetableStore};
use nova::engine::AudioTelemetry;
use nova::params::NovaParams;
use nova::ui::app::{draw_app, AppState, Meters, UiViewModel};
use nova::ui::UiShared;
use std::sync::Arc;

pub const PAGE_NAMES: [&str; 7] = ["osc", "mix", "fx", "mod", "arp", "clip", "global"];

/// `ParamSetter` back-end that accepts and ignores everything (there is no host).
struct FakeGui;

impl GuiContext for FakeGui {
    fn plugin_api(&self) -> PluginApi {
        PluginApi::Standalone
    }
    fn request_resize(&self) -> bool {
        false
    }
    unsafe fn raw_begin_set_parameter(&self, _param: ParamPtr) {}
    unsafe fn raw_set_parameter_normalized(&self, _param: ParamPtr, _normalized: f32) {}
    unsafe fn raw_end_set_parameter(&self, _param: ParamPtr) {}
    fn get_state(&self) -> PluginState {
        PluginState {
            version: String::new(),
            params: Default::default(),
            fields: Default::default(),
        }
    }
    fn set_state(&self, _state: PluginState) {}
}

/// How lively the screenshot should look.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Idle,
    /// Fake meters, live modulation marker and held notes.
    Playing,
}

pub struct Scene {
    params: NovaParams,
    shared: UiShared,
    state: AppState,
    renderer: Renderer,
}

impl Scene {
    pub fn new(page: usize, mode: Mode) -> Self {
        let (tx, _rx) = rtrb::RingBuffer::new(256);
        let table: Wavetable = generate_factory_wavetable(1, 48_000.0)
            .unwrap_or_else(|_| Wavetable::fallback_sine(48_000.0));
        let shared = UiShared::new(
            tx,
            Arc::new(AudioTelemetry::default()),
            Arc::new(WavetableStore::new(table)),
        );
        let mut state = AppState {
            page,
            ..AppState::default()
        };
        if mode == Mode::Playing {
            // C major triad plus octave: C3 E3 G3 C4.
            for key in [0usize, 4, 7, 12] {
                state.external_held[key] = true;
            }
        }
        Self {
            params: NovaParams::new(),
            shared,
            state,
            renderer: Renderer::new(),
        }
    }

    pub fn render(&mut self, width: usize, height: usize, scale: f32, mode: Mode) -> Image {
        let gui = FakeGui;
        let setter = ParamSetter::new(&gui);
        let meters = if mode == Mode::Playing {
            Meters {
                peak_left: 0.62,
                peak_right: 0.48,
                cpu_load: 0.037,
                active_voices: 4,
                env1_level: Some(0.78),
            }
        } else {
            Meters::default()
        };
        let params = &self.params;
        let shared = &self.shared;
        let state = &mut self.state;
        self.renderer.render(width, height, scale, 3, |ctx| {
            let mut view = UiViewModel {
                params,
                setter: &setter,
                shared,
                state,
                meters,
            };
            draw_app(ctx, &mut view);
        })
    }
}

/// Convenience: one screenshot of `page` at the given size in points and scale.
pub fn screenshot(page: usize, width_pt: usize, height_pt: usize, scale: f32, mode: Mode) -> Image {
    let mut scene = Scene::new(page, mode);
    scene.render(
        (width_pt as f32 * scale) as usize,
        (height_pt as f32 * scale) as usize,
        scale,
        mode,
    )
}

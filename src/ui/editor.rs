use crate::params::NovaParams;
use crate::ui::app::{draw_app, AppState, Meters, UiViewModel};
use crate::ui::UiShared;
use nih_plug::prelude::Editor;
use nih_plug_egui::create_egui_editor;
use std::sync::Arc;

/// Thin plugin shell: all drawing happens in [`draw_app`].
pub fn editor(params: Arc<NovaParams>, shared: Arc<UiShared>) -> Option<Box<dyn Editor>> {
    let egui_state = params.editor_state.clone();
    create_egui_editor(
        egui_state,
        AppState::default(),
        |_, _| {},
        move |ctx, setter, state| {
            let mut view = UiViewModel {
                params: &params,
                setter,
                shared: &shared,
                state,
                meters: Meters::from_telemetry(&shared.telemetry),
            };
            draw_app(ctx, &mut view);
        },
    )
}

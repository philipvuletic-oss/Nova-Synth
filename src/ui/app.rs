use crate::engine::{AudioTelemetry, KeyboardCommand};
use crate::params::NovaParams;
use crate::ui::{motion, pages, scale, theme, widgets, UiShared};
use nih_plug::prelude::{Param, ParamSetter};
use nih_plug_egui::{
    egui::{self, Pos2, Rect, Sense, Stroke, Vec2},
    resizable_window::ResizableWindow,
};

pub struct AppState {
    pub page: usize,
    pub pressed_keys: [bool; 48],
    /// Notes held from outside the GUI (MIDI input, sequencers); highlighted on the keyboard.
    pub external_held: [bool; 48],
    pub pitch_wheel: f32,
    pub mod_wheel: f32,
    pub last_gui_scale: f32,
    /// Live global settings; `None` (tests, snapshots) disables all file I/O.
    pub settings: Option<crate::settings::Settings>,
    pub settings_dirty_since: Option<f64>,
    /// Smoothed meter levels (time-based release).
    pub meter_display: [f32; 2],
    pub meter_initialised: bool,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            page: 0,
            pressed_keys: [false; 48],
            external_held: [false; 48],
            pitch_wheel: 0.0,
            mod_wheel: 0.0,
            last_gui_scale: 1.0,
            settings: None,
            settings_dirty_since: None,
            meter_display: [0.0; 2],
            meter_initialised: false,
        }
    }
}

/// Live values read from the audio thread (or faked by tests).
#[derive(Clone, Copy, Debug, Default)]
pub struct Meters {
    pub peak_left: f32,
    pub peak_right: f32,
    pub cpu_load: f32,
    pub active_voices: u32,
    /// Live ENV 1 output (0..1) when a note is sounding; drawn as a marker in the envelope graph.
    pub env1_level: Option<f32>,
}

impl Meters {
    pub fn from_telemetry(telemetry: &AudioTelemetry) -> Self {
        Self {
            peak_left: telemetry.peak_left(),
            peak_right: telemetry.peak_right(),
            cpu_load: telemetry.cpu_load(),
            active_voices: telemetry.active_voices(),
            env1_level: None,
        }
    }
}

/// Everything the GUI needs for one frame. The plugin editor and the headless
/// snapshot renderer both build one of these and call [`draw_app`].
pub struct UiViewModel<'a> {
    pub params: &'a NovaParams,
    pub setter: &'a ParamSetter<'a>,
    pub shared: &'a UiShared,
    pub state: &'a mut AppState,
    pub meters: Meters,
}

/// Page for a pressed key (1-7). Only honoured while a Nova control has keyboard focus, so
/// Ableton's Computer MIDI Keyboard and other host shortcuts are never hijacked.
pub fn page_for_key(key: egui::Key) -> Option<usize> {
    match key {
        egui::Key::Num1 => Some(0),
        egui::Key::Num2 => Some(1),
        egui::Key::Num3 => Some(2),
        egui::Key::Num4 => Some(3),
        egui::Key::Num5 => Some(4),
        egui::Key::Num6 => Some(5),
        egui::Key::Num7 => Some(6),
        _ => None,
    }
}

/// Keyboard policy: nothing is handled unless a Nova widget has focus; page shortcuts are
/// disabled while a text field is being edited; Escape or a click on empty space releases
/// focus so keys flow back to the host again.
fn handle_keyboard_focus(ctx: &egui::Context, state: &mut AppState) {
    let focused = ctx.memory(|memory| memory.focused());
    let Some(focused) = focused else { return };
    let (escape, pressed_key, clicked_empty) = ctx.input(|input| {
        let escape = input.key_pressed(egui::Key::Escape);
        let key = input.events.iter().find_map(|event| match event {
            egui::Event::Key {
                key,
                pressed: true,
                modifiers,
                ..
            } if !modifiers.any() => Some(*key),
            _ => None,
        });
        (escape, key, input.pointer.any_pressed())
    });
    if escape || (clicked_empty && !ctx.wants_pointer_input()) {
        ctx.memory_mut(|memory| memory.surrender_focus(focused));
        return;
    }
    if !ctx.wants_keyboard_input() {
        if let Some(page) = pressed_key.and_then(page_for_key) {
            state.page = page;
        }
    }
}

/// Smooths the displayed meter levels with a time-based release and keeps repainting only
/// while they are still moving.
fn update_meters(ctx: &egui::Context, state: &mut AppState, meters: &mut Meters) {
    let dt = ctx.input(|input| input.unstable_dt);
    let targets = [meters.peak_left, meters.peak_right];
    for (shown, target) in state.meter_display.iter_mut().zip(targets) {
        *shown = if state.meter_initialised {
            motion::meter_fall(*shown, target, dt, 0.25)
        } else {
            target
        };
    }
    state.meter_initialised = true;
    if state
        .meter_display
        .iter()
        .zip(targets)
        .any(|(shown, target)| motion::is_settling(*shown, target))
    {
        ctx.request_repaint();
    }
    meters.peak_left = state.meter_display[0];
    meters.peak_right = state.meter_display[1];
}

/// Persists GUI scale and the last-used window size into the global settings file shortly
/// after they stop changing (no disk access while the mouse is still dragging).
fn sync_settings(ctx: &egui::Context, state: &mut AppState, params: &NovaParams, gui_scale: f32) {
    let Some(settings) = state.settings.as_mut() else {
        return;
    };
    let (width, height) = params.editor_state.size();
    let mut changed = false;
    if settings.window_width != width || settings.window_height != height {
        settings.window_width = width;
        settings.window_height = height;
        changed = true;
    }
    if (settings.gui_scale - gui_scale).abs() > f32::EPSILON {
        settings.gui_scale = gui_scale;
        changed = true;
    }
    let now = ctx.input(|input| input.time);
    if changed {
        settings.sanitize();
        state.settings_dirty_since = Some(now);
    }
    if let Some(since) = state.settings_dirty_since {
        let dragging = ctx.input(|input| input.pointer.any_down());
        if now - since >= 0.5 && !dragging {
            crate::settings::save(settings);
            state.settings_dirty_since = None;
        } else {
            ctx.request_repaint_after(std::time::Duration::from_millis(250));
        }
    }
}

/// Draws the whole Nova GUI. All GUI logic lives behind this function.
pub fn draw_app(ctx: &egui::Context, view: &mut UiViewModel<'_>) {
    let params = view.params;
    let setter = view.setter;
    let shared = view.shared;
    let mut meters = view.meters;
    let state = &mut *view.state;
    theme::apply(ctx);
    let scale = scale::clamp_gui_scale(params.global.gui_scale.value());
    ctx.set_zoom_factor(scale);
    if (state.last_gui_scale - scale).abs() > f32::EPSILON {
        if let Ok(mut persisted) = params.persisted.lock() {
            persisted.gui_scale = scale;
            crate::state::migrations::migrate(&mut persisted);
        }
        state.last_gui_scale = scale;
    }
    update_meters(ctx, state, &mut meters);
    handle_keyboard_focus(ctx, state);
    sync_settings(ctx, state, params, scale);
    ResizableWindow::new("nova-main")
        .min_size(Vec2::new(scale::MIN_WINDOW_WIDTH, scale::MIN_WINDOW_HEIGHT))
        .show(ctx, params.editor_state.as_ref(), |ui| {
            theme::paint_background(ui);
            ui.spacing_mut().item_spacing = Vec2::new(8.0, 6.0);
            draw_top_bar(ui, params, setter, &meters);
            ui.add_space(2.0);
            widgets::page_tabs::show(ui, &mut state.page);
            ui.separator();
            egui::ScrollArea::vertical()
                .id_salt("nova-page-area")
                .max_height((ui.available_height() - 336.0).max(120.0))
                .show(ui, |ui| pages::show(ui, state.page, params, setter, shared));
            draw_modulator_strip(ui, params, setter, meters.env1_level);
            draw_keyboard(ui, state, shared);
        });
}

fn draw_top_bar(ui: &mut egui::Ui, params: &NovaParams, setter: &ParamSetter<'_>, meters: &Meters) {
    egui::Frame::NONE
        .fill(theme::PANEL)
        .stroke(Stroke::new(1.0_f32, theme::PANEL_BORDER))
        .corner_radius(egui::CornerRadius::same(8))
        .show(ui, |ui| {
            ui.set_min_height(theme::TOP_BAR_HEIGHT);
            ui.horizontal(|ui| {
                let logo = ui.label(
                    egui::RichText::new("NOVA")
                        .size(22.0)
                        .strong()
                        .color(theme::CYAN),
                );
                logo.context_menu(|ui| {
                    let values = [0.75_f32, 1.0, 1.25, 1.5, 1.75, 2.0];
                    let current = params.global.gui_scale.value();
                    egui::ComboBox::from_id_salt("nova-gui-scale")
                        .selected_text(format!("GUI scale · {:.0}%", current * 100.0))
                        .show_ui(ui, |ui| {
                            for value in values {
                                if ui
                                    .selectable_label(
                                        (current - value).abs() < 0.001,
                                        format!("{:.0}%", value * 100.0),
                                    )
                                    .clicked()
                                {
                                    setter.begin_set_parameter(&params.global.gui_scale);
                                    setter.set_parameter(&params.global.gui_scale, value);
                                    setter.end_set_parameter(&params.global.gui_scale);
                                }
                            }
                        });
                });
                ui.separator();
                let _ = ui.button("‹");
                let _ = ui
                    .label(egui::RichText::new("Init").strong())
                    .on_hover_text("Mouse wheel changes presets");
                let _ = ui.button("›");
                let _ = ui.button("Browser");
                let _ = ui.button("Save");
                let _ = ui.button("Undo");
                let _ = ui.button("Redo");
                let _ = ui.button("A/B");
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(format!("{} voices", meters.active_voices));
                    ui.label(format!("CPU {:.1}%", meters.cpu_load * 100.0));
                    let _ = widgets::meter::meter(ui, meters.peak_left, meters.peak_right);
                    let _ = widgets::knob::knob(ui, &params.master_volume, setter, "Master");
                });
            });
        });
}

fn draw_modulator_strip(
    ui: &mut egui::Ui,
    params: &NovaParams,
    setter: &ParamSetter<'_>,
    live_level: Option<f32>,
) {
    egui::Frame::group(ui.style())
        .fill(theme::PANEL.gamma_multiply(0.88))
        .stroke(Stroke::new(1.0_f32, theme::PANEL_BORDER))
        .corner_radius(egui::CornerRadius::same(theme::PANEL_RADIUS))
        .show(ui, |ui| {
            ui.set_min_height(theme::MODULATOR_HEIGHT);
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.horizontal(|ui| {
                        let _ = widgets::button::pill(ui, "ENV 1", true);
                        for name in ["ENV 2", "ENV 3", "ENV 4", "LFO 1", "LFO 2", "LFO 3"] {
                            let _ = widgets::button::pill(ui, name, false);
                        }
                    });
                    ui.label(egui::RichText::new("ENV 1 · AMPLITUDE").color(theme::LIME));
                    envelope_graph(ui, params, setter, live_level);
                    ui.horizontal(|ui| {
                        let _ = widgets::knob::knob(ui, &params.env1.delay, setter, "Delay");
                        let _ = widgets::knob::knob(ui, &params.env1.attack, setter, "Attack");
                        let _ = widgets::knob::knob(ui, &params.env1.hold, setter, "Hold");
                        let _ = widgets::knob::knob(ui, &params.env1.decay, setter, "Decay");
                        let _ = widgets::knob::knob(ui, &params.env1.sustain, setter, "Sustain");
                        let _ = widgets::knob(ui, &params.env1.release, setter, "Release");
                    });
                });
                ui.separator();
                ui.vertical(|ui| {
                    ui.label(egui::RichText::new("ENV CURVES").color(theme::LABEL_TEXT));
                    ui.horizontal(|ui| {
                        let _ =
                            widgets::knob::knob(ui, &params.env1.attack_curve, setter, "Attack");
                        let _ = widgets::knob::knob(ui, &params.env1.decay_curve, setter, "Decay");
                        let _ =
                            widgets::knob::knob(ui, &params.env1.release_curve, setter, "Release");
                    });
                    ui.label("8 macro knobs · Coming in M6");
                });
            });
        });
}

fn envelope_graph(
    ui: &mut egui::Ui,
    params: &NovaParams,
    setter: &ParamSetter<'_>,
    live_level: Option<f32>,
) {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 52.0), Sense::hover());
    let painter = ui.painter_at(rect);
    let ppp = ui.ctx().pixels_per_point();
    let hairline = Stroke::new(scale::hairline(ppp), theme::GRID);
    for i in 1..4 {
        let x = scale::snap_line(rect.left() + rect.width() * i as f32 / 4.0, ppp);
        painter.line_segment(
            [Pos2::new(x, rect.top()), Pos2::new(x, rect.bottom())],
            hairline,
        );
    }
    let base_y = scale::snap_line(rect.bottom() - 2.0, ppp);
    painter.line_segment(
        [
            Pos2::new(rect.left(), base_y),
            Pos2::new(rect.right(), base_y),
        ],
        hairline,
    );

    let points = [
        Pos2::new(rect.left(), rect.bottom()),
        Pos2::new(
            rect.left()
                + rect.width() * (0.08 + params.env1.delay.unmodulated_normalized_value() * 0.08),
            rect.bottom(),
        ),
        Pos2::new(
            rect.left()
                + rect.width() * (0.22 + params.env1.attack.unmodulated_normalized_value() * 0.12),
            rect.top(),
        ),
        Pos2::new(
            rect.left()
                + rect.width() * (0.42 + params.env1.hold.unmodulated_normalized_value() * 0.08),
            rect.top(),
        ),
        Pos2::new(
            rect.left()
                + rect.width() * (0.62 + params.env1.decay.unmodulated_normalized_value() * 0.12),
            rect.bottom() - rect.height() * params.env1.sustain.unmodulated_normalized_value(),
        ),
        Pos2::new(
            rect.left()
                + rect.width() * (0.86 + params.env1.release.unmodulated_normalized_value() * 0.14),
            rect.bottom(),
        ),
    ];
    painter.add(egui::Shape::line(
        points.to_vec(),
        Stroke::new(5.0_f32, theme::LIME.gamma_multiply(0.1)),
    ));
    painter.add(egui::Shape::line(
        points.to_vec(),
        Stroke::new(2.0_f32, theme::LIME),
    ));
    if let Some(level) = live_level {
        let level = level.clamp(0.0, 1.0);
        let marker = Pos2::new(
            rect.left()
                + rect.width() * (0.22 + params.env1.attack.unmodulated_normalized_value() * 0.12),
            rect.bottom() - rect.height() * level,
        );
        painter.circle_filled(marker, 5.0, theme::MAGENTA);
    }
    for (index, point) in points.into_iter().enumerate() {
        painter.circle_filled(point, 3.5, theme::LIME);
        if (1..=5).contains(&index) {
            let response = ui.interact(
                Rect::from_center_size(point, Vec2::splat(16.0)),
                ui.id().with(("env-breakpoint", index)),
                Sense::drag(),
            );
            let (time_param, level_param) = match index {
                1 => (&params.env1.delay, None),
                2 => (&params.env1.attack, Some(&params.env1.attack_curve)),
                3 => (&params.env1.hold, None),
                4 => (&params.env1.decay, Some(&params.env1.sustain)),
                _ => (&params.env1.release, Some(&params.env1.release_curve)),
            };
            let drag_id = response.id.with("breakpoint-start");
            if response.drag_started() {
                ui.data_mut(|data| {
                    data.insert_temp(
                        drag_id,
                        (
                            time_param.unmodulated_normalized_value(),
                            level_param.map(|param| param.unmodulated_normalized_value()),
                        ),
                    )
                });
                setter.begin_set_parameter(time_param);
                if let Some(param) = level_param {
                    setter.begin_set_parameter(param);
                }
            }
            if response.dragged() {
                let start = ui
                    .data(|data| data.get_temp::<(f32, Option<f32>)>(drag_id))
                    .unwrap_or((time_param.unmodulated_normalized_value(), None));
                let time = (start.0 + response.drag_delta().x / rect.width()).clamp(0.0, 1.0);
                setter.set_parameter_normalized(time_param, time);
                if let (Some(param), Some(start_level)) = (level_param, start.1) {
                    let level =
                        (start_level - response.drag_delta().y / rect.height()).clamp(0.0, 1.0);
                    setter.set_parameter_normalized(param, level);
                }
            }
            if response.drag_stopped() {
                setter.end_set_parameter(time_param);
                if let Some(param) = level_param {
                    setter.end_set_parameter(param);
                }
                ui.data_mut(|data| data.remove::<(f32, Option<f32>)>(drag_id));
            }
        }
    }
}

fn draw_keyboard(ui: &mut egui::Ui, state: &mut AppState, shared: &UiShared) {
    egui::Frame::NONE
        .fill(theme::PANEL)
        .stroke(Stroke::new(1.0_f32, theme::PANEL_BORDER))
        .show(ui, |ui| {
            ui.set_min_height(theme::KEYBOARD_HEIGHT);
            ui.horizontal(|ui| {
                ui.spacing_mut().slider_width = 44.0;
                let pitch_response = ui.add(
                    egui::Slider::new(&mut state.pitch_wheel, -1.0..=1.0)
                        .vertical()
                        .text("PITCH"),
                );
                if pitch_response.changed() {
                    queue_keyboard(
                        shared,
                        KeyboardCommand::PitchBend {
                            value: state.pitch_wheel,
                        },
                    );
                }
                let mod_response = ui.add(
                    egui::Slider::new(&mut state.mod_wheel, 0.0..=1.0)
                        .vertical()
                        .text("MOD"),
                );
                if mod_response.changed() {
                    queue_keyboard(
                        shared,
                        KeyboardCommand::ModWheel {
                            value: state.mod_wheel,
                        },
                    );
                }
                let _ = ui.button("−");
                egui::ScrollArea::horizontal()
                    .id_salt("nova-keyboard")
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            for index in 0..48 {
                                let note = 48 + index as u8;
                                let pitch_class = note % 12;
                                let black = [1, 3, 6, 8, 10].contains(&pitch_class);
                                let (rect, response) =
                                    ui.allocate_exact_size(Vec2::new(20.0, 46.0), Sense::click());
                                let down = response.is_pointer_button_down_on();
                                if down && !state.pressed_keys[index] {
                                    state.pressed_keys[index] = queue_keyboard(
                                        shared,
                                        KeyboardCommand::NoteOn {
                                            note,
                                            velocity: 0.8,
                                        },
                                    );
                                } else if !down
                                    && state.pressed_keys[index]
                                    && queue_keyboard(shared, KeyboardCommand::NoteOff { note })
                                {
                                    state.pressed_keys[index] = false;
                                }
                                let fill =
                                    if state.pressed_keys[index] || state.external_held[index] {
                                        theme::CYAN
                                    } else if black {
                                        theme::KEY_BLACK
                                    } else {
                                        theme::KEY_WHITE
                                    };
                                ui.painter().rect_filled(rect, 2.0, fill);
                                ui.painter().rect_stroke(
                                    rect,
                                    egui::CornerRadius::same(2),
                                    Stroke::new(1.0_f32, theme::PANEL_BORDER),
                                    egui::StrokeKind::Inside,
                                );
                                if pitch_class == 0 {
                                    ui.painter().text(
                                        rect.left_bottom() + Vec2::new(2.0, -2.0),
                                        egui::Align2::LEFT_BOTTOM,
                                        format!("C{}", note / 12 - 1),
                                        egui::FontId::proportional(8.0),
                                        theme::BACKGROUND,
                                    );
                                }
                            }
                        });
                    });
                let _ = ui.button("+");
            });
        });
}

fn queue_keyboard(shared: &UiShared, command: KeyboardCommand) -> bool {
    if let Ok(mut producer) = shared.keyboard_tx.lock() {
        producer.push(command).is_ok()
    } else {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn page_shortcuts_map_digits_one_to_seven_only() {
        assert_eq!(page_for_key(egui::Key::Num1), Some(0));
        assert_eq!(page_for_key(egui::Key::Num7), Some(6));
        assert_eq!(page_for_key(egui::Key::Num8), None);
        // Letters used by Ableton's Computer MIDI Keyboard must never be shortcuts.
        for key in [egui::Key::A, egui::Key::W, egui::Key::Z, egui::Key::K] {
            assert_eq!(page_for_key(key), None);
        }
    }
}

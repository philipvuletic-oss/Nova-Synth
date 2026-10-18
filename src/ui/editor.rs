use crate::engine::{AudioTelemetry, KeyboardCommand};
use crate::params::NovaParams;
use crate::ui::{pages, theme, widgets, UiShared};
use nih_plug::prelude::{Editor, Param, ParamSetter};
use nih_plug_egui::{
    create_egui_editor,
    egui::{self, Pos2, Rect, Sense, Stroke, Vec2},
    resizable_window::ResizableWindow,
};
use std::sync::Arc;

struct AppState {
    page: usize,
    pressed_keys: [bool; 48],
    pitch_wheel: f32,
    mod_wheel: f32,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            page: 0,
            pressed_keys: [false; 48],
            pitch_wheel: 0.0,
            mod_wheel: 0.0,
        }
    }
}

pub fn editor(params: Arc<NovaParams>, shared: Arc<UiShared>) -> Option<Box<dyn Editor>> {
    let egui_state = params.editor_state.clone();
    let telemetry = shared.telemetry.clone();
    create_egui_editor(
        egui_state.clone(),
        AppState::default(),
        |_, _| {},
        move |ctx, setter, state| {
            theme::apply(ctx);
            let scale = params.global.gui_scale.value().clamp(0.75, 2.0);
            ctx.set_zoom_factor(scale);
            if let Some(key) = ctx.input(|input| {
                input.events.iter().find_map(|event| match event {
                    egui::Event::Key {
                        key, pressed: true, ..
                    } => Some(*key),
                    _ => None,
                })
            }) {
                let key_index = match key {
                    egui::Key::Num1 => Some(0),
                    egui::Key::Num2 => Some(1),
                    egui::Key::Num3 => Some(2),
                    egui::Key::Num4 => Some(3),
                    egui::Key::Num5 => Some(4),
                    egui::Key::Num6 => Some(5),
                    egui::Key::Num7 => Some(6),
                    _ => None,
                };
                if let Some(index) = key_index {
                    state.page = index;
                }
            }
            ResizableWindow::new("nova-main")
                .min_size(Vec2::new(840.0, 600.0))
                .show(ctx, egui_state.as_ref(), |ui| {
                    ui.spacing_mut().item_spacing = Vec2::new(8.0, 6.0);
                    draw_top_bar(ui, &params, setter, &telemetry);
                    ui.add_space(2.0);
                    widgets::page_tabs::show(ui, &mut state.page);
                    ui.separator();
                    egui::ScrollArea::vertical()
                        .id_salt("nova-page-area")
                        .max_height((ui.available_height() - 308.0).max(120.0))
                        .show(ui, |ui| pages::show(ui, state.page, &params, setter));
                    draw_modulator_strip(ui, &params, setter);
                    draw_keyboard(ui, state, &shared);
                });
        },
    )
}

fn draw_top_bar(
    ui: &mut egui::Ui,
    params: &NovaParams,
    setter: &ParamSetter<'_>,
    telemetry: &AudioTelemetry,
) {
    egui::Frame::NONE
        .fill(theme::PANEL)
        .stroke(Stroke::new(1.0_f32, theme::PANEL_BORDER))
        .corner_radius(egui::CornerRadius::same(8))
        .show(ui, |ui| {
            ui.set_min_height(theme::TOP_BAR_HEIGHT);
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new("NOVA")
                        .size(22.0)
                        .strong()
                        .color(theme::CYAN),
                );
                ui.separator();
                let _ = ui.button("‹");
                let _ = ui
                    .label(egui::RichText::new("Init").strong())
                    .on_hover_text("Mouse wheel changes presets");
                let _ = ui.button("›");
                let _ = ui.button("Browser");
                let _ = ui.button("Save");
                let _ = ui.button("↶");
                let _ = ui.button("↷");
                let _ = ui.button("A/B");
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(format!("{} voices", telemetry.active_voices()));
                    ui.label(format!("CPU {:.1}%", telemetry.cpu_load() * 100.0));
                    let _ =
                        widgets::meter::meter(ui, telemetry.peak_left(), telemetry.peak_right());
                    let _ = widgets::knob::knob(ui, &params.master_volume, setter, "Master");
                });
            });
        });
}

fn draw_modulator_strip(ui: &mut egui::Ui, params: &NovaParams, setter: &ParamSetter<'_>) {
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
                    envelope_graph(ui, params, setter);
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
                    ui.label(egui::RichText::new("MACROS").color(theme::LABEL_TEXT));
                    ui.label("8 assignable macro controls · Coming in M6");
                });
            });
        });
}

fn envelope_graph(ui: &mut egui::Ui, params: &NovaParams, setter: &ParamSetter<'_>) {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 52.0), Sense::hover());
    let painter = ui.painter_at(rect);
    for i in 1..4 {
        let x = rect.left() + rect.width() * i as f32 / 4.0;
        painter.line_segment(
            [Pos2::new(x, rect.top()), Pos2::new(x, rect.bottom())],
            Stroke::new(1.0_f32, theme::GRID),
        );
    }
    painter.line_segment(
        [
            Pos2::new(rect.left(), rect.bottom() - 2.0),
            Pos2::new(rect.right(), rect.bottom() - 2.0),
        ],
        Stroke::new(1.0_f32, theme::GRID),
    );

    let points = [
        Pos2::new(rect.left(), rect.bottom()),
        Pos2::new(rect.left() + rect.width() * 0.2, rect.top()),
        Pos2::new(rect.left() + rect.width() * 0.38, rect.top()),
        Pos2::new(
            rect.left() + rect.width() * 0.62,
            rect.bottom() * 0.48 + rect.top() * 0.52,
        ),
        Pos2::new(rect.right(), rect.bottom()),
    ];
    painter.add(egui::Shape::line(
        points.to_vec(),
        Stroke::new(5.0_f32, theme::LIME.gamma_multiply(0.1)),
    ));
    painter.add(egui::Shape::line(
        points.to_vec(),
        Stroke::new(2.0_f32, theme::LIME),
    ));
    for (index, point) in points.into_iter().enumerate() {
        painter.circle_filled(point, 3.5, theme::LIME);
        if (1..=3).contains(&index) {
            let response = ui.interact(
                Rect::from_center_size(point, Vec2::splat(16.0)),
                ui.id().with(("env-breakpoint", index)),
                Sense::drag(),
            );
            let param = match index {
                1 => &params.env1.attack,
                2 => &params.env1.hold,
                _ => &params.env1.decay,
            };
            if response.drag_started() {
                setter.begin_set_parameter(param);
            }
            if response.dragged() {
                let normalized = (param.unmodulated_normalized_value()
                    - response.drag_delta().y * 0.004)
                    .clamp(0.0, 1.0);
                setter.set_parameter_normalized(param, normalized);
            }
            if response.drag_stopped() {
                setter.end_set_parameter(param);
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
                                    queue_keyboard(
                                        shared,
                                        KeyboardCommand::NoteOn {
                                            note,
                                            velocity: 0.8,
                                        },
                                    );
                                    state.pressed_keys[index] = true;
                                } else if !down && state.pressed_keys[index] {
                                    queue_keyboard(shared, KeyboardCommand::NoteOff { note });
                                    state.pressed_keys[index] = false;
                                }
                                let fill = if state.pressed_keys[index] {
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

fn queue_keyboard(shared: &UiShared, command: KeyboardCommand) {
    if let Ok(mut producer) = shared.keyboard_tx.lock() {
        let _ = producer.push(command);
    }
}

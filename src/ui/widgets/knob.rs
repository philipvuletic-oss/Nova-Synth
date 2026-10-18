use crate::ui::theme;
use nih_plug::prelude::{FloatParam, Param, ParamSetter};
use nih_plug_egui::egui::{self, Pos2, Response, Sense, Stroke, Ui, Vec2};

pub fn knob(ui: &mut Ui, param: &FloatParam, setter: &ParamSetter<'_>, label: &str) -> Response {
    knob_with_modulation(ui, param, setter, label, None, 0)
}

pub fn knob_with_modulation(
    ui: &mut Ui,
    param: &FloatParam,
    setter: &ParamSetter<'_>,
    label: &str,
    modulation: Option<(f32, f32)>,
    modulation_count: usize,
) -> Response {
    let (rect, response) = ui.allocate_exact_size(Vec2::splat(66.0), Sense::click_and_drag());
    let painter = ui.painter_at(rect);
    let center = Pos2::new(rect.center().x, rect.top() + 28.0);
    let radius = 22.0;
    let start = std::f32::consts::PI * 0.75;
    let sweep = std::f32::consts::PI * 1.5;
    let value = param.unmodulated_normalized_value().clamp(0.0, 1.0);
    let steps = 48;

    draw_arc(
        &painter,
        center,
        radius,
        start,
        sweep,
        steps,
        Stroke::new(3.0_f32, theme::PANEL_BORDER),
    );
    draw_arc(
        &painter,
        center,
        radius,
        start,
        sweep * value,
        (steps as f32 * value).ceil() as usize,
        Stroke::new(7.0_f32, theme::CYAN.gamma_multiply(0.08)),
    );
    draw_arc(
        &painter,
        center,
        radius,
        start,
        sweep * value,
        (steps as f32 * value).ceil() as usize,
        Stroke::new(5.0_f32, theme::CYAN.gamma_multiply(0.18)),
    );
    draw_arc(
        &painter,
        center,
        radius,
        start,
        sweep * value,
        (steps as f32 * value).ceil() as usize,
        Stroke::new(3.0_f32, theme::CYAN),
    );

    if let Some((depth, live_position)) = modulation {
        let end = (value + depth).clamp(0.0, 1.0);
        let arc_start = value.min(end);
        let arc_sweep = sweep * (end - value).abs();
        draw_arc(
            &painter,
            center,
            radius + 4.0,
            start + sweep * arc_start,
            arc_sweep,
            (steps as f32 * (end - value).abs()).ceil() as usize,
            Stroke::new(2.0_f32, theme::MAGENTA),
        );
        let marker_angle = start + sweep * live_position.clamp(0.0, 1.0);
        let marker = Pos2::new(
            center.x + marker_angle.cos() * (radius + 4.0),
            center.y + marker_angle.sin() * (radius + 4.0),
        );
        painter.circle_filled(marker, 2.5, theme::MAGENTA);
    }
    painter.circle_filled(center, 2.5, theme::VALUE_TEXT);

    let text = param.to_string();
    painter.text(
        Pos2::new(rect.center().x, rect.top() + 53.0),
        egui::Align2::CENTER_CENTER,
        &text,
        egui::FontId::monospace(10.0),
        theme::VALUE_TEXT,
    );
    painter.text(
        Pos2::new(rect.center().x, rect.bottom() - 2.0),
        egui::Align2::CENTER_BOTTOM,
        label.to_uppercase(),
        egui::FontId::proportional(9.0),
        theme::LABEL_TEXT,
    );

    let id = response.id.with("knob-drag-start");
    if response.drag_started() {
        ui.data_mut(|data| data.insert_temp(id, value));
        setter.begin_set_parameter(param);
    }
    if response.dragged() {
        let initial = ui.data(|data| data.get_temp::<f32>(id)).unwrap_or(value);
        let fine = ui.input(|input| input.modifiers.shift || input.modifiers.ctrl);
        let sensitivity = if fine { 0.0015 } else { 0.006 };
        let normalized = (initial - response.drag_delta().y * sensitivity).clamp(0.0, 1.0);
        setter.set_parameter_normalized(param, normalized);
    }
    if response.drag_stopped() {
        setter.end_set_parameter(param);
        ui.data_mut(|data| data.remove::<f32>(id));
    }
    if response.double_clicked() {
        setter.begin_set_parameter(param);
        setter.set_parameter(param, param.default_plain_value());
        setter.end_set_parameter(param);
    }
    if response.clicked() && ui.input(|input| input.modifiers.alt) {
        ui.data_mut(|data| data.insert_temp(response.id.with("type-value"), true));
    }
    if response.hovered() {
        let wheel = ui.input(|input| input.smooth_scroll_delta.y);
        if wheel.abs() > 0.0 {
            let finer = ui.input(|input| input.modifiers.shift);
            let normalized = if wheel > 0.0 {
                param.next_normalized_step(value, finer)
            } else {
                param.previous_normalized_step(value, finer)
            };
            setter.begin_set_parameter(param);
            setter.set_parameter_normalized(param, normalized);
            setter.end_set_parameter(param);
        }
    }

    let modulation_label = match modulation_count {
        0 => String::from("no modulations"),
        1 => String::from("1 modulation"),
        count => format!("{count} modulations"),
    };
    let response = response.on_hover_text(format!("{label}: {text} · {modulation_label}"));
    let type_value_id = response.id.with("type-value");
    response.context_menu(|ui| {
        if ui.button("Reset").clicked() {
            setter.begin_set_parameter(param);
            setter.set_parameter(param, param.default_plain_value());
            setter.end_set_parameter(param);
        }
        if ui.button("Type value…").clicked() {
            ui.data_mut(|data| data.insert_temp(type_value_id, true));
        }
        ui.separator();
        let _ = ui.add_enabled(false, egui::Button::new("MIDI learn"));
        let _ = ui.add_enabled(false, egui::Button::new("Clear MIDI learn"));
        let _ = ui.add_enabled(false, egui::Button::new("Show modulations"));
        let _ = ui.add_enabled(false, egui::Button::new("Remove all modulations"));
        let _ = ui.add_enabled(false, egui::Button::new("Copy/Paste value"));
    });

    let mut show_type = ui.data(|data| data.get_temp::<bool>(type_value_id).unwrap_or(false));
    if show_type {
        let text_id = type_value_id.with("input");
        let mut typed = ui
            .data(|data| data.get_temp::<String>(text_id))
            .unwrap_or_else(|| text.clone());
        egui::Window::new(format!("Set {label}"))
            .id(type_value_id)
            .collapsible(false)
            .resizable(false)
            .show(ui.ctx(), |ui| {
                let response = ui.text_edit_singleline(&mut typed);
                let submit =
                    response.lost_focus() && ui.input(|input| input.key_pressed(egui::Key::Enter));
                if ui.button("Set").clicked() || submit {
                    if let Some(normalized) = param.string_to_normalized_value(&typed) {
                        setter.begin_set_parameter(param);
                        setter.set_parameter_normalized(param, normalized);
                        setter.end_set_parameter(param);
                        show_type = false;
                    }
                }
                if ui.button("Cancel").clicked() {
                    show_type = false;
                }
            });
        ui.data_mut(|data| {
            if show_type {
                data.insert_temp(type_value_id, true);
                data.insert_temp(text_id, typed);
            } else {
                data.remove::<bool>(type_value_id);
                data.remove::<String>(text_id);
            }
        });
    }
    response
}

fn draw_arc(
    painter: &egui::Painter,
    center: Pos2,
    radius: f32,
    start: f32,
    sweep: f32,
    steps: usize,
    stroke: Stroke,
) {
    if steps == 0 || sweep <= 0.0 {
        return;
    }
    let points = (0..=steps)
        .map(|step| {
            let angle = start + sweep * step as f32 / steps as f32;
            Pos2::new(
                center.x + angle.cos() * radius,
                center.y + angle.sin() * radius,
            )
        })
        .collect::<Vec<_>>();
    painter.add(egui::Shape::line(points, stroke));
}

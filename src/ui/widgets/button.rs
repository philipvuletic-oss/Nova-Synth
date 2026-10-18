use crate::ui::theme;
use nih_plug_egui::egui::{Button, Response, Ui};

pub fn pill(ui: &mut Ui, text: &str, selected: bool) -> Response {
    let button = Button::new(text)
        .fill(if selected {
            theme::CYAN.gamma_multiply(0.2)
        } else {
            theme::PANEL
        })
        .stroke(nih_plug_egui::egui::Stroke::new(
            1.0_f32,
            if selected {
                theme::CYAN
            } else {
                theme::PANEL_BORDER
            },
        ))
        .corner_radius(nih_plug_egui::egui::CornerRadius::same(12));
    ui.add(button)
}

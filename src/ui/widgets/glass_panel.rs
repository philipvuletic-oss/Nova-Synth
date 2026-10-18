use crate::ui::theme;
use nih_plug_egui::egui::{self, Ui};

pub fn show<R>(
    ui: &mut Ui,
    title: &str,
    enabled: &mut bool,
    add_contents: impl FnOnce(&mut Ui) -> R,
) -> R {
    egui::Frame::group(ui.style())
        .fill(theme::PANEL.gamma_multiply(0.92))
        .stroke(egui::Stroke::new(1.0_f32, theme::PANEL_BORDER))
        .corner_radius(egui::CornerRadius::same(theme::PANEL_RADIUS))
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            ui.horizontal(|ui| {
                if super::button::pill(ui, if *enabled { "●" } else { "○" }, *enabled)
                    .on_hover_text("Power")
                    .clicked()
                {
                    *enabled = !*enabled;
                }
                ui.label(egui::RichText::new(title).strong().color(theme::LABEL_TEXT));
            });
            add_contents(ui)
        })
        .inner
}

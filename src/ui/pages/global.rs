use super::coming_soon;
use nih_plug_egui::egui::Ui;

pub fn show(ui: &mut Ui) {
    coming_soon(
        ui,
        "GLOBAL",
        "M20",
        "Global performance, tuning, and interface preferences.",
    );
}

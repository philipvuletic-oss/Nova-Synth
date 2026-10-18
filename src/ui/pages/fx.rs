use super::coming_soon;
use nih_plug_egui::egui::Ui;

pub fn show(ui: &mut Ui) {
    coming_soon(
        ui,
        "EFFECTS",
        "M9–M11",
        "Three flexible effect racks with buses and splitters.",
    );
}

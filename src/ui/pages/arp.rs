use super::coming_soon;
use nih_plug_egui::egui::Ui;

pub fn show(ui: &mut Ui) {
    coming_soon(
        ui,
        "ARPEGGIATOR",
        "M12",
        "Create note patterns and scale-locked performances.",
    );
}

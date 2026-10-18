use super::coming_soon;
use nih_plug_egui::egui::Ui;

pub fn show(ui: &mut Ui) {
    coming_soon(
        ui,
        "CLIP SEQUENCER",
        "M17",
        "Arrange clips, notes, velocities, and automation.",
    );
}

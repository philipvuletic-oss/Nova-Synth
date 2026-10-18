use super::coming_soon;
use nih_plug_egui::egui::Ui;

pub fn show(ui: &mut Ui) {
    coming_soon(
        ui,
        "MODULATION",
        "M6–M7",
        "Drag modulation sources onto controls and shape each connection.",
    );
}

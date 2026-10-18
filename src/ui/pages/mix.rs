use super::coming_soon;
use nih_plug_egui::egui::Ui;

pub fn show(ui: &mut Ui) {
    coming_soon(ui, "MIXER", "M8", "Channel strips and signal routing.");
}

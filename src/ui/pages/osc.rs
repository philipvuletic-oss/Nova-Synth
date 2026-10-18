use super::coming_soon;
use nih_plug_egui::egui::Ui;

pub fn show(ui: &mut Ui) {
    coming_soon(
        ui,
        "OSCILLATORS",
        "M1–M3",
        "The three oscillator cards, wavetable displays, sub and noise live here.",
    );
}

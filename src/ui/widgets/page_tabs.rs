use crate::ui::theme;
use nih_plug_egui::egui::{self, Ui};

pub const PAGES: [&str; 7] = ["OSC", "MIX", "FX", "MOD", "ARP", "CLIP", "GLOBAL"];

pub fn show(ui: &mut Ui, selected: &mut usize) {
    ui.horizontal(|ui| {
        for (index, name) in PAGES.iter().enumerate() {
            let response = super::button::pill(ui, name, *selected == index);
            if response.clicked() {
                *selected = index;
            }
            if *selected == index {
                ui.painter().line_segment(
                    [response.rect.left_bottom(), response.rect.right_bottom()],
                    egui::Stroke::new(2.0_f32, theme::CYAN),
                );
            }
        }
    });
}

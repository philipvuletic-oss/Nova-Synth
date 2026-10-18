use crate::params::NovaParams;
use nih_plug::prelude::ParamSetter;
use nih_plug_egui::egui::{self, Ui};

pub mod arp;
pub mod clip;
pub mod fx;
pub mod global;
pub mod mix;
pub mod modulation;
pub mod osc;

pub fn show(ui: &mut Ui, page: usize, params: &NovaParams, setter: &ParamSetter<'_>) {
    match page {
        0 => osc::show(ui),
        1 => mix::show(ui),
        2 => fx::show(ui),
        3 => modulation::show(ui),
        4 => arp::show(ui),
        5 => clip::show(ui),
        _ => global::show(ui),
    }
    let _ = (params, setter);
}

pub fn coming_soon(ui: &mut Ui, title: &str, milestone: &str, description: &str) {
    egui::Frame::group(ui.style()).show(ui, |ui| {
        ui.set_min_size(egui::vec2(ui.available_width(), 150.0));
        ui.vertical_centered(|ui| {
            ui.heading(title);
            ui.label(format!("Coming in {milestone}"));
            ui.label(description);
        });
    });
}

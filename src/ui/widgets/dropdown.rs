use nih_plug_egui::egui::{ComboBox, Ui};

pub fn dropdown(ui: &mut Ui, id: &'static str, selected: &mut usize, values: &[&str]) {
    let current = values.get(*selected).copied().unwrap_or("Select");
    ComboBox::from_id_salt(id)
        .selected_text(current)
        .show_ui(ui, |ui| {
            for (index, value) in values.iter().enumerate() {
                ui.selectable_value(selected, index, *value);
            }
        });
}

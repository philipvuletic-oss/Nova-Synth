use crate::ui::theme;
use nih_plug_egui::egui::{self, Response, Sense, Ui, Vec2};

pub fn meter(ui: &mut Ui, left: f32, right: f32) -> Response {
    let (rect, response) = ui.allocate_exact_size(Vec2::new(112.0, 14.0), Sense::hover());
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 3.0, theme::PANEL_BORDER);
    let half = rect.width() * 0.5;
    for (index, level) in [left, right].into_iter().enumerate() {
        let x = rect.left() + index as f32 * half;
        let width = half * level.clamp(0.0, 1.0);
        let color = if level > 0.707 {
            theme::AMBER
        } else {
            theme::CYAN
        };
        painter.rect_filled(
            egui::Rect::from_min_size(egui::pos2(x, rect.top()), Vec2::new(width, rect.height())),
            2.0,
            color,
        );
    }
    response.on_hover_text("Stereo output peak")
}

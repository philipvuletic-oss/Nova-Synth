use nih_plug_egui::egui::Color32;

pub const WINDOW_WIDTH: f32 = 1240.0;
pub const WINDOW_HEIGHT: f32 = 780.0;
pub const TOP_BAR_HEIGHT: f32 = 48.0;
pub const TAB_HEIGHT: f32 = 36.0;
pub const MODULATOR_HEIGHT: f32 = 190.0;
pub const KEYBOARD_HEIGHT: f32 = 56.0;
pub const PANEL_RADIUS: u8 = 12;

pub const BACKGROUND: Color32 = Color32::from_rgb(10, 13, 20);
pub const BACKGROUND_CENTER: Color32 = Color32::from_rgb(14, 19, 32);
pub const PANEL: Color32 = Color32::from_rgb(18, 24, 38);
pub const PANEL_BORDER: Color32 = Color32::from_rgb(31, 42, 61);
pub const CYAN: Color32 = Color32::from_rgb(62, 242, 255);
pub const MAGENTA: Color32 = Color32::from_rgb(255, 62, 165);
pub const LIME: Color32 = Color32::from_rgb(182, 255, 62);
pub const VIOLET: Color32 = Color32::from_rgb(139, 123, 255);
pub const AMBER: Color32 = Color32::from_rgb(255, 178, 62);
pub const MUTED: Color32 = Color32::from_rgb(91, 102, 122);
pub const VALUE_TEXT: Color32 = Color32::from_rgb(230, 236, 245);
pub const LABEL_TEXT: Color32 = Color32::from_rgb(138, 148, 168);
pub const GRID: Color32 = Color32::from_rgb(15, 15, 15);
pub const GRID_DOT: Color32 = Color32::from_rgba_premultiplied(8, 8, 8, 8);
pub const KEY_WHITE: Color32 = Color32::from_rgb(215, 223, 235);
pub const KEY_BLACK: Color32 = Color32::from_rgb(35, 43, 59);

// TODO: Embed Inter and JetBrains Mono (SIL OFL); use the default egui font stack meanwhile.

pub fn apply(ctx: &nih_plug_egui::egui::Context) {
    use nih_plug_egui::egui::{style::WidgetVisuals, Visuals};
    let mut visuals = Visuals::dark();
    visuals.panel_fill = BACKGROUND;
    visuals.window_fill = PANEL;
    visuals.extreme_bg_color = BACKGROUND;
    visuals.faint_bg_color = PANEL;
    visuals.widgets.noninteractive = WidgetVisuals {
        bg_fill: PANEL,
        weak_bg_fill: PANEL,
        bg_stroke: nih_plug_egui::egui::Stroke::new(1.0_f32, PANEL_BORDER),
        fg_stroke: nih_plug_egui::egui::Stroke::new(1.0_f32, VALUE_TEXT),
        corner_radius: nih_plug_egui::egui::CornerRadius::same(PANEL_RADIUS),
        expansion: 0.0,
    };
    visuals.override_text_color = Some(VALUE_TEXT);
    ctx.set_visuals(visuals);
}

pub fn paint_background(ui: &mut nih_plug_egui::egui::Ui) {
    use nih_plug_egui::egui::{pos2, Stroke};
    let bounds = ui.max_rect();
    let painter = ui.painter();
    painter.rect_filled(bounds, 0.0, BACKGROUND);
    let center = bounds.center();
    painter.circle_filled(
        center,
        bounds.width().min(bounds.height()) * 0.46,
        BACKGROUND_CENTER.gamma_multiply(0.16),
    );
    let mut y = bounds.top();
    while y < bounds.bottom() {
        let mut x = bounds.left();
        while x < bounds.right() {
            painter.circle_filled(pos2(x, y), 0.65, GRID_DOT);
            x += 24.0;
        }
        y += 24.0;
    }
    painter.rect_stroke(
        bounds,
        0.0,
        Stroke::new(1.0_f32, PANEL_BORDER),
        nih_plug_egui::egui::StrokeKind::Inside,
    );
}

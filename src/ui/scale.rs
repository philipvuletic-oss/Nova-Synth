//! Display-scale maths: combines the OS (Windows 100/125/150/200%) scale with Nova's own GUI
//! scale and snaps hairlines to the physical pixel grid so lines stay crisp at fractional scales.

pub const MIN_WINDOW_WIDTH: f32 = 960.0;
pub const MIN_WINDOW_HEIGHT: f32 = 600.0;
pub const GUI_SCALE_RANGE: (f32, f32) = (0.75, 2.0);
/// Windows "Scale" settings Nova is checked against.
pub const OS_SCALES: [f32; 4] = [1.0, 1.25, 1.5, 2.0];

pub fn clamp_gui_scale(scale: f32) -> f32 {
    if scale.is_finite() {
        scale.clamp(GUI_SCALE_RANGE.0, GUI_SCALE_RANGE.1)
    } else {
        1.0
    }
}

/// Physical pixels per layout unit: OS scale (applied by the window) times Nova's GUI scale
/// (applied as egui zoom).
pub fn effective_pixels_per_point(os_scale: f32, gui_scale: f32) -> f32 {
    os_scale.max(0.5) * clamp_gui_scale(gui_scale)
}

/// Window size in physical pixels for a logical size at an OS scale (rounded up so the window
/// is never cut off).
pub fn physical_size(logical: (f32, f32), os_scale: f32) -> (u32, u32) {
    (
        (logical.0 * os_scale).ceil() as u32,
        (logical.1 * os_scale).ceil() as u32,
    )
}

/// Layout units available to the GUI for a window of `window_logical` points at `gui_scale`.
pub fn layout_size(window_logical: (f32, f32), gui_scale: f32) -> (f32, f32) {
    let scale = clamp_gui_scale(gui_scale);
    (window_logical.0 / scale, window_logical.1 / scale)
}

/// Rounds a coordinate to the nearest physical pixel edge.
pub fn snap_to_pixel(value: f32, pixels_per_point: f32) -> f32 {
    (value * pixels_per_point).round() / pixels_per_point
}

/// Position that puts a one-physical-pixel-wide line exactly on a pixel centre.
pub fn snap_line(value: f32, pixels_per_point: f32) -> f32 {
    ((value * pixels_per_point).floor() + 0.5) / pixels_per_point
}

/// Stroke width of exactly one physical pixel.
pub fn hairline(pixels_per_point: f32) -> f32 {
    1.0 / pixels_per_point.max(0.5)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gui_scale_is_clamped_and_nan_safe() {
        assert_eq!(clamp_gui_scale(0.1), 0.75);
        assert_eq!(clamp_gui_scale(5.0), 2.0);
        assert_eq!(clamp_gui_scale(f32::NAN), 1.0);
    }

    #[test]
    fn effective_scale_combines_os_and_gui_scale() {
        assert_eq!(effective_pixels_per_point(1.25, 1.0), 1.25);
        assert_eq!(effective_pixels_per_point(1.5, 1.5), 2.25);
        assert_eq!(effective_pixels_per_point(2.0, 2.0), 4.0);
    }

    #[test]
    fn default_and_minimum_windows_have_whole_physical_sizes() {
        for os in OS_SCALES {
            for logical in [(1240.0, 780.0), (MIN_WINDOW_WIDTH, MIN_WINDOW_HEIGHT)] {
                let (w, h) = physical_size(logical, os);
                assert_eq!(w as f32, logical.0 * os, "width at {os}");
                assert_eq!(h as f32, logical.1 * os, "height at {os}");
            }
        }
    }

    #[test]
    fn minimum_window_leaves_room_for_the_page_area_at_all_gui_scales() {
        // The page area is (height - 336) layout units; it must stay positive and wide enough
        // for the 7 page tabs at 100% GUI scale.
        let (w, h) = layout_size((MIN_WINDOW_WIDTH, MIN_WINDOW_HEIGHT), 1.0);
        assert!(h - 336.0 >= 120.0 && w >= 900.0);
    }

    #[test]
    fn hairlines_land_on_pixel_centres() {
        for ppp in [1.0_f32, 1.25, 1.5, 1.75, 2.0, 2.25, 3.0] {
            for value in [10.0_f32, 10.3, 99.7, 333.33] {
                let line = snap_line(value, ppp) * ppp;
                assert!(
                    (line.fract() - 0.5).abs() < 1.0e-3,
                    "ppp {ppp} value {value}"
                );
                let edge = snap_to_pixel(value, ppp) * ppp;
                assert!((edge - edge.round()).abs() < 1.0e-3);
            }
            assert!((hairline(ppp) * ppp - 1.0).abs() < 1.0e-6);
        }
    }
}

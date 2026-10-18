use snapshot::nova_pages::{screenshot, Mode, PAGE_NAMES};

#[test]
fn every_page_renders_non_empty_frames() {
    for (index, name) in PAGE_NAMES.iter().enumerate() {
        let image = screenshot(index, 640, 480, 1.0, Mode::Idle);
        assert_eq!(image.rgba.len(), 640 * 480 * 4, "{name}");
        assert!(image.distinct_colors() > 40, "{name} page looks empty");
    }
}

#[test]
fn playing_mode_differs_from_idle() {
    let idle = screenshot(0, 800, 600, 1.0, Mode::Idle);
    let playing = screenshot(0, 800, 600, 1.0, Mode::Playing);
    assert_ne!(idle.rgba, playing.rgba);
}

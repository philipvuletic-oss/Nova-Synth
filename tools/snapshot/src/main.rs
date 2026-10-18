//! `cargo run -p snapshot [-- <output dir>]` writes the GUI screenshots to docs/screenshots.

use snapshot::nova_pages::{screenshot, Mode, PAGE_NAMES};
use std::path::PathBuf;

fn main() {
    let out: PathBuf = std::env::args_os()
        .nth(1)
        .map_or_else(|| PathBuf::from("docs/screenshots"), PathBuf::from);
    let mut jobs: Vec<(String, usize, usize, usize, f32, Mode)> = vec![
        ("osc_page_1240x780".into(), 0, 1240, 780, 1.0, Mode::Idle),
        ("osc_page_1240x780_2x".into(), 0, 1240, 780, 2.0, Mode::Idle),
        ("osc_page_960x600".into(), 0, 960, 600, 1.0, Mode::Idle),
    ];
    for (index, name) in PAGE_NAMES.iter().enumerate().skip(1) {
        jobs.push((
            format!("{name}_page_1240x780"),
            index,
            1240,
            780,
            1.0,
            Mode::Idle,
        ));
    }
    for (index, name) in PAGE_NAMES.iter().enumerate() {
        jobs.push((
            format!("{name}_page_playing_1240x780"),
            index,
            1240,
            780,
            1.0,
            Mode::Playing,
        ));
    }
    for (name, page, width, height, scale, mode) in jobs {
        let image = screenshot(page, width, height, scale, mode);
        let path = out.join(format!("{name}.png"));
        if let Err(error) = image.write_png(&path) {
            eprintln!("failed to write {}: {error}", path.display());
            std::process::exit(1);
        }
        println!(
            "wrote {} ({}x{})",
            path.display(),
            image.width,
            image.height
        );
    }
}

//! Visual regression guard: compares rendered screenshots against golden PNGs and writes
//! before/after side-by-side images for anything that differs.

use crate::Image;
use std::path::Path;

/// A pixel counts as different when any colour channel differs by more than this.
pub const CHANNEL_TOLERANCE: u8 = 6;
/// Fail when more than this fraction of pixels differ.
pub const MAX_DIFFERING_FRACTION: f64 = 0.001;

#[derive(Debug, PartialEq)]
pub enum Verdict {
    Same { differing_fraction: f64 },
    Different { differing_fraction: f64 },
    SizeChanged,
}

impl Verdict {
    pub fn is_ok(&self) -> bool {
        matches!(self, Verdict::Same { .. })
    }
}

pub fn read_png(path: &Path) -> Result<Image, String> {
    let file = std::fs::File::open(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let mut reader = png::Decoder::new(std::io::BufReader::new(file))
        .read_info()
        .map_err(|e| e.to_string())?;
    let mut buffer = vec![0; reader.output_buffer_size().ok_or("image too large")?];
    let info = reader.next_frame(&mut buffer).map_err(|e| e.to_string())?;
    buffer.truncate(info.buffer_size());
    let rgba = match info.color_type {
        png::ColorType::Rgba => buffer,
        png::ColorType::Rgb => buffer
            .chunks_exact(3)
            .flat_map(|p| [p[0], p[1], p[2], 255])
            .collect(),
        _ => return Err(format!("unsupported PNG colour type in {}", path.display())),
    };
    Ok(Image {
        width: info.width as usize,
        height: info.height as usize,
        rgba,
    })
}

pub fn compare(before: &Image, after: &Image) -> Verdict {
    if before.width != after.width || before.height != after.height {
        return Verdict::SizeChanged;
    }
    let differing = before
        .rgba
        .chunks_exact(4)
        .zip(after.rgba.chunks_exact(4))
        .filter(|(a, b)| {
            a.iter()
                .zip(b.iter())
                .take(3)
                .any(|(x, y)| x.abs_diff(*y) > CHANNEL_TOLERANCE)
        })
        .count();
    let differing_fraction = differing as f64 / (before.width * before.height).max(1) as f64;
    if differing_fraction > MAX_DIFFERING_FRACTION {
        Verdict::Different { differing_fraction }
    } else {
        Verdict::Same { differing_fraction }
    }
}

/// Before (left) and after (right) with a 8 px separator.
pub fn side_by_side(before: Option<&Image>, after: &Image) -> Image {
    let gap = 8;
    let height = before.map_or(0, |b| b.height).max(after.height);
    let width = before.map_or(0, |b| b.width) + gap + after.width;
    let mut rgba = vec![255u8; width * height * 4];
    let mut blit = |image: &Image, x0: usize| {
        for y in 0..image.height {
            let src = &image.rgba[y * image.width * 4..(y + 1) * image.width * 4];
            let start = (y * width + x0) * 4;
            rgba[start..start + src.len()].copy_from_slice(src);
        }
    };
    if let Some(before) = before {
        blit(before, 0);
    }
    blit(after, before.map_or(0, |b| b.width) + gap);
    Image {
        width,
        height,
        rgba,
    }
}

/// Compares every PNG in `after_dir` with the same-named PNG in `before_dir`. Writes
/// `<name>_before_after.png` into `out_dir` for each differing, new or removed image.
/// Returns the list of problems (empty = identical within tolerance).
pub fn compare_dirs(before_dir: &Path, after_dir: &Path, out_dir: &Path) -> Vec<String> {
    let names = |dir: &Path| -> Vec<String> {
        let mut names: Vec<String> = std::fs::read_dir(dir)
            .into_iter()
            .flatten()
            .flatten()
            .filter_map(|e| e.file_name().into_string().ok())
            .filter(|n| n.ends_with(".png"))
            .collect();
        names.sort();
        names
    };
    let before_names = names(before_dir);
    let mut problems = Vec::new();
    for name in names(after_dir) {
        let after = match read_png(&after_dir.join(&name)) {
            Ok(image) => image,
            Err(error) => {
                problems.push(format!("{name}: {error}"));
                continue;
            }
        };
        let before = if before_names.contains(&name) {
            read_png(&before_dir.join(&name)).ok()
        } else {
            None
        };
        let verdict = before.as_ref().map(|b| compare(b, &after));
        let problem = match &verdict {
            None => Some(format!("{name}: new image (no golden)")),
            Some(Verdict::SizeChanged) => Some(format!("{name}: size changed")),
            Some(Verdict::Different { differing_fraction }) => Some(format!(
                "{name}: {:.3}% of pixels differ",
                differing_fraction * 100.0
            )),
            Some(Verdict::Same { .. }) => None,
        };
        if let Some(problem) = problem {
            let image = side_by_side(before.as_ref(), &after);
            let _ = image
                .write_png(&out_dir.join(format!("{}_before_after.png", &name[..name.len() - 4])));
            problems.push(problem);
        }
    }
    for name in before_names {
        if !after_dir.join(&name).exists() {
            problems.push(format!("{name}: golden has no matching screenshot"));
        }
    }
    problems
}

#[cfg(test)]
mod tests {
    use super::*;

    fn flat(value: u8) -> Image {
        Image {
            width: 10,
            height: 10,
            rgba: vec![value; 400],
        }
    }

    #[test]
    fn identical_and_tiny_differences_pass() {
        assert!(compare(&flat(100), &flat(100)).is_ok());
        assert!(compare(&flat(100), &flat(104)).is_ok());
    }

    #[test]
    fn large_differences_and_size_changes_fail() {
        assert!(!compare(&flat(100), &flat(160)).is_ok());
        let mut small = flat(100);
        small.width = 5;
        assert_eq!(compare(&flat(100), &small), Verdict::SizeChanged);
    }

    #[test]
    fn side_by_side_is_wider_than_both() {
        let image = side_by_side(Some(&flat(1)), &flat(2));
        assert_eq!(image.width, 28);
        assert_eq!(image.height, 10);
    }
}

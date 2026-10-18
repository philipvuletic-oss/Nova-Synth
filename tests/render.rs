mod common;

use approx::assert_abs_diff_eq;

#[test]
fn a4_renders_at_440_hz() {
    let samples = common::render(48_000.0, 65_536, 512, None);
    let frequency = common::dominant_frequency(&samples, 48_000.0);
    assert_abs_diff_eq!(frequency, 440.0, epsilon = 1.0);
}

#[test]
fn default_render_is_finite_and_below_full_scale() {
    let samples = common::render(48_000.0, 48_000, 64, None);
    assert!(samples.iter().all(|sample| sample.is_finite()));
    assert!(common::peak(&samples) < 1.0);
}

#[test]
fn release_reaches_silence_within_one_hundred_milliseconds_after_release_finishes() {
    let release_end = (0.3 * 48_000.0) as usize;
    let note_off_at = 48_000;
    let samples = common::render(
        48_000.0,
        note_off_at + release_end + 4_800,
        64,
        Some(note_off_at),
    );
    assert!(common::peak(&samples[note_off_at + release_end..]) < 1.0e-5);
}

#[test]
fn output_is_invariant_to_block_size() {
    let one_sample = common::render(48_000.0, 8_192, 1, None);
    let large_block = common::render(48_000.0, 8_192, 512, None);
    for (left, right) in one_sample.iter().zip(large_block.iter()) {
        assert_abs_diff_eq!(left, right, epsilon = 1.0e-6);
    }
}

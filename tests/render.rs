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
fn release_reaches_silence_after_release_time() {
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

#[test]
fn render_is_stable_at_common_sample_rates_and_block_sizes() {
    for sample_rate in [44_100.0, 48_000.0, 96_000.0] {
        for block_size in [1, 64, 512] {
            let samples = common::render(sample_rate, 4_096, block_size, None);
            assert!(samples.iter().all(|sample| sample.is_finite()));
            assert!(common::peak(&samples) < 1.0);
        }
    }
}

#[test]
fn random_variable_block_sizes_match_fixed_block_render() {
    let expected = common::render(48_000.0, 16_384, 512, None);
    let mut state = 0xA341_316C_u32;
    let mut sizes = [0usize; 128];
    for size in &mut sizes {
        state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        *size = 1 + (state as usize % 512);
    }
    let actual = common::render_with_blocks(48_000.0, 16_384, &sizes, None);
    for (expected, actual) in expected.iter().zip(actual.iter()) {
        assert_abs_diff_eq!(expected, actual, epsilon = 1.0e-6);
    }
}

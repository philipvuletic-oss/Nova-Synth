//! One-pole DC blocker (about 5 Hz high-pass). Removes the offset that asymmetric waveforms pick up
//! when they pass through the saturating safety clipper.

#[derive(Clone, Copy, Debug)]
pub struct DcBlocker {
    coefficient: f32,
    previous_input: f32,
    previous_output: f32,
}

impl DcBlocker {
    pub const CUTOFF_HZ: f32 = 5.0;

    pub fn new(sample_rate: f32) -> Self {
        let mut blocker = Self {
            coefficient: 0.0,
            previous_input: 0.0,
            previous_output: 0.0,
        };
        blocker.set_sample_rate(sample_rate);
        blocker
    }

    pub fn set_sample_rate(&mut self, sample_rate: f32) {
        let sample_rate = if sample_rate.is_finite() {
            sample_rate.max(8_000.0)
        } else {
            48_000.0
        };
        self.coefficient = 1.0 - std::f32::consts::TAU * Self::CUTOFF_HZ / sample_rate;
    }

    pub fn reset(&mut self) {
        self.previous_input = 0.0;
        self.previous_output = 0.0;
    }

    #[inline]
    pub fn process(&mut self, input: f32) -> f32 {
        let output = input - self.previous_input + self.coefficient * self.previous_output;
        self.previous_input = input;
        self.previous_output = output;
        output
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn removes_constant_offset() {
        let mut blocker = DcBlocker::new(48_000.0);
        let mut last = 1.0;
        for _ in 0..96_000 {
            last = blocker.process(0.5);
        }
        assert!(last.abs() < 1.0e-3);
    }

    #[test]
    fn passes_audio_frequencies() {
        let mut blocker = DcBlocker::new(48_000.0);
        let mut peak = 0.0f32;
        for i in 0..48_000 {
            let x = (std::f32::consts::TAU * 440.0 * i as f32 / 48_000.0).sin();
            let y = blocker.process(x);
            if i > 24_000 {
                peak = peak.max(y.abs());
            }
        }
        assert!((peak - 1.0).abs() < 0.01);
    }
}

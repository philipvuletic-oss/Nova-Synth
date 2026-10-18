//! Time-based (frame-rate independent) animation helpers. Never advance animations per frame;
//! always pass the frame delta time so 30, 60 and 144 Hz behave the same.

/// Longest delta time honoured per frame (protects against pauses and first-frame spikes).
pub const MAX_DT: f32 = 0.1;

/// Exponentially approaches `target` with time constant `tau` seconds. Exactly composable:
/// two steps of dt/2 equal one step of dt, so frame rate does not change the result.
pub fn smooth_toward(current: f32, target: f32, dt: f32, tau: f32) -> f32 {
    let dt = dt.clamp(0.0, MAX_DT);
    if tau <= 0.0 {
        return target;
    }
    current + (target - current) * (1.0 - (-dt / tau).exp())
}

/// Level display with an instant attack and a time-based release (meters).
pub fn meter_fall(current: f32, target: f32, dt: f32, release_tau: f32) -> f32 {
    if target >= current {
        target
    } else {
        smooth_toward(current, target, dt, release_tau)
    }
}

/// True while a displayed value is still visibly moving, so the GUI knows to keep repainting.
pub fn is_settling(current: f32, target: f32) -> bool {
    (current - target).abs() > 0.002
}

#[cfg(test)]
mod tests {
    use super::*;

    fn simulate(fps: f32, seconds: f32, tau: f32) -> f32 {
        let dt = 1.0 / fps;
        let mut value = 1.0;
        for _ in 0..(seconds * fps).round() as usize {
            value = meter_fall(value, 0.0, dt, tau);
        }
        value
    }

    #[test]
    fn timing_is_identical_at_30_60_and_144_hz() {
        let reference = simulate(60.0, 0.5, 0.25);
        for fps in [30.0, 144.0] {
            assert!(
                (simulate(fps, 0.5, 0.25) - reference).abs() < 1.0e-4,
                "{fps} Hz"
            );
        }
        assert!((reference - (-2.0_f32).exp()).abs() < 1.0e-3);
    }

    #[test]
    fn attack_is_instant_and_dt_is_clamped() {
        assert_eq!(meter_fall(0.2, 0.9, 0.016, 0.25), 0.9);
        let paused = smooth_toward(1.0, 0.0, 10.0, 0.25);
        let capped = smooth_toward(1.0, 0.0, MAX_DT, 0.25);
        assert_eq!(paused, capped);
    }
}

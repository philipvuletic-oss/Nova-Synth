//! Objective sound metrics, WAV writer and spectrogram renderer.

use crate::host::{Note, SAMPLE_RATE};
use realfft::RealFftPlanner;
use serde::Serialize;
use std::path::Path;

#[derive(Serialize, Debug, Clone)]
pub struct Report {
    pub patch: String,
    pub phrase: String,
    pub duration_s: f64,
    pub peak_dbfs: f64,
    pub rms_dbfs: f64,
    pub loudness_lufs_ish: f64,
    pub crest_factor_db: f64,
    pub dc_offset: f64,
    pub stereo_correlation: f64,
    /// Worst-case energy outside harmonic bins relative to total (dB); `None` for chords/pads.
    pub aliasing_db: Option<f64>,
    /// Largest sample-to-sample jump divided by local RMS (20 ms window).
    pub click_ratio: f64,
    /// Wall time inside `process` divided by audio time (lower is better).
    pub cpu_real_time_factor: f64,
    pub warnings: Vec<String>,
}

fn db(x: f64) -> f64 {
    20.0 * x.max(1.0e-12).log10()
}

struct Biquad {
    b: [f64; 3],
    a: [f64; 2],
    z: [f64; 2],
}

impl Biquad {
    fn new(b: [f64; 3], a: [f64; 2]) -> Self {
        Self { b, a, z: [0.0; 2] }
    }
    fn tick(&mut self, x: f64) -> f64 {
        let y = self.b[0] * x + self.z[0];
        self.z[0] = self.b[1] * x - self.a[0] * y + self.z[1];
        self.z[1] = self.b[2] * x - self.a[1] * y;
        y
    }
}

/// BS.1770 K-weighted, 400 ms blocks (100 ms hop), absolute gate at -70 LUFS. 48 kHz only.
fn lufs_ish(left: &[f32], right: &[f32]) -> f64 {
    let mut weighted = [Vec::new(), Vec::new()];
    for (channel, data) in [left, right].into_iter().enumerate() {
        let mut shelf = Biquad::new(
            [1.53512485958697, -2.69169618940638, 1.19839281085285],
            [-1.69065929318241, 0.73248077421585],
        );
        let mut high_pass = Biquad::new([1.0, -2.0, 1.0], [-1.99004745483398, 0.99007225036621]);
        weighted[channel] = data
            .iter()
            .map(|x| high_pass.tick(shelf.tick(*x as f64)))
            .collect();
    }
    let block = (0.4 * SAMPLE_RATE as f64) as usize;
    let hop = (0.1 * SAMPLE_RATE as f64) as usize;
    let mut energies = Vec::new();
    let mut start = 0;
    while start + block <= left.len() {
        let mut sum = 0.0;
        for channel in &weighted {
            sum += channel[start..start + block]
                .iter()
                .map(|x| x * x)
                .sum::<f64>()
                / block as f64;
        }
        if -0.691 + 10.0 * sum.max(1.0e-20).log10() > -70.0 {
            energies.push(sum);
        }
        start += hop;
    }
    if energies.is_empty() {
        return -70.0;
    }
    -0.691 + 10.0 * (energies.iter().sum::<f64>() / energies.len() as f64).log10()
}

/// Fraction of spectral energy that is not within +-2 bins of a harmonic of `f0`, in dB.
fn aliasing_db(segment: &[f32], f0: f64) -> Option<f64> {
    let mut n = 1usize << 13;
    while n > segment.len() {
        n >>= 1;
    }
    // Harmonics must be at least 8 bins apart or the +-2 bin exclusion covers the whole spectrum.
    if n < 4096 || f0 < 8.0 * SAMPLE_RATE as f64 / n as f64 {
        return None;
    }
    let mut planner = RealFftPlanner::<f32>::new();
    let fft = planner.plan_fft_forward(n);
    let mut input: Vec<f32> = segment[..n]
        .iter()
        .enumerate()
        .map(|(i, x)| *x * (0.5 - 0.5 * (std::f32::consts::TAU * i as f32 / (n - 1) as f32).cos()))
        .collect();
    let mut spectrum = fft.make_output_vec();
    fft.process(&mut input, &mut spectrum).ok()?;
    let bin_hz = SAMPLE_RATE as f64 / n as f64;
    let mut harmonic = vec![false; spectrum.len()];
    let mut k = 1.0;
    while k * f0 < SAMPLE_RATE as f64 / 2.0 {
        let centre = (k * f0 / bin_hz).round() as i64;
        for bin in centre - 2..=centre + 2 {
            if bin >= 1 && (bin as usize) < harmonic.len() {
                harmonic[bin as usize] = true;
            }
        }
        k += 1.0;
    }
    let (mut total, mut other) = (0.0f64, 0.0f64);
    for (bin, value) in spectrum.iter().enumerate().skip(1) {
        let energy = value.norm_sqr() as f64;
        total += energy;
        if !harmonic[bin] {
            other += energy;
        }
    }
    (total > 1.0e-12).then(|| 10.0 * (other / total).max(1.0e-12).log10())
}

pub fn analyse(
    patch: &str,
    phrase: &str,
    left: &[f32],
    right: &[f32],
    notes: &[Note],
    monophonic: bool,
    real_time_factor: f64,
) -> Report {
    let active_start = left
        .iter()
        .zip(right)
        .position(|(l, r)| l.abs().max(r.abs()) > 1.0e-4)
        .unwrap_or(0);
    let active_end = left
        .iter()
        .zip(right)
        .rposition(|(l, r)| l.abs().max(r.abs()) > 1.0e-4)
        .map_or(left.len(), |i| i + 1);
    let (l, r) = (
        &left[active_start..active_end.max(active_start)],
        &right[active_start..active_end.max(active_start)],
    );
    let count = l.len().max(1) as f64;
    let peak = l
        .iter()
        .chain(r.iter())
        .fold(0.0f64, |peak, x| peak.max(x.abs() as f64));
    let ms: f64 = l
        .iter()
        .chain(r.iter())
        .map(|x| (*x as f64) * (*x as f64))
        .sum::<f64>()
        / (2.0 * count);
    let rms = ms.sqrt();
    let mean_l = l.iter().map(|x| *x as f64).sum::<f64>() / count;
    let mean_r = r.iter().map(|x| *x as f64).sum::<f64>() / count;
    let dc = mean_l.abs().max(mean_r.abs());
    let (mut sll, mut srr, mut slr) = (0.0, 0.0, 0.0);
    for (a, b) in l.iter().zip(r) {
        let (a, b) = (*a as f64 - mean_l, *b as f64 - mean_r);
        sll += a * a;
        srr += b * b;
        slr += a * b;
    }
    let correlation = if sll > 1.0e-12 && srr > 1.0e-12 {
        slr / (sll * srr).sqrt()
    } else {
        1.0
    };

    // Clicks: biggest jump relative to the local RMS (20 ms centred window, mono sum).
    let mono: Vec<f64> = left
        .iter()
        .zip(right)
        .map(|(a, b)| (*a as f64 + *b as f64) * 0.5)
        .collect();
    let window = (0.02 * SAMPLE_RATE as f64) as usize;
    let mut prefix = vec![0.0; mono.len() + 1];
    for (i, x) in mono.iter().enumerate() {
        prefix[i + 1] = prefix[i] + x * x;
    }
    let mut click: f64 = 0.0;
    for i in 1..mono.len() {
        let lo = i.saturating_sub(window / 2);
        let hi = (i + window / 2).min(mono.len());
        let local = ((prefix[hi] - prefix[lo]) / (hi - lo) as f64).sqrt();
        if local > 1.0e-3 {
            click = click.max((mono[i] - mono[i - 1]).abs() / local);
        }
    }

    let aliasing = monophonic
        .then(|| {
            notes
                .iter()
                .filter_map(|note| {
                    let f0 = 440.0 * 2f64.powf((note.note as f64 - 69.0) / 12.0);
                    // Analyse the end of the held note (just before note-off), where the amplitude
                    // envelope has settled as much as it will.
                    let end = (((note.start + note.length) * SAMPLE_RATE as f64) as usize)
                        .min(mono.len());
                    let start = ((note.start + 0.05) * SAMPLE_RATE as f64) as usize;
                    if end <= start {
                        return None;
                    }
                    let segment: Vec<f32> = mono[start..end].iter().map(|x| *x as f32).collect();
                    let segment = &segment[segment.len().saturating_sub(8192)..];
                    aliasing_db(segment, f0)
                })
                .fold(None, |worst: Option<f64>, value| {
                    Some(worst.map_or(value, |w| w.max(value)))
                })
        })
        .flatten();

    let mut report = Report {
        patch: patch.to_string(),
        phrase: phrase.to_string(),
        duration_s: left.len() as f64 / SAMPLE_RATE as f64,
        peak_dbfs: db(peak),
        rms_dbfs: db(rms),
        loudness_lufs_ish: lufs_ish(left, right),
        crest_factor_db: db(peak) - db(rms),
        dc_offset: dc,
        stereo_correlation: correlation,
        aliasing_db: aliasing,
        click_ratio: click,
        cpu_real_time_factor: real_time_factor,
        warnings: Vec::new(),
    };
    if report.peak_dbfs > -0.3 {
        report
            .warnings
            .push(format!("peak {:.2} dBFS above -0.3", report.peak_dbfs));
    }
    if report.dc_offset > 0.001 {
        report
            .warnings
            .push(format!("DC offset {:.4} > 0.001", report.dc_offset));
    }
    if report.click_ratio > 4.0 {
        report
            .warnings
            .push(format!("click ratio {:.1} > 4", report.click_ratio));
    }
    if report.stereo_correlation < 0.0 {
        report
            .warnings
            .push(String::from("negative stereo correlation"));
    }
    if report.cpu_real_time_factor > 0.5 {
        report.warnings.push(format!(
            "CPU real-time factor {:.2} > 0.5",
            report.cpu_real_time_factor
        ));
    }
    report
}

pub fn write_wav(path: &Path, left: &[f32], right: &[f32]) -> Result<(), String> {
    let spec = hound::WavSpec {
        channels: 2,
        sample_rate: SAMPLE_RATE as u32,
        bits_per_sample: 24,
        sample_format: hound::SampleFormat::Int,
    };
    let mut writer = hound::WavWriter::create(path, spec).map_err(|e| e.to_string())?;
    let scale = 8_388_607.0f32;
    for (l, r) in left.iter().zip(right) {
        for sample in [l, r] {
            let value = (sample.clamp(-1.0, 1.0) * scale).round() as i32;
            writer.write_sample(value).map_err(|e| e.to_string())?;
        }
    }
    writer.finalize().map_err(|e| e.to_string())
}

fn colour(t: f32) -> [u8; 3] {
    const STOPS: [(f32, [f32; 3]); 6] = [
        (0.0, [0.0, 0.0, 0.0]),
        (0.2, [40.0, 10.0, 90.0]),
        (0.45, [150.0, 25.0, 130.0]),
        (0.7, [240.0, 90.0, 40.0]),
        (0.9, [255.0, 210.0, 70.0]),
        (1.0, [255.0, 255.0, 230.0]),
    ];
    let t = t.clamp(0.0, 1.0);
    for pair in STOPS.windows(2) {
        let (t0, c0) = pair[0];
        let (t1, c1) = pair[1];
        if t <= t1 {
            let f = (t - t0) / (t1 - t0);
            return [
                (c0[0] + (c1[0] - c0[0]) * f) as u8,
                (c0[1] + (c1[1] - c0[1]) * f) as u8,
                (c0[2] + (c1[2] - c0[2]) * f) as u8,
            ];
        }
    }
    [255, 255, 230]
}

/// STFT spectrogram of the mono mix: log frequency axis (30 Hz..24 kHz), dB colour map
/// (-100..0 dBFS), 1 px column per hop, 480 px tall. Grid lines mark 100 Hz, 1 kHz and 10 kHz.
pub fn write_spectrogram(path: &Path, left: &[f32], right: &[f32]) -> Result<(), String> {
    const N: usize = 4096;
    const HEIGHT: usize = 480;
    let hop = (left.len() / 1400).max(512);
    let columns = left.len().saturating_sub(N) / hop + 1;
    let mut planner = RealFftPlanner::<f32>::new();
    let fft = planner.plan_fft_forward(N);
    let window: Vec<f32> = (0..N)
        .map(|i| 0.5 - 0.5 * (std::f32::consts::TAU * i as f32 / (N - 1) as f32).cos())
        .collect();
    let window_sum: f32 = window.iter().sum();
    let (f_min, f_max) = (30.0f64, 24_000.0f64);
    let mut pixels = vec![0u8; columns * HEIGHT * 3];
    let mut input = vec![0.0f32; N];
    let mut spectrum = fft.make_output_vec();
    for column in 0..columns {
        let start = column * hop;
        for i in 0..N {
            let idx = start + i;
            let mono = if idx < left.len() {
                (left[idx] + right[idx]) * 0.5
            } else {
                0.0
            };
            input[i] = mono * window[i];
        }
        fft.process(&mut input, &mut spectrum)
            .map_err(|e| e.to_string())?;
        for row in 0..HEIGHT {
            let frac = 1.0 - row as f64 / (HEIGHT - 1) as f64;
            let freq = f_min * (f_max / f_min).powf(frac);
            let bin = (freq / (SAMPLE_RATE as f64 / N as f64)).clamp(0.0, (N / 2) as f64);
            let lo = bin.floor() as usize;
            let hi = (lo + 1).min(N / 2);
            let mix = (bin - lo as f64) as f32;
            let magnitude = spectrum[lo].norm() * (1.0 - mix) + spectrum[hi].norm() * mix;
            let level = 20.0 * (2.0 * magnitude / window_sum).max(1.0e-9).log10();
            let rgb = colour((level + 100.0) / 100.0);
            let offset = (row * columns + column) * 3;
            pixels[offset..offset + 3].copy_from_slice(&rgb);
        }
    }
    for hz in [100.0f64, 1_000.0, 10_000.0] {
        let row = ((1.0 - (hz / f_min).ln() / (f_max / f_min).ln()) * (HEIGHT - 1) as f64) as usize;
        for column in (0..columns).step_by(2) {
            let offset = (row.min(HEIGHT - 1) * columns + column) * 3;
            pixels[offset..offset + 3].copy_from_slice(&[70, 80, 100]);
        }
    }
    let file = std::fs::File::create(path).map_err(|e| e.to_string())?;
    let mut encoder =
        png::Encoder::new(std::io::BufWriter::new(file), columns as u32, HEIGHT as u32);
    encoder.set_color(png::ColorType::Rgb);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header().map_err(|e| e.to_string())?;
    writer.write_image_data(&pixels).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sine(frequency: f64, seconds: f64, amplitude: f64) -> Vec<f32> {
        (0..(seconds * SAMPLE_RATE as f64) as usize)
            .map(|i| {
                (amplitude
                    * (std::f64::consts::TAU * frequency * i as f64 / SAMPLE_RATE as f64).sin())
                    as f32
            })
            .collect()
    }

    #[test]
    fn clean_sine_has_expected_metrics() {
        let signal = sine(440.0, 1.0, 0.5);
        let note = Note {
            start: 0.0,
            length: 1.0,
            note: 69,
            velocity: 1.0,
        };
        let report = analyse("t", "t", &signal, &signal, &[note], true, 0.01);
        assert!((report.peak_dbfs - -6.02).abs() < 0.05);
        assert!((report.crest_factor_db - 3.01).abs() < 0.1);
        assert!(report.dc_offset < 1.0e-3);
        assert!((report.stereo_correlation - 1.0).abs() < 1.0e-6);
        assert!(report.aliasing_db.is_some_and(|a| a < -40.0));
        assert!(report.click_ratio < 0.5);
        assert!(report.warnings.is_empty());
    }

    #[test]
    fn detects_dc_clicks_and_phase_inversion() {
        let mut left = sine(220.0, 1.0, 0.3);
        let right: Vec<f32> = left.iter().map(|x| -x).collect();
        for x in &mut left {
            *x += 0.05;
        }
        left[24_000] += 0.9;
        let report = analyse("t", "t", &left, &right, &[], false, 0.01);
        assert!(report.dc_offset > 0.001);
        assert!(report.click_ratio > 4.0);
        assert!(report.stereo_correlation < 0.0);
        assert!(report.warnings.len() >= 3);
    }

    #[test]
    fn wav_writer_round_trips_24_bit() {
        let path =
            std::env::temp_dir().join(format!("nova_demo_render_test_{}.wav", std::process::id()));
        let signal = sine(1000.0, 0.1, 0.25);
        write_wav(&path, &signal, &signal).unwrap();
        let reader = hound::WavReader::open(&path).unwrap();
        assert_eq!(reader.spec().bits_per_sample, 24);
        assert_eq!(reader.spec().sample_rate, 48_000);
        assert_eq!(reader.spec().channels, 2);
        let _ = std::fs::remove_file(path);
    }
}

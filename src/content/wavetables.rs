use realfft::num_complex::Complex32;
use realfft::RealFftPlanner;
use std::io::{Read, Seek, Write};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

pub const FRAME_SIZE: usize = 2048;
pub const MAX_FRAMES: usize = 256;
const MIDI_LEVELS: [u8; 12] = [12, 24, 36, 48, 60, 72, 84, 96, 108, 120, 132, 135];
const MAX_WAV_BYTES: u64 = 64 * 1024 * 1024;
const MAX_SMOOTH_FRAMES: usize = 256;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FrameInterpolation {
    Frames,
    Smooth,
    Step,
}

#[derive(Clone, Debug)]
pub struct Wavetable {
    pub name: String,
    pub source: String,
    pub sample_rate: f32,
    pub frames: Vec<Vec<f32>>,
    pub spectra: Vec<Vec<Complex32>>,
    pub mipmaps: Vec<Vec<Vec<f32>>>,
    pub smooth_mipmaps: Vec<Vec<Vec<f32>>>,
    pub max_frequencies: Vec<f32>,
}

impl Wavetable {
    pub fn fallback_sine(sample_rate: f32) -> Self {
        let frame: Vec<f32> = (0..FRAME_SIZE)
            .map(|index| (std::f32::consts::TAU * index as f32 / FRAME_SIZE as f32).sin())
            .collect();
        let mut guarded = Vec::with_capacity(FRAME_SIZE + 3);
        guarded.push(frame[FRAME_SIZE - 1]);
        guarded.extend_from_slice(&frame);
        guarded.extend_from_slice(&frame[..2]);
        Self {
            name: String::from("Basic Shapes"),
            source: String::from("Nova built-in fallback"),
            sample_rate,
            frames: vec![frame],
            spectra: Vec::new(),
            mipmaps: vec![vec![guarded.clone()]; MIDI_LEVELS.len()],
            smooth_mipmaps: vec![vec![guarded]; MIDI_LEVELS.len()],
            max_frequencies: MIDI_LEVELS
                .iter()
                .map(|note| 440.0 * 2.0f32.powf((*note as f32 - 69.0) / 12.0))
                .collect(),
        }
    }

    pub fn from_frames(
        name: impl Into<String>,
        source: impl Into<String>,
        frames: Vec<Vec<f32>>,
        sample_rate: f32,
    ) -> Result<Self, &'static str> {
        if frames.is_empty() || frames.len() > MAX_FRAMES {
            return Err("A wavetable must contain between 1 and 256 frames");
        }
        if frames.iter().any(|frame| {
            frame.len() != FRAME_SIZE || frame.iter().any(|sample| !sample.is_finite())
        }) {
            return Err("Every wavetable frame must contain 2048 finite samples");
        }

        let sample_rate = if sample_rate.is_finite() {
            sample_rate.clamp(8_000.0, 384_000.0)
        } else {
            48_000.0
        };
        let mut planner = RealFftPlanner::<f32>::new();
        let forward = planner.plan_fft_forward(FRAME_SIZE);
        let inverse = planner.plan_fft_inverse(FRAME_SIZE);
        let mut spectra = Vec::with_capacity(frames.len());
        let mut mipmaps = vec![Vec::with_capacity(frames.len()); MIDI_LEVELS.len()];
        let mut smooth_mipmaps = vec![Vec::new(); MIDI_LEVELS.len()];
        let mut input = forward.make_input_vec();
        let mut spectrum = forward.make_output_vec();
        let mut inverse_spectrum = inverse.make_input_vec();
        let mut output = inverse.make_output_vec();
        let max_frequencies: Vec<f32> = MIDI_LEVELS
            .iter()
            .map(|note| 440.0 * 2.0f32.powf((*note as f32 - 69.0) / 12.0))
            .collect();

        for frame in &frames {
            input.copy_from_slice(frame);
            forward
                .process(&mut input, &mut spectrum)
                .map_err(|_| "Unable to analyze wavetable frame")?;
            spectra.push(spectrum.clone());
            for (level, max_frequency) in max_frequencies.iter().enumerate() {
                inverse_spectrum.copy_from_slice(&spectrum);
                let max_harmonic = ((sample_rate * 0.5 / max_frequency * 0.95).floor() as usize)
                    .clamp(1, FRAME_SIZE / 2);
                for bin in inverse_spectrum.iter_mut().skip(max_harmonic + 1) {
                    *bin = Complex32::new(0.0, 0.0);
                }
                inverse
                    .process(&mut inverse_spectrum, &mut output)
                    .map_err(|_| "Unable to synthesize wavetable mipmap")?;
                let scale = 1.0 / FRAME_SIZE as f32;
                mipmaps[level].push(guarded_table(&output, scale));
            }
        }
        for level in 0..MIDI_LEVELS.len() {
            smooth_mipmaps[level].push(mipmaps[level][0].clone());
        }
        let smooth_subdivisions = if frames.len() > 1 {
            ((MAX_SMOOTH_FRAMES - 1) / (frames.len() - 1)).clamp(1, 4)
        } else {
            1
        };
        let mut morph_spectrum = forward.make_output_vec();
        for interval in 0..spectra.len().saturating_sub(1) {
            for step in 1..smooth_subdivisions {
                let amount = step as f32 / smooth_subdivisions as f32;
                for (index, bin) in morph_spectrum.iter_mut().enumerate() {
                    let first = spectra[interval][index];
                    let second = spectra[interval + 1][index];
                    let magnitude = first.norm() + (second.norm() - first.norm()) * amount;
                    let first_phase = first.arg();
                    let phase_delta = (second.arg() - first_phase + std::f32::consts::PI)
                        .rem_euclid(std::f32::consts::TAU)
                        - std::f32::consts::PI;
                    *bin = Complex32::from_polar(magnitude, first_phase + phase_delta * amount);
                }
                morph_spectrum[0].im = 0.0;
                let nyquist = morph_spectrum.len() - 1;
                morph_spectrum[nyquist].im = 0.0;
                for (level, max_frequency) in max_frequencies.iter().enumerate() {
                    inverse_spectrum.copy_from_slice(&morph_spectrum);
                    let max_harmonic = ((sample_rate * 0.5 / max_frequency * 0.95).floor()
                        as usize)
                        .clamp(1, FRAME_SIZE / 2);
                    for bin in inverse_spectrum.iter_mut().skip(max_harmonic + 1) {
                        *bin = Complex32::new(0.0, 0.0);
                    }
                    inverse
                        .process(&mut inverse_spectrum, &mut output)
                        .map_err(|_| "Unable to synthesize smooth wavetable frame")?;
                    smooth_mipmaps[level].push(guarded_table(&output, 1.0 / FRAME_SIZE as f32));
                }
            }
            for level in 0..MIDI_LEVELS.len() {
                smooth_mipmaps[level].push(mipmaps[level][interval + 1].clone());
            }
        }
        Ok(Self {
            name: name.into(),
            source: source.into(),
            sample_rate,
            frames,
            spectra,
            mipmaps,
            smooth_mipmaps,
            max_frequencies,
        })
    }

    pub fn rebuild_for_sample_rate(&self, sample_rate: f32) -> Result<Self, &'static str> {
        Self::from_frames(
            self.name.clone(),
            self.source.clone(),
            self.frames.clone(),
            sample_rate,
        )
    }

    pub fn sample(
        &self,
        phase: f64,
        frequency: f32,
        position: f32,
        interpolation: FrameInterpolation,
    ) -> f32 {
        let (lower_level, upper_level, level_mix) = self.levels_for_frequency(frequency);
        let lower = self.sample_level(lower_level, phase, position, interpolation);
        if lower_level == upper_level {
            lower
        } else {
            let upper = self.sample_level(upper_level, phase, position, interpolation);
            lower + (upper - lower) * level_mix
        }
    }

    fn levels_for_frequency(&self, frequency: f32) -> (usize, usize, f32) {
        if self.max_frequencies.len() < 2 || frequency <= self.max_frequencies[0] {
            return (0, 0, 0.0);
        }
        let hz = frequency.abs().max(1.0);
        for level in 1..self.max_frequencies.len() {
            if hz <= self.max_frequencies[level] {
                let start = self.max_frequencies[level - 1].ln();
                let end = self.max_frequencies[level].ln();
                let amount = ((hz.ln() - start) / (end - start)).clamp(0.0, 1.0);
                return (
                    level,
                    (level + 1).min(self.max_frequencies.len() - 1),
                    amount,
                );
            }
        }
        let last = self.max_frequencies.len() - 1;
        (last, last, 0.0)
    }

    fn sample_level(
        &self,
        level: usize,
        phase: f64,
        position: f32,
        interpolation: FrameInterpolation,
    ) -> f32 {
        let tables = match self.mipmaps.get(level) {
            Some(tables) if !tables.is_empty() => tables,
            _ => return 0.0,
        };
        let normalized = if phase.is_finite() {
            phase.rem_euclid(1.0) as f32
        } else {
            0.0
        };
        let position = match interpolation {
            FrameInterpolation::Smooth => {
                let smooth = match self.smooth_mipmaps.get(level) {
                    Some(smooth) if !smooth.is_empty() => smooth,
                    _ => tables,
                };
                let position = position.clamp(0.0, 1.0) * (smooth.len() - 1) as f32;
                return cubic_interpolate_tables(smooth, position, normalized);
            }
            _ => position.clamp(0.0, 1.0) * (tables.len() - 1) as f32,
        };
        match interpolation {
            FrameInterpolation::Step => {
                let frame = position.round() as usize;
                sample_hermite(&tables[frame.min(tables.len() - 1)], normalized)
            }
            FrameInterpolation::Frames => {
                let frame = position.floor() as usize;
                let next = (frame + 1).min(tables.len() - 1);
                let amount = position - frame as f32;
                let first = sample_hermite(&tables[frame], normalized);
                let second = sample_hermite(&tables[next], normalized);
                first + (second - first) * amount
            }
            FrameInterpolation::Smooth => 0.0,
        }
    }
}

fn cubic_interpolate_tables(tables: &[Vec<f32>], position: f32, phase: f32) -> f32 {
    let frame = position.floor() as isize;
    let amount = position - frame as f32;
    let a = sample_hermite(&tables[clamp_frame(frame - 1, tables.len())], phase);
    let b = sample_hermite(&tables[clamp_frame(frame, tables.len())], phase);
    let c = sample_hermite(&tables[clamp_frame(frame + 1, tables.len())], phase);
    let d = sample_hermite(&tables[clamp_frame(frame + 2, tables.len())], phase);
    let amount2 = amount * amount;
    let amount3 = amount2 * amount;
    0.5 * ((2.0 * b)
        + (-a + c) * amount
        + (2.0 * a - 5.0 * b + 4.0 * c - d) * amount2
        + (-a + 3.0 * b - 3.0 * c + d) * amount3)
}

fn guarded_table(samples: &[f32], scale: f32) -> Vec<f32> {
    let mut table = Vec::with_capacity(FRAME_SIZE + 3);
    table.push(samples[FRAME_SIZE - 1] * scale);
    table.extend(samples.iter().map(|sample| sample * scale));
    table.push(samples[0] * scale);
    table.push(samples[1] * scale);
    table
}

pub struct WavetableStore {
    current: arc_swap::ArcSwap<Wavetable>,
    generation: AtomicU64,
    audio_generation: AtomicU64,
    retired: Mutex<Vec<(u64, Arc<Wavetable>)>>,
}

impl WavetableStore {
    pub fn new(initial: Wavetable) -> Self {
        Self {
            current: arc_swap::ArcSwap::from_pointee(initial),
            generation: AtomicU64::new(0),
            audio_generation: AtomicU64::new(0),
            retired: Mutex::new(Vec::new()),
        }
    }

    pub fn load(&self) -> Arc<Wavetable> {
        self.current.load_full()
    }

    pub fn load_for_audio(&self) -> (Arc<Wavetable>, u64) {
        let table = self.current.load_full();
        let generation = self.generation.load(Ordering::Acquire);
        (table, generation)
    }

    pub fn audio_finished(&self, generation: u64) {
        self.audio_generation.store(generation, Ordering::Release);
    }

    pub fn replace(&self, table: Wavetable) {
        let mut retired = self
            .retired
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let generation = self.generation.load(Ordering::Relaxed);
        let previous = self.current.swap(Arc::new(table));
        self.generation
            .store(generation.wrapping_add(1), Ordering::Release);
        retired.push((generation, previous));
        let audio_generation = self.audio_generation.load(Ordering::Acquire);
        retired.retain(|(retired_generation, _)| *retired_generation < audio_generation);
    }
}

fn clamp_frame(frame: isize, count: usize) -> usize {
    frame.clamp(0, count.saturating_sub(1) as isize) as usize
}

pub fn sample_hermite(table: &[f32], phase: f32) -> f32 {
    if table.len() < FRAME_SIZE + 3 {
        return 0.0;
    }
    let position = phase.rem_euclid(1.0) * FRAME_SIZE as f32;
    let index = position.floor() as usize;
    let fraction = position - index as f32;
    let index = index.min(FRAME_SIZE - 1) + 1;
    let y0 = table[index - 1];
    let y1 = table[index];
    let y2 = table[index + 1];
    let y3 = table[index + 2];
    let c0 = y1;
    let c1 = 0.5 * (y2 - y0);
    let c2 = y0 - 2.5 * y1 + 2.0 * y2 - 0.5 * y3;
    let c3 = 0.5 * (y3 - y0) + 1.5 * (y1 - y2);
    ((c3 * fraction + c2) * fraction + c1) * fraction + c0
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FactoryWavetableInfo {
    pub name: &'static str,
    pub category: &'static str,
}

pub const FACTORY_TABLES: [FactoryWavetableInfo; 24] = [
    FactoryWavetableInfo {
        name: "Basic Shapes",
        category: "Basic Shapes",
    },
    FactoryWavetableInfo {
        name: "Analog Saw Stack",
        category: "Analog",
    },
    FactoryWavetableInfo {
        name: "PWM Square",
        category: "Analog",
    },
    FactoryWavetableInfo {
        name: "Harmonic Sweep",
        category: "Harmonic",
    },
    FactoryWavetableInfo {
        name: "Odd Harmonics",
        category: "Harmonic",
    },
    FactoryWavetableInfo {
        name: "Additive Organ",
        category: "Harmonic",
    },
    FactoryWavetableInfo {
        name: "Formant Vowels",
        category: "Vocal",
    },
    FactoryWavetableInfo {
        name: "Formant Bass",
        category: "Vocal",
    },
    FactoryWavetableInfo {
        name: "FM Bell",
        category: "Digital",
    },
    FactoryWavetableInfo {
        name: "FM Growl",
        category: "Digital",
    },
    FactoryWavetableInfo {
        name: "Sync Sweep",
        category: "Digital",
    },
    FactoryWavetableInfo {
        name: "Fold Sweep",
        category: "Digital",
    },
    FactoryWavetableInfo {
        name: "PD Resonance",
        category: "Digital",
    },
    FactoryWavetableInfo {
        name: "Digital Grit",
        category: "Digital",
    },
    FactoryWavetableInfo {
        name: "Spectral Noise Pad",
        category: "Texture",
    },
    FactoryWavetableInfo {
        name: "Supersaw Morph",
        category: "Analog",
    },
    FactoryWavetableInfo {
        name: "Hollow Pipes",
        category: "Harmonic",
    },
    FactoryWavetableInfo {
        name: "Glass Partials",
        category: "Digital",
    },
    FactoryWavetableInfo {
        name: "Comb Sweep",
        category: "Digital",
    },
    FactoryWavetableInfo {
        name: "Chebyshev Sweep",
        category: "Harmonic",
    },
    FactoryWavetableInfo {
        name: "Square-to-Sine Sub",
        category: "Basic Shapes",
    },
    FactoryWavetableInfo {
        name: "Vocal Choir",
        category: "Vocal",
    },
    FactoryWavetableInfo {
        name: "Metallic Ring",
        category: "Digital",
    },
    FactoryWavetableInfo {
        name: "Pluck Spectrum Decay",
        category: "Harmonic",
    },
];

pub fn generate_factory(index: usize, sample_rate: f32) -> Result<Wavetable, &'static str> {
    let info = FACTORY_TABLES
        .get(index)
        .ok_or("Unknown factory wavetable index")?;
    let mut frames = Vec::with_capacity(64);
    for frame_index in 0..64 {
        let morph = frame_index as f32 / 63.0;
        let mut frame = Vec::with_capacity(FRAME_SIZE);
        for sample_index in 0..FRAME_SIZE {
            let phase = sample_index as f32 / FRAME_SIZE as f32;
            let sample = factory_sample(index, morph, phase);
            frame.push(sample);
        }
        remove_dc_and_normalize(&mut frame, true);
        frames.push(frame);
    }
    Wavetable::from_frames(info.name, "Nova factory", frames, sample_rate)
}

fn factory_sample(index: usize, morph: f32, phase: f32) -> f32 {
    let angle = std::f32::consts::TAU * phase;
    let sine = angle.sin();
    let saw = 2.0 * phase - 1.0;
    let triangle = 1.0 - 4.0 * (phase - 0.5).abs();
    match index {
        0 | 20 => {
            let position = if index == 20 {
                1.0 - morph
            } else {
                morph * 3.0
            };
            if position < 1.0 {
                sine + (triangle - sine) * position
            } else if position < 2.0 {
                let amount = position - 1.0;
                triangle + (saw - triangle) * amount
            } else {
                let square = if phase < 0.5 { 1.0 } else { -1.0 };
                saw + (square - saw) * (position - 2.0)
            }
        }
        1 => {
            let detune = 0.006 + morph * 0.008;
            (angle).sin() * 0.45
                + (angle * (1.0 + detune)).sin() * 0.3
                + (angle * (1.0 - detune * 0.7)).sin() * 0.25
        }
        2 => {
            let duty = 0.05 + 0.9 * morph;
            if phase < duty {
                1.0
            } else {
                -1.0
            }
        }
        3 => harmonic_series(phase, 1 + (morph * 127.0) as usize, false, morph),
        4 => harmonic_series(phase, 2 + (morph * 63.0) as usize * 2, true, morph),
        5 => {
            let mut value = 0.0;
            for harmonic in 1..=16 {
                let weight = if harmonic % 4 == 0 {
                    0.3
                } else {
                    1.0 / harmonic as f32
                };
                value += weight * (angle * harmonic as f32).sin();
            }
            value
        }
        6 | 7 | 21 => {
            let formants = [0.04, 0.09, 0.17, 0.27, 0.39];
            let position = morph * 4.0;
            let first = position.floor() as usize;
            let second = (first + 1).min(4);
            let blend = position - first as f32;
            let a = (angle * (1.0 + first as f32 * 2.0)).sin()
                * (1.0 + 9.0 * (phase - formants[first]).abs()).recip();
            let b = (angle * (1.0 + second as f32 * 2.0)).sin()
                * (1.0 + 9.0 * (phase - formants[second]).abs()).recip();
            a + (b - a) * blend
        }
        8 => sine * (1.0 + (angle * (1.0 + morph * 5.0)).sin() * (0.2 + morph)),
        9 => (angle + (angle * 2.0).sin() * (0.3 + morph * 5.0)).sin(),
        10 => (std::f32::consts::TAU * (phase * (1.0 + morph * 7.0)).fract()).sin(),
        11 => (saw * (1.0 + morph * 3.0)).sin(),
        12 => ((phase + morph * (phase * 8.0).sin()).fract() * 2.0 - 1.0).sin(),
        13 => {
            let bits = (4.0 + morph * 12.0).round();
            (((sine + 1.0) * 0.5 * bits).floor() / bits) * 2.0 - 1.0
        }
        14 => {
            let seed = ((phase * 4096.0) as u32).wrapping_mul(747_796_405);
            let noise = ((seed ^ (seed >> 16)) & 0xffff) as f32 / 32768.0 - 1.0;
            sine * (1.0 - morph) + noise * morph * 0.4
        }
        15 => {
            let mut value = 0.0;
            for voice in -3..=3 {
                let detune = voice as f32 * (0.002 + morph * 0.006);
                value += (angle * (1.0 + detune)).sin() / 7.0;
            }
            value
        }
        16 => harmonic_series(phase, 12, true, 0.25),
        17 => {
            let ratio = 1.0 + morph * 0.37;
            (angle * ratio).sin() * 0.6 + (angle * 2.71).sin() * 0.3 + (angle * 6.13).sin() * 0.1
        }
        18 => {
            let harmonics = 1 + (morph * 8.0) as usize;
            harmonic_series(phase, harmonics, false, 0.0)
        }
        19 => harmonic_series(phase, 32, false, morph),
        22 => (angle * (1.0 + morph * 12.0)).sin() * (angle * 3.01).sin(),
        23 => {
            let decay = (1.0 - morph).max(0.03);
            harmonic_series(phase, 1 + (morph * 48.0) as usize, false, 0.0) * decay
        }
        _ => sine,
    }
}

fn harmonic_series(phase: f32, count: usize, odd_only: bool, tilt: f32) -> f32 {
    let mut value = 0.0;
    let mut norm = 0.0;
    for harmonic in 1..=count.max(1) {
        if odd_only && harmonic % 2 == 0 {
            continue;
        }
        let weight = (1.0 / harmonic as f32).powf(tilt * 1.5);
        value += (std::f32::consts::TAU * phase * harmonic as f32).sin() * weight;
        norm += weight;
    }
    value / norm.max(1.0)
}

pub fn remove_dc_and_normalize(frame: &mut [f32], normalize: bool) {
    if frame.is_empty() {
        return;
    }
    let mean = (frame.iter().map(|sample| *sample as f64).sum::<f64>() / frame.len() as f64) as f32;
    for sample in frame.iter_mut() {
        *sample -= mean;
    }
    if normalize {
        let peak = frame
            .iter()
            .fold(0.0f32, |peak, sample| peak.max(sample.abs()));
        if peak > 1.0e-9 {
            let gain = 10.0f32.powf(-0.5 / 20.0) / peak;
            frame.iter_mut().for_each(|sample| *sample *= gain);
        }
    }
}

pub fn import_wav<R: Read + Seek>(
    mut reader: R,
    name: impl Into<String>,
    source: impl Into<String>,
    requested_frame_size: Option<usize>,
    auto_pitch_detect: bool,
    normalize: bool,
    sample_rate: f32,
) -> Result<Wavetable, String> {
    reader
        .seek(std::io::SeekFrom::Start(0))
        .map_err(|error| error.to_string())?;
    let mut wav_bytes = Vec::new();
    reader
        .take(MAX_WAV_BYTES + 1)
        .read_to_end(&mut wav_bytes)
        .map_err(|error| error.to_string())?;
    if wav_bytes.len() as u64 > MAX_WAV_BYTES {
        return Err(String::from("WAV file exceeds the 64 MB import limit"));
    }
    let detected_clm = detect_clm_frame_size_bytes(&wav_bytes);
    let wav_bytes = strip_clm_chunks(&wav_bytes);
    let mut reader = hound::WavReader::new(std::io::Cursor::new(wav_bytes))
        .map_err(|error| error.to_string())?;
    let spec = reader.spec();
    if spec.channels == 0 {
        return Err("The WAV file has no audio channels".into());
    }
    let mut mono = Vec::new();
    match spec.sample_format {
        hound::SampleFormat::Float => {
            let mut values = Vec::new();
            for sample in reader.samples::<f32>() {
                values.push(sample.map_err(|error| error.to_string())?);
            }
            for channels in values.chunks(spec.channels as usize) {
                let mean = channels.iter().copied().sum::<f32>() / channels.len().max(1) as f32;
                mono.push(if mean.is_finite() { mean } else { 0.0 });
            }
        }
        hound::SampleFormat::Int => {
            let scale = 2.0f32.powi(spec.bits_per_sample.saturating_sub(1) as i32);
            let mut values = Vec::new();
            for sample in reader.samples::<i32>() {
                values.push(sample.map_err(|error| error.to_string())? as f32 / scale);
            }
            for channels in values.chunks(spec.channels as usize) {
                let mean = channels.iter().copied().sum::<f32>() / channels.len().max(1) as f32;
                mono.push(mean);
            }
        }
    }
    if mono.is_empty() {
        return Err("The WAV file contains no samples".into());
    }
    let frames = if auto_pitch_detect {
        pitch_detect_frames(&mono, spec.sample_rate)
    } else {
        let size = requested_frame_size
            .filter(|size| [256, 512, 1024, 2048, 4096].contains(size))
            .or(detected_clm)
            .unwrap_or(FRAME_SIZE);
        slice_frames(&mono, size)
    };
    let mut normalized_frames = Vec::with_capacity(frames.len().min(MAX_FRAMES));
    for frame in frames.into_iter().take(MAX_FRAMES) {
        let mut frame = resample(&frame, FRAME_SIZE);
        remove_dc_and_normalize(&mut frame, normalize);
        normalized_frames.push(frame);
    }
    Wavetable::from_frames(name, source, normalized_frames, sample_rate).map_err(str::to_string)
}

fn slice_frames(samples: &[f32], frame_size: usize) -> Vec<Vec<f32>> {
    let frame_count = ((samples.len() as f64 / frame_size as f64).round() as usize).max(1);
    let target_len = frame_count.saturating_mul(frame_size);
    let resized = if target_len == samples.len() {
        samples.to_vec()
    } else {
        resample(samples, target_len)
    };
    let count = frame_count.min(MAX_FRAMES);
    (0..count)
        .map(|frame| {
            let selected = if frame_count > count {
                frame * (frame_count - 1) / (count - 1).max(1)
            } else {
                frame
            };
            let start = selected * frame_size;
            resized[start..start + frame_size].to_vec()
        })
        .collect()
}

fn resample(input: &[f32], output_len: usize) -> Vec<f32> {
    if input.is_empty() || output_len == 0 {
        return vec![0.0; output_len];
    }
    if input.len() == 1 {
        return vec![input[0]; output_len];
    }
    (0..output_len)
        .map(|index| {
            let position = index as f64 * (input.len() - 1) as f64 / (output_len - 1).max(1) as f64;
            let base = position.floor() as usize;
            let amount = (position - base as f64) as f32;
            let next = (base + 1).min(input.len() - 1);
            input[base] + (input[next] - input[base]) * amount
        })
        .collect()
}

fn pitch_detect_frames(samples: &[f32], sample_rate: u32) -> Vec<Vec<f32>> {
    let region = &samples[..samples.len().min(8192)];
    let sample_rate = sample_rate.max(1) as usize;
    let min_lag = (sample_rate / 2_000).max(2);
    let max_lag = (sample_rate / 40).max(min_lag);
    let mut best = 0.0f64;
    let mut best_lag = 0;
    for lag in min_lag..=max_lag.min(region.len().saturating_sub(1)) {
        let mut correlation = 0.0f64;
        let mut energy = 0.0f64;
        for index in 0..region.len().saturating_sub(lag) {
            let left = region[index] as f64;
            let right = region[index + lag] as f64;
            correlation += left * right;
            energy += left * left;
        }
        let score = correlation / energy.max(1.0e-12);
        if score > best {
            best = score;
            best_lag = lag;
        }
    }
    if best_lag == 0 || best < 0.15 {
        return slice_frames(samples, FRAME_SIZE);
    }
    let count = (samples.len() / best_lag).clamp(1, MAX_FRAMES);
    (0..count)
        .map(|frame_index| {
            let start = frame_index * best_lag;
            let end = (start + best_lag).min(samples.len());
            resample(&samples[start..end], FRAME_SIZE)
        })
        .collect()
}

pub fn detect_clm_frame_size_bytes(bytes: &[u8]) -> Option<usize> {
    if bytes.len() < 12 || &bytes[..4] != b"RIFF" || &bytes[8..12] != b"WAVE" {
        return None;
    }
    let riff_end = u32::from_le_bytes(bytes[4..8].try_into().ok()?) as usize;
    let end_of_file = riff_end.saturating_add(8).min(bytes.len());
    let mut index = 12usize;
    while index.checked_add(8).is_some_and(|end| end <= end_of_file) {
        let chunk_id = &bytes[index..index + 4];
        let chunk_size = u32::from_le_bytes(bytes[index + 4..index + 8].try_into().ok()?) as usize;
        let data_start = index + 8;
        let data_end = match data_start.checked_add(chunk_size) {
            Some(end) if end <= end_of_file => end,
            _ => return None,
        };
        if chunk_id == b"clm " {
            let data = &bytes[data_start..data_end];
            let digit_start = data
                .iter()
                .position(|byte| *byte != 0 && *byte != b' ')
                .unwrap_or(data.len());
            let digit_end = data[digit_start..]
                .iter()
                .position(|byte| !byte.is_ascii_digit())
                .map_or(data.len(), |length| digit_start + length);
            if digit_end > digit_start {
                if let Ok(text) = std::str::from_utf8(&data[digit_start..digit_end]) {
                    if let Ok(value) = text.parse::<usize>() {
                        if [256, 512, 1024, 2048, 4096].contains(&value) {
                            return Some(value);
                        }
                    }
                }
            }
        }
        index = data_end.saturating_add(chunk_size & 1);
    }
    None
}

fn strip_clm_chunks(bytes: &[u8]) -> Vec<u8> {
    if bytes.len() < 12 || &bytes[..4] != b"RIFF" || &bytes[8..12] != b"WAVE" {
        return bytes.to_vec();
    }
    let mut stripped = bytes[..12].to_vec();
    let mut index = 12usize;
    while index + 8 <= bytes.len() {
        let id = &bytes[index..index + 4];
        let size = u32::from_le_bytes(match bytes[index + 4..index + 8].try_into() {
            Ok(size) => size,
            Err(_) => break,
        }) as usize;
        let end = match index
            .checked_add(8)
            .and_then(|start| start.checked_add(size))
        {
            Some(end) if end <= bytes.len() => end,
            _ => break,
        };
        let padded_end = end.saturating_add(size & 1).min(bytes.len());
        if id != b"clm " {
            stripped.extend_from_slice(&bytes[index..padded_end]);
        }
        index = padded_end;
    }
    if index < bytes.len() {
        stripped.extend_from_slice(&bytes[index..]);
    }
    if stripped.len() >= 8 {
        let riff_size = (stripped.len() - 8) as u32;
        stripped[4..8].copy_from_slice(&riff_size.to_le_bytes());
    }
    stripped
}

pub fn export_wav<W: Write>(
    table: &Wavetable,
    frame_size: usize,
    mut output: W,
) -> Result<(), String> {
    let frame_size = if [256, 512, 1024, 2048, 4096].contains(&frame_size) {
        frame_size
    } else {
        FRAME_SIZE
    };
    let mut wav = std::io::Cursor::new(Vec::new());
    {
        let spec = hound::WavSpec {
            channels: 1,
            sample_rate: table.sample_rate.round() as u32,
            bits_per_sample: 32,
            sample_format: hound::SampleFormat::Float,
        };
        let mut writer =
            hound::WavWriter::new(&mut wav, spec).map_err(|error| error.to_string())?;
        let mut samples = Vec::with_capacity(table.frames.len() * frame_size);
        for frame in &table.frames {
            let resampled = resample(frame, frame_size);
            samples.extend(resampled);
        }
        for sample in samples {
            writer
                .write_sample(sample)
                .map_err(|error| error.to_string())?;
        }
        writer.finalize().map_err(|error| error.to_string())?;
    }
    let mut bytes = wav.into_inner();
    let data_offset = find_wav_data_chunk(&bytes).ok_or("Generated WAV has no data chunk")?;
    let mut clm_data = vec![0];
    clm_data.extend_from_slice(frame_size.to_string().as_bytes());
    clm_data.push(0);
    let chunk_size = clm_data.len() as u32;
    let mut chunk = Vec::with_capacity(8 + clm_data.len() + 1);
    chunk.extend_from_slice(b"clm ");
    chunk.extend_from_slice(&chunk_size.to_le_bytes());
    chunk.extend_from_slice(&clm_data);
    if !chunk_size.is_multiple_of(2) {
        chunk.push(0);
    }
    bytes.splice(data_offset..data_offset, chunk);
    let riff_size = (bytes.len() - 8) as u32;
    bytes[4..8].copy_from_slice(&riff_size.to_le_bytes());
    output.write_all(&bytes).map_err(|error| error.to_string())
}

fn find_wav_data_chunk(bytes: &[u8]) -> Option<usize> {
    let mut index = 12;
    while index + 8 <= bytes.len() {
        let id = &bytes[index..index + 4];
        let size = u32::from_le_bytes(bytes[index + 4..index + 8].try_into().ok()?) as usize;
        if id == b"data" {
            return Some(index);
        }
        index = index.checked_add(8 + size + (size & 1))?;
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_factory_table_has_64_valid_frames() {
        assert_eq!(FACTORY_TABLES.len(), 24);
        for (index, info) in FACTORY_TABLES.iter().enumerate() {
            let table = generate_factory(index, 48_000.0).expect("factory index is valid");
            assert_eq!(table.frames.len(), 64);
            assert!(table
                .frames
                .iter()
                .flatten()
                .all(|sample| sample.is_finite()));
            for (frame_index, frame) in table.frames.iter().enumerate() {
                let mean =
                    frame.iter().map(|sample| *sample as f64).sum::<f64>() / frame.len() as f64;
                let peak = frame
                    .iter()
                    .fold(0.0f32, |peak, sample| peak.max(sample.abs()));
                assert!(
                    mean.abs() < 1.0e-5 && (peak - 10.0f32.powf(-0.5 / 20.0)).abs() < 0.002,
                    "factory table {} frame {} has mean {} and peak {}",
                    info.name,
                    frame_index,
                    mean,
                    peak
                );
            }
            assert!(table.mipmaps.iter().all(|level| level.len() == 64));
        }
    }

    #[test]
    fn mipmaps_rebuild_for_a_new_sample_rate() {
        let table = generate_factory(0, 48_000.0).expect("factory table builds");
        let rebuilt = table
            .rebuild_for_sample_rate(96_000.0)
            .expect("mipmaps rebuild");
        assert_eq!(rebuilt.sample_rate, 96_000.0);
        assert_ne!(table.mipmaps[11][0], rebuilt.mipmaps[11][0]);
    }

    #[test]
    fn wav_export_import_preserves_frame_content_and_clm_frame_size() {
        let table = generate_factory(0, 48_000.0).expect("factory table builds");
        let mut bytes = Vec::new();
        export_wav(&table, FRAME_SIZE, &mut bytes).expect("WAV export succeeds");
        assert_eq!(detect_clm_frame_size_bytes(&bytes), Some(FRAME_SIZE));
        let imported = import_wav(
            std::io::Cursor::new(bytes),
            "round trip",
            "memory",
            None,
            false,
            true,
            48_000.0,
        )
        .expect("WAV import succeeds");
        assert_eq!(imported.frames.len(), table.frames.len());
        let difference = imported.frames[0]
            .iter()
            .zip(&table.frames[0])
            .map(|(left, right)| (left - right).abs())
            .fold(0.0f32, f32::max);
        assert!(difference < 1.0e-4);
    }

    #[test]
    fn wav_import_without_clm_uses_2048_sample_frames() {
        let mut bytes = std::io::Cursor::new(Vec::new());
        let spec = hound::WavSpec {
            channels: 1,
            sample_rate: 48_000,
            bits_per_sample: 32,
            sample_format: hound::SampleFormat::Float,
        };
        {
            let mut writer = hound::WavWriter::new(&mut bytes, spec).expect("WAV writer opens");
            for index in 0..FRAME_SIZE {
                writer
                    .write_sample((std::f32::consts::TAU * index as f32 / FRAME_SIZE as f32).sin())
                    .expect("sample is valid");
            }
            writer.finalize().expect("WAV writer finalizes");
        }
        let table = import_wav(
            std::io::Cursor::new(bytes.into_inner()),
            "one cycle",
            "memory",
            None,
            false,
            false,
            48_000.0,
        )
        .expect("WAV import succeeds without CLM");
        assert_eq!(table.frames.len(), 1);
        assert_eq!(table.frames[0].len(), FRAME_SIZE);
    }

    #[test]
    fn wav_import_accepts_16_24_and_32_bit_stereo_pcm() {
        for bits in [16, 24, 32] {
            let mut bytes = std::io::Cursor::new(Vec::new());
            let spec = hound::WavSpec {
                channels: 2,
                sample_rate: 48_000,
                bits_per_sample: bits,
                sample_format: hound::SampleFormat::Int,
            };
            {
                let mut writer = hound::WavWriter::new(&mut bytes, spec).expect("WAV writer opens");
                let scale = ((1i64 << (bits - 1)) - 1) as f32;
                for index in 0..FRAME_SIZE {
                    let sample = (std::f32::consts::TAU * index as f32 / FRAME_SIZE as f32).sin()
                        * scale
                        * 0.5;
                    writer
                        .write_sample(sample as i32)
                        .expect("left channel sample is valid");
                    writer
                        .write_sample(sample as i32)
                        .expect("right channel sample is valid");
                }
                writer.finalize().expect("WAV writer finalizes");
            }
            let table = import_wav(
                std::io::Cursor::new(bytes.into_inner()),
                "integer stereo",
                "memory",
                Some(FRAME_SIZE),
                false,
                false,
                48_000.0,
            )
            .expect("integer stereo WAV imports");
            assert_eq!(table.frames.len(), 1);
            assert!(table.frames[0].iter().all(|sample| sample.is_finite()));
        }
    }

    #[test]
    fn harmonic_sweep_mipmap_rejects_aliasing_partials_at_high_pitch() {
        let table = generate_factory(3, 48_000.0).expect("harmonic sweep builds");
        let sample_count = 16_384;
        let frequency = 48_000.0 * 900.0 / sample_count as f32;
        let mut phase = 0.0f64;
        let mut samples: Vec<f32> = (0..sample_count)
            .map(|_| {
                let sample = table.sample(phase, frequency, 1.0, FrameInterpolation::Frames);
                phase += frequency as f64 / 48_000.0;
                phase -= phase.floor();
                sample
            })
            .collect();
        let mut planner = RealFftPlanner::<f32>::new();
        let fft = planner.plan_fft_forward(sample_count);
        let mut spectrum = fft.make_output_vec();
        fft.process(&mut samples, &mut spectrum)
            .expect("FFT buffer matches");
        let fundamental = spectrum[900].norm_sqr().max(1.0e-20);
        let unwanted = spectrum
            .iter()
            .skip(900 * 5 + 8)
            .map(|bin| bin.norm_sqr())
            .sum::<f32>();
        let relative_db = 10.0 * (unwanted / fundamental).max(1.0e-20).log10();
        assert!(
            relative_db < -80.0,
            "high partial energy is {relative_db} dB"
        );
    }

    #[test]
    fn pitch_sweep_has_no_mipmap_switch_discontinuity() {
        let table = generate_factory(0, 48_000.0).expect("basic table builds");
        let mut phase = 0.0f64;
        let mut previous = 0.0;
        let mut max_difference = 0.0f32;
        for index in 0..48_000 {
            let frequency = 20.0 * (15_000.0f32 / 20.0).powf(index as f32 / 47_999.0);
            let current = table.sample(phase, frequency, 0.37, FrameInterpolation::Frames);
            if index > 0 {
                max_difference = max_difference.max((current - previous).abs());
            }
            previous = current;
            phase += frequency as f64 / 48_000.0;
            phase -= phase.floor();
        }
        assert!(
            max_difference < 1.5,
            "maximum sample delta {max_difference}"
        );
    }

    #[test]
    fn sample_position_and_mode_are_finite_and_continuous() {
        let table = generate_factory(3, 48_000.0).expect("factory table builds");
        for mode in [
            FrameInterpolation::Frames,
            FrameInterpolation::Smooth,
            FrameInterpolation::Step,
        ] {
            let mut previous = table.sample(0.25, 440.0, 0.0, mode);
            for index in 1..=1000 {
                let current = table.sample(0.25, 440.0, index as f32 / 1000.0, mode);
                assert!(current.is_finite());
                if mode != FrameInterpolation::Step {
                    assert!((current - previous).abs() < 0.25);
                }
                previous = current;
            }
        }
    }
}

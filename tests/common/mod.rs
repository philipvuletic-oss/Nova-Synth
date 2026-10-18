use nova::{
    content::wavetables::{FrameInterpolation, Wavetable},
    engine::{VoiceManager, VoiceRenderSettings},
    params::NovaParams,
};

pub fn render(
    sample_rate: f32,
    samples: usize,
    block_size: usize,
    note_off_at: Option<usize>,
) -> Vec<f32> {
    render_with_blocks(sample_rate, samples, &[block_size], note_off_at)
}

pub fn render_with_blocks(
    sample_rate: f32,
    samples: usize,
    block_sizes: &[usize],
    note_off_at: Option<usize>,
) -> Vec<f32> {
    render_with_wavetable_blocks(
        sample_rate,
        samples,
        block_sizes,
        note_off_at,
        None,
        FrameInterpolation::Frames,
    )
}

pub fn render_with_wavetable(
    sample_rate: f32,
    samples: usize,
    block_size: usize,
    wavetable: &Wavetable,
    interpolation: FrameInterpolation,
) -> Vec<f32> {
    render_with_wavetable_blocks(
        sample_rate,
        samples,
        &[block_size],
        None,
        Some(wavetable),
        interpolation,
    )
}

fn render_with_wavetable_blocks(
    sample_rate: f32,
    samples: usize,
    block_sizes: &[usize],
    note_off_at: Option<usize>,
    wavetable: Option<&Wavetable>,
    interpolation: FrameInterpolation,
) -> Vec<f32> {
    let params = NovaParams::new();
    params.reset_smoothers();
    let mut voice = VoiceManager::new(sample_rate);
    voice.note_on(0, 69, 1.0, 1.0, params.env1.delay.value(), 1);
    let mut output = Vec::with_capacity(samples);
    let bend = [0.0; 16];
    let mut cursor = 0;
    let mut block_index = 0;
    while cursor < samples {
        let block_size = block_sizes
            .get(block_index % block_sizes.len().max(1))
            .copied()
            .unwrap_or(1)
            .max(1);
        block_index += 1;
        let end = (cursor + block_size).min(samples);
        while cursor < end {
            if note_off_at == Some(cursor) {
                voice.note_off(0, 69, false, 0.0);
            }
            let settings = VoiceRenderSettings {
                polyphony: 1,
                a4_tuning: params.global.a4_tuning.value(),
                pitch_bend_range: params.global.pitchbend_range.value(),
                pitch_bend: &bend,
                envelope: params.env1.next_values(),
                velocity_sensitivity: params.global.velocity_sensitivity.smoothed.next(),
                wavetable,
                wt_position: 0.0,
                interpolation,
                oscillator_level: 1.0,
                oscillator_enabled: true,
                unison: Default::default(),
            };
            let sample =
                (voice.render_sample(&settings) * nih_plug::prelude::util::db_to_gain(-6.0)).tanh();
            output.push(sample);
            cursor += 1;
        }
    }
    output
}

pub fn dominant_frequency(samples: &[f32], sample_rate: f32) -> f32 {
    let mut planner = realfft::RealFftPlanner::<f32>::new();
    let fft = planner.plan_fft_forward(samples.len());
    let mut input = samples.to_vec();
    let mut output = fft.make_output_vec();
    let denominator = (samples.len() - 1).max(1) as f32;
    for (index, sample) in input.iter_mut().enumerate() {
        let window = 0.5 - 0.5 * (std::f32::consts::TAU * index as f32 / denominator).cos();
        *sample *= window;
    }
    fft.process(&mut input, &mut output)
        .expect("FFT buffers have the expected lengths");
    let peak = output
        .iter()
        .take(samples.len() / 2)
        .enumerate()
        .skip(1)
        .max_by(|(_, left), (_, right)| left.norm_sqr().total_cmp(&right.norm_sqr()))
        .map(|(index, _)| index)
        .unwrap_or(0);
    peak as f32 * sample_rate / samples.len() as f32
}

pub fn peak(samples: &[f32]) -> f32 {
    samples
        .iter()
        .map(|sample| sample.abs())
        .fold(0.0, f32::max)
}

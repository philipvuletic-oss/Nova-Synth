use criterion::{criterion_group, criterion_main, Criterion};
use nova::{
    engine::{VoiceManager, VoiceRenderSettings},
    params::{envelope::Env1Params, NovaParams},
};
use std::hint::black_box;

fn render_16_voices(criterion: &mut Criterion) {
    let params = NovaParams::new();
    params.reset_smoothers();
    criterion.bench_function("16-voice sine render", |bencher| {
        bencher.iter(|| {
            let mut voices = VoiceManager::new(48_000.0);
            for note in 48..64 {
                voices.note_on(0, note, 0.8, 1.0, 0.0, 16);
            }
            let envelope = Env1Params::default();
            let pitch_bend = [0.0; 16];
            for _ in 0..black_box(512) {
                let settings = VoiceRenderSettings {
                    polyphony: 16,
                    a4_tuning: params.global.a4_tuning.value(),
                    pitch_bend_range: params.global.pitchbend_range.value(),
                    pitch_bend: &pitch_bend,
                    envelope: envelope.next_values(),
                    velocity_sensitivity: params.global.velocity_sensitivity.smoothed.next(),
                };
                black_box(voices.render_sample(&settings));
            }
        })
    });
}

criterion_group!(benches, render_16_voices);
criterion_main!(benches);

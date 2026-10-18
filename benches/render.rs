use criterion::{black_box, criterion_group, criterion_main, Criterion};
use nova::{
    engine::VoiceManager,
    params::{envelope::Env1Params, NovaParams},
};

fn render_16_voices(criterion: &mut Criterion) {
    let params = NovaParams::new();
    criterion.bench_function("16-voice sine render", |bencher| {
        bencher.iter(|| {
            let mut voices = VoiceManager::new(48_000.0);
            for note in 48..64 {
                voices.note_on(0, note, 0.8, 1.0, 0.0, 16);
            }
            let envelope = Env1Params::default();
            let pitch_bend = [0.0; 16];
            let mut left = 0.0;
            let mut right = 0.0;
            for _ in 0..black_box(512) {
                voices.render_sample(
                    16,
                    params.global.a4_tuning.value(),
                    params.global.pitchbend_range.value(),
                    &pitch_bend,
                    &envelope,
                    params.global.velocity_sensitivity.value(),
                    &mut left,
                    &mut right,
                );
                black_box((left, right));
            }
        })
    });
}

criterion_group!(benches, render_16_voices);
criterion_main!(benches);

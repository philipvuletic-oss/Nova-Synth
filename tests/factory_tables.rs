use nova::content::factory::{generate_factory_wavetable, FACTORY_TABLES};

#[test]
fn every_factory_table_frame_is_finite_bounded_and_dc_free() {
    for (index, info) in FACTORY_TABLES.iter().enumerate() {
        let table = generate_factory_wavetable(index, 48_000.0).expect("factory table generates");
        assert_eq!(table.name, info.name);
        for (frame_index, frame) in table.frames.iter().enumerate() {
            let mean = frame.iter().sum::<f32>() / frame.len() as f32;
            let peak = frame.iter().fold(0.0f32, |peak, x| peak.max(x.abs()));
            assert!(
                frame.iter().all(|x| x.is_finite()),
                "{} frame {frame_index}",
                info.name
            );
            assert!(
                peak <= 1.0 + 1.0e-4,
                "{} frame {frame_index} peak {peak}",
                info.name
            );
            assert!(
                mean.abs() < 1.0e-3,
                "{} frame {frame_index} has DC offset {mean}",
                info.name
            );
        }
    }
}

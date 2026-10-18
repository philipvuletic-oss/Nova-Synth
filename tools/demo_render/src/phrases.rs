//! Standard MIDI phrases used for every demo patch.

use crate::host::Note;

pub struct Phrase {
    pub name: &'static str,
    pub notes: Vec<Note>,
    /// Total rendered length in seconds (notes plus release tail).
    pub seconds: f64,
    /// Monophonic phrases get the per-note aliasing analysis.
    pub monophonic: bool,
}

const TAIL: f64 = 2.5;
const BPM: f64 = 128.0;

fn note(start: f64, length: f64, note: u8, velocity: f32) -> Note {
    Note {
        start,
        length,
        note,
        velocity,
    }
}

pub fn all() -> Vec<Phrase> {
    let beat = 60.0 / BPM;
    let bar = beat * 4.0;

    let stab = Phrase {
        name: "a_chord_stab",
        notes: [48u8, 51, 55, 58]
            .iter()
            .map(|n| note(0.1, 2.0, *n, 0.9))
            .collect(),
        seconds: 0.1 + 2.0 + TAIL,
        monophonic: false,
    };

    let bass_pattern: [u8; 8] = [29, 29, 41, 29, 32, 29, 36, 39];
    let mut bass_notes = Vec::new();
    for b in 0..4 {
        for (i, n) in bass_pattern.iter().enumerate() {
            let start = b as f64 * bar + i as f64 * beat / 2.0;
            bass_notes.push(note(start, beat / 2.0 * 0.8, *n, 0.9));
        }
    }
    let bass = Phrase {
        name: "b_bass_f_128bpm",
        notes: bass_notes,
        seconds: 4.0 * bar + 1.0,
        // Notes are too short/low for a meaningful spectral analysis (see analysis.rs).
        monophonic: false,
    };

    // F minor melody: (semitone note, length in beats); each note overlaps the next (legato).
    let melody: [(u8, f64); 14] = [
        (65, 1.0),
        (68, 0.5),
        (70, 0.5),
        (72, 1.0),
        (70, 1.0),
        (68, 1.0),
        (65, 2.0),
        (67, 1.0),
        (68, 1.0),
        (72, 1.0),
        (75, 1.0),
        (72, 2.0),
        (70, 1.0),
        (65, 1.0),
    ];
    let mut lead_notes = Vec::new();
    let mut at = 0.0;
    for (n, beats) in melody {
        let len = beats * beat;
        lead_notes.push(note(at, len + 0.03, n, 0.85));
        at += len;
    }
    let lead = Phrase {
        name: "c_lead_legato",
        notes: lead_notes,
        seconds: at.max(4.0 * bar) + 1.0,
        monophonic: false,
    };

    let pad = Phrase {
        name: "d_pad_chord",
        notes: [41u8, 48, 56, 63, 67]
            .iter()
            .map(|n| note(0.1, 6.0, *n, 0.8))
            .collect(),
        seconds: 0.1 + 6.0 + TAIL,
        monophonic: false,
    };

    let sweep_notes: Vec<Note> = (0..25)
        .map(|i| note(0.1 + i as f64 * 0.4, 0.35, 24 + 3 * i as u8, 0.9))
        .collect();
    let sweep = Phrase {
        name: "e_sweep_c1_c7",
        notes: sweep_notes,
        seconds: 0.1 + 25.0 * 0.4 + 0.5,
        monophonic: true,
    };

    vec![stab, bass, lead, pad, sweep]
}

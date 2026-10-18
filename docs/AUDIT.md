# Nova audit (honest status)

Scope: current `main` (M0 + M1 plus the feedback tooling added in this PR) compared with ROADMAP.md and with a top commercial hybrid wavetable synth (3 wavetable/sample/granular/spectral oscillators + sub + noise, 16-voice unison per oscillator, dual filters with ~40 models, 4 envelopes, 10 drawable LFOs, 8 macros, drag-and-drop modulation + matrix, 13 effects, 5 reverbs, convolution, splitters, 2 FX busses, arpeggiator, clip sequencer, preset browser).

How this was measured: the GUI was rendered headlessly (`cargo run --release -p snapshot`, PNGs in `docs/screenshots/`) and 10 patches x 5 phrases were rendered through the real `Plugin::process` (`cargo run --release -p demo_render`, WAV + spectrogram + JSON in `target/demos/`, table in CI step summary).

## Summary

Nova today is a **single wavetable oscillator + one ADSR (ENV 1) + master volume, polyphonic, with a good wavetable core (mipmapped, band-limited, 24 factory tables, WAV import/export)**. Everything else on the commercial feature list is missing; ~45 of the ~50 source files under `src/engine/` are one-line placeholders. It cannot yet sound like a commercial hybrid synth, mostly because there is no unison, no filter, no modulation and no effects – i.e. none of the things that make a patch move or sound wide.

## Per-area status

| Area | Status | Quality concerns found | What blocks parity |
|---|---|---|---|
| Wavetable oscillator (1 of 3) | **partial** | Band-limited mipmaps per octave, Hermite sampling, 3 frame interpolation modes. Sweep demo (C1..C7, single notes) shows non-harmonic energy of about -28 dB for "Basic Shapes" and -33 dB for most others (metric includes real inharmonic table content, so this is an upper bound); worth a dedicated aliasing/interpolation-image test and 2x oversampling or higher-order table interpolation at high notes. Warp modes absent. | Osc B/C (M2), warp (M4), 2x/4x oversampling for FM/sync/PM |
| Sample / multisample / granular / spectral osc | **missing** | Stub files only. | M13-M16 |
| Sub oscillator, noise | **missing** | Stub files. Noise factory content exists in `content/noises.rs` stub only. | M2 |
| Unison (16 voices/osc) | **missing** | No detune/spread/blend, so supersaw, Reese and "wide" patches cannot be built; Supersaw/Wide Chord demos are approximated with factory tables and a pan. Stereo correlation is 1.00 on every demo (output is mono panned). | M2 |
| Filters (~40 models) | **missing** | Nothing yet (Acid/Growl demos have no resonant filter). | M5 (ZDF/TPT, ladder, diode, MG, comb, formant) |
| Envelopes (4) | **partial** | Only ENV 1 (amplitude), ADSR+delay/hold+curves. Not routable. GUI breakpoint editor exists. | M6 |
| LFOs (10, drawable), macros (8), matrix, drag-and-drop mod | **missing** | Knobs have mod-ring plumbing but nothing feeds it; "8 macro knobs - Coming in M6" placeholder in the GUI. | M6/M7 |
| Effects (13) incl. Hyper/Dimension, OTT-style multiband | **missing** | All `engine/fx/*` files are placeholders. Only the safety tanh clipper exists. | M9/M10 |
| Reverbs (5), HQ delay, convolution, splitters, 2 FX busses | **missing** | Same. | M9-M11 |
| Arpeggiator, clip sequencer | **missing** | GUI pages show "Coming in Mxx" placeholders. | M12, M17 |
| Preset browser | **missing** | Top bar buttons (Browser, Save, Undo, Redo, A/B) are inert. `state_version` migration exists, which is good. | M18 |
| Mixer / routing | **missing** | Only one signal path. | M8 |
| Voice engine / MIDI | **done (basic)** | Poly/legato-free voice allocation, sustain, pitch bend, channel/poly pressure, mod wheel. No portamento, no mono/legato mode (the legato demo is polyphonic overlap). | M2 voicing |
| Output stage | **fixed in this PR** | Asymmetric waveforms (e.g. "Pluck Spectrum Decay") passed through the tanh safety clipper produced DC offsets up to 0.004 (demo check limit is 0.001). Added a 5 Hz one-pole DC blocker after the clipper (`dsp/dc_blocker.rs`, unit-tested). All demo DC readings are now below 0.001. | - |
| Persistence / parameter IDs | **done** | IDs stable; versioned state with migration. | - |
| Performance | **good so far** | Demo CPU real-time factor 0.005-0.025 (release, one core) for up to 5 voices; no unison yet so this will grow by ~16x per oscillator. | Bench with heavy unison once M2 lands |

## GUI audit (from the new screenshots)

Compared against DESIGN.md:

- **Works:** palette, knob arc + glow, ENV 1 graph with live-level marker, tabs, top bar, keyboard strip, resizable layout, 0.75-2x scale.
- **Fixed in this PR:** knob value text overlapped the label (knob widget was too short); missing-glyph boxes for the undo/redo arrows and the Import arrow; keyboard strip clipped at the bottom of the default 1240x780 window; held (external) notes could not be highlighted on the keyboard (new `external_held`).
- **Still deviating from DESIGN.md (not fixed here, needs owner decision / later milestones):**
  - Wavetable display is a flat 2D line, not the 3D 64-frame stack with gradient fill; it uses 128 points so it looks coarse.
  - Page area is one big panel with large empty regions; knobs sit far to the right of the display, leaving the middle empty (OSC page) and the lower half of the modulator strip is empty.
  - Modulator strip is 190 px (DESIGN says 220) because the page area plus 220 px strip plus keyboard no longer fits 780 px with the current widget sizes; at 960x600 the OSC page is cut off and scrolls.
  - PITCH/MOD slider captions are clipped by the window bottom; wheels should be redesigned as compact strips.
  - No panel shadows, no dot-grid at the specified 3% opacity (grid is nearly invisible), no JetBrains Mono / Inter (egui default fonts, TODO in theme.rs), no 120-180 ms transitions.
  - Other pages (MIX, FX, MOD, ARP, CLIP, GLOBAL) are text placeholders.
  - Top bar order differs: Master knob and meter are right-aligned with a large gap; meter has no amber/peak-hold.

## Sound audit (from the demo reports)

- No clipping above -0.3 dBFS on any demo (loudest: "Lead" chord/pad at about -0.4 dBFS; master volume default -6 dB with tanh safety clipper).
- No clicks (click ratio below 3 everywhere; highest are Acid/Growl sweeps at about 2.9 where the tables are intrinsically harsh).
- Stereo correlation is always 1.00 – the synth is effectively mono; this blocks "wide" sounds until unison and stereo effects exist.
- Loudness varies by about 14 LUFS-ish between patches at default levels (Pad about -20, Lead about -6): there is no patch level normalisation yet.
- Aliasing estimate is only valid for stationary single notes; it is reported for the sweep phrase only. Detuned/inharmonic tables (Analog Saw Stack, FM Bell, Vocal Choir, Spectral Noise Pad) naturally score high because their content is not harmonic.

## Bugs fixed in M0/M1 code in this PR (with tests)

1. DC offset after the safety clipper -> DC blocker (`src/dsp/dc_blocker.rs`, tests `removes_constant_offset`, `passes_audio_frequencies`; verified by demo reports).
2. GUI: overlapping knob text, tofu glyphs, clipped keyboard strip (verified in screenshots; `tools/snapshot/tests/nova_gui.rs` guards that every page renders).
3. New regression test `tests/factory_tables.rs`: every factory wavetable frame is finite, peak-bounded and DC-free (no bug found; the DC above came from the clipper, not the tables).

## Recommended next steps

1. M2: unison + sub + noise + Osc B/C (largest audible gap), then M5 filters and M6 modulation.
2. Redesign the OSC page layout to use the available space and implement the 3D wavetable display; re-check with screenshots each time.
3. Add an aliasing guard (threshold per patch) to demo_render once oscillator oversampling exists.

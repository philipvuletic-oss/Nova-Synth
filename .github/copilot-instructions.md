# Nova — permanent instructions for Copilot

## Project
Nova is a hybrid wavetable synthesizer, VST3 instrument, written in Rust with nih-plug (git: https://github.com/robbert-vdh/nih-plug, version pinned by Cargo.lock) and nih_plug_egui for the GUI. Plugin name "Nova", vendor "Nova Audio", VST3 class ID "NovaSynthPlugin1" (never change it). VST3 export only unless explicitly asked.
The owner is not a programmer and tests in Ableton Live 12 on Windows 11. Every PR description must contain plain-language, step-by-step test instructions for Ableton and a list of known limitations.

## Build and verify (all must pass before you finish)
1. cargo fmt --all
2. cargo clippy --workspace --all-targets -- -D warnings  (warnings that originate inside nih-plug macros may be allowed locally with a comment explaining why)
3. cargo test --workspace
4. cargo xtask bundle nova --release
Fix problems; do not silence tests or delete them to make CI pass. Never leave main in a non-compiling state.

## Real-time audio rules (Plugin::process and everything it calls)
- No heap allocation or deallocation. Keep the nih-plug "assert_process_allocs" feature enabled.
- No blocking locks (Mutex/RwLock that can block), no file/network I/O, no logging/println, no thread spawning.
- No panics: no unwrap/expect on runtime data, no unchecked indexing with untrusted values; clamp and validate.
- Preallocate everything in Plugin::initialize / constructors: voices, unison voices, grains, delay lines, FFT buffers, modulation slots, FX module pools.
- GUI -&gt; audio communication: parameters through ParamSetter (with begin/end gesture so Ableton records automation). Everything that is not a parameter (modulation routings, wavetables, samples, FX rack layout, arp patterns, clips) goes through lock-free mechanisms: rtrb ring buffers for small commands; arc-swap or triple_buffer to swap whole immutable objects. Objects replaced on the audio thread are sent back through a "garbage" ring buffer and dropped on a non-audio thread.
- Audio -&gt; GUI: meters, playheads, live modulation values, active voice count, CPU load via atomics or triple_buffer.
- Heavy work (audio file decoding, FFT analysis, mipmap generation, impulse responses, preset loading) runs on nih-plug's background task executor or a dedicated std::thread — never on the audio thread and never blocking the GUI thread. File dialogs (rfd) must not freeze the GUI.
- Flush denormals (FTZ/DAZ via MXCSR on x86_64, restored afterwards) for the duration of process().
- Smooth every parameter that could click. Modulation is computed at control rate in 32-sample sub-blocks with per-sample linear interpolation; audio-rate paths (FM, PM, ring mod, filter audio-rate modulation) are per-sample.
- Safety: if a voice produces NaN/inf, reset that voice silently. Master output has a soft safety clipper (toggle in GLOBAL page, default on).

## Parameters and saved state
- Parameter IDs (#[id = "..."]) are permanent once merged to main: never rename or remove them, only add new ones. Breaking IDs breaks the owner's saved Ableton projects.
- Use #[nested(group = "...")] and #[nested(array, group = "...")] for repeated structures (oscillators A/B/C, filters 1/2, envelopes 1-4, LFOs 1-10, macros 1-8).
- Non-parameter state (mod matrix, LFO shapes, loaded wavetable/sample data or references, FX rack layout, arp/clip data, GUI size/scale, theme) is persisted with #[persist = "..."] serde fields, wrapped in a versioned struct with a `state_version` and migration code. Old projects must always load.
- Use nih-plug's units/formatters so Ableton shows readable values (Hz, dB, %, ms, semitones, note names).

## Code layout (keep it)
src/lib.rs                plugin entry, nih_export_vst3!
src/params/               parameter tree (one file per section)
src/state/                persisted non-parameter state + migrations
src/engine/               voice manager, voice, signal routing, mixer
src/engine/osc/           wavetable, warp, sample, multisample, granular, spectral, sub, noise, unison
src/engine/filter/        filter models
src/engine/mod_system/    envelopes, lfo, macros, matrix, sources
src/engine/fx/            rack, busses, splitters, one file per effect
src/engine/arp/           arpeggiator
src/engine/clip/          clip sequencer
src/dsp/                  shared DSP: smoothing, interpolation, oversampling, fft helpers, delay lines, simd helpers, denormal guard
src/content/              code-generated factory wavetables, noises, IRs, presets
src/ui/                   theme.rs, widgets/, pages/, editor.rs
tests/                    offline render tests (tests/common has the render harness)
benches/                  criterion benchmarks

## DSP quality bar
- Oscillators must be band-limited: mipmapped wavetables (FFT-based, one table per octave, harmonics above that octave's limit removed with a little headroom); PolyBLEP/PolyBLAMP for analytic waveforms; 2x/4x polyphase half-band oversampling for nonlinear stages, sync, FM/PM.
- Filters: zero-delay-feedback / TPT designs (Zavalishin "The Art of VA Filter Design", Cytomic/Simper SVF). Nonlinear ladders use tanh-style saturation inside the loop with oversampling.
- Delays use fractional interpolation (Hermite/cubic). Reverbs: FDN and Dattorro-plate designs.
- f32 for audio, f64 for phase accumulators and long-running timers.
- Implement from public DSP literature. Do not copy code from other synthesizers.

## Testing standard
- tests/common provides an offline render harness: instantiate the plugin, set parameters, send MIDI, render at 44.1/48/96 kHz with block sizes 1, 64, 512 and random variable sizes.
- Every new DSP module gets tests: output finite, bounded peak, correct pitch (FFT peak or zero crossings), aliasing bounds where relevant, silence after release, extreme parameter values stable, sample-rate changes handled.
- Benchmarks in benches/ track the cost of a 16-voice render with heavy unison. Report the numbers in the PR.

## GUI rules
- Follow DESIGN.md exactly. Colours, sizes, fonts only from src/ui/theme.rs. Reuse widgets from src/ui/widgets/.
- Everything scales with the GUI scale factor; window is resizable and remembers its size.
- You cannot see the GUI: keep each panel in its own clearly named function/file, draw carefully, and describe in the PR what each changed area should look like so the owner can compare with a screenshot.

## Legal / originality
Nova is inspired by modern hybrid wavetable synths but is original. Never use the names Serum or Xfer, their presets, wavetables, samples, graphics or exact layout. All factory content (wavetables, noises, impulse responses, presets) is generated by code or created in this repository. Third-party crates must be under licenses compatible with GPLv3 (MIT, Apache-2.0, BSD, ISC, Zlib, MPL-2.0). Fonts only under the SIL Open Font License.

## Workflow
- Read ROADMAP.md and work only on the milestone requested in the task. Tick its checkboxes when done.
- If a milestone is too large to finish in one session, deliver a coherent, compiling, tested subset, leave unchecked boxes for the rest, and list what remains in the PR description.
- Keep README.md up to date (features, build, install).

# Nova

Nova is an original hybrid wavetable synthesizer plug-in for VST3, built in Rust with nih-plug. Its long-term goal is a flexible wavetable instrument with deep modulation, effects, sequencing, and a clear, beginner-friendly interface.

## Feature status

| Milestone | Status |
| --- | --- |
| M0 Foundation | In progress |
| M1 Wavetable engine | In progress |
| M2 Oscillator section | Planned |
| M3 OSC page and main interface | Planned |
| M4 Warp modes | Planned |
| M5 Dual filters | Planned |
| M6 Modulation engine | Planned |
| M7 Modulation interface | Planned |
| M8 Mixer and routing | Planned |
| M9 FX rack infrastructure | Planned |
| M10 Effects | Planned |
| M11 FX interface | Planned |
| M12 Arpeggiator | Planned |
| M13 Sample oscillator | Planned |
| M14 SFZ multisample | Planned |
| M15 Granular oscillator | Planned |
| M16 Spectral oscillator | Planned |
| M17 Clip sequencer | Planned |
| M18 Presets and browser | Planned |
| M19 Wavetable editor | Planned |
| M20 Performance and polish | Planned |

See [ROADMAP.md](ROADMAP.md) for the detailed checklist.

The M1 work adds procedural factory wavetables, FFT-derived pitch mipmaps, three wavetable position interpolation modes, stereo WAV import/export, and the first OSC A controls. M1 remains in progress until its remaining real-time/host acceptance checks are verified.

## Build

Install a current stable Rust toolchain with the `clippy` and `rustfmt` components. On Linux, install the audio and OpenGL/X11 development libraries listed in `.github/workflows/copilot-setup-steps.yml`.

Run the checks and create the VST3 bundle:

```text
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo xtask bundle nova --release
```

The VST3 bundle is written under `target/bundled`. On Windows, CI uploads it as the `nova-windows` artifact.

## Install in Ableton Live 12 on Windows

1. Download and unzip the `nova-windows` build artifact.
2. Copy the complete `nova.vst3` folder into `C:\Program Files\Common Files\VST3`.
3. In Ableton Live, open **Settings → Plug-Ins**.
4. Enable **VST3 system folders**, then select **Rescan**.
5. Create a MIDI track and load Nova from the instrument browser.
6. Play notes from a MIDI keyboard or use Nova's on-screen keyboard. Adjust the envelope and master volume, then record automation by moving Master Volume while the track is armed for automation.

## Current interface and limitations

The resizable window uses the Nova dark-glass theme. The header contains the NOVA logo, preset navigation placeholders, CPU/voice readouts, Master Volume, and stereo output meter. The OSC page now shows an Osc A wavetable card with grouped factory tables, background WAV import/export, frame-size and auto-pitch import options, position/interpolation/level/pan controls, and a 2D frame display. Other pages identify the milestone in which their controls will arrive. The bottom area contains draggable ENV 1 breakpoints and envelope knobs, a macro placeholder, pitch/mod wheels, and a four-octave playable keyboard.

Known limitations: Smooth mode uses a precomputed spectral morph grid rather than a continuously recomputed spectral interpolation; factory selection and WAV decoding are covered by offline tests but have not yet been verified in Ableton on Windows. The remainder of the oscillator section, preset controls, MIDI learn, modulation assignment, other pages, and font embedding are placeholders or planned work. VST3 is the only exported plug-in format.

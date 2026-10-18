# Nova

Nova is an original hybrid wavetable synthesizer plug-in for VST3, built in Rust with nih-plug. Its long-term goal is a flexible wavetable instrument with deep modulation, effects, sequencing, and a clear, beginner-friendly interface.

## Feature status

| Milestone | Status |
| --- | --- |
| M0 Foundation | In progress |
| M1–M20 | Planned — see [ROADMAP.md](ROADMAP.md) |

M0 establishes a playable sine voice, a six-stage amplitude envelope, MIDI pitch bend and sustain, the first interface shell, and the build/test foundation. The remaining oscillator modes, modulation matrix, effects, and sequencers are not implemented yet.

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

The resizable window uses the Nova dark-glass theme. The header contains the NOVA logo, preset navigation placeholders, CPU/voice readouts, Master Volume, and stereo output meter. All seven page tabs are present; feature pages identify the milestone in which their controls will arrive. The bottom area contains draggable ENV 1 breakpoints and envelope knobs, a macro placeholder, pitch/mod wheels, and a four-octave playable keyboard.

Known limitations: only the M0 sine voice and amplitude envelope are implemented; preset controls, MIDI learn, modulation assignment, other pages, extra oscillators, and font embedding are placeholders or planned work. VST3 is the only exported plug-in format.

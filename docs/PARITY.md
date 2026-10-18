# Nova parity checklist

Written from the current code (not the roadmap). Statuses: Missing / Partial / Done.

| ID | Area | Feature | Nova status | Notes / quality issues | Files |
|---|---|---|---|---|---|
| P-OSC-01 | Oscillators | Three oscillators, 5 modes | Partial | Only Osc A, wavetable mode only | src/params/oscillator.rs |
| P-OSC-02 | Oscillators | Multi-target osc routing | Missing | No filters/buses to route to | - |
| P-OSC-03 | Oscillators | Copy/paste osc settings | Missing |  | - |
| P-OSC-04 | Oscillators | Osc as mod source | Missing | No mod system | - |
| P-OSC-05 | Oscillators | Smooth interpolation | Partial | interp_mode exists (frames/smooth); quality unreviewed | src/engine/voice.rs |
| P-OSC-06 | Oscillators | Enhanced unison | Partial | Osc A: voices, detune, range, blend, width, random phase. No stack modes, spread curves option, WT/pan spread, drift | src/engine/voice.rs |
| P-OSC-07 | Oscillators | Tuning modes | Missing | No octave/semi/fine/ratio params | - |
| P-OSC-08 | Oscillators | Dual warp (all warps) | Missing |  | - |
| P-OSC-09 | Oscillators | Phase memory/random | Partial | Params exist; behaviour per voice start | src/params/oscillator.rs |
| P-OSC-10 | Oscillators | Mouse wheel on presets/knobs | Missing | No preset browser; knob wheel unverified | src/ui/widgets/knob.rs |
| P-OSC-11 | Oscillators | Wavetable editor | Missing | Import only | - |
| P-SMP-01 | Oscillators | Sample mode | Missing |  | - |
| P-MSM-01 | Oscillators | Multisample + SFZ | Missing |  | - |
| P-GRN-01 | Oscillators | Granular mode | Missing |  | - |
| P-SPC-01 | Oscillators | Spectral mode | Missing |  | - |
| P-SUB-01 | Oscillators | Sub oscillator | Missing |  | - |
| P-NSE-01 | Oscillators | Noise oscillator | Missing |  | - |
| P-FLT-01 | Filters | Two filters, series/parallel | Missing | No filter at all | - |
| P-FLT-02 | Filters | Mouse cutoff/res in display and mixer | Missing |  | - |
| P-FLT-03 | Filters | Clean drive mode | Missing |  | - |
| P-FLT-04 | Filters | Filters as mod sources | Missing |  | - |
| P-FLT-05 | Filters | Source routing buttons A/B/C/S/N | Missing |  | - |
| P-FLT-06 | Filters | Filter controls incl. variable knob | Missing |  | - |
| P-FLT-07 | Filters | Filter model menu (all groups) | Missing |  | - |
| P-ENV-01 | Mod | Four envelopes | Partial | Only ENV 1 (amp), no BPM/invert legato | src/params/envelope.rs |
| P-LFO-01 | Mod | Up to 10 LFOs, 7-10 reveal | Missing |  | - |
| P-LFO-02 | Mod | LFO editor, presets | Missing |  | - |
| P-LFO-03 | Mod | 1000 Hz, chaos, phase, swing, direction, path | Missing |  | - |
| P-MAC-01 | Mod | Eight macros | Missing |  | - |
| P-MTX-01 | Mod | Matrix with remap curves | Missing |  | - |
| P-MTX-02 | Mod | Per-voice random pitch/pan | Missing |  | - |
| P-ARP-01 | Arp/Clip | Arpeggiator | Missing | Page placeholder only | src/ui/pages |
| P-CLP-01 | Arp/Clip | Clip sequencer | Missing | Page placeholder only | src/ui/pages |
| P-CLP-02 | Arp/Clip | MIDI export | Missing |  | - |
| P-FX-01 | FX | 13 effects | Missing | No effects | - |
| P-FX-02 | FX | 3 splitters | Missing |  | - |
| P-FX-03 | FX | 2 busses + Main, reorder, presets | Missing |  | - |
| P-MIX-01 | Mix | Mix page | Partial | Page layout exists, no real routing | src/ui/pages |
| P-GLB-01 | Global | Voicing, portamento, quality, MPE | Partial | Polyphony, bend range, A4, safety clipper, GUI scale only | src/params/global.rs |
| P-BRW-01 | Browser | Preset browser, tags, Hybridize | Missing | Plugin-format presets only | - |
| P-BRW-02 | Browser | Undo/redo incl. presets/wavetables | Missing |  | - |
| P-BRW-03 | Browser | Key/scale, swing | Missing |  | - |
| P-CNT-01 | Content | >=128 wavetables, >=200 presets | Partial | 24 code-generated wavetables, 0 factory presets | src/content |

## Counts (baseline)

Done: 0 · Partial: 8 · Missing: 35

## Next (priority order)
Filters and routing buttons, warps, mod system (remap/aux curves), LFO extras, osc copy/paste and multi-target routing, browser, undo, MPE, content scale.

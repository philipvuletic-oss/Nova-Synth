# NOVA design system — futuristic, clean, easy to use

## Principles
1. A beginner can make a great sound in 60 seconds; an expert finds every deep control.
2. Every control has a tooltip, a sensible default, double-click reset, and shows its value while dragging.
3. Modulation is visible everywhere: you can always see what moves what.
4. Futuristic but calm: glow is used for meaning (signal, modulation, activity), not decoration.

## Window
- Resizable, default 1240 x 780 logical px, GUI scale 75–200% (setting in GLOBAL page and right-click on logo). Remembers size per project.
- Regions from top to bottom: Top bar (48 px) · Page tabs (36 px) · Page area (flex) · Modulator strip (220 px) · Keyboard strip (56 px).

## Colours (src/ui/theme.rs is the only place colours live)
- Background #0A0D14 with very subtle radial gradient toward #0E1320 at the centre and a faint 24 px dot grid (#FFFFFF at 3% opacity).
- Glass panel #121826 at 85% opacity, border 1 px #1F2A3D, radius 12 px, soft shadow (0, 6, 24, #000 at 45%). Panel header strip 28 px with title in label style and on/off power button at left.
- Cyan #3EF2FF = audio/signal/primary value. Magenta #FF3EA5 = modulation. Lime #B6FF3E = envelopes. Violet #8B7BFF = LFOs. Amber #FFB23E = warnings/clipping/record. Muted #5B667A for inactive.
- Text: values #E6ECF5, labels #8A94A8 uppercase 10–11 px with +6% letter spacing, headings 13 px semibold.
- Fonts: Inter (UI) and JetBrains Mono (numbers), both SIL OFL, embedded. If they cannot be downloaded, use egui defaults and leave a TODO.

## Widgets
- Knob: 270° sweep, thin background arc (#1F2A3D, 3 px), value arc cyan 3 px with soft glow (draw 2–3 wider, low-alpha strokes underneath), centre dot; bipolar knobs fill from the centre. Modulation depth = magenta outer arc (2 px, 4 px outside); live modulated position = small magenta dot that moves in real time. Interaction: vertical drag, Shift or Ctrl = fine, double-click = reset, mouse wheel = step, Alt-click = type value. Right-click menu: Reset, Type value, MIDI learn, Clear MIDI learn, Show modulations, Remove all modulations, Copy/Paste value. Tooltip shows name, value with unit, and number of modulations.
- Slider/fader: same arc language in linear form. Meters: cyan gradient, amber above -3 dBFS, peak hold line.
- Buttons: pill shape, outline when off, cyan fill with glow when on. Dropdowns: glass popup with search field when the list is long.
- Drop target highlight: while dragging a modulator, every modulatable knob shows a dashed magenta ring; the hovered one glows.
- Displays (waveforms, filters, envelopes, LFOs): 2 px glowing line, gradient fill to transparent below, grid lines at 6% white, value read-outs in JetBrains Mono.
- Wavetable display: 3D perspective stack of up to 64 sampled frames (back frames darker and thinner), current frame bright cyan with glow; toggle 2D/3D; drag horizontally to change position.
- Transitions 120–180 ms ease-out. No flashing. Respect a "Reduce motion" setting.

## Top bar
Logo "NOVA" (wordmark, cyan gradient) · preset name with < > arrows (mouse wheel over name switches presets) · Browser button · Save · Undo · Redo · A/B compare · CPU % · Voices active · Master volume knob · Output meter (stereo).

## Page tabs
Pill tabs with animated underline: OSC · MIX · FX · MOD · ARP · CLIP · GLOBAL. Keyboard shortcuts 1–7.

## OSC page
Row 1: three oscillator cards A, B, C (each: mode selector Wavetable/Sample/Multisample/Granular/Spectral, display, tuning row Oct/Semi/Fine/Coarse, main row Position/Level/Pan, Unison row Voices/Detune/Blend/Width, Warp 1 and Warp 2 selectors with amount knobs, routing selector).
Row 2: Sub card, Noise card, Filter 1 card, Filter 2 card (each filter: type dropdown, response display you can drag for cutoff/resonance, Cutoff/Res/Drive/Fat/Mix/Pan knobs, routing).
Page-area width is split proportionally; cards never overlap; at small scales labels shorten rather than overlap.

## MIX page
Channel strips for Osc A, B, C, Sub, Noise, Filter 1, Filter 2, Bus 1, Bus 2, Main: fader, pan, mute, solo, routing dropdowns, send knobs, meter.

## FX page
Three rack columns: MAIN, BUS 1, BUS 2. Modules stacked vertically as collapsible glass cards with drag handle, power button, preset dropdown, mix knob and a compact interactive display. "+ Add effect" button at the bottom of each rack. Splitter modules show their bands as side-by-side sub-racks.

## MOD page
Full modulation matrix table: Source · Aux source · Amount · Curve · Destination · Uni/Bi · Output scale · Bypass · Delete. Drag rows to reorder. Filter by source or destination. Live activity bar per row.

## ARP page / CLIP page
ARP: 12 slot buttons, pattern editor grid, settings panel. CLIP: 12 clip slots, piano roll with velocity and automation lanes, transport/record controls.

## GLOBAL page
Polyphony, voicing, portamento, pitch bend range, MPE off/on (future), oversampling quality, GUI scale, theme intensity (glow amount), reduce motion, safety clipper, tuning (A4 frequency), about.

## Modulator strip (bottom, every page)
Left: tabs ENV 1–4 (lime) and LFO 1–10 (violet; LFO 7–10 appear once LFO 6 is used). Each tab has a crosshair drag handle to create modulations by drag-and-drop. Shows the selected modulator's editor. Right: 8 macro knobs (2 rows of 4) with editable names, each with a drag handle.

## Keyboard strip
Pitch wheel, mod wheel, 4-octave on-screen keyboard (plays notes, shows held notes, scale highlighting when key/scale lock is on), octave shift buttons.

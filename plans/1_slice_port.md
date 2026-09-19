# Source-faithful slice port

```rust
fn geometry(seed: u64, k: &[f64]) -> Vec<Vec<Point>> {
    // Reproduce composeSeal/layoutRings/band and angularCore in source coordinates.
}
fn compile(paths: Vec<Vec<Point>>, seed: u64, k: &[f64]) -> Vec<Stroke> {
    // Equal arc-length cuts; seeded chord flight; rank and schedule.
}
fn paint(frame: &mut ModeFrame<'_>, strokes: &[Stroke], k: &[f64]) {
    // Evaluate explicit playback time, source pose, then bounded ASCII rasterization.
}
```

Each render borrows a fresh grid and seed. No simulation history persists. Numeric controls resolve from positional arguments, captured parameter values, then param_f32; ASCII_T supplies seconds and converts to source milliseconds. Mode identity is `slice`, with one generated registry dispatch. Source seed uses Mulberry32's low 32 bits.

Storage: all finite source SliceParams and core controls are declared Mode::params and therefore captured by existing presets/traces. Source browser viewport visibility, DOM attachment ownership, reduced-motion preference and transport seek events have no terminal equivalents; run/time/speed/loop reproduce an explicit frame.

Source mapping: kit/slice/0_spec.ts → reveal, cut, angle, jit, dist, flight, order, curve, ordN, gain, silence, spread, burst, fease, stretch, os, ailen, ai, aiOpacity, aiFade, weight, finalWeight, time, speed, run. pages/1_slice.tsx → core, coreSides, coreSize, coreTurn, coreFrame. Source seed maps to the existing seed input. Additional shape chooses seal versus source polyShape. Select values preserve source declaration order. Schedule source only uses burst for the `steps` curve; it does not globally quantize every progression despite its hint.

Complexity: geometry bounded by the source 240-unit hero radius and fixed seal grammar (13 candidate bands). Equal cuts have minimum 8 units, at most 41 samples per stroke. Compilation O(P + 41S log P + S log S + 24S), rendering O(width*height + segment raster samples); no allocation or environment access inside cell loops. Segment steps are bounded by visible grid dimensions; pen radius is capped to 3 cells.

Implementation and remaining portability differences are recorded below.

## Implemented files and coverage

| Mode | Source | Coverage |
|---|---|---|
| `slice` | `pages/1_slice.tsx`, `lib/3_seal.ts`, `lib/3a_sealCore.ts`, `lib/6_slice.ts`, `lib/6a_slicePaths.ts`, `lib/6b_slicePose.ts`, `lib/7c_sliceVariation.ts` | Original seal grammar, iris/blades/lattice cores, star/zigzag/spiral test shape; equal arc-length strokes; all five reveal modes, eleven orders, nineteen progression curves/generators, seven flight easings; landing width pulse, final width, untransformed riding blade and independently controlled afterimages |
| `fma` | `algos/2_fma.ts` | Shared symmetry, nearest coprime star, tangent satellites, recursive outward child frames, child symmetry reduction, dual polygon, nested core/eye, detail thresholds. Source Slice animation controls apply to resulting paths. |
| `gothic-icons` | `pages/2_icons.tsx`, `lib/3_seal.ts` | Name hash xor seed multiplication, source icon-order RNG draws, SVG-size-dependent grammar and detail thresholds; selectable source name pools; configurable grid. Slice animation is an added art treatment for this mode. |
| `gothic-circles` | `pages/5_circles.tsx`, `lib/legacy/0_circles.js` | Flower-of-life hex lattice and circular clipping, 13-node Metatron complete graph, opposed vesica circles, source hypotrochoid formula and 720 samples per denominator turn. Slice animates these sacred-geometry paths as on the source page. |

The other sections appended by `pages/14_fma.tsx` (envelope, braid, conformal, cells, resonance), the legacy random ring-diagram JSON editor, legacy seed-to-seed DOM path morph, eye-icon variants and arbitrary icon text input are outside these four art modes. `fma` uses the Fullmetal 2 compositor, not the separate legacy random-diagram compositor.

## Knob mapping

All keys have prefix `SL_` unless stated otherwise. Select knobs display source option names in the existing parameter pane.

| Source property | Knob suffix | Units/behavior |
|---|---|---|
| reveal | REVEAL | draw, cut, slide, draw+slide, glow |
| cut, angle, jit, dist | CUT, ANGLE, JIT, DIST | source px; sweep degrees; chord jitter degrees; chord-length multiplier |
| flight | FLIGHT | milliseconds |
| order, curve | ORDER, CURVE | source declaration-order options |
| ordN, gain, silence | ORD_N, GAIN, SILENCE | stochastic shape parameter, contrast, median gap ceiling |
| spread, burst | SPREAD, BURST | start-window milliseconds; steps-curve bins |
| fease, stretch, os | FEASE, STRETCH, OS | fly ease, chord-axis stretch, overshoot |
| ailen, ai, aiOpacity, aiFade | AILEN, AI, AI_OPACITY, AI_FADE | afterimage half-span in stroke diagonals, enabled, opacity, milliseconds |
| weight, finalWeight | WEIGHT, FINAL_WEIGHT | moving/final width multipliers, including the source 80ms landing pulse |
| time, speed, run | TIME, SPEED, RUN | normalized saved position, speed multiplier, clock advance |
| core, coreSides, coreSize, coreTurn, coreFrame | CORE, CORE_SIDES, CORE_SIZE, CORE_TURN, CORE_FRAME | source core shape, symmetry, radius fraction, degrees, enclosure |
| source polyShape | SHAPE | source seal or star/zigzag/spiral |
| playback loop option | LOOP | repeat or land and hold |
| seed | existing mode seed | Mulberry32 low 32 bits |

FMA knobs: `FMA_N`, `FMA_STEP`, `FMA_DEPTH`, `FMA_SAT`, `FMA_MIN_PX`, `FMA_SCRIPT`, `FMA_PUPIL`. Icons knobs: `ICON_COLS`, `ICON_ROWS`, `ICON_SIZE`, `ICON_MIN_PX`, `ICON_NAMES`, `ICON_PUPIL`. Circle knobs: `CIRCLE_SHAPE`, `CIRCLE_RINGS`, `CIRCLE_N`, `CIRCLE_P`, `CIRCLE_Q`, `CIRCLE_D`.

## Fixed motion knobs

User selected fixed live knobs, with no JSON input. Four independent tracks each expose seventeen knobs, `SL_M1_*` through `SL_M4_*`:

- TARGET: one of the thirty finite source Slice/core properties in Slice, or the twenty-five animation properties in the other modes.
- SCOPE: input or per-stroke. Per-stroke applies to weight, finalWeight, ailen, aiOpacity, aiFade, stretch and os, matching source `STROKE_PROPERTIES`; other targets apply at input scope.
- MODE: off, two-keyframe interpolation, harmonic, drift or hold.
- LOW/HIGH: normalized endpoints within target range. Selects use the source probability pools; booleans use the source 0.5 threshold.
- PERIOD, DELAY: milliseconds.
- EASING: linear, inOutSine, inOutQuad. DIRECTION: normal, reverse, alternate. LOOP: repeat or hold.
- DIST, SECONDARY: normal, uniform, triangular, arcsine, exponential. MIX and DEPTH control distribution mixture and envelope.
- HARMONICS, PHASE, SEED: source random-access variation parameters.

Shared `SL_M_TIME`, `SL_M_SPEED`, `SL_M_RUN` control the property-motion transport. All 71 motion values live in Mode::params; existing saved presets and traces capture them. Slice has 103 knobs. The other modes expose the 97 animation knobs, excluding Slice-only base geometry controls, plus their own geometry controls.

Each track has two endpoints at normalized times 0 and 1. Four tracks can run simultaneously; later tracks replace earlier tracks targeting the same property, matching the source one-track-per-property map. The shared motion cycle is the longest active track's period (doubled for alternate), plus its delay, with a 4000ms minimum. The general browser editor's unbounded keyframe lists and track counts are reduced to this explicit finite knob representation. Source generic inheritance is resolved internally; there is no file, JSON CLI or shared runtime configuration seam.

## Native rendering differences

- Source SVG circular arcs are flattened to 128-segment circles and 32 samples per foil lobe. Source coordinates and SVG serialization use different rounding, so seeds preserve grammar/RNG decisions while arc-length cuts and stroke identity hashes can differ slightly from browser path measurement.
- Terminal cells quantize opacity and stroke width into color/glyph density; palettes color the paths. Filled SVG pupils/dots render as outlined ASCII paths. Rune text is represented by ASCII inscription strokes in FMA. Browser font layout is not reproduced.
- Source pose/easing equations are evaluated from explicit time. Native seek is TIME with RUN disabled. Pausing by changing RUN uses the saved TIME knob; no hidden retained browser elapsed-time state exists.
- The original browser property-motion system can integrate a changing speed across prior frames. This stateless renderer uses current speed multiplied by explicit elapsed time; interactive knob edits do not carry clock history.
- Input tracks rebuild geometry deterministically each frame. Stroke variation identities derive from the sampled native paths; they are stable across replay and independent of render order.
- No viewport observer, DOM ownership, browser reduced-motion preference or mutation/resize observer is installed.

## Bounds and verification

FMA depth is at most 3, symmetry at most 9, giving at most 1 + 10 + 100 recursive circles before source detail culling. Icons have at most 36 cells. Flower lattice has at most 37 circles; Metatron has 13 circles and 78 chords; vesica has at most 25 circles; hypotrochoid has at most 2881 points. Geometry is compiled into cumulative arc tables, so stroke sampling uses binary search. Rendering clips each line to the visible grid before bounded raster work. Motion uses at most four tracks.

Tests cover ordinary/animated fixed-seed snapshots, replay equality, changing times, zero/tiny dimensions and simultaneous knob extrema; source schedules are checked across all 209 order/progression combinations, and all seven eases are checked for finite values. Motion tests cover two-keyframe interpolation, reverse/alternate/delay/hold behavior, periodic random-access variation, independent stroke tracks, and unique knob keys/defaults.

Resource steering: no additional builds or probes from the implementation lane after the user reported CPU/RAM pressure. Root owns guarded single-job compilation, full tests, large-grid probes and live iTerm validation. Test/probe results must be reported from their actual receipts.

## Slice playback variant

`slice` preserves the initial playback implementation. `slice-2` is a copied mode with separate `SL2_` knob keys. Its repeat-cycle knob replays the reveal and motion clocks; playback direction selects forward (0), reverse (1), or alternate (2). Reverse starts from the completed composition. Alternate reverses each successive cycle; disabling repeat plays one forward cycle. Turning off the reveal or motion clock leaves its saved-position knob available for scrubbing.

Random exploration in `slice-2` retains tuned clock positions, clock speeds, run toggles, repeat and playback direction. Other unpinned knobs continue to reroll. With the options pane open in random mode, Left/Right and -/+ change the roll counter. With the pane closed, Left/Right pause and step the native playback clock backward/forward by 0.06 seconds; Space resumes playback.

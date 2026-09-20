# isfahan

Girih portal mode (`src/modes/_101_p02_glm53f_max_c.rs`), lane
`art/p02-glm53f-max-c`. Prompt sequence:
`briefs/p02__glm53f-max-c.PROMPTS.md` (pitch + all parent messages verbatim).

## Composition

Two-centered pointed pishtaq profile (four-centered ogee is sub-cell at
80x24), jamb `0.80*min(world_w*0.5, (h-3)*0.60).max(2.0)`, cusp rise
`jamb*(0.22+0.20*TENSION)`. Seven measure_layer timers per frame:
field, night, scaffold, straps, knots, portal, crown.

- Girih field: rule-based lattice, decagram {10/3} + spokes, pentagram +
  spokes, filler diamonds; decagons across interior `4 + 4*WEAVE`
  (density scales with portal width, absolute radius collapsed in C1).
- Crown: muqarnas fan in the lunette, rings concentric on the apex
  (ring width `(apex-spring)/3.5`), `10 + 2*hash` radial sectors,
  alternating tier/void cells, seam diagonals, impost zone protected.
- Night: band frame, pilaster shadow strip, interior contact shadow.
- Knots: crossings recolored, focal glyphs (heart/eye) per tile variant.

## Animation

One `lantern_wave(look, wx, wy)` phase field (period `TAU*(0.18*t - 0.16*d)`)
sampled by every layer: strap glow, knot flash front (`>0.82` pops the
crossing glyph to `#`), scaffold shimmer, heart twinkle, crown ring
breathing. Continuous in t, no per-frame state, adjacent frames stable.
Live knobs resolve + clamp through `param_f32` with registry defaults
equal to renderer fallbacks.

## Knobs (all [0,1])

| knob | default | effect |
| --- | --- | --- |
| GROWTH | 0.62 | expansion set of live tiles |
| TENSION | 0.35 | cusp rise + lattice rotation |
| LUMINA | 0.55 | glow amplitude (chars invariant, tint only) |
| WEAVE | 0.50 | girih density `4 + 4*weave` |

## Validation receipts

- Snapshots: 80x24 seeds 42/7/2026, t0/t4/t9 phases, 120x40, clip 40x12;
  all visually inspected (seed 7 plumper/filled, seed 2026 sparser/void).
- Invariants: determinism + seed divergence; adjacent WEAVE values closer
  than sweep endpoints; LUMINA 0.50 vs 0.55 keeps plain text identical
  while shifting fg; time moves the drawing.
- Layer coverage (release, 200x60, mitla, 3 reps): 7 layers, 93.3%
  attributed, 0 thin, 0 nested (`perf/layer_coverage.sh isfahan`).
- Knob sweep (release, 200x60, 3s/knob, dt 0.06): baseline 0.12 ms avg;
  worst knob WEAVE=1 at 0.14 ms avg, 6908 fps, 1.19x baseline. Budget
  6 ms avg beaten ~43x. Hotspots at WEAVE=1: night 32.7%, portal 23.6%,
  straps 13.5%, knots 11.8%, crown 11.4% (`perf/knob_sweep.sh isfahan`).
- frame_cost test asserts avg < 6 ms at 200x60 in release builds.

## E2E status (C3, unchanged here)

`scripts/13_e2e.sh` under `scripts/5_probe_guard.py`: guard suite 4/4 OK,
safety/restoration 3/3 OK. Live iTerm2 cases: direct guarded run tripped
the workflow focus guard (`app_active=False`) in an unattended session;
remaining cases unexecuted. Not established; limits untouched. Rerun with
a human keeping the iTerm window focused.

## Pre-existing suite failures (not this lane)

- `morph::iterate_frame_tests::gem_bad_roll6_ansi_regression` (unit)
- `perf_sweep::every_registered_mode...layer_timers` naming
  slice/fma/gothic-icons/gothic-circles/slice-2 (unit, commit a1ee1be)
- `nightglass_seed_42`, `nightglass_rain_running_t9` (integration)
All stash-verified failing without lane changes.

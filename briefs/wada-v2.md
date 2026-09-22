# wada-v2

Braille raster of the wada field: the same Mobius-warped Newton plane, but
each terminal cell packs eight subcell solves (2 columns x 4 rows) into one
U+2800 braille bitmap, so coastlines dither at dot precision instead of one
glyph per cell. src/modes/_103_wada_v2.rs; wada itself is untouched.

## How it reads

Subcell samples disagreeing with the cell's majority basin, unsettled
samples, and samples past the dot fill threshold light their dot. Calm
interiors stay open saturated color with a halftone lattice, interleaved
meet-zones become dense braille weave, charred blocks mark cells whose dots
all unsettled. Clean majority splits still ink as # veins, origin dives
still leave trap sparks, and grain closes with a radial burn.

Knobs: [order] [relax] [twist] [warp] [spin] [trap] [vein] [dot] [hue]
[grain] [aspect], same keys and positions as wada.

## Validation

- `cargo test`: 680 passed. The same three pre-existing failures as at HEAD,
  none of them wada-v2: gridio ansi RGB collapse, morph gem_bad_roll6, and
  the layer-timer gate failing on the slice/fma/gothic suite from a1ee1be.
- In-module: wada_v2_80x24, wada_v2_80x24_t6, determinism, seed sensitivity,
  time sensitivity, knob sensitivity, frame_cost. Release frame_cost at
  200x60: avg 4.92 to 4.96 ms, down from 5.10 ms after the efficiency pass,
  under the 6 ms budget.
- Integration: snapshot_modes wada_v2_seed_42 and wada_v2_dots_t7.
- Snapshots accepted after inspecting the braille output, never blindly.

## Efficiency pass

Requested as "more efficient without snapshot changes", so every edit is
bit-identical: no floating point was restructured and no visual threshold
moved. Two levers only. nearest_root stops once a squared distance under
1e-4 proves the index the full argmin would return (roots are half a unit
apart). And the per-cell labels (majority basin, average dwell) are now
computed once in the solve pass and read by dots, veins and traps instead
of being recomputed up to five times per cell: vein neighbor scans dropped
5229 to 672 us and traps 3340 to 541 us at ORDER=12. All eight committed
snapshots re-ran byte-identical afterward: 12 in-module plus 4 integration
tests green with zero .snap.new files.

## Frame budget work

First cut measured 6.305 ms average, then 7.34 ms on a rerun: six libm
calls per subcell at 96k subcells. The swirl now evaluates once per cell
center as a shared rotation, the convergence tolerance is |f|^2 < 1e-4, and
MAXIT is 24. Both edits changed the image, so snapshots were regenerated
and re-reviewed after each.

## Layer coverage

`perf/layer_coverage.sh 400 120 3 moss wada-v2 perf/results/wada_v2_layers.md`

| mode | layers | calls/frame | attributed | nested | thin |
| --- | ---: | ---: | ---: | --- | --- |
| wada-v2 | 5 | 5.0 | 99.7% | no | no |

1 modes reported: 0 thin under 85 percent, 0 nested over 100 percent, 0 with
no timers at all

## Knob sweep

`perf/knob_sweep.sh wada-v2 2000 1000 2` (raw: perf/results/wada-v2.md)

| knob at max | frames | fps | avg ms | p50 ms | p99 ms | max ms | vs baseline |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| baseline | 8 | 3.7 | 270.79 | 262.60 | 284.33 | 287.39 | 1.00x |
| ORDER=12 | 3 | 1.3 | 777.59 | 778.39 | 778.39 | 792.95 | 2.87x |
| RELAX=1.3 | 7 | 3.1 | 321.05 | 318.07 | 337.67 | 338.72 | 1.19x |
| ASPECT=4 | 7 | 3.4 | 296.58 | 295.90 | 302.44 | 322.99 | 1.10x |
| WARP=1.5 | 8 | 3.6 | 276.04 | 271.54 | 299.78 | 302.24 | 1.02x |
| HUE=90 | 8 | 3.7 | 273.01 | 270.10 | 283.20 | 293.42 | 1.01x |
| TRAP=1.5 | 8 | 3.7 | 272.33 | 264.13 | 287.54 | 292.82 | 1.01x |
| GRAIN=1 | 8 | 3.7 | 271.85 | 263.46 | 280.86 | 296.03 | 1.00x |
| VEIN=1.5 | 8 | 3.8 | 265.51 | 261.33 | 278.84 | 285.04 | 0.98x |
| TWIST=0.9 | 8 | 3.8 | 261.68 | 255.41 | 275.65 | 282.89 | 0.97x |
| SPIN=1 | 8 | 3.8 | 260.15 | 256.85 | 273.55 | 279.80 | 0.96x |
| CONTOUR=12 | 8 | 3.9 | 257.40 | 250.85 | 275.80 | 279.13 | 0.95x |

worst: ORDER=12

### Hotspots at ORDER=12

| layer | calls/frame | avg us | max us | share of frame |
| --- | ---: | ---: | ---: | ---: |
| basins | 1.0 | 806802.2 | 831120.2 | 98.8% |
| dots | 1.0 | 6167.5 | 6747.6 | 0.8% |
| grain | 1.0 | 1209.0 | 1214.9 | 0.1% |
| veins | 1.0 | 672.4 | 736.0 | 0.1% |
| traps | 1.0 | 540.8 | 637.0 | 0.1% |

Baseline is 16M subcell solves (2000x1000 cells x 8 dots); it improved from
311.55 to 270.79 ms through the efficiency pass, and the solve layer carries
98.8 percent at worst-knob with every paint pass under 0.8 percent. The
interactive budget is the in-module frame_cost gate (200x60, under 6 ms),
which measures 4.92 to 4.96 ms in release.

## Previews

perf/previews/103_wada_v2/0_render.py regenerates the reviewed set with
braille decoded to real 2x4 dot lattices: gallery.png plus per-case PNG and
.grid dumps (seeds 42/7/913, t0/t5/t11, order 3/7/12, warp and twist
extremes, relax 0.6/1.3, bare, neon, 160x56, 24x9).

## Prompt sequence

briefs/wada-v2.PROMPTS.md carries the verbatim prompts and the revision log.

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
  200x60: avg 5.14 to 5.16 ms over repeated runs, under the 6 ms budget.
- Integration: snapshot_modes wada_v2_seed_42 and wada_v2_dots_t7.
- Snapshots accepted after inspecting the braille output, never blindly.

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
| wada-v2 | 5 | 5.0 | 99.3% | no | no |

1 modes reported: 0 thin under 85 percent, 0 nested over 100 percent, 0 with
no timers at all

## Knob sweep

`perf/knob_sweep.sh wada-v2 2000 1000 2` (raw: perf/results/wada-v2.md)

| knob at max | frames | fps | avg ms | p50 ms | p99 ms | max ms | vs baseline |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| baseline | 7 | 3.2 | 311.55 | 310.30 | 328.60 | 346.47 | 1.00x |
| ORDER=12 | 3 | 1.1 | 905.09 | 905.71 | 905.71 | 908.36 | 2.91x |
| RELAX=1.3 | 6 | 2.8 | 356.53 | 355.39 | 365.65 | 369.69 | 1.14x |
| ASPECT=4 | 7 | 3.0 | 330.72 | 329.35 | 339.20 | 343.21 | 1.06x |
| WARP=1.5 | 7 | 3.1 | 324.23 | 317.30 | 344.00 | 345.68 | 1.04x |
| GRAIN=1 | 7 | 3.2 | 312.95 | 307.24 | 330.69 | 352.85 | 1.00x |
| VEIN=1.5 | 7 | 3.2 | 307.99 | 312.54 | 314.59 | 315.74 | 0.99x |
| TWIST=0.9 | 7 | 3.2 | 307.88 | 311.41 | 321.25 | 321.27 | 0.99x |
| HUE=90 | 7 | 3.3 | 299.90 | 296.93 | 306.82 | 330.45 | 0.96x |
| CONTOUR=12 | 7 | 3.4 | 297.69 | 302.18 | 313.03 | 314.77 | 0.96x |
| TRAP=1.5 | 7 | 3.4 | 294.30 | 294.47 | 303.63 | 343.64 | 0.94x |
| SPIN=1 | 7 | 3.4 | 291.01 | 285.84 | 301.32 | 335.76 | 0.93x |

worst: ORDER=12

### Hotspots at ORDER=12

| layer | calls/frame | avg us | max us | share of frame |
| --- | ---: | ---: | ---: | ---: |
| basins | 1.0 | 789938.9 | 804161.9 | 97.6% |
| dots | 1.0 | 8828.7 | 9536.4 | 1.1% |
| veins | 1.0 | 5229.6 | 5905.3 | 0.6% |
| traps | 1.0 | 3339.8 | 3612.1 | 0.4% |
| grain | 1.0 | 1442.2 | 1615.5 | 0.2% |

Baseline is 16M subcell solves (2000x1000 cells x 8 dots); the solve layer
carries 97.6 percent at worst-knob as designed and every paint pass stays
under 1.2 percent. The interactive budget is the in-module frame_cost gate
(200x60, under 6 ms), which passes in release with headroom.

## Previews

perf/previews/103_wada_v2/0_render.py regenerates the reviewed set with
braille decoded to real 2x4 dot lattices: gallery.png plus per-case PNG and
.grid dumps (seeds 42/7/913, t0/t5/t11, order 3/7/12, warp and twist
extremes, relax 0.6/1.3, bare, neon, 160x56, 24x9).

## Prompt sequence

briefs/wada-v2.PROMPTS.md carries the verbatim prompts and the revision log.

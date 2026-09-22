# wada

Mobius-twisted Newton basins whose territories share one infinite boundary
(the Wada property). src/modes/_102_wada.rs, registered through the generated
registry; 11 knobs, animation kind Iterate.

## How it reads

Each cell runs Newton's method on z^order - c from a spin-rotated,
Mobius-mapped, swirl-warped start point. Where an orbit lands decides the
basin hue; how long it took sets the dwell contour glyphs. Meet-zones where
labels interleave at pixel scale desaturate into silver weave instead of
fighting five hues, clean splits between calm basins ink as bright # veins,
orbits that plunged through the origin leave trap sparks, and non-settling
cells become hue-tinted char. Grain ends the frame with a radial burn.

Knobs: [order] [relax] [twist] [warp] [spin] [trap] [vein] [contour] [hue]
[grain] [aspect].

## Validation

- `cargo test`: 680 passed. Three failures are pre-existing at HEAD and none
  is wada: `gridio::ansi_frame_tests::animation_encoder_collapses_adjacent_rgb_levels`
  and `morph::iterate_frame_tests::gem_bad_roll6_ansi_regression` (known
  baseline), plus `perf_sweep::every_registered_mode_that_renders_in_process_has_layer_timers`
  which fails on slice, fma, gothic-icons, gothic-circles and slice-2 from
  commit a1ee1be (no layer timers). wada is not in that list.
- In-module: wada_80x24, wada_80x24_t6, determinism, seed sensitivity, time
  sensitivity, knob sensitivity, frame_cost. Release frame_cost at 200x60
  passes the under 6 ms average budget.
- Integration: snapshot_modes wada_seed_42 and wada_dendrite_t7.
- Snapshots accepted after inspecting the rendered PNGs in
  perf/previews/102_wada/, never blindly.

## Efficiency pass

Requested as "more efficient without snapshot changes", so every edit is
bit-identical: no floating point was restructured and no visual threshold
moved. Two levers only. The rayon threshold dropped from 20,480 to 4,096
cells (per-cell work is independent, so thread scheduling cannot change a
byte), which took the serial 200x60 frame from 3.03 ms to 1.10 ms average.
And nearest_root stops once a squared distance under 1e-4 proves the index
the full argmin would return, since roots are half a unit apart. All eight
committed snapshots re-ran byte-identical afterward: 12 in-module plus 4
integration tests green with zero .snap.new files.

## Layer coverage

`perf/layer_coverage.sh 400 120 3 moss wada perf/results/wada_layers.md`

| mode | layers | calls/frame | attributed | nested | thin |
| --- | ---: | ---: | ---: | --- | --- |
| wada | 5 | 5.0 | 98.6% | no | no |

1 modes reported: 0 thin under 85 percent, 0 nested over 100 percent, 0 with
no timers at all

## Knob sweep

`perf/knob_sweep.sh wada 2000 1000 2` (raw: perf/results/wada.md)

| knob at max | frames | fps | avg ms | p50 ms | p99 ms | max ms | vs baseline |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| baseline | 40 | 19.9 | 50.25 | 49.22 | 57.02 | 57.45 | 1.00x |
| ORDER=12 | 17 | 8.2 | 121.46 | 118.62 | 132.64 | 146.68 | 2.42x |
| RELAX=1.3 | 32 | 15.9 | 63.07 | 62.12 | 69.74 | 73.42 | 1.26x |
| TRAP=1.5 | 38 | 18.9 | 52.88 | 51.60 | 59.02 | 71.78 | 1.05x |
| GRAIN=1 | 39 | 19.1 | 52.31 | 51.23 | 61.74 | 63.23 | 1.04x |
| HUE=90 | 39 | 19.1 | 52.24 | 51.19 | 57.51 | 59.11 | 1.04x |
| SPIN=1 | 39 | 19.1 | 52.22 | 51.19 | 59.78 | 61.26 | 1.04x |
| ASPECT=4 | 39 | 19.3 | 51.74 | 51.18 | 60.85 | 60.89 | 1.03x |
| CONTOUR=12 | 39 | 19.4 | 51.64 | 50.19 | 59.17 | 84.66 | 1.03x |
| TWIST=0.9 | 39 | 19.4 | 51.60 | 50.52 | 61.19 | 64.05 | 1.03x |
| WARP=1.5 | 40 | 19.7 | 50.85 | 50.42 | 57.82 | 60.01 | 1.01x |
| VEIN=1.5 | 40 | 20.0 | 50.12 | 50.26 | 56.57 | 57.57 | 1.00x |

worst: ORDER=12

### Hotspots at ORDER=12

| layer | calls/frame | avg us | max us | share of frame |
| --- | ---: | ---: | ---: | ---: |
| basins | 1.0 | 109567.6 | 124123.6 | 92.5% |
| shade | 1.0 | 5622.9 | 6832.5 | 4.7% |
| grain | 1.0 | 1143.0 | 1547.7 | 1.0% |
| veins | 1.0 | 770.8 | 1190.6 | 0.7% |
| traps | 1.0 | 325.8 | 455.0 | 0.3% |

The solve dominates as designed; paint passes together stay under 7 percent.
ORDER=12 costs 2.42x baseline because z^12 needs eleven complex multiplies
per iteration. Baseline improved from 58.32 to 50.25 ms at 2000x1000 through
the efficiency pass below. The in-module frame_cost gate (200x60, under 6 ms)
now measures 1.10 ms average in release, down from 3.03 ms.

## Previews

perf/previews/102_wada/0_render.py regenerates the reviewed set:
gallery.png plus per-case PNG and .grid dumps (seeds 42/7/913, t0/t5/t11,
order 3/7/12, warp and twist extremes, relax 0.6/1.3, bare, neon, 160x56,
24x9). Reviewed at both 80x24 and 160x56.

## Prompt sequence

briefs/wada.PROMPTS.md carries the verbatim prompts and the revision log.

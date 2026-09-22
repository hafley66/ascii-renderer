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

- `cargo test`: 674 passed. Three failures are pre-existing at HEAD and none
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
| baseline | 35 | 17.1 | 58.32 | 56.68 | 70.28 | 74.13 | 1.00x |
| ORDER=12 | 15 | 7.0 | 143.18 | 141.93 | 164.45 | 165.54 | 2.45x |
| RELAX=1.3 | 27 | 13.4 | 74.83 | 74.32 | 86.74 | 100.20 | 1.28x |
| WARP=1.5 | 34 | 16.7 | 59.91 | 57.44 | 76.48 | 85.40 | 1.03x |
| TRAP=1.5 | 34 | 16.8 | 59.39 | 58.56 | 74.30 | 77.60 | 1.02x |
| SPIN=1 | 34 | 16.9 | 59.13 | 56.40 | 66.97 | 79.97 | 1.01x |
| CONTOUR=12 | 34 | 16.9 | 59.04 | 58.08 | 70.18 | 76.18 | 1.01x |
| ASPECT=4 | 34 | 17.0 | 58.90 | 58.62 | 65.35 | 70.08 | 1.01x |
| GRAIN=1 | 35 | 17.1 | 58.43 | 56.74 | 71.32 | 72.50 | 1.00x |
| HUE=90 | 35 | 17.2 | 58.24 | 57.22 | 73.76 | 75.50 | 1.00x |
| VEIN=1.5 | 35 | 17.5 | 57.24 | 55.90 | 66.58 | 70.62 | 0.98x |
| TWIST=0.9 | 36 | 18.0 | 55.68 | 54.01 | 74.70 | 78.39 | 0.95x |

worst: ORDER=12

### Hotspots at ORDER=12

| layer | calls/frame | avg us | max us | share of frame |
| --- | ---: | ---: | ---: | ---: |
| basins | 1.0 | 124280.1 | 137927.9 | 92.8% |
| shade | 1.0 | 6083.4 | 6905.4 | 4.5% |
| grain | 1.0 | 1294.9 | 1726.9 | 1.0% |
| veins | 1.0 | 926.4 | 1314.8 | 0.7% |
| traps | 1.0 | 398.9 | 706.6 | 0.3% |

The solve dominates as designed; paint passes together stay under 7 percent.
ORDER=12 costs 2.45x baseline because z^12 needs eleven complex multiplies per
iteration. Baseline 58 ms at 2000x1000 is 2 million cells; the in-module
frame_cost gate (200x60, under 6 ms) is the interactive-size budget and
passes in release.

## Previews

perf/previews/102_wada/0_render.py regenerates the reviewed set:
gallery.png plus per-case PNG and .grid dumps (seeds 42/7/913, t0/t5/t11,
order 3/7/12, warp and twist extremes, relax 0.6/1.3, bare, neon, 160x56,
24x9). Reviewed at both 80x24 and 160x56.

## Prompt sequence

briefs/wada.PROMPTS.md carries the verbatim prompts and the revision log.

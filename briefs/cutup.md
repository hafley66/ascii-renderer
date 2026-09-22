# cutup

A woodblock landscape torn into a cut-up collage. One printed plate (sun with
rim and rays, two carved ridges, water strata with a cut sun column, broken
horizon, three birds) is cut along wobbled vertical and horizontal tears, and
the fragments are pasted back over bare paper at hashed offsets in hashed z
order. Scraps mirror, shift their ink mix, and some print as negatives with
the block color as ground. The key block then runs again over the reassembled
sheet, off register. src/modes/_104_cutup.rs.

## How it reads

Torn column seams and displaced scraps of mountain, water and sky over visible
paper; the horizon survives as a broken bar across several scraps; the
misregistered key impression ghosts sun rim, rays, crests and horizon in a
second ink. Grain and worn ink close over the print, and the sheet edge takes
a press bite. Under animation the scraps drift at sub-cell resolution (nearest
cell sampled from a continuous offset) and the impression slides off register;
both rest at zero when time is zero.

Knobs: [vcuts] [hcuts] [tear] [shift] [flip] [drift] [misreg] [ink] [grain]
[hue] [aspect].

## Validation

- `cargo test --release`: 684 passed. Five failures, none of them cutup: three
  documented pre-existing (gridio ansi RGB collapse, morph gem_bad_roll6, the
  layer-timer gate naming slice, fma, gothic-icons, gothic-circles, slice-2),
  plus `polytope::tests::snapshot_polytope_small` (one-cell snapshot drift:
  src/polytope.rs changed at c36cbae after the snapshot was recorded at
  afc078d) and `wada_v2::tests::frame_cost`, which passes at 4.965 ms avg on a
  quiet machine and only failed under my concurrent load.
- In-module: cutup_80x24, cutup_80x24_t6, determinism, seed sensitivity, time
  sensitivity, three-frame motion, shift sensitivity, frame_cost. Release
  frame_cost at 200x60: avg 0.297 to 0.312 ms, worst 0.371 ms, against the
  6 ms budget.
- Integration: snapshot_modes cutup_seed_42 (matches the in-module frame,
  CLI and unit paths agree) and cutup_negatives_t7.
- Color smoke: the CLI render at 80x24 carries 93 distinct RGB foregrounds.
- Snapshots accepted after two rounds of visual inspection, never blindly.

## Layer coverage

`perf/layer_coverage.sh 400 120 3 moss cutup perf/results/cutup_layers.md`

| mode | layers | calls/frame | attributed | nested | thin |
| --- | ---: | ---: | ---: | --- | --- |
| cutup | 6 | 6.0 | 95.0% | no | no |

1 modes reported: 0 thin under 85 percent, 0 nested over 100 percent, 0 with
no timers at all

## Knob sweep

`perf/knob_sweep.sh cutup 2000 1000 2` (raw: perf/results/cutup.md)

| knob at max | frames | fps | avg ms | p50 ms | p99 ms | max ms | vs baseline |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| baseline | 208 | 103.5 | 9.66 | 9.37 | 14.38 | 15.50 | 1.00x |
| GRAIN=1 | 177 | 88.4 | 11.32 | 10.48 | 19.14 | 42.31 | 1.17x |
| INK=1 | 199 | 99.1 | 10.09 | 9.68 | 15.54 | 21.97 | 1.04x |
| SHIFT=24 | 200 | 99.8 | 10.02 | 9.72 | 15.26 | 30.84 | 1.04x |
| DRIFT=1 | 202 | 100.6 | 9.95 | 9.56 | 16.91 | 20.11 | 1.03x |
| VCUTS=12 | 202 | 100.9 | 9.91 | 9.67 | 13.15 | 16.32 | 1.03x |
| MISREG=6 | 204 | 101.7 | 9.83 | 9.44 | 14.86 | 16.32 | 1.02x |
| TEAR=4 | 211 | 105.4 | 9.49 | 9.28 | 12.50 | 13.39 | 0.98x |
| FLIP=1 | 214 | 106.6 | 9.38 | 9.11 | 13.14 | 14.91 | 0.97x |
| HCUTS=12 | 215 | 107.3 | 9.32 | 9.09 | 12.25 | 14.96 | 0.96x |
| HUE=360 | 218 | 108.8 | 9.19 | 8.89 | 13.17 | 14.84 | 0.95x |
| ASPECT=4 | 240 | 119.6 | 8.36 | 8.17 | 11.71 | 15.42 | 0.87x |

worst: GRAIN=1

### Hotspots at GRAIN=1: 217 frames, 108.2 fps

| layer | calls/frame | avg us | max us | share of frame |
| --- | ---: | ---: | ---: | ---: |
| plate | 1.0 | 3581.2 | 7164.3 | 38.8% |
| collage | 1.0 | 1792.3 | 2946.8 | 19.4% |
| grain | 1.0 | 1141.1 | 1952.4 | 12.4% |
| backing | 1.0 | 945.8 | 2400.2 | 10.2% |
| overprint | 1.0 | 710.7 | 1900.7 | 7.7% |
| cuts | 1.0 | 65.7 | 249.5 | 0.7% |

Baseline at 2M cells is 9.66 ms and no knob crosses 1.17x: SHIFT=24 stays at
1.04x because fragment overlap is bounded (each scrap clamps its shift to
three quarters of its own size), and cuts at 0.7 percent means the wobble
tables are noise. The interactive budget is the in-module frame_cost gate
(200x60, under 6 ms), which measures 0.30 ms in release.

## Terminal E2E

`scripts/13_e2e.sh --headless --mode cutup --case all`
(raw: perf/results/e2e-1790051767850)

| case | status |
| --- | --- |
| workflow | passed |
| seed-search | passed |
| bad-400x200 | passed |
| max-400x200 | passed |
| backpressure | failed |

The first run failed the workflow case on "art pixels did not change during
animation": the suite compares captures across 3 animation frames (dt 0.06)
and the integer-rounded drift moved nothing in that window. Fragment drift
became continuous sub-cell offsets (nearest cell from a continuous sample),
the two t=0 snapshots stayed byte-identical through the refactor, and
motion_within_three_frames now pins the contract. The rerun is the table
above: animation motion, live knobs, seed controls, resize, pause/resume and
termios restoration all pass for cutup, and bad-400x200 and max-400x200 hold
the animation cadence budget at 400x200 (p95 <= 33.34 ms, max <= 100 ms) with
every knob at maximum.

backpressure fails at its setup precondition (4_test_input_latency.py:169,
stalled-consumer terminal_wait_us never crossing 500 ms), before any knob
gate. The same assertion fails with the default gem-aetherium-2 mode
(perf/results/e2e-1790051847322), so it is mode-independent, and
perf/0_E2E.md already documents this case as failing regression
evidence at HEAD. GUI painting is untested in headless mode, and pin-inputs is
not part of the headless case set.

## Prompt sequence

briefs/cutup.PROMPTS.md carries the verbatim prompts and the revision log.

# kolam performance receipt

Mode: `kolam` (`src/modes/_101_p02_glm53f_max_a.rs`), 11 knobs, 6 layer timers.
All probes run release, machine otherwise idle, existing guard limits unchanged.

## Layer coverage

Command: `perf/layer_coverage.sh 400 120 3 moss kolam`

| mode | layers | calls/frame | attributed | nested | thin |
| --- | ---: | ---: | --- | --- | --- |
| kolam | 6 | 6.0 | 90.8% | no | no |

Layers: look, field, knots, lotus, ornament, embers. Passes the 85 percent
coverage bar with no nested attribution. (At 80x24 the same run attributes
43 percent; the fixed per-frame scaffolding dominates tiny grids.)

## Knob sweep

Command: `perf/knob_sweep.sh kolam 2000 1000 2` (full table in
`perf/results/kolam.md`)

- baseline: 14.63 ms avg (68.4 fps) at 2000x1000
- worst knob: CURL=1.2 at 20.43 ms avg (49.0 fps), 1.40x baseline
- next: RINGS=7 at 1.24x; every other knob at max stays within 0.91x to 1.04x
- no knob reaches a cadence-breaking cost at 2 megapixels

## Layer hotspots at worst knob (CURL=1.2)

| layer | avg us | share of frame |
| --- | ---: | ---: |
| field | 6279.8 | 46.2% |
| knots | 4663.2 | 34.3% |
| ornament | 1300.5 | 9.6% |
| lotus | 223.8 | 1.6% |
| look | 14.0 | 0.1% |
| embers | 2.3 | 0.0% |

The scalar field trace and isoline pass dominate, matching the design: one
phi evaluation per cell plus one isoline/dot pass. Embers are O(count).

## Unit frame cost

`frame_cost` test, 200x60, 100 frames, release: passes the 6 ms average gate
(gate is release-only; debug builds only print).

## Environmental note

Two stale-cache incidents during probing (a release test binary predating the
mode registration served by the shared lane target) produced a false
"0 modes reported" and a false "does not render natively". After
`cargo clean -p ascii-renderer` the probes above measure the current binary.

# opus-5-dover

Six math chapters on one timer, cycling every DWELL seconds.

## Contents

1. [Chapters](#chapters)
2. [Knobs](#knobs)
3. [Perf receipt](#perf-receipt)
4. [Hotspots](#hotspots)
5. [Wiring](#wiring)

## Chapters

| # | chapter | what moves | source |
| --- | --- | --- | --- |
| 0 | fourier analysis | epicycle chain of 2N+1 arms sweeps th, trace paints behind the tip | `ch_fourier`, src/opus_5_dover.rs:376 |
| 1 | differential geometry | torus turns in yaw and pitch, glyph tint splits on the sign of K | `ch_geometry`, src/opus_5_dover.rs:576 |
| 2 | graph theory | BFS front walks out from vertex 0 over a spanning tree plus chords | `ch_graph`, src/opus_5_dover.rs:1011 |
| 3 | lambda calculus | normal-order beta reduction, one step per tick, redex highlighted | `ch_lambda`, src/opus_5_dover.rs:1596 |
| 4 | topology | Mobius band turns, a marker with its normal walks the centerline | `ch_topology`, src/opus_5_dover.rs:724 |
| 5 | linear algebra | A = I + s(M - I) breathes, lattice shears, unit circle to ellipse | `ch_linalg`, src/opus_5_dover.rs:1114 |

Chapter 3 reduces a seeded term from `plus 2 3`, `mult 2 3`, `S K K 2`, `3 succ 0`, `2 2`
with de Bruijn indices; `church_arithmetic_reaches_normal_form` pins `mult 2 3` to
`\x.\y.x (x (x (x (x (x y)))))`.

## Knobs

| key | meaning | default | range |
| --- | --- | ---: | --- |
| CHAPTER | -1 cycles, 0..5 pins one | -1 | -1..5 |
| DWELL | seconds per chapter | 14 | 2..120 |
| SPEED | time scale for every chapter | 1.0 | 0..6 |
| HARM | fourier harmonics N | 9 | 1..24 |
| NODES | graph vertices | 22 | 4..60 |
| CHORDS | chords per vertex | 0.35 | 0..2 |
| STEPS | beta step cap / 40 | 0.6 | 0.05..4 |
| TWIST | band half-twists | 1 | 1..6 |
| TUBE | torus tube radius, band width | 0.42 | 0.12..0.85 |
| MESH | surface sample density | 1.0 | 0.25..3 |
| TRAIL | epicycle trace length | 0.55 | 0.02..1 |
| LABEL | chapter caption bar | 1 | 0..1 |
| ASPECT | cols per row | 2.0 | 0.25..4 |

Positional CLI args follow the same order:
`ascii-renderer 42 opus-5-dover moss [chapter] [dwell] [speed] ...`

## Perf receipt

`perf/knob_sweep.sh opus-5-dover 2000 1000 2`, full output in `perf/results/opus-5-dover.md`.

| knob at max | frames | fps | avg ms | p99 ms | vs baseline |
| --- | ---: | ---: | ---: | ---: | ---: |
| baseline | 1346 | 672.7 | 1.49 | 2.06 | 1.00x |
| MESH=3 | 1113 | 556.5 | 1.80 | 3.96 | 1.21x |
| NODES=60 | 1252 | 625.2 | 1.60 | 2.80 | 1.08x |
| LABEL=1 | 1273 | 636.4 | 1.57 | 2.21 | 1.06x |
| STEPS=4 | 1287 | 643.1 | 1.56 | 2.24 | 1.05x |
| TWIST=6 | 1310 | 654.9 | 1.53 | 2.26 | 1.03x |
| ASPECT=4 | 1315 | 657.2 | 1.52 | 2.55 | 1.02x |
| CHORDS=2 | 1355 | 677.3 | 1.48 | 2.31 | 0.99x |
| TRAIL=1 | 1358 | 678.9 | 1.47 | 1.86 | 0.99x |
| TUBE=0.85 | 1359 | 679.1 | 1.47 | 1.98 | 0.99x |
| HARM=24 | 1396 | 697.9 | 1.43 | 2.07 | 0.96x |
| SPEED=6 | 1438 | 718.8 | 1.39 | 1.63 | 0.94x |
| CHAPTER=5 | 1468 | 733.7 | 1.36 | 1.63 | 0.92x |
| DWELL=120 | 1502 | 750.7 | 1.33 | 1.56 | 0.90x |

Unit `frame_cost` at 200x60 release: avg 0.060 ms, worst 1.733 ms over 200 frames spanning all six chapters.

## Hotspots

At MESH=3, 1151 frames, 575.3 fps.

| layer | calls/frame | avg us | share of frame |
| --- | ---: | ---: | ---: |
| page | 1.0 | 411.9 | 23.7% |
| topology-band | 0.2 | 1090.7 | 11.8% |
| geometry-mesh | 0.2 | 658.4 | 7.7% |
| caption | 1.0 | 5.3 | 0.3% |
| geometry-curve | 0.2 | 22.9 | 0.3% |
| topology-edge | 0.2 | 24.5 | 0.3% |
| graph-edges | 0.2 | 18.5 | 0.2% |
| fourier-arms | 0.2 | 8.7 | 0.1% |
| remaining 7 layers | 0.2 | < 3 | 0.0% |

Two reads on the table. The four `linalg-*` layers are absent because the 2 s run at
DWELL=14 reaches five chapters, not six; pin `CHAPTER=5` to time them. Timed layers sum
to 45 percent of the frame rather than the 85 percent `perf/INSTRUMENT.md` asks for: the
untimed remainder is the fixed ~0.9 ms encode and present cost per frame, the same
absolute overhead chladni carries, which only looks large next to a 1.5 ms frame.

## Wiring

| touchpoint | site |
| --- | --- |
| module | src/opus_5_dover.rs |
| dispatch | src/cli.rs:2042 |
| help | src/cli.rs:465 |
| mod decl | src/main.rs:96 |
| demo list | src/opts.rs:708 |
| registry form | src/registry.rs, `ModeForm { names: &["opus-5-dover"] }` |
| native animation | src/morph.rs:882 |
| perf list | src/perf_sweep.rs:257 |
| snapshots | src/snapshots/*opus_5_dover* (6), tests/snapshots/*opus_5_dover* (2) |

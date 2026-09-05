# Vesper render efficiency

Release build, 2000x1000 cells, seed 42, moss theme, 3 seconds per scenario,
0.06 animation-time steps. Existing `perf_sweep::perf_knob_sweep` harness;
original and final binaries ran sequentially. These timings exclude ANSI
encoding, terminal presentation, and first-frame cache construction. Host load
was not isolated; tail latency remains variable.

| Scenario | Before avg ms | After avg ms | Speedup |
| --- | ---: | ---: | ---: |
| Defaults | 44.01 | 12.46 | 3.53x |
| THREADS=160 | 77.52 | 36.24 | 2.14x |
| FOLDS=9 | 45.56 | 18.44 | 2.47x |
| OPEN=0.65 | 43.46 | 14.57 | 2.98x |
| SPEED=2 | 51.43 | 17.94 | 2.87x |

## Changes

- Cache eclipse cells and occlusion depths by dimensions, seed, and full palette.
  Knob changes and time advances reuse this static background.
- Retain scratch storage in one thread-local workspace. The animation worker
  persists between frames, so this cache survives normal demo playback.
- Compute angular sine/cosine, tube deformation, and illumination terms once
  per angular sample rather than once per filament per sample. Filament sampling
  density and depth-compositing order are unchanged.
- Composite only touched cells; store lighting as its exact 8-bit ramp index,
  reducing this buffer from 8 MB to 2 MB at the measured dimensions.

Caching retains an additional background and eclipse-depth array per rendering
thread. Capacity tracks the largest visited size; this is a memory/time tradeoff.
The first frame and seed/palette/size changes still construct the background.
No frame skipping, sample reduction, or control-range changes were introduced.

## Correctness

The original renderer produced a new fixed regression snapshot of 54 full-color
frame hashes before optimization. All remain identical in debug and release.
Existing readable art snapshots remain unchanged. Tests also exercise palette
cache invalidation, control changes, small grids, nonfinite inputs, and replay.

`cargo test --offline`: 375 unit + 1 generator + 181 integration passed,
10 ignored. All 3 Vesper release tests passed.

## Original sweep

# knob sweep: vesper 2000x1000, 3s per run, dt 0.06, theme moss

| knob at max | frames | fps | avg ms | p50 ms | p99 ms | max ms | vs baseline |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| baseline | 69 | 22.7 | 44.01 | 42.08 | 61.01 | 83.31 | 1.00x |
| THREADS=160 | 39 | 12.9 | 77.52 | 72.42 | 121.90 | 136.19 | 1.76x |
| SPEED=2 | 59 | 19.4 | 51.43 | 46.13 | 74.77 | 187.22 | 1.17x |
| FOLDS=9 | 66 | 21.9 | 45.56 | 44.60 | 57.20 | 63.56 | 1.04x |
| OPEN=0.65 | 70 | 23.0 | 43.46 | 41.86 | 68.70 | 81.53 | 0.99x |

worst: THREADS=160

## hotspots at THREADS=160: 31 frames, 10.1 fps

| layer | calls/frame | avg us | max us | share of frame |
| --- | ---: | ---: | ---: | ---: |
| filaments | 1.0 | 67979.4 | 98513.9 | 68.8% |
| eclipse | 1.0 | 21701.5 | 48423.8 | 22.0% |
| composite | 1.0 | 3909.8 | 6269.5 | 4.0% |

## Final sweep

# knob sweep: vesper 2000x1000, 3s per run, dt 0.06, theme moss

| knob at max | frames | fps | avg ms | p50 ms | p99 ms | max ms | vs baseline |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| baseline | 241 | 80.2 | 12.46 | 11.61 | 22.91 | 32.47 | 1.00x |
| THREADS=160 | 83 | 27.6 | 36.24 | 35.15 | 53.62 | 75.87 | 2.91x |
| FOLDS=9 | 165 | 54.2 | 18.44 | 14.02 | 52.71 | 169.20 | 1.48x |
| SPEED=2 | 168 | 55.7 | 17.94 | 16.30 | 34.02 | 47.33 | 1.44x |
| OPEN=0.65 | 206 | 68.6 | 14.57 | 13.37 | 27.90 | 36.34 | 1.17x |

worst: THREADS=160

## hotspots at THREADS=160: 61 frames, 20.3 fps

| layer | calls/frame | avg us | max us | share of frame |
| --- | ---: | ---: | ---: | ---: |
| filaments | 1.0 | 39887.2 | 191118.1 | 80.9% |
| composite | 1.0 | 4414.5 | 32884.5 | 9.0% |
| restore | 1.0 | 3032.5 | 70981.5 | 6.2% |

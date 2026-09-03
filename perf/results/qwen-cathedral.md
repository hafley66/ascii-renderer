# knob sweep: qwen-cathedral 2000x1000, 1s per run, dt 0.06, theme moss

| knob at max | frames | fps | avg ms | p50 ms | p99 ms | max ms | vs baseline |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| baseline | 27 | 26.7 | 37.47 | 37.30 | 38.51 | 38.66 | 1.00x |
| MOSAIC=1 | 26 | 25.5 | 39.18 | 38.91 | 40.17 | 43.20 | 1.05x |
| CANDLES=40 | 26 | 26.0 | 38.53 | 38.15 | 39.92 | 44.01 | 1.03x |
| ROSE=18 | 27 | 26.1 | 38.30 | 38.42 | 39.58 | 39.63 | 1.02x |
| TOWERS=4 | 27 | 26.3 | 38.03 | 37.55 | 40.89 | 43.64 | 1.01x |
| RAY=1 | 27 | 26.3 | 37.97 | 37.49 | 40.03 | 40.38 | 1.01x |
| GLOW=1.5 | 27 | 26.4 | 37.88 | 37.58 | 39.51 | 40.50 | 1.01x |
| SPEED=3 | 27 | 26.4 | 37.82 | 37.54 | 39.16 | 39.69 | 1.01x |
| NAVES=9 | 27 | 26.5 | 37.78 | 37.33 | 39.06 | 45.04 | 1.01x |
| SMOKE=24 | 27 | 26.5 | 37.69 | 37.46 | 38.93 | 39.77 | 1.01x |
| DEPTH=5 | 27 | 26.6 | 37.64 | 37.50 | 39.14 | 39.44 | 1.00x |
| ARCH=1 | 27 | 26.6 | 37.58 | 37.48 | 38.25 | 38.25 | 1.00x |
| BANNERS=8 | 27 | 26.6 | 37.56 | 37.52 | 38.12 | 38.21 | 1.00x |

worst: MOSAIC=1

## hotspots at MOSAIC=1: 26 frames, 25.6 fps

no measure_layer timers fired for qwen-cathedral; wrap its painters in crate::_0_profile::measure_layer

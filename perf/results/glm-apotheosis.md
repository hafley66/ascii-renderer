# knob sweep: glm-apotheosis 2000x1000, 1s per run, dt 0.06, theme moss

| knob at max | frames | fps | avg ms | p50 ms | p99 ms | max ms | vs baseline |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| baseline | 22 | 21.4 | 46.66 | 46.40 | 48.01 | 48.15 | 1.00x |
| CLOUDS=4 | 21 | 21.0 | 47.65 | 47.47 | 48.58 | 49.95 | 1.02x |
| SPOKES=24 | 22 | 21.3 | 46.87 | 46.63 | 47.45 | 49.57 | 1.00x |
| GLOW=1.5 | 22 | 21.4 | 46.70 | 46.74 | 47.08 | 47.15 | 1.00x |
| DENS=1 | 22 | 21.4 | 46.67 | 46.37 | 47.91 | 48.83 | 1.00x |
| SPARKS=60 | 22 | 21.5 | 46.53 | 46.40 | 47.12 | 49.39 | 1.00x |
| RAYS=1 | 22 | 21.5 | 46.52 | 46.38 | 47.39 | 48.36 | 1.00x |
| RUNES=1 | 22 | 21.5 | 46.50 | 46.43 | 47.14 | 47.80 | 1.00x |
| WINGS=2 | 22 | 21.5 | 46.49 | 46.33 | 47.25 | 48.28 | 1.00x |
| MOTES=80 | 22 | 21.5 | 46.47 | 46.15 | 48.49 | 48.58 | 1.00x |
| SPEED=3 | 22 | 21.6 | 46.37 | 46.32 | 46.80 | 47.04 | 0.99x |
| RINGS=9 | 22 | 21.6 | 46.37 | 46.28 | 46.96 | 47.21 | 0.99x |
| BOB=3 | 22 | 21.6 | 46.32 | 46.19 | 47.31 | 47.45 | 0.99x |

worst: CLOUDS=4

## hotspots at CLOUDS=4: 22 frames, 21.2 fps

no measure_layer timers fired for glm-apotheosis; wrap its painters in crate::_0_profile::measure_layer

# knob sweep: gem-aetherium 2000x1000, 1s per run, dt 0.06, theme moss

| knob at max | frames | fps | avg ms | p50 ms | p99 ms | max ms | vs baseline |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| baseline | 15 | 14.6 | 68.35 | 68.03 | 70.68 | 71.10 | 1.00x |
| NEBULA=1.5 | 14 | 13.7 | 72.74 | 72.56 | 73.55 | 73.63 | 1.06x |
| RAYS=1.5 | 15 | 14.5 | 68.93 | 68.74 | 70.13 | 70.87 | 1.01x |
| ZODIAC=24 | 15 | 14.6 | 68.28 | 68.01 | 69.44 | 71.11 | 1.00x |
| HARMONY=8 | 15 | 14.7 | 68.13 | 67.65 | 70.65 | 71.19 | 1.00x |
| TILT=1 | 15 | 14.7 | 67.98 | 67.68 | 68.74 | 70.14 | 0.99x |
| RINGS=10 | 15 | 14.7 | 67.92 | 67.68 | 69.15 | 70.05 | 0.99x |
| PLANETS=12 | 15 | 14.7 | 67.85 | 67.57 | 68.66 | 68.66 | 0.99x |
| COMETS=12 | 15 | 14.8 | 67.77 | 67.71 | 68.16 | 68.61 | 0.99x |
| PULSE=2 | 15 | 14.8 | 67.76 | 67.62 | 68.21 | 68.34 | 0.99x |
| GEARS=8 | 15 | 14.8 | 67.74 | 67.81 | 68.42 | 68.70 | 0.99x |
| SPEED=3 | 15 | 14.8 | 67.73 | 67.68 | 68.15 | 68.64 | 0.99x |
| RUNES=1 | 15 | 14.8 | 67.66 | 67.49 | 68.33 | 68.48 | 0.99x |

worst: NEBULA=1.5

## hotspots at NEBULA=1.5: 14 frames, 13.8 fps

no measure_layer timers fired for gem-aetherium; wrap its painters in crate::_0_profile::measure_layer

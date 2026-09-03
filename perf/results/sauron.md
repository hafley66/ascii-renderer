# knob sweep: sauron 2000x1000, 1s per run, dt 0.06, theme moss

| knob at max | frames | fps | avg ms | p50 ms | p99 ms | max ms | vs baseline |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| baseline | 28 | 27.9 | 35.90 | 35.78 | 36.44 | 36.61 | 1.00x |
| SLIT=5 | 28 | 27.7 | 36.04 | 35.83 | 37.25 | 37.48 | 1.00x |
| EMBERS=90 | 28 | 27.8 | 35.95 | 35.86 | 36.73 | 37.07 | 1.00x |
| GAZE=1 | 28 | 27.9 | 35.83 | 35.68 | 36.38 | 36.81 | 1.00x |
| TURB=3 | 28 | 27.9 | 35.81 | 35.77 | 36.56 | 36.57 | 1.00x |
| IRIS=0.95 | 29 | 28.1 | 35.63 | 35.54 | 36.48 | 37.00 | 0.99x |
| BLAZE=2 | 29 | 28.2 | 35.52 | 35.50 | 36.02 | 36.11 | 0.99x |

worst: SLIT=5

## hotspots at SLIT=5: 28 frames, 27.8 fps

no measure_layer timers fired for sauron; wrap its painters in crate::_0_profile::measure_layer

# knob sweep: astra-opus-1-chronofold 2000x1000, 2s per run, dt 0.06, theme deep

| knob at max | frames | fps | avg ms | p50 ms | p99 ms | max ms | vs baseline |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| baseline | 76 | 37.8 | 26.45 | 26.38 | 27.17 | 31.89 | 1.00x |
| ZOOM=1.5 | 33 | 16.1 | 62.06 | 60.82 | 78.63 | 81.91 | 2.35x |
| WIDTH=0.28 | 40 | 19.7 | 50.71 | 50.37 | 53.03 | 53.75 | 1.92x |
| ORBITS=3 | 51 | 25.4 | 39.42 | 36.76 | 57.55 | 116.01 | 1.49x |
| TWIST=4 | 58 | 28.5 | 35.08 | 32.90 | 55.19 | 86.91 | 1.33x |
| LATTICE=1 | 58 | 28.7 | 34.87 | 33.98 | 46.46 | 48.76 | 1.32x |
| LOBES=7 | 63 | 31.0 | 32.23 | 31.72 | 34.99 | 52.56 | 1.22x |
| SPEED=2 | 72 | 35.1 | 28.50 | 26.78 | 39.59 | 100.64 | 1.08x |

worst: ZOOM=1.5

## hotspots at ZOOM=1.5: 41 frames, 20.3 fps

no measure_layer timers fired for astra-opus-1-chronofold; wrap its painters in crate::_0_profile::measure_layer

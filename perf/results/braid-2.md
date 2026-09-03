# knob sweep: braid-2 2000x1000, 1s per run, dt 0.06, theme moss

| knob at max | frames | fps | avg ms | p50 ms | p99 ms | max ms | vs baseline |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| baseline | 608 | 607.9 | 1.65 | 1.63 | 1.86 | 1.97 | 1.00x |
| STRANDS=16 | 399 | 398.1 | 2.51 | 2.48 | 3.00 | 3.12 | 1.53x |
| WIDTH=9 | 545 | 544.9 | 1.84 | 1.82 | 1.95 | 2.12 | 1.12x |
| FILL=1 | 601 | 600.6 | 1.67 | 1.65 | 1.81 | 2.04 | 1.01x |
| GAP=12 | 606 | 605.6 | 1.65 | 1.63 | 1.87 | 2.38 | 1.00x |
| TWIST=200 | 608 | 607.1 | 1.65 | 1.63 | 1.78 | 2.18 | 1.00x |
| PULSE=40 | 608 | 607.3 | 1.65 | 1.63 | 1.80 | 1.99 | 1.00x |
| SPEED=30 | 610 | 609.1 | 1.64 | 1.62 | 1.85 | 1.99 | 1.00x |
| SLIP=1 | 610 | 609.6 | 1.64 | 1.62 | 1.86 | 1.91 | 1.00x |
| CROSS=1 | 615 | 614.8 | 1.63 | 1.61 | 1.75 | 2.14 | 0.99x |
| TRAIL=40 | 619 | 618.9 | 1.62 | 1.60 | 1.80 | 2.76 | 0.98x |
| PITCH=60 | 621 | 620.8 | 1.61 | 1.59 | 1.74 | 2.91 | 0.98x |
| BEADS=200 | 632 | 631.9 | 1.58 | 1.56 | 1.76 | 3.94 | 0.96x |

worst: STRANDS=16

## hotspots at STRANDS=16: 401 frames, 400.6 fps

no measure_layer timers fired for braid-2; wrap its painters in crate::_0_profile::measure_layer

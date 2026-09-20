# knob sweep: isfahan 200x60, 3s per run, dt 0.06, theme mitla

| knob at max | frames | fps | avg ms | p50 ms | p99 ms | max ms | vs baseline |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| baseline | 24754 | 8251.3 | 0.12 | 0.12 | 0.15 | 0.76 | 1.00x |
| WEAVE=1 | 20724 | 6907.8 | 0.14 | 0.14 | 0.33 | 2.25 | 1.19x |
| TENSION=1 | 22642 | 7547.0 | 0.13 | 0.13 | 0.17 | 0.75 | 1.09x |
| GROWTH=1 | 22673 | 7557.3 | 0.13 | 0.13 | 0.18 | 1.13 | 1.09x |
| LUMINA=1 | 23388 | 7795.8 | 0.13 | 0.13 | 0.16 | 0.65 | 1.06x |

worst: WEAVE=1

## hotspots at WEAVE=1: 21350 frames, 7116.5 fps

| layer | calls/frame | avg us | max us | share of frame |
| --- | ---: | ---: | ---: | ---: |
| night | 1.0 | 45.9 | 2087.0 | 32.7% |
| portal | 1.0 | 33.1 | 1866.6 | 23.6% |
| straps | 1.0 | 19.0 | 732.3 | 13.5% |
| knots | 1.0 | 16.6 | 2296.5 | 11.8% |
| crown | 1.0 | 16.1 | 302.3 | 11.4% |
| field | 1.0 | 1.6 | 63.5 | 1.1% |
| scaffold | 1.0 | 1.1 | 20.5 | 0.8% |

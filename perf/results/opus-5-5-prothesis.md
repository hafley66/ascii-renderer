# knob sweep: opus-5-5-prothesis 2000x1000, 2s per run, dt 0.06, theme moss

| knob at max | frames | fps | avg ms | p50 ms | p99 ms | max ms | vs baseline |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| baseline | 2180 | 1089.8 | 0.92 | 0.91 | 1.30 | 2.13 | 1.00x |
| STATURE=0.8 | 2081 | 1040.0 | 0.96 | 0.95 | 1.29 | 4.63 | 1.05x |
| MOURNERS=7 | 2137 | 1068.5 | 0.94 | 0.92 | 1.15 | 7.18 | 1.02x |
| BIER=0.9 | 2161 | 1080.3 | 0.93 | 0.93 | 1.04 | 1.63 | 1.01x |
| VIGIL=3600 | 2172 | 1085.9 | 0.92 | 0.91 | 1.11 | 4.95 | 1.00x |
| NIGHT=1 | 2205 | 1102.5 | 0.91 | 0.90 | 1.01 | 1.52 | 0.99x |

worst: STATURE=0.8

## hotspots at STATURE=0.8: 2186 frames, 1092.5 fps

| layer | calls/frame | avg us | max us | share of frame |
| --- | ---: | ---: | ---: | ---: |
| body | 1.0 | 3.8 | 344.0 | 0.4% |
| bier | 1.0 | 2.1 | 10.7 | 0.2% |
| mourners | 1.0 | 0.4 | 4.1 | 0.0% |
| lamp | 1.0 | 0.3 | 5.5 | 0.0% |
| knobs | 1.0 | 0.2 | 3.4 | 0.0% |
| inscription | 1.0 | 0.0 | 0.5 | 0.0% |

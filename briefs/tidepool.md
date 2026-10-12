# tidepool

Rocky shore at low tide. Caustics shimmer across the shallows, kelp sways up
from the waterline, sea stars breathe on the speckled wet sand, foam bobs on
the tide edge. All per-element phases come from a splitmix hash of
(seed, layer, index), so random seeds and random knobbing keep the scene
pleasantly variable without touching the rng stream.

Knobs: TIDE SHIMMER STARS KELP SWAY HUE FOAM PEBBLES CHOP (9).

## Layer coverage (perf/layer_coverage.sh 400 120 3 reef tidepool)

| mode | layers | calls/frame | attributed | nested | thin |
| --- | ---: | ---: | ---: | --- | --- |
| tidepool | 6 | 6.0 | 95.0% | no | no |

1 modes reported: 0 thin under 85 percent, 0 nested over 100 percent, 0 with no timers at all

## Knob sweep (perf/knob_sweep.sh tidepool 2000 1000 2)

| knob at max | frames | fps | avg ms | p50 ms | p99 ms | max ms | vs baseline |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| baseline | 198 | 98.6 | 10.14 | 9.97 | 12.36 | 14.43 | 1.00x |
| TIDE=0.95 | 184 | 91.5 | 10.93 | 10.78 | 12.21 | 12.68 | 1.08x |
| SWAY=2 | 189 | 94.5 | 10.59 | 10.30 | 12.77 | 14.34 | 1.04x |
| HUE=180 | 194 | 96.7 | 10.34 | 10.13 | 11.69 | 16.97 | 1.02x |
| CHOP=2 | 196 | 97.8 | 10.23 | 10.00 | 11.42 | 29.00 | 1.01x |
| KELP=1 | 196 | 97.9 | 10.22 | 9.97 | 11.55 | 32.99 | 1.01x |
| STARS=8 | 197 | 98.5 | 10.15 | 9.99 | 11.91 | 15.20 | 1.00x |
| PEBBLES=1 | 198 | 98.9 | 10.11 | 10.01 | 12.16 | 12.56 | 1.00x |
| FOAM=1.5 | 199 | 99.3 | 10.07 | 10.00 | 11.07 | 11.53 | 0.99x |
| SHIMMER=3 | 200 | 99.8 | 10.02 | 9.98 | 11.00 | 11.15 | 0.99x |

worst: TIDE=0.95

### hotspots at TIDE=0.95: 181 frames, 90.5 fps

| layer | calls/frame | avg us | max us | share of frame |
| --- | ---: | ---: | ---: | ---: |
| sea | 1.0 | 7408.1 | 9285.7 | 67.0% |
| sand | 1.0 | 1460.0 | 2004.6 | 13.2% |
| foam | 1.0 | 1214.7 | 1652.9 | 11.0% |
| kelp | 1.0 | 14.5 | 55.7 | 0.1% |
| stars | 1.0 | 0.8 | 6.2 | 0.0% |
| critters | 1.0 | 0.3 | 0.7 | 0.0% |

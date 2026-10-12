# lagoon

Moonlit lagoon: crescent moon and twinkling stars over dark rippling water,
raindrop rings spreading on independent phases, the moon glare widening toward
the shore, reeds swaying at the bank, lotus pads drifting with pulsing flowers.

Knobs: SKY (sky rows fraction) DROPS RIPPLE MOON GLARE SHIMMER REEDS SWAY HUE DRIFT.

## knob_sweep (perf/knob_sweep.sh lagoon 2000 1000 2)

| knob at max | frames | fps | avg ms | p50 ms | p99 ms | max ms | vs baseline |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| baseline | 475 | 237.3 | 4.21 | 4.15 | 4.68 | 14.48 | 1.00x |
| HUE=360 | 469 | 234.3 | 4.27 | 4.14 | 6.47 | 15.07 | 1.01x |
| MOON=0.9 | 473 | 236.3 | 4.23 | 4.14 | 5.89 | 18.14 | 1.00x |
| RIPPLE=2 | 478 | 239.0 | 4.18 | 4.15 | 4.78 | 7.49 | 0.99x |
| SWAY=2 | 480 | 239.7 | 4.17 | 4.14 | 4.94 | 5.65 | 0.99x |
| SHIMMER=2 | 480 | 239.8 | 4.17 | 4.14 | 4.93 | 5.14 | 0.99x |
| DRIFT=1 | 483 | 241.0 | 4.15 | 4.13 | 4.50 | 7.60 | 0.98x |
| DROPS=1 | 483 | 241.4 | 4.14 | 4.13 | 4.51 | 5.16 | 0.98x |
| GLARE=2 | 484 | 241.6 | 4.14 | 4.13 | 4.50 | 4.58 | 0.98x |
| REEDS=1 | 484 | 241.7 | 4.14 | 4.13 | 4.44 | 4.63 | 0.98x |
| SKY=0.5 | 544 | 271.7 | 3.68 | 3.64 | 4.27 | 6.17 | 0.87x |

worst: HUE=360

## hotspots at HUE=360: 481 frames, 240.1 fps

| layer | calls/frame | avg us | max us | share of frame |
| --- | ---: | ---: | ---: | ---: |
| water | 1.0 | 2719.2 | 3448.8 | 65.3% |
| sky | 1.0 | 491.0 | 697.1 | 11.8% |
| ripples | 1.0 | 3.0 | 24.8 | 0.1% |
| reeds | 1.0 | 0.3 | 1.5 | 0.0% |
| lotus | 1.0 | 0.1 | 0.7 | 0.0% |

## layer coverage (perf/layer_coverage.sh 400 120 3 moss lagoon)

| mode | layers | calls/frame | attributed | nested | thin |
| --- | ---: | ---: | ---: | --- | --- |
| lagoon | 5 | 5.0 | 88.2% | no | no |

1 modes reported: 0 thin under 85 percent, 0 nested over 100 percent, 0 with no timers at all

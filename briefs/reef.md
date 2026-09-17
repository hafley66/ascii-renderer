# reef

Subject: a coral reef seen through a water column. Several colonies are grown by
diffusion-limited aggregation off the seabed, each in its own hue, under a depth
gradient lit by sun shafts and a caustic net, with bubbles rising off the colony
crowns and small fish drifting across.

## Growth

Each colony owns an ellipse of half-axes `ax` (columns) and `ay` (rows), placed on
an undulating seabed and sized from its slot in the reef band. A seed arc is laid
along the floor inside the ellipse, then walkers launch on the upper rim and random
walk with a horizontal step preference, so the aggregate spreads the way a terminal
cell is shaped. A walker sticks by coordination number: one kin neighbour sticks at
`STICK^2 * 0.5`, two at `STICK`, three or more always. That single rule is what turns
a wispy dendrite into a solid coral head with lobes.

Walkers die on leaving 1.3 ellipse radii, and each colony stops at its cell cap, so
the cost is bounded by the pane rather than by the knob. Foreign colonies block both
movement and sticking, which leaves natural water channels between heads.

Glyph weight is the subtree support count of each cell plus an interior bonus
`(kin - 3)^2 * 3`, so trunks read heavy and surface polyps read fine. Lone
protruding tips are pruned at 70 percent to keep the mass from looking like dust.

## Glyph families

1. Coral by branch thickness: `.`, `:`, `o`, `O`, `@`, `#`, `%`, `&`
2. Caustic net and lit surface band: `-`, `~`, `=`
3. Seabed sand and ripples: `.`, `,`, `_`, `:`, `` ` ``
4. Plankton motes in open water: `.`, `` ` ``
5. Bubbles: `o`, `.`
6. Fish: `><>`, `<><`, `><(((>`, `<)))><`

## Motion

Everything is a pure function of `t`, and the grown bed is cached on
`(seed, w, h, knobs)` so a frame never regrows the aggregate. Bubbles rise on a
per-bubble phase and wobble, fish drift and bob, caustics and sun shafts scroll,
and coral tips pulse in lightness by `tip * sin(t * 1.55 + colony_phase)`. Every
per-bubble and per-fish phase comes from `hash(seed, layer, index, salt)`, never
from the main rng.

## Knobs

| key | label | default | range |
| --- | --- | ---: | --- |
| COLONIES | coral colonies per 80 columns | 4 | 1 .. 12 |
| WALKERS | dla walker budget | 1400 | 40 .. 12000 |
| STICK | stickiness | 0.5 | 0.02 .. 1 |
| DEPTH | water column depth | 0.86 | 0.35 .. 0.98 |
| BUBBLES | bubble density | 0.7 | 0 .. 3 |
| FISH | fish per 80 columns | 7 | 0 .. 40 |
| SPREAD | colony spread | 0.86 | 0.2 .. 1 |
| CAUST | caustics and shafts | 0.7 | 0 .. 1.5 |
| GROW | colony size | 1.0 | 0.1 .. 1.5 |
| HUE | hue rotation deg | 0 | -180 .. 180 |
| SPEED | time scale | 1.0 | 0 .. 3 |

Positional order: `reef [colonies] [walkers] [stickiness] [depth] [bubbles] [fish]`.

Colony hues cycle coral pink 350, orange 25, violet 275, teal 175, each with a
+-8 degree per-colony jitter and a +37 degree step once the cycle wraps.

## Knob sweep, 2000x1000, 2 s per run, dt 0.06, theme moss, release

| knob at max | frames | fps | avg ms | p50 ms | p99 ms | max ms | vs baseline |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| baseline | 200 | 99.6 | 10.04 | 9.98 | 10.61 | 10.71 | 1.00x |
| CAUST=1.5 | 186 | 92.9 | 10.77 | 10.79 | 11.57 | 11.85 | 1.07x |
| HUE=180 | 186 | 93.0 | 10.76 | 10.70 | 11.43 | 11.58 | 1.07x |
| GROW=1.5 | 187 | 93.1 | 10.74 | 10.68 | 11.55 | 12.04 | 1.07x |
| SPEED=3 | 187 | 93.3 | 10.72 | 10.65 | 11.42 | 11.53 | 1.07x |
| BUBBLES=3 | 195 | 97.1 | 10.29 | 10.24 | 11.16 | 11.29 | 1.03x |
| FISH=40 | 197 | 98.5 | 10.16 | 10.10 | 10.90 | 11.13 | 1.01x |
| COLONIES=12 | 198 | 98.7 | 10.13 | 10.06 | 10.90 | 11.14 | 1.01x |
| STICK=1 | 198 | 98.9 | 10.11 | 10.04 | 10.90 | 11.14 | 1.01x |
| SPREAD=1 | 199 | 99.0 | 10.10 | 10.03 | 11.10 | 11.33 | 1.01x |
| WALKERS=12000 | 199 | 99.0 | 10.10 | 10.04 | 10.89 | 11.24 | 1.01x |
| DEPTH=0.98 | 213 | 106.2 | 9.42 | 9.36 | 10.02 | 10.32 | 0.94x |

Worst knob: `CAUST=1.5`.

## Hotspots at CAUST=1.5, 176 frames, 87.9 fps

| layer | calls/frame | avg us | max us | share of frame |
| --- | ---: | ---: | ---: | ---: |
| caustics | 1.0 | 5517.3 | 12456.1 | 48.5% |
| water | 1.0 | 4535.6 | 4746.5 | 39.9% |
| bubbles | 1.0 | 162.1 | 645.2 | 1.4% |
| coral_paint | 1.0 | 95.9 | 333.5 | 0.8% |
| fish | 1.0 | 91.7 | 284.7 | 0.8% |
| dla_grow | 1.0 | 0.0 | 0.1 | 0.0% |

`dla_grow` reads as zero because the bed is cached after the first frame; the whole
cost of growth is paid once per `(seed, w, h, knobs)`. Layer coverage at 400x120 is
6 layers, 90.9 percent attributed, neither thin nor nested.

Whole-process render, release: 80x24 p50 7.2 ms, 400x120 p50 13.3 ms.

## Render commands

```bash
ASCII_GRID_W=100 ASCII_GRID_H=32 ./target/release/ascii-renderer 42 reef deep | sed 's/\x1b\[[0-9;]*m//g'
ASCII_GRID_W=100 ASCII_GRID_H=32 ASCII_T=9 ./target/release/ascii-renderer 7 reef moss | sed 's/\x1b\[[0-9;]*m//g'
perf/layer_coverage.sh 400 120 3 moss reef
perf/knob_sweep.sh reef 2000 1000 2
```

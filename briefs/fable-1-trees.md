# fable-1-trees

Subject: a two-row sample sheet of five tree-thing growth algorithms written for this lane, one column per species, full energy on top and scrub energy below, each tree standing on a dotted ground line with its own roots or flare.

What moves and why: the sheet is grown once per (size, seed, knobs, palette) into a cached page. Each frame copies the page with a slow per-column lean (SWAY, two summed sines with periods of about 18 s and 7 s, stronger toward the top of each tree) and flips a few canopy glyphs between their light and dark forms (FLICKER hashed per cell per step). Both effects are gated on `t > 0`, so t=0 is the static page.

## Species

1. `colonize`, space colonization. Attractors: N points sampled uniformly in an ellipse above a short trunk (N = half * crown_h * 0.5 * (0.5 + BRANCH), clamped 12..160), with one side thinned by a seeded bias so the crown is lopsided. Loop up to 120 iterations: every attractor finds its nearest node in metric space (dx/2, dy); attractors within 1.5 are absorbed; attractors within `infl = 4 + 4*BRANCH + 0.08*crown_h` push their node by a unit vector; every pushed node grows one child one row away along the averaged direction plus a jitter of +-0.25; growth stops when no node moved. Segments are drawn parent-to-child with the slope glyph (`┃` for the main stem where subtree share is above 30 percent, `│ ╱ ╲ ─` elsewhere); nodes without children get a `◦` tip and a `▒ ░` cluster; thin twigs get side leaves.
2. `banyan`, turtle walk with aerial roots. Trunk: `┃` up to half the height, wobbling one column every third row with probability 0.35, doubled with `│` on both sides when the plot is wide. Limbs: 2 + 2.5*BRANCH walks that alternate sides, each step `─` or a `╱ ╲` rise with probability 0.12..0.37. Every third limb cell drops an aerial root with probability 0.6*ROOTS: `┊` straight down, drifting one column with probability 0.12, 65 percent of roots reach the ground (`│` for the lower third, `┴` foot), the rest dangle and end in `╷`. Canopy: for every limb cell, 1..4 rows above are filled with `▒ ░ ◦` at density LEAF * (1 - 0.28*row), plus `∙` underhang.
3. `mangrove`, prop-root arch plus dome. Stilts: 3 + 4*ROOTS roots leave a hub 30 percent of the height up the trunk; each root alternates `─` and `╱ ╲` steps outward until it reaches its seeded reach, then falls straight to the ground as `│` and plants a `┴`. Trunk: `┃` with one seeded lean step. Crown: an ellipse (rx = spread, ry = 0.22*h) sitting on the trunk top; 2 + 3*BRANCH inner strokes from the trunk top to random points inside, then leaves filled where `rng < LEAF * (1.05 - 0.75*nd)` (nd = normalized radius squared): `▒` core, `░` middle, `◦ ∙` rim. Propagules hang from the dome bottom as `╎` over `•` with probability 0.35*FRUIT.
4. `baobab`, bottle trunk plus recursive twigs. Trunk rows i = 0..0.62*h: half-width `hw = base_hw * (1 - bulge * f^2 * 0.8)` with `base_hw = min(0.45*spread, 0.2*h)`, edges `│` or `╱ ╲` when the row narrows, interior bark stripes `░ ▒` with `▓` knots at probability 0.06, and a `╱ ╲` flare on the ground row. Crown: 3 + 4*BRANCH twigs start along the trunk top with a random slope in -1.6..1.6 columns per row; `twig(len, depth)` walks `len` rows then, while depth < 2, splits into one or two children of length 0.5..0.75 * len with slopes offset by 0.5..1.5. Tips get `╵`, sparse `∙` leaves at 0.35*LEAF, and `○` pods at 0.6*FRUIT.
5. `coral`, diffusion-limited aggregation. A `┃` stem of 1..5 rows seeds the aggregate. Walkers (ellipse area * 1.1 * (0.6 + 0.4*LEAF), clamped 24..260) spawn uniformly inside an ellipse above the stem and random-walk (dx in {-1,0,1}, dy biased downward, an 0.18 pull toward the ellipse center) for up to 300 steps. A walker touching the aggregate on a 4-neighbor sticks with probability `0.5 + 0.45*BRANCH` when it touches exactly one cell, 0.2 with two, 0.1 with more, taking the lowest neighbor as parent. Cells draw as `│` or `─` toward their parent, colored by generation quartile from trunk to leaf_lit; childless cells are `•` or `◦` polyps, `✶` fruit at 0.6*FRUIT.

## Glyph families

1. Trunk and stems: `┃ │ ╱ ╲ ─`
2. Roots and feet: `┊ ╎ ┴ ╷ ╌`
3. Canopy: `▒ ░ ◦ ∙`
4. Bark and knots: `░ ▒ ▓`
5. Tips and fruit: `╵ • ● ○ ✶`

## Knobs

- ENERGY: tree height fraction of the plot (default 0.9)
- FRUIT: fruit probability (default 0.25)
- BRANCH: branching factor (default 0.7)
- LEAF: canopy density (default 0.9)
- ROOTS: aerial and prop root rate (default 0.7)
- SCRUB: second row energy factor (default 0.55)
- FLICKER: leaf flicker steps per second (default 1)
- SWAY: column lean amplitude in cells (default 1)

Positional order: energy fruit branch leaf roots scrub flicker sway.

## Render commands

```bash
ASCII_GRID_W=110 ASCII_GRID_H=36 ./target/release/ascii-renderer 7 fable-1-trees moss | sed 's/\x1b\[[0-9;]*m//g'
ASCII_GRID_W=110 ASCII_GRID_H=36 ASCII_T=9 ./target/release/ascii-renderer 7 fable-1-trees moss | sed 's/\x1b\[[0-9;]*m//g'
```

## Perf receipt

`perf/knob_sweep.sh fable-1-trees 2000 1000 2`, release, Apple Silicon. Every knob at max stays above 140 fps. The two per-frame layers cover 84 percent of the frame; the rest is the renderer's own back-buffer clear, which runs before the draw fn and cannot be timed from inside it.

| knob at max | frames | fps | avg ms | p50 ms | p99 ms | max ms | vs baseline |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| baseline | 341 | 170.4 | 5.87 | 5.84 | 6.34 | 6.89 | 1.00x |
| SWAY=3 | 292 | 145.5 | 6.87 | 6.13 | 11.99 | 15.38 | 1.17x |
| FLICKER=4 | 306 | 152.7 | 6.55 | 6.15 | 10.63 | 12.18 | 1.12x |
| ROOTS=1 | 324 | 161.9 | 6.18 | 6.08 | 7.07 | 7.49 | 1.05x |
| SCRUB=0.9 | 331 | 165.3 | 6.05 | 5.99 | 6.68 | 14.60 | 1.03x |
| ENERGY=1 | 337 | 168.1 | 5.95 | 5.88 | 6.45 | 11.90 | 1.01x |
| FRUIT=1 | 340 | 169.9 | 5.88 | 5.85 | 6.33 | 6.44 | 1.00x |
| BRANCH=1.5 | 341 | 170.3 | 5.87 | 5.85 | 6.24 | 6.38 | 1.00x |
| LEAF=1.5 | 343 | 171.2 | 5.84 | 5.83 | 6.28 | 6.75 | 1.00x |

worst: SWAY=3

## hotspots at SWAY=3: 292 frames, 146.0 fps

| layer | calls/frame | avg us | max us | share of frame |
| --- | ---: | ---: | ---: | ---: |
| sway | 1.0 | 3286.6 | 9353.3 | 48.0% |
| flicker | 1.0 | 2453.0 | 6298.3 | 35.8% |

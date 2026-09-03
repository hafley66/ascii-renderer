# fable-2-trees

Subject: a sample sheet of five tree species grown by five different algorithms, one column each, full-energy specimens on the top row and scrub-energy specimens below, every tree standing on a ground line with its own roots underneath.

What moves and why: the sheet is grown once per (size, seed, palette, growth knobs) and cached as sprites. With `t > 0` every sprite shears sideways by SWAY times a per-tree sine on a 24 second cycle (rows above the root move by height squared, roots stay put), and leaf glyphs flicker between paired glyphs at FLICKER density, three ticks per second. At `t == 0` the sheet is byte-identical to the static frame.

## Species

1. Colonizer, space colonization. Trunk: wobble walk of `0.26..0.38 * height` rows. A skewed ellipse above the trunk holds `DETAIL * 90` attractor points in visual units (one column = half a row). Iterate up to 70 times: every live attractor within influence radius `0.4 * crown_h` votes a unit vector toward its nearest node; every node with votes grows a child one step (`crown_h / 22`) along the summed vector plus a small upward bias; attractors within kill radius `0.08 * crown_h` of a new node die. Segments are drawn parent to child with `│ ╱ ╲ ─`, thick (`┃`) when a subtree holds over 30 percent of the nodes, color lightening with depth. Leaf blobs sit on childless nodes; dead attractors leave a scatter of `·`.

2. Banyan, limbs with prop roots. Thick trunk (`height / 9` columns, `┃` core and `│` bark). Three to five limbs leave the upper third of the trunk and walk sideways: each step is `─`, or with probability `0.16 + 0.12 * depth` a rise step (`╱` or `╲`); a sub-limb forks with probability `0.10 * BRANCH` per step past the first third, up to depth 2. Every `max(3, half / 4)`th limb cell drops a prop root with probability `0.8 * ROOTS`: 60 percent reach the ground and get a `╱│╲` flare, the rest hang with a `╷` tip. A leaf dome rides each limb cell, height from a two-sine noise of x, density `0.95 * (1 - f)^0.4 + 0.15` by row.

3. Mangrove, stilt roots plus turtle branches. Four to seven roots leave a base point `0.26 * height` above the ground and follow `x = reach * (1 - (1 - u)^2)` downward, so they arch outward first and land vertical, continuing below the ground row into the mud. A thin wobbling trunk rises from the base; `2 + 3 * BRANCH` turtle branches leave the upper 45 percent at 25 to 75 degrees from vertical, jitter by 0.3 radians per step, and fork with probability `0.15 * BRANCH` (depth 2, length 0.55). Tips carry elongated `◆ ◇ ∙` tufts; propagules `╷ •` hang with probability FRUIT.

4. Coral, linked diffusion-limited aggregation. A lattice of `crown_h / s` by `2 * half / s` cells (`s = crown_h / 30`, at least 1) seeds one cell at the trunk top. Particles launch on an arc `maxr + 3` above the seed and random-walk (30 percent left, 30 right, 10 to 25 down, rest up, plus a seeded side pull); the first occupied 8-neighbor becomes the particle's parent. Up to `DETAIL * 150` particles or 30 percent of the lattice. Each cell is drawn as a segment from its parent, so the aggregate reads as a dendrite; childless cells get a leaf tuft or a `○` fruit.

5. Sunburst, phyllotactic crown. Trunk of `0.36..0.48 * height`, then a neck to a crown center of radius `R = min((height - trunk) / 2, half / 2)`. `3 + 3 * BRANCH` spokes leave the center at random angles over the upper 300 degrees, with a 60 percent chance of a mid-spoke fork. `5 R^2` leaves at `r = R sqrt(i / N)`, `theta = i * 137.508 deg + phase`, radius scaled by `1 + skew * 0.25 * cos(theta - skew_angle)` for asymmetry; glyph by radial band `● ◆ • ∙ ·`, outer band swapping to `○` fruit with probability `0.4 * FRUIT`.

Every species ends with a root fan below the ground row: two to four roots stepping down and outward with `│ ╱ ╲ ─` and a `·` tip, plus a `╱│╲` flare on the row itself.

## Glyph families

1. Trunk and bark: `│ ┃ ╱ ╲`
2. Limbs, spokes, segments: `─ ╱ ╲ │ ╶ ╴`
3. Leaves: `● • ∙ ·` (colonizer, banyan, coral, sunburst), `◆ ◇ ∙` (mangrove)
4. Fruit and hanging parts: `● • ○ ◦ ╷`
5. Roots and ground: `╱ ╲ │ ─ · ─`

## Knobs

- ENERGY: tree size as a fraction of the plot (default 0.9, scrub row uses 0.6 of it)
- FRUIT: fruit chance per tip (default 0.25)
- BRANCH: branching factor for limbs, turtle forks and spokes (default 1.0)
- SWAY: sway amplitude in cells per 12 rows of height (default 0.5)
- SPEED: clock multiplier for sway and flicker (default 1.0)
- FLICKER: fraction of leaf cells that swap glyph per tick (default 0.5)
- DETAIL: attractor and particle budget scale for colonizer and coral (default 1.0)
- ROOTS: root reach, root depth and prop root rate (default 1.0)

Positional order: energy fruit branch sway speed flicker detail roots.

Layout: five columns of `width / 5`, two rows of `max(12, height / 2)`; each cell holds a label row, `cell_h / 8` root rows, a ground line and the plot above it. Grids shorter than the sheet clip the bottom row.

## Render commands

```bash
ASCII_GRID_W=110 ASCII_GRID_H=36 ./target/release/ascii-renderer 7 fable-2-trees moss | sed 's/\x1b\[[0-9;]*m//g'
ASCII_GRID_W=110 ASCII_GRID_H=36 ASCII_T=12 ./target/release/ascii-renderer 7 fable-2-trees moss | sed 's/\x1b\[[0-9;]*m//g'
```

## Perf receipt

`perf/knob_sweep.sh fable-2-trees 2000 1000 1`, theme moss:

| knob at max | frames | fps | avg ms | p50 ms | p99 ms | max ms | vs baseline |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| baseline | 323 | 322.1 | 3.10 | 3.00 | 4.61 | 10.09 | 1.00x |
| FLICKER=1 | 258 | 257.2 | 3.89 | 3.74 | 5.53 | 7.30 | 1.25x |
| ENERGY=1.2 | 293 | 292.6 | 3.42 | 3.37 | 3.85 | 4.50 | 1.10x |
| ROOTS=1 | 306 | 305.7 | 3.27 | 3.15 | 4.86 | 6.69 | 1.05x |
| BRANCH=1.5 | 309 | 308.0 | 3.25 | 3.25 | 3.49 | 3.60 | 1.05x |
| DETAIL=2 | 316 | 315.9 | 3.17 | 3.17 | 3.45 | 3.66 | 1.02x |
| SPEED=3 | 322 | 321.6 | 3.11 | 3.09 | 3.48 | 4.05 | 1.00x |
| FRUIT=1 | 322 | 321.6 | 3.11 | 2.97 | 4.62 | 6.14 | 1.00x |
| SWAY=2 | 325 | 324.2 | 3.08 | 3.06 | 3.37 | 3.52 | 0.99x |

Hotspots at FLICKER=1, 252 fps:

| layer | calls/frame | avg us | max us | share of frame |
| --- | ---: | ---: | ---: | ---: |
| trees | 1.0 | 1864.5 | 4100.2 | 47.0% |
| clear | 1.0 | 1060.1 | 6951.0 | 26.7% |
| ground | 1.0 | 4.2 | 9.6 | 0.1% |

The remaining quarter of the frame is the sweep harness clearing its own 2000x1000 grid before the draw fn runs; the frame is light enough (3 ms) that this fixed cost shows. The grow layer only fires on a cache miss.

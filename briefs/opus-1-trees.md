# opus-1-trees

Subject: a nursery sample sheet for the five opus-1 tree species, one column each,
full-energy specimen on the top row and a scrub-energy specimen below, every trunk
rooted on a ground line with its own root flare and labelled underneath.

What moves and why: the sheet is a frozen bake. Each of the ten specimens carries a
seeded oscillator (period 7 to 16 s) and each baked cell carries a height weight, so
when `t > 0` the crown leans and the trunk base stays planted. A rotating window over
the baked leaf cells brightens one seventh of the foliage per step, which reads as
leaf flicker. At `t == 0` every offset is zero and the frame equals the static bake
byte for byte.

## the five growth algorithms

**venation, space colonization (Runions).** Sample N attractor points inside a lobed
ellipse over the crown, N = crown area * 2.8 * DETAIL, clamped to 45..480. Seed the
node list with a vertical chain from the trunk top to the crown center. Each round:
every live attractor finds its nearest node within influence radius `ri = 5.5 * step`
using a uniform bucket grid, and contributes a unit vector; every node with at least
one contributor emits a child at `normalize(sum + up_bias + jitter) * step`; then any
attractor within kill radius `rk = 1.5 * step` of a node dies. `step` scales with the
crown radius (0.85 to 2.4 cells) so the venation is equally fine at any plot size.
Stop at 640 nodes, 90 rounds, or no growth. A reverse pass accumulates subtree mass
per node; mass picks the glyph weight, so thick near the root and hairline at the
tips. Nodes of mass <= 5 stamp a 5x3 foliage patch through a hash gate, which turns
the skeleton into a canopy without hiding it. Absent from the existing species files.

**mangrove, prop-root arches.** The trunk starts in the air at `ry - stilt`, stilt
being 20 to 34 percent of tree height. Three to six prop roots leave the trunk at
varied heights and follow a quarter-ellipse `x = ax + reach*sin(a)`,
`y = ay + drop*(1 - cos(a))` for `a` in `[0, pi/2]`, so each root leaves horizontally
and lands vertically on a `┴` foot; reach is signed by alternating side and scaled by
a per-root random, which makes the stance lopsided. The crown is a wide low dome:
every cell inside a lobe-modulated ellipse gets a density glyph by normalized radius,
with a hash gate that frays the rim. Limbs fan over the finished foliage, and
propagules hang below the crown as `│` runs ending in `◆`. Absent from the existing
species files.

**phyllotaxis, golden-angle ray crown.** A short S-curved column ends at a hub. For
`i` in `0..n`, `theta = spin + i * 2.3999632` rad and `r = sqrt((i + 0.5) / n)`, the
Vogel construction, which fills the crown at uniform areal density. The point is
placed on an ellipse whose radius is multiplied by the seeded three-term lobe, so the
crown is never round. Every `n / rays`-th index also draws a limb from hub to point,
which makes the ray fan visible under the beads. Glyph steps by radius: `▒` core,
`░`, `∙`, `◦`, `◇` rim, so the crown reads solid at the middle and beaded at the edge.

**aggregate, diffusion-limited aggregation.** A local occupancy bitmap covers the
plot; the trunk column seeds it. Walkers, count = crown area * 2.2 * DETAIL clamped to
25..480, spawn on an ellipse at 1.18x the crown radius and random-walk with a weak
drift toward the crown center (0.22 + 0.28 * BRANCH per step, x scaled by the cell
aspect). A walker that would land 8-adjacent to an occupied cell sticks there and
records its stick order. The weak drift is what makes the cluster dendritic instead of
a filled blob. Glyph comes from the occupied-neighbour count (1 `·` up to 7 `▓`) and
tone from stick order, so the interior reads old and dense and the rim reads new and
bright. The trunk is repainted over the cluster at the end. Absent from the existing
species files.

**shelf, recursive subdivision.** Split the canopy rect along its longer axis (aspect
corrected) at a random 0.32 to 0.68 fraction, recursing with keep probability
`BRANCH * (1 - depth / (max_depth + 1)) + 0.20`, so the split tree is asymmetric and
terminates early on one side. Leaf rects whose center falls outside a lobed crown
ellipse are dropped, which gives the stack a tree silhouette instead of a wall. Each
surviving rect draws a shelf bar on its bottom row (`╰ ─ ┬ ╴ ╯`), one to three
foliage rows above it inset upward, and an L-limb back to the trunk ending in a
`├`/`┤` knot. Shelves paint top-down so lower ones overlap. The trunk is repainted
over the lower shelves at the end.

**root flares** (shared, four styles cycled by ROOTS + column): buttress diagonals,
knuckle arcs `╰──┴──╯`, a two-row mound, and a surface root mat.

## glyph families

1. Trunk and heavy limb: `┃ │ ┼ ├ ┤`
2. Limb and shelf: `─ ╱ ╲ ╭ ╮ ╰ ╯ ┬ ┴ ╴ ╶`
3. Foliage mass: `▓ ▒ ░`
4. Leaf and bead: `◆ ◇ ● ◦ ∙ ·`
5. Ground and roots: `─ ╴ ╶ ┴ ∙ ·`

## knobs

| key | label | range | default |
| --- | --- | --- | ---: |
| ENERGY | top row crown energy | 0.2 .. 1.0 | 0.88 |
| FRUIT | fruit and bloom rate | 0.0 .. 1.0 | 0.18 |
| BRANCH | branch keep probability | 0.05 .. 1.0 | 0.62 |
| SCRUB | bottom row energy factor | 0.2 .. 1.0 | 0.58 |
| ROOTS | root flare style offset | 0 .. 3 | 0 |
| SWAY | sway amplitude in cells | 0.0 .. 4.0 | 0.9 |
| SPEED | time scale | 0.0 .. 3.0 | 1.0 |
| DETAIL | growth sample count factor | 0.2 .. 2.0 | 1.0 |
| HUE | hue rotation deg | -180 .. 180 | 0 |
| BARE | bare trunk fraction of height | 0.08 .. 0.60 | 0.26 |

Positional order: `energy fruit branch scrub roots sway speed detail hue`.

## render commands

```bash
ASCII_GRID_W=110 ASCII_GRID_H=36 ./target/release/ascii-renderer 7 opus-1-trees moss | sed 's/\x1b\[[0-9;]*m//g'
ASCII_GRID_W=110 ASCII_GRID_H=36 ASCII_T=9 ./target/release/ascii-renderer 42 opus-1-trees ember | sed 's/\x1b\[[0-9;]*m//g'
```

## perf receipt

`perf/knob_sweep.sh opus-1-trees 2000 1000 1`, full table in `perf/results/opus-1-trees.md`.

| knob at max | fps | avg ms | vs baseline |
| --- | ---: | ---: | ---: |
| baseline | 480.6 | 2.08 | 1.00x |
| SCRUB=1 | 466.3 | 2.14 | 1.03x |
| ENERGY=1 | 468.7 | 2.13 | 1.03x |
| DETAIL=2 | 479.3 | 2.09 | 1.00x |
| SWAY=4 | 480.9 | 2.08 | 1.00x |
| BARE=0.6 | 497.5 | 2.01 | 0.97x |

Worst knob SCRUB=1 holds 465 fps at 2000x1000, 15x the 30 fps bar.

| layer | avg us | share of frame |
| --- | ---: | ---: |
| clear | 928.2 | 43.2% |
| trees | 273.7 | 12.7% |
| ground | 6.7 | 0.3% |
| flicker | 3.8 | 0.2% |

The four timers cover every painter the mode owns. The residual 44 percent is the
blank-fill that `IterateFrameRenderer::render` performs on the shared grid before it
calls any mode, which no mode can wrap; at 2000x1000 that fixed 0.9 ms is comparable
to this mode's entire frame.

# opus-2-trees

Subject: a sample sheet of five growth algorithms written for this lane, one column per species, full energy on the top band and scrub energy on the bottom band, each tree standing on its own ground line with its own root work.

What moves and why: the sheet is static geometry, grown once and cached as a sparse cell list keyed by (width, height, seed, shape knobs). Every frame blits that list. Two things ride on `t`: a sway shear that shifts each tree by `SWAY * sin(t * w + phase) * up^2` where `up` is the height fraction inside the band, so crowns lean and roots stay planted; and a leaf flicker that swaps one in eleven tip glyphs for a twinkle glyph at `FLICK` hops per second. At `t = 0` both are off and the frame is the static sheet.

## The five algorithms

**Mangrove.** Prop roots are power curves, not strokes. For leg `k` on side `s`, pick `reach` from the plot half width and a bend exponent `p = 1.4 + 1.3 * rand + GNARL`, then walk `u` from 0 to 1 emitting `x = cx + s * reach * u^0.55`, `y = collar + drop * u^p`. The low x exponent throws the root outward fast, the high y exponent holds it near the collar and then drops it steeply, which is the mangrove arch. A collar bar of `─` and `┴` ties the legs to the trunk at 40 percent of tree height, the trunk continues as a wandering turtle walk above it, boughs alternate sides across the top 68 percent of the trunk span, and each bough ends in a leaf mass. Pneumatophore spikes stand in the mud between the feet, count from ROOTS.

**Colony.** Space colonization. Scatter `N = clamp(rx * ry / 1.35, 16, 240)` attractor points inside an ellipse crown, squashed on one seeded side so the crown is lopsided. March a trunk up from the root until an attractor falls inside the influence radius. Then iterate at most 34 times: every attractor finds its nearest node within influence and adds a unit vector to that node's pull; every node with a nonzero pull spawns one child at `node + normalize(pull) * step` with x scaled 1.8 for cell aspect; attractors within the kill radius of any new node are removed. Nodes are capped at 420. Subtree weights are accumulated in reverse index order, so a segment's thickness and color slot follow how much crown hangs off it. The canopy is a weight kernel: each outer node stamps `6 - |dx| - 2|dy|` into a local accumulator, and the accumulator is drawn as `▓ ▒ ░ ·` under the skeleton.

**Banyan.** A wide trunk column climbs to a crotch at 42 percent of height, then two to six boughs run horizontally with a one-in-three rise. Along each bough, with probability `ROOTS * 0.16` per column, an aerial root drops: a vertical walk with a one-in-six lateral jitter that stops early with probability 0.13 once it is past 45 percent of the drop. A root that reaches soil is redrawn as a `┃` pillar with splayed feet; one that stops early ends in a `╵` dangle. The crown above the crotch is a row-profile fill, half width `hw * prof * (0.7 + BRANCH * 0.45)` where `prof` peaks a third of the way up and carries a random-walk wobble, so the silhouette is a lopsided dome.

**Bracket.** The tree-thing that is not a tree. A tapering stipe of `▓` and `▒` runs the full height with a gnarl wander. Shelves step up the stipe at `gap = max(2, 0.12 * height)` intervals, alternating sides and doubling with probability 0.25. Each shelf is a radial half fan: for `dx` from 1 to `r`, thickness is `sqrt(1 - u^2) * lip * thick` where `lip` is a per-column random walk in 0.6 to 1.35, and the whole shelf sags by `u^2 * droop` so the outer edge droops. The underside rim is `◡`, the top edge `─`, and spores fall as `·` below the outer third. A wider scalloped cap closes the top. A mycelium mat of `╌` and `·` spreads on the ground, width from ROOTS.

**Coral.** Diffusion-limited aggregation, bounded. Seed a stalk from the ground. Walkers spawn on the perimeter of an ellipse (`rx = 0.46w`, `ry = 0.62h`) centered above the stalk, then random walk with a 45 percent inward drift, stopping when any of the eight neighbors is occupied. A walker that leaves the ellipse above the stalk line dies. Particle count is `area * 0.5 * (0.45 + BRANCH)` clamped to 60..4200. One morphological closing pass (an empty cell with five or more occupied neighbors fills) closes the interior without eating the dendritic edge. Glyphs then come from local connectivity: `▓ ▒` for dense cells, `│ ─ ┼ ╱ ╲` for the strands, `·` for isolated tips, with `●` polyps at a fraction of the ends.

## Glyph families

1. Trunk and pillar: `┃ │ ╿ ┴ ├ ┤ ┼`
2. Branch and root arc: `─ ╱ ╲ ╴ ╶ ╵ ╷ ╎ ╌`
3. Canopy mass: `▓ ▒ ░ ·`
4. Leaf and tip: `◆ ◇ ∘ ◦ ✦`
5. Fruit and spore: `●`

## Knobs

- ENERGY: top row growth energy (default 0.92)
- FRUIT: fruiting bodies per tip (default 0.3)
- BRANCH: branch and shelf reach (default 0.7)
- GNARL: trunk wander (default 0.35)
- ROOTS: root and mat spread (default 0.6)
- SWAY: sway columns (default 1.0)
- FLICK: leaf flicker hops per second (default 1.0)
- HUE: hue shift degrees (default 0)
- SCRUB: bottom row energy factor (default 0.72)

Positional order: energy fruit branch gnarl roots sway flick hue scrub.

## Render commands

```bash
ASCII_GRID_W=110 ASCII_GRID_H=36 ./target/release/ascii-renderer 7 opus-2-trees moss | sed 's/\x1b\[[0-9;]*m//g'
ASCII_GRID_W=110 ASCII_GRID_H=36 ASCII_T=12 ./target/release/ascii-renderer 7 opus-2-trees moss | sed 's/\x1b\[[0-9;]*m//g'
```

## Perf receipt

`perf/knob_sweep.sh opus-2-trees 2000 1000 1`, full table in `perf/results/opus-2-trees.md`.

| knob at max | fps | avg ms | vs baseline |
| --- | ---: | ---: | ---: |
| baseline | 473.2 | 2.11 | 1.00x |
| BRANCH=1 | 453.7 | 2.20 | 1.04x |
| SCRUB=1 | 468.7 | 2.13 | 1.01x |
| ENERGY=1 | 470.9 | 2.12 | 1.00x |
| SWAY=4 | 483.4 | 2.07 | 0.98x |

Worst knob BRANCH=1 at 453 fps, 2000x1000. Hotspots there:

| layer | avg us | share of frame |
| --- | ---: | ---: |
| blit | 901.3 | 40.8% |
| backdrop | 394.4 | 17.8% |
| inks | 0.6 | 0.0% |
| grow | 0.1 | 0.0% |

`grow` reads as free because the sheet is grown once and cached; the first frame after a knob change pays it (8.1 ms max at 2000x1000). The 40 percent of the frame outside the timers is the `IterateFrameRenderer` grid reset in `src/morph.rs`, which clears two million cells before the mode is called.

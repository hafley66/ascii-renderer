# sonnet-2-trees

Subject: a nursery sample sheet for six original growth algorithms, one column
each, a full-energy specimen on the top row and a scrub-energy specimen below,
every tree rooted on a ground line with its own roots, knees or shelf feet and
labelled underneath.

What moves and why: the sheet is grown once per (size, seed, palette, growth
knobs) into a cached page of sparse row lists (a `Sprite`). Each frame blits
the cached sprite with a per-column shear (`SWAY`, one slow sine per tree) that
grows with height squared above the root row, so trunks stay planted and
crowns lean; leaf-family glyphs (`● • ∙ · ◆ ◇ ○ ◦ ▪ ▫ ▓ ▒ ░`) swap to a paired
alternate glyph at a hashed per-cell rate (`FLICKER`) for a canopy flicker.
Both effects gate on `t > 0.0`, so `t == 0` reproduces the static bake exactly.

## Species

**Spire, L-system whorled conifer.** A live growth tip `C` rewrites under the
rule `C -> FFF[+C]FFF[-C]FFFC`, iterated 2 to 4 times depending on plot size
and `DETAIL` (the iteration count is picked from the tree's own height, not a
hard move budget, after an earlier version silently truncated the string mid
branch and produced one-sided stubs). A turtle with a continuous float heading
(not a fixed 8-way table) interprets `F` as one step forward, `+`/`-` as a
turn by a per-seed `branch_angle` (25 to 40 degrees) with per-use jitter, `[`
`]` as push/pop, and leftover terminal `C`s as needle stamps. The leader
(depth 0) gets a bark-fill second column while its own move count is under 42
percent of the tree's height budget, so the trunk visibly tapers from a
two-column base to a hairline tip. Every branch split stamps a `╭ ╮ ╰ ╯ ├ ┤`
corner glyph chosen from the parent and child headings. Needle density at each
stamp falls off with recursion depth, so the canopy thins toward the tips.

**Krummholz, wind-flagged dwarf (novel).** Two to four co-dominant stems leave
the base at slightly different points and grow as a turtle walk whose heading
accumulates a steady downwind term (`wind_dir * 0.1 * wind_strength` per step)
plus a late upward kick in the last 20 percent of the walk, so each stem bends
smoothly from near-vertical at the base to a wind-flagged, slightly upturned
tip, never zigzagging. After the stems are drawn, every stem cell samples its
own local `wind_align` (how much its heading points downwind); needle-mat
density at that cell is `0.85 * max(0, wind_align)`, so the windward flank of
the tree stays bare and scoured while the leeward flank grows a dense low
needle mat. The species is stunted on purpose: it grows into 62 percent of the
plot's usual energy. Absent from every existing species file in this repo.

**Strangler, fig lattice on a dead host (novel).** A thin, pale host column is
drawn first with a broken stub at the top. Four or more fig strands descend
from the canopy to the ground alongside it, each following
`x = host_x(row) + amp(row) * sin(freq * row + phase)` where `amp` grows with
`(row / host_h)^1.5`: near the ground the strands hug the host tightly (near
zero amplitude, effectively fused), and near the canopy they loop wide and
open. Whenever two strands compute the same integer column on the same row,
the cell is a `╳` fusion node instead of a plain segment, which reads as
woven lattice tightening toward the base. The lowest fifth of the host is
overwritten by a solid `▓ ▒ ░` root mass, consuming the host visually. A leaf
canopy sits on top with hanging `╷ •` propagules. Absent from every existing
species file in this repo.

**Windrake, radial angle-sweep fan.** A short, tapering, wind-leaned trunk
(shared `taper_trunk` helper) ends in a joint glyph chosen from the trunk-to-
canopy direction change. From there, 5 to 10 rays sweep a narrow arc centered
60 to 100 degrees off vertical on one side only (`base_angle +/- sweep/2`),
each ray a straight segment of independently randomized length; about half
the rays fork once near their tip at a jittered angle. Foliage density along
each ray is `(1 - distance/length)^1.3`, so every ray is dense near the trunk
and frays to `∙` dust at the tip. This is a straight-ray angle sweep, not a
Vogel/golden-angle disk, so the crown reads as a one-sided raked fan rather
than a full round canopy.

**Bracket, fungal shelf stack on a snag.** A tapering dead trunk (no crown)
carries 4 to 10 bracket fungi stacked up alternating sides, spaced at least
two rows apart. Each bracket is a small quarter-dome: for `dx` in `0..=w`,
height falls as `(w - dx) / w * 2`, and the cap glyph steps `◍` (young cap
core) to `▤` (rings) to `░` (gill underside) by normalized height. Bracket
width scales with `age_frac = 1 - row / trunk_len`, so brackets low on the
snag (older) are wider than brackets near the top (younger), and each
attaches to the trunk with a `├`/`┤` joint. Moss patches (`▒`) accumulate
preferentially on the lower, older rows. The base spreads thin `╌ ·` mycelium
threads instead of a root flare.

**Cypress, buttressed trunk with root knees.** The trunk half-width is
`1 + base_flare * (1 - row/height)^2.6`: a strong flare at the ground that
tapers to a hairline near the crown, with fluted bark (`▓` ribs on wide rows,
`▒` fill, `│` edges) that changes glyph mix as the half-width shrinks. Three
to eight root knees are scattered in a ring outside the buttress footprint,
each a short vertical `│`/`╷` stub with a rounded `·` cap, mimicking the
above-ground root knees of a swamp cypress. The canopy is flat and wide (a
`leaf_blob` with a small vertical radius and a large horizontal one) rather
than tall, density thinning toward the rim as with every other species here.

## Glyph families

1. Trunk and heavy limb: `│ ┃ ╱ ╲ ─`
2. Joints and corners: `╭ ╮ ╰ ╯ ├ ┤ ┬ ┴ ┼`
3. Bark and fill: `▓ ▒ ░ █`
4. Needle and canopy: `▪ ▫ ▲ ● • ∙ · ◆ ◇ ○ ◦`
5. Fungal cap and fusion: `◍ ▤ ╳ ✶`
6. Roots and ground: `╱ ╲ │ ─ · ∙ ╷ ╵ ╌`

## Knobs

| key | label | range | default |
| --- | --- | --- | ---: |
| ENERGY | top row crown energy | 0.3 .. 1.2 | 0.9 |
| FRUIT | fruit / cone rate | 0.0 .. 1.0 | 0.25 |
| BRANCH | branching factor | 0.3 .. 1.5 | 1.0 |
| SWAY | sway amplitude in cells | 0.0 .. 2.0 | 0.5 |
| SPEED | time scale | 0.2 .. 3.0 | 1.0 |
| FLICKER | leaf flicker rate | 0.0 .. 1.0 | 0.5 |
| DETAIL | growth sample count factor | 0.4 .. 2.0 | 1.0 |
| ROOTS | root / knee rate | 0.0 .. 1.0 | 1.0 |

Positional order: `energy fruit branch sway speed flicker detail roots`.

Layout: six columns of `width / 6`, two rows of `height / 2`; each cell holds
the plot, a ground line, a root-depth band under it and a label row. Grids
shorter than the sheet clip the bottom.

## Render commands

```bash
ASCII_GRID_W=110 ASCII_GRID_H=36 ./target/release/ascii-renderer 7 sonnet-2-trees moss | sed 's/\x1b\[[0-9;]*m//g'
ASCII_GRID_W=110 ASCII_GRID_H=36 ASCII_T=9 ./target/release/ascii-renderer 7 sonnet-2-trees ember | sed 's/\x1b\[[0-9;]*m//g'
```

## Perf receipt

`perf/knob_sweep.sh sonnet-2-trees 2000 1000 1`, theme moss:

| knob at max | fps | avg ms | vs baseline |
| --- | ---: | ---: | ---: |
| baseline | 430.4 | 2.32 | 1.00x |
| ENERGY=1.2 | 389.6 | 2.57 | 1.10x |
| FLICKER=1 | 401.3 | 2.49 | 1.07x |
| SWAY=2 | 431.2 | 2.32 | 1.00x |
| BRANCH=1.5 | 431.3 | 2.32 | 1.00x |
| ROOTS=1 | 432.5 | 2.31 | 1.00x |
| FRUIT=1 | 432.9 | 2.31 | 0.99x |
| SPEED=3 | 433.5 | 2.31 | 0.99x |
| DETAIL=2 | 434.9 | 2.30 | 0.99x |

Worst knob ENERGY=1.2 holds 387 fps at 2000x1000, 13x the 30 fps bar.

| layer | avg us | share of frame |
| --- | ---: | ---: |
| clear | 970.6 | 37.6% |
| trees | 636.7 | 24.7% |
| ground | 4.1 | 0.2% |

The two per-frame layers cover 62.5 percent of this run's frame (the `grow`
layer, which does all the growth-algorithm work, only fires on a cache miss
and did not fire during this sweep). The residual is the outer grid clear
that `IterateFrameRenderer::render` performs on the shared grid before it
calls any mode, which no mode can wrap; at 2000x1000 that fixed cost is
comparable to this mode's own already-tiny 2.3 ms frame.

## Iteration log

1. First 110x36 render (seed 7): Spire and Krummholz read as tangled scribble,
   not trees; Windrake's canopy drooped down and sideways off the ground
   instead of sitting above the trunk; Strangler/Bracket/Cypress read fine.
2. Narrowed Spire's branch angle and lengthened its L-system F-run per rule;
   smoothed Krummholz's per-step jitter and widened stem start spacing;
   raised Windrake's base angle off horizontal so the fan sits above the
   trunk instead of drooping.
3. Krummholz now read as a near-straight vertical column with no wind flag
   at all (over-corrected); Spire was still thin and one-sided; Windrake's
   fan was legible now.
4. Found the actual bug: Spire's hard move-budget break exited the whole
   L-system interpreter loop partway through the expanded string once
   iteration count grew, silently dropping entire branches. Removed the
   truncation, sized iteration count off the tree's own height instead of a
   budget fraction, and reworked the trunk-taper condition to use an actual
   leader-move counter. Increased Krummholz's per-step heading drift so
   stems visibly bend from vertical to wind-flagged by the tip.
5. Re-rendered at seeds 1, 7, 22, 99: Spire now shows a tapered leader with
   needle-tipped side branches and thinning canopy; Krummholz shows a clear
   downwind bend with a leeward needle mat and a bare windward flank; all
   six species read as distinct, seed-varied trees with visible roots.

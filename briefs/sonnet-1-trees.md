# sonnet-1-trees

Subject: a six-species sample sheet, one column per species, two energy rows (full canopy on top,
scrub-sized on the bottom), each tree standing on its own ground line with roots, boles, stilts or
buttresses under it. The six algorithms share no rule set: a wind-biased turtle walk, a strangler
wrapping a host column, real space colonization toward an attractor cloud, banyan-style prop roots
that drop from a forked canopy, a baobab bulge-profile trunk, and mangrove legs that interlock over
a water line.

## Species

**krummholz** -- wind-flagged turtle walk. The stem climbs one row at a time from the ground; its
horizontal drift is `wind * 0.55 * f^2` where `f` is the fraction of stem already climbed, so
growth low on the trunk barely moves and growth near the tip gets shoved hard downwind (real
krummholz dwarfing). Every direction reversal in the drift gets a rounded corner glyph
(`mark_wobble`), not a plain rule, so the stem reads gnarled. Branches spawn along the upper 55-95%
of the stem; each rolls a coin weighted by `0.5 + |wind| * 0.42` toward the downwind side. A
downwind hit gets a long limb and a leaf-cloud tuft; an upwind miss gets a stub 6-16% of tree
spread long and no canopy at all, so the whole crown reads as blown to one side.

**fig** -- a strangler wrapping a host trunk. A plain thin `draw_bark_column` is drawn first as the
host. Four to six root strands launch near the top of the host and descend as sine-wrapped strokes,
`x(f) = host_x + sin(phase + f*wraps*TAU) * amp * f`, so the wrap amplitude grows with descent and
every strand differs by its own phase and wrap count. Past a coin flip on `energy`, each strand
gets a second parallel stroke one cell over (fusion thickening). All strands land in a root mat at
the base. The fig's own leaf cloud is drawn large and dense over the host's crown region; a second,
much smaller and sparser cloud lower down is what is left of the host's own canopy.

**colonist** -- true space colonization toward a leaf cloud. Attractor points fill a bimodal cloud:
each point picks one of two lobes offset by a random light bias, so the crown reaches unevenly
toward one side. A single skeleton node starts at the trunk top. Each iteration, every live
attractor finds its nearest skeleton node (a bucket grid keeps this to neighbor cells only, not
O(n^2)); nodes pulled by one or more attractors sprout a new node toward the averaged pull
direction; attractors within the kill radius of any node die. Iterate until attractors run out,
capped at 420 nodes / 70 passes. Edge thickness comes from subtree mass, computed in one reverse
pass over parents. Along the heaviest limbs, a low chance per node sprouts a small epicormic leaf
tuft directly off the wood, which none of the sibling lanes' space-colonization species do.

**proproot** -- banyan-style aerial prop roots. A short trunk forks into three to seven
near-horizontal limbs (`ang` only 0.08-0.32 radians off horizontal, the flat banyan silhouette),
each of which forks once more into two shorter limbs. Every tip gets a wide, dense leaf-cloud
cluster. From a subset of tips, an aerial root turtle drops straight down with a small quadratic
sway; if the drop is short enough relative to tree height it reaches the ground, thickens (a second
`┃` column beside the `│` trail) and gets its own small root flare; if the canopy is too high above
ground for that tip, the root just dangles (`╷`) unrooted. Roots start a third of the way below
their tip, not at the densest part of the canopy, so they read as emerging from under the foliage.

**bottle** -- a baobab bottle trunk with a tiny crown. Trunk radius follows a bump profile,
`r(f) = base_r * (0.22 + 0.85 * sqrt(max(0, 1 - ((f-peak)/0.40)^2)))`, peaking in the lower half of
the trunk and tapering hard at both the crown and the ground flare -- a true bottle silhouette, not
a tapered cylinder. A wide buttress flare sits at the base. Three to six short stubby limbs fan from
the very top of the trunk, each closing in a small, sparse leaf-cloud tuft (`thin_pow` 1.7, steeper
falloff than any other species) so the crown reads as an afterthought next to the trunk.

**stilt** -- mangrove stilt roots interlocking over water. Above a fixed waterline the trunk is a
normal tapered column; below it, three to five legs launch from alternating sides of the trunk.
Each leg's x position is `trunk_x + side*reach*f - side*reach*0.4*sin(f*PI)`, which pushes the leg
out then bends it back past center before it reaches the water, so neighboring legs (launched from
opposite sides) cross rather than fan apart -- a real interlocking mesh, not a radial spray.
Pneumatophore stubs poke up between the legs at the waterline; a ripple row of `~`/`≈` marks the
water surface itself.

## Glyph families

1. Trunk/bark taper: `│ ┃ █ ▓ ▒ ░ ┼` (thin rings are a plain rule, thick rings get a ridged fill)
2. Branch joints: `├ ┤ ┬ ┴` picked by trunk orientation and limb side, never a generic line char
3. Turns and corners: `╭ ╮ ╰ ╯` for wobble reversals and root arcs
4. Canopy: `▓ ▒ ∙ · ◆ ◇ ●` thinning outward from core to fringe
5. Roots, water, ground: `╱ ╲ ─ ╴ ╶ ~ ≈ ┴`

## Knobs

- ENERGY: top row crown energy (default 0.88)
- FRUIT: fruit and bloom rate (default 0.16)
- BRANCH: branch density (default 0.6)
- SCRUB: bottom row energy factor (default 0.56)
- ROOTS: root and flare style offset (default 0)
- WIND: prevailing wind, negative blows left (default 0.35)
- SWAY: sway amplitude in cells (default 0.9)
- SPEED: time scale (default 1.0)
- DETAIL: growth sample count factor (default 1.0)
- HUE: hue rotation degrees (default 0)
- BARE: bare trunk fraction of height (default 0.26)

Positional order: energy fruit branch scrub roots wind sway speed detail hue.

## Render commands

```bash
ASCII_GRID_W=110 ASCII_GRID_H=36 ./target/release/ascii-renderer 7 sonnet-1-trees moss | sed 's/\x1b\[[0-9;]*m//g'
ASCII_GRID_W=110 ASCII_GRID_H=36 ASCII_T=5 ./target/release/ascii-renderer 7 sonnet-1-trees moss | sed 's/\x1b\[[0-9;]*m//g'
```

## Perf receipt

`perf/knob_sweep.sh sonnet-1-trees 2000 1000 1`, worst knob ENERGY=1: 434 frames, 433.2 fps
(2.31ms avg). Every other knob at max stayed at or above 426 fps. Hotspot table at the worst knob:

| layer | share of frame |
| --- | ---: |
| clear | 39.6% |
| trees | 20.0% |
| ground | 0.3% |
| flicker | 0.2% |

The bake pass (space colonization, root wrapping, all six species) runs once per distinct
`(w, h, seed, knobs)` key behind a `thread_local!` cache and is amortized to near zero across the
frames the sweep measures, the same shape as `opus-1-trees`' own hotspot table.

## Iteration log

- Iteration 1: rendered at 110x36 and 180x42. Krummholz's trunk barely read as gnarled (mostly a
  single `│`), the flagged-branch canopy was too small to see, and proproot's aerial roots drew
  right through the densest part of its own canopy, reading as clutter instead of separate limbs.
- Iteration 2: shortened krummholz's stunt range (0.26-0.50 vs 0.32-0.68) and moved its branch band
  higher up the stem (45-95% instead of 30-92%); widened its flagged-branch leaf clouds from
  `len*0.55/0.30` to `len*0.75/0.46` and dropped `thin_pow` from 1.1 to 0.85 for a denser tuft;
  grew proproot's canopy clusters (`tl*1.25/0.75`, `thin_pow` 0.9) and pushed the aerial root start
  point a third of a tip-length below the tip so roots read as hanging under the foliage.
- Iteration 3: fig's crown still looked thin and scattered at the top of its column. Enlarged the
  host height fraction (0.55 to 0.68 of spread) and the canopy radii (`crx` 1.15 to 1.30, `cry`
  0.55 to 0.62 of the host gap), dropped `thin_pow` from 1.35 to 0.9, and shrank the host's own
  remnant crown further (0.35/0.30 to 0.22/0.20 of the fig canopy radii) for contrast.
- Iteration 4: rendered seeds 3, 7, 42 side by side. Composition, lean direction, wrap pattern,
  branch angle and bulge peak all differ per seed as intended; no further species-shape changes
  needed. Bumped colonist's epicormic-sprout chance from 0.12 to 0.20 and its tuft radius up
  slightly so the space-colonization crown reads less skeletal in isolation.
- Iteration 5: confirmed `t=0` renders identical to the static frame and `t>0` sways branches and
  flickers a rotating slice of leaf cells; accepted the in-module and integration snapshots after
  visual review. No further geometry changes.

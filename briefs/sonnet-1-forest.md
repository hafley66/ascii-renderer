# sonnet-1-forest

Subject: a depth-layered stand of the sonnet-1-trees species (krummholz, fig, colonist, proproot,
bottle, stilt), three to six ground layers deep, under a starfield-and-moon sky, with one of three
atmospheres that each touch the trees themselves rather than floating over them, and a four-stop
day cycle that shifts the whole palette over one full CYCLE.

## Composition

Geometry bakes once per `(w, h, seed, density, layers, energy, fruit, branch, detail, horizon,
atmos, bare, wind)` key into a `thread_local!` cache (`ForestBake`), then every frame just indexes
that cache and blits with a sway offset -- no per-frame allocation or growth. Species mix is a
seeded cumulative-weight table (`species_mix`) so every seed favors a different subset of the six;
the back layer is nudged toward krummholz and the front layer away from it, so distant silhouettes
skew sparse and windswept while the nearest trees skew toward the fuller-canopy species. Colonist
is capped at 46 instances per bake (`MAX_COLONIST`) and falls back to proproot past that, since its
attractor search is the most expensive species and a dense stand could otherwise spend the whole
frame budget on skeletons no one is looking at up close.

Placement is a per-layer left-to-right walk with random step and plot-size jitter (`bake_forest`'s
`'layers` loop), not `pack_forest` -- each layer needs its own tree-height-to-slot-width ratio and
depth-scaled overlap, which the generic packer does not carry. Overlap is capped at 0.45 and the
front layer's step widens by `1 + 1.6*f`, so near trees stay legible as separate trunks instead of
merging into a wall of glyphs.

Parallax: every tree's sway phase group also carries a `depth_scale` in `(0.25 + 0.75*f)`, folded
directly into the sway offset (`k.sway * depth_scale * sin(...)`), so back-layer trees visibly sway
less than front-layer trees at the same SWAY setting, not just on a longer period.

## Atmosphere

ATMOS picks one of three (0 lets the seed choose), each interacting with the baked tree geometry
instead of drawing independently over it:

1. **fog** -- horizontal bands drift sideways (`x0 + t*speed*sp*3`) at a handful of fixed heights.
   Everywhere a band overlaps a cached trunk cell (a `trunks: Vec<u32>` list captured at bake time,
   one every other trunk-band cell) within nine columns of the band center, the trunk cell is
   overwritten with a `~`/`≈` mist glyph weighted by how deep into the band it sits -- the mist
   genuinely occludes the trunks it passes over, it does not just sit on top visually.
2. **fireflies** -- each firefly is assigned at bake time to one tree's canopy bounding box
   (`canopy: Vec<(cx, cy, rx, ry)>`, recorded when that tree was placed) and wanders by a sine/cosine
   orbit scaled to that box's own radius, so fireflies are structurally confined to the canopy they
   were born under rather than clipped to a shared region.
3. **leaffall** -- a rotating slice of the baked `leaves` list (real `BAND_LEAF` cells from the
   species' own canopies, the same capture technique as the sheet's flicker layer) gets a per-leaf
   fall animation seeded from its own cell hash, drifting down from its canopy origin and wrapping
   after `h*0.6+6` rows. The canopy itself is untouched -- leaves fall from it without depleting it,
   the way a real tree keeps shedding.

## Light cycle

`light_stop` interpolates four stops (day, dusk, night, dawn) around a `p = (t*speed/CYCLE) % 1`
clock: sky top color, horizon haze color, and a scalar light level that darkens every band's bark,
limb, leaf, bloom and soil ramp together. A full cycle is CYCLE seconds (20-60, default 40), so the
palette visibly drifts from noon light to dusk haze to night dark to dawn haze and back over the
render, not a hard cut.

## Glyph families

Same six as sonnet-1-trees (bark taper, branch joints, corners, canopy, roots/ground) plus:
6. Atmosphere: `~ ≈` (fog/water), `◦ ∙` (fireflies), `◇ ·` (falling leaves)

## Knobs

- DENSITY: trees per depth layer (default 1.0)
- LAYERS: depth layers (default 4)
- SWAY: sway amplitude in cells (default 1.1)
- SPEED: time scale (default 1.0)
- HUE: hue rotation degrees (default 0)
- ATMOS: 0 seed picks, 1 fog, 2 fireflies, 3 leaffall (default 0)
- ENERGY: crown energy (default 0.88)
- FRUIT: fruit and bloom rate (default 0.12)
- BRANCH: branch density (default 0.58)
- DETAIL: growth sample count factor (default 1.0)
- CYCLE: day cycle seconds (default 40)
- HORIZON: horizon height fraction (default 0.40)
- BARE: bare trunk fraction of height (default 0.28)
- WIND: prevailing wind, negative blows left (default 0.35, `ASCII_P_WIND` only, no positional slot)

Positional order: density layers sway speed hue atmos energy fruit branch detail cycle horizon.

## Render commands

```bash
ASCII_GRID_W=110 ASCII_GRID_H=36 ASCII_T=12 ./target/release/ascii-renderer 7 sonnet-1-forest moss | sed 's/\x1b\[[0-9;]*m//g'
ASCII_P_ATMOS=1 ASCII_GRID_W=110 ASCII_GRID_H=36 ASCII_T=8 ./target/release/ascii-renderer 3 sonnet-1-forest moss | sed 's/\x1b\[[0-9;]*m//g'
```

## Perf receipt

`perf/knob_sweep.sh sonnet-1-forest 2000 1000 1`, worst knob DENSITY=2.5: 287 frames, 286.1 fps
(3.49ms avg). Every other knob at max stayed at or above 285 fps (HORIZON=0.7 actually sped up to
415 fps by shrinking the ground area). Hotspot table at the worst knob:

| layer | share of frame |
| --- | ---: |
| trees | 28.5% |
| clear | 26.6% |
| ground | 13.7% |
| atmos | 2.1% |
| sky | 1.9% |

Bake (species growth, sky, ground bands, atmosphere seeding for the whole stand) runs once per key
behind the `thread_local!` cache, same shape as `opus-1-forest`.

## Iteration log

- Iteration 1: rendered at 110x36. The near layer's trees overlapped into a single dense blob of
  mixed glyphs (a colonist skeleton and several bottle trunks merging visually), and the top-of-band
  ground rule (`─`) fired at 70% density, reading as a near-solid fence line across the whole width.
- Iteration 2: dropped the ground-rule threshold from `n<0.30` to `n<0.55` (sparser, more natural
  line) and tightened front-layer packing: overlap cap 0.62 to 0.45, front step multiplier 1.3 to
  1.6. Re-rendered: individual trunks read as separate trees again, the ground line broke up.
- Iteration 3: rendered all three ATMOS kinds at t=8 on the same seed. Fog visibly ate into trunk
  columns it crossed (confirmed by diffing static vs t=8 output), fireflies stayed near canopy
  boxes rather than drifting across open sky, falling leaves appeared at a spread of fall depths
  under real canopy positions. No code changes needed; all three matched the design intent.
- Iteration 4: rendered seed 42 under the ember theme. Species mix, hue and the bottle-trunk bulge
  in the front layer all differ from the moss/seed-7 renders as expected; confirmed the day-cycle
  tint (`b.tint`, seed-derived) shifts the warm/cool balance per seed on top of the HUE knob.
- Iteration 5: ran the full snapshot suite and the knob sweep after the packing fix; both new
  integration snapshots (static and t=15 drifting) and the two in-module snapshots matched the
  renders already reviewed by eye, so accepted them as-is with no further geometry changes.

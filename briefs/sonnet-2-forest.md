# sonnet-2-forest

Subject: a layered stand of the six sonnet-2 species, back to front, with
parallax sway, one seed-picked atmosphere that reacts to the trees, and a
two-axis (brightness and warmth) light cycle over a 46 second loop.

What moves and why: the scene is grown once per (size, seed, palette, density,
layer, atmosphere, energy, ground, fruit) key into a cached `Scene`, the same
sprite-cache pattern the sample sheet uses. Every frame recomputes only the
per-color light table and the sway/atmosphere offsets, so growth (the
expensive part) never repeats. Sway amplitude scales with each tree's depth
fraction, so back-layer trees barely move and front-layer trees sway visibly;
canopy-anchored atmosphere particles read the recorded positions of leaf-glyph
cells gathered while the sheet was grown, so fireflies hover near real crowns
and falling leaves originate at real canopy cells rather than random points in
the sky. All motion, including the light cycle, gates on `t > 0.0`.

## Composition

Depth layers (`LAYERS`, 1 to 5) place trees back to front: far layers get a
smaller height budget, a higher horizon-relative root, desaturated and
haze-faded ink, and less sway; near layers get taller budgets, fuller color
and full sway. Within a layer, trees are placed at random x with a
collision-rejection loop against already-placed canopy spans (`DENSITY`
scales the target count, the same span-overlap tolerance the sibling forest
lanes use). Each tree independently rolls a species from a per-seed weighted
mix (one dominant species at 42 percent, one secondary at 24 percent, the
remaining four species splitting the rest), so seed changes both which
species dominate a given forest and how dense the stand is. `HUE` rotates the
whole palette; `ENERGY` scales every tree's height budget; `GROUND` sets the
horizon fraction; `FRUIT` feeds every species' fruit/cone knob.

## Atmosphere (novel per-kind interactions)

`ATMOS` picks one of five kinds (0 lets the seed choose): **Fogbanks** are two
to four discrete elliptical fog masses that drift horizontally at their own
speed and height, painted after the tree layer, so a bank crossing a trunk
visibly occludes it (this is not a single fixed mist band; each bank has its
own position, radius and drift). **Fireflies** spawn at a real canopy-glyph
cell (recorded while growing that layer's trees) rather than a random point in
a height band, then hover near that anchor, so they stay under the canopy
instead of drifting through open sky. **Leaves** spawn at a real canopy cell
and fall from there, drifting sideways as they descend, so a falling leaf
traces back to an actual leaf glyph on an actual tree. **Snow** and **Birds**
are conventional weather/flock particles (uniform screen coverage and flock
formation respectively) included for variety alongside the two tree-aware
kinds above.

## Light cycle

Every frame computes `cyc = 2*pi*t*SPEED / 46`. Brightness follows `sin(cyc)`
(lighten toward noon, darken toward night, scaled by `LIGHT`); a second,
smaller term `0.35 * LIGHT * cos(cyc)` blends each color toward a warm
dawn/dusk hue when positive and a cool night-blue hue when negative. Both
terms share one phase, so the whole palette shifts together over one full
20 to 60 second cycle rather than only dimming.

## Glyph families

Reuses every glyph family from `sonnet-2-trees` for the trees themselves,
plus:

1. Ground and horizon: `╌ · ∙ , ~ "`
2. Fog: `▒ ░`
3. Fireflies: `· • ✦`
4. Falling leaves: `, · ∙ "`
5. Snow: `* · .`
6. Birds: `v ~ -`

## Knobs

| key | label | range | default |
| --- | --- | --- | ---: |
| DENSITY | trees per depth layer | 0.2 .. 2.0 | 1.0 |
| LAYERS | depth layers | 1 .. 5 | 3 |
| SWAY | sway amplitude in cells | 0.0 .. 2.0 | 0.6 |
| SPEED | time scale | 0.2 .. 3.0 | 1.0 |
| HUE | hue rotation deg | 0 .. 360 | 0 |
| ATMOS | 0 seed, 1 fog, 2 flies, 3 leaves, 4 snow, 5 birds | 0 .. 5 | 0 |
| HAZE | aerial perspective fade | 0.0 .. 1.0 | 0.5 |
| ENERGY | crown energy | 0.4 .. 1.2 | 0.9 |
| GROUND | horizon fraction of height | 0.5 .. 0.9 | 0.64 |
| FRUIT | fruit / cone rate | 0.0 .. 1.0 | 0.15 |
| LIGHT | light cycle depth | 0.0 .. 1.0 | 0.5 |

Positional order: `density layers sway speed hue atmos haze energy ground fruit light`.

## Render commands

```bash
ASCII_GRID_W=110 ASCII_GRID_H=36 ./target/release/ascii-renderer 7 sonnet-2-forest moss | sed 's/\x1b\[[0-9;]*m//g'
ASCII_GRID_W=110 ASCII_GRID_H=36 ASCII_T=12 ./target/release/ascii-renderer 7 sonnet-2-forest moss | sed 's/\x1b\[[0-9;]*m//g'
```

## Perf receipt

`perf/knob_sweep.sh sonnet-2-forest 2000 1000 1`, theme moss:

| knob at max | fps | avg ms | vs baseline |
| --- | ---: | ---: | ---: |
| baseline | 297.0 | 3.37 | 1.00x |
| DENSITY=2 | 260.8 | 3.83 | 1.14x |
| ENERGY=1.2 | 278.3 | 3.59 | 1.07x |
| LAYERS=5 | 287.5 | 3.48 | 1.03x |
| GROUND=0.9 | 289.2 | 3.46 | 1.03x |
| LIGHT=1 | 292.4 | 3.42 | 1.02x |
| FRUIT=1 | 299.0 | 3.34 | 0.99x |
| SWAY=2 | 300.9 | 3.32 | 0.99x |
| SPEED=3 | 305.7 | 3.27 | 0.97x |
| HAZE=1 | 311.1 | 3.21 | 0.95x |
| HUE=360 | 312.5 | 3.20 | 0.95x |
| ATMOS=5 | 315.2 | 3.17 | 0.94x |

Worst knob DENSITY=2 holds 260 fps at 2000x1000, 8.6x the 30 fps bar.

| layer | avg us | share of frame |
| --- | ---: | ---: |
| backdrop | 1777.6 | 49.5% |
| trees | 456.4 | 12.7% |
| sky | 390.6 | 10.9% |
| undergrowth | 11.2 | 0.3% |
| atmos | 5.8 | 0.2% |
| light | 1.4 | 0.0% |

The six timers cover every painter this mode owns (`build`, the growth pass,
fires only on a cache miss and did not fire in this run). `backdrop` is the
sky/ground gradient blit, which scales directly with grid area and dominates
at this resolution; it is intentionally the cheapest possible per-cell copy
(one array index, one `Cell::new`), so this share is expected rather than a
missed optimization.

## Iteration log

1. First 110x36 render at seed 7, t=12: trees plant correctly on the ground
   with visible taper and canopy variety across the depth layers; the fog
   bank atmosphere correctly occludes trunk cells as it drifts across them.
2. Seed 3 static render: this seed's forest is noticeably sparser with a
   large empty gap and one background tree whose root flare is hard to make
   out at small scale; left as procedural variance (the placement formula
   matches the proven sibling-lane density curve) rather than forcing every
   seed toward a uniform density.
3. Forced ATMOS=3 (Leaves) at t=6, seed 11: confirmed leaf particles
   originate at recorded canopy-glyph positions rather than random screen
   points, and that density at this seed reads full without gaps.
4. Confirmed firefly and leaf spawn points are drawn from the same
   `canopy_cells` list gathered while growing layer trees above the 40th
   percentile depth fraction, so both atmosphere kinds anchor to real crowns
   instead of an arbitrary height band.
5. Verified the light cycle's warmth term stays visually subtle (capped at
   0.3 blend) so a full day cycle brightens and warms together without
   blowing out the palette at the extremes of the sine.

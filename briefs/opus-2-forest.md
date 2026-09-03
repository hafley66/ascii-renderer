# opus-2-forest

Subject: a stand of the opus-2 species seen from inside it, four depth bands deep, under a sky that turns from night to day and back over the length of a slow cycle.

What moves and why: `t` drives three clocks at different rates. The light cycle runs `phase = (t * SPEED / CYCLE) mod 1` through four key tints (night, dawn, day, dusk); sky, ridges, ground, haze and tree color all read from the interpolated key, so the scene warms and cools together. Sway runs at 0.19 to 0.24 radians per second per layer with a per-tree phase and a gust envelope at 0.11 radians per second; the shear is `SWAY * gust * layer * sin(t * w + phase) * up^2`, zero at the roots. Weather drifts on its own clock. At `t = 0` all three are frozen at phase zero and the frame is byte identical to the static render.

## Composition

- **Sky.** Row fill from the top key color to the horizon key color on a 1.6 power curve. Clouds appear when the light is above 0.42: two to six wisps of `░` and `·` drifting at 0.35 columns per second. Stars appear when the light is below 0.62, hashed positions in the top 82 percent of the sky, each twinkling on its own phase. The moon is a disc of `▓` and `▒` with x scaled by 2, crossing the sky once per light cycle.
- **Ridges.** Two silhouette bands from a three-sine ridge profile with seeded phases. Everything from the crest down to the horizon is filled solid, the crest row one shade lighter, so they read as land and not as texture.
- **Ground.** A soil gradient from the horizon color at the back to a dark trunk color at the front, with the texture stride tightening from 16 to 5 columns as it comes forward, and grass tufts of `╵ │ ╷` scattered in the last three rows.
- **Canopy.** Trees are stamped back to front from a sprite bank. The backmost band is redrawn as a flat silhouette (`▓` for wood, `▒` for foliage) so distance reads as loss of detail, not as noise.
- **Weather.** One of five, chosen by seed or forced by ATMOS: mist bands drifting along the horizon, fireflies on slow lissajous paths that twinkle, falling leaves that sway as they drop, slanted rain, or a flock of birds crossing high with a flapping glyph pair.

## Placement and the sprite bank

The stand is built once per (width, height, seed, shape knobs) and kept in a thread local. For each layer it bakes 5 species by 6 variants into sparse cell lists, growing each one into a scratch grid with the species algorithms from `src/opus_2_trees.rs`, then keeping only the painted cells and their color slots. A frame does no growth and no rng.

Placement walks x across each layer. Step is `tree_width * (1.25 - 0.75 * DENSITY) * (0.82 + 0.55 * layer)`, so the far bands crowd and the near band spreads. Two overlap rules run: a new trunk within two columns of a trunk in the layer behind is nudged right by 2, and within two columns of its own layer's previous trunk by 3. Species are drawn from a seeded weight vector that MIX flattens toward uniform; brackets are held to the front 30 percent of depth and coral to the front 55 percent, since both are understory. Each tree gets a variant, a mirror flag (glyphs mirror with it), a tone bucket and a sway phase from hashes of (seed, layer, index).

Depth falloff is size and color together: tree height is `base * (0.22 + 0.78 * layer^1.4)`, and each layer's ink is lerped toward the horizon color by HAZE before it is lit by the light cycle.

## Glyph families

1. Trunk and pillar: `┃ │ ╿ ├ ┤ ┼`
2. Branch, root arc and grass: `─ ╱ ╲ ╴ ╶ ╵ ╷ ╌ ╎`
3. Canopy, ridge and soil mass: `▓ ▒ ░ ·`
4. Leaf, shelf rim and fruit: `◆ ◇ ◡ ●`
5. Weather: `~ ∘ ◦ ✦ ▪`

## Knobs

- DENSITY: stand crowding (default 0.3)
- LAYERS: depth bands (default 4)
- SWAY: sway columns (default 1.2)
- SPEED: clock multiplier (default 1.0)
- HUE: hue shift degrees (default 0)
- ATMOS: 0 seed, 1 mist, 2 fireflies, 3 leaves, 4 rain, 5 birds (default 0)
- ENERGY: tree growth energy (default 0.86)
- GROUND: ground fraction of canvas (default 0.5)
- HAZE: aerial fade with depth (default 0.7)
- MOTES: weather density (default 1.0)
- SCALE: tree size (default 1.0)
- CYCLE: seconds per light cycle (default 44)
- FRUIT: fruiting bodies per tip (default 0.22)
- BRANCH: branch and shelf reach (default 0.7)
- GNARL: trunk wander (default 0.35)
- MIX: species mix evenness (default 0.5)

Positional order: density layers sway speed hue atmos energy ground haze motes scale cycle.

## Render commands

```bash
ASCII_GRID_W=110 ASCII_GRID_H=36 ./target/release/ascii-renderer 42 opus-2-forest moss | sed 's/\x1b\[[0-9;]*m//g'
ASCII_GRID_W=110 ASCII_GRID_H=36 ASCII_T=12 ./target/release/ascii-renderer 7 opus-2-forest moss | sed 's/\x1b\[[0-9;]*m//g'
```

## Perf receipt

`perf/knob_sweep.sh opus-2-forest 2000 1000 1`, full table in `perf/results/opus-2-forest.md`.

| knob at max | fps | avg ms | vs baseline |
| --- | ---: | ---: | ---: |
| baseline | 334.5 | 2.99 | 1.00x |
| DENSITY=1 | 319.5 | 3.13 | 1.05x |
| GROUND=0.6 | 321.8 | 3.11 | 1.04x |
| SCALE=2 | 323.2 | 3.09 | 1.04x |
| LAYERS=6 | 327.1 | 3.06 | 1.02x |
| ENERGY=1 | 333.0 | 3.00 | 1.00x |
| SPEED=4 | 333.3 | 3.00 | 1.00x |
| SWAY=5 | 335.6 | 2.98 | 1.00x |

Worst knob DENSITY=1 at 319 fps, 2000x1000. Hotspots there:

| layer | avg us | share of frame |
| --- | ---: | ---: |
| ground | 1152.1 | 37.7% |
| ridges | 434.6 | 14.2% |
| canopy | 288.2 | 9.4% |
| sky | 249.5 | 8.2% |
| atmos | 17.8 | 0.6% |
| grow | 0.0 | 0.0% |

`grow` reads as free because the stand and the sprite bank are cached; a knob change pays the rebuild once. The 30 percent of the frame outside the timers is the `IterateFrameRenderer` grid reset in `src/morph.rs`, which clears two million cells before the mode is called.

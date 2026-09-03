# opus-1-forest

Subject: a stand of the five opus-1 species receding into haze across three to six
depth layers, standing on a layered floor under a star field with a moon, cloud
lenses and one weather element, lit by a day cycle that takes 20 to 60 seconds to
come round.

What moves and why: the whole scene bakes once per `(w, h, seed, geometry knobs)`
into a sparse cell list, so a frame is a blank fill plus three blits plus the
weather. Three things then move on `t`. Each tree carries a seeded oscillator of
period 9 to 22 s and each baked cell a height weight, so crowns lean while trunks
stay planted, back layers a touch faster than front. The day cycle walks four stops,
day, dusk, night, dawn, at `t / CYCLE`, and rebuilds the per-layer color table each
frame from the current sky and haze colors, so the light changes without touching
geometry. The weather motes are pure functions of `t` with horizontal wrap. At
`t == 0` sway offsets are zero, the cycle sits on the day stop and the motes sit at
their spawn points, so the frame equals the static bake.

## composition

Placement is a perspective packer, not `pack_forest`, because the layers need
independent species mixes and independent thinning:

- The horizon sits at `h * HORIZON`. Layer `i` of `L` gets a ground line at
  `horizon + (h - 1 - horizon) * ((i+1)/L)^1.35`, so the floor bands compress toward
  the horizon.
- Apparent tree height is `(ground_line - horizon) * 1.15`, the perspective relation:
  a tree far from the camera stands near the horizon and is small, a near tree stands
  low and towers.
- Slot width is `0.85 *` height. The walk along x steps by
  `slot_w * (1 - 0.30*DENSITY) * (1 + 1.3*f)` with an 0.80 to 1.24 jitter, `f` being
  the layer's depth fraction. The depth term is what keeps the foreground to two or
  three wide-spaced silhouettes while the horizon carries a solid tree line, so the
  back layers stay visible through the gaps.
- The seed draws a five-way species weight vector, so no two seeds get the same mix.
  The back layer never draws mangroves and the front layer never draws aggregates.
- The seed also sets a hue offset of -30 to +30 degrees, the moon position and size,
  the cloud count, and the weather kind when ATMOS is 0.
- Bare trunk fraction scales with depth, `BARE * (0.75 + 0.85*f)`, so front trees are
  about half clear trunk and you can see under the canopy.

Depth falloff: layer `i` mixes its whole color ramp toward the current haze color by
`0.78 * (1 - i/(L-1))`, and the sky, ground, trunk, limb, leaf and bloom ramps are
rebuilt per layer per frame from a 32-step table each. Baked cells carry a palette
index only, so recoloring the entire scene costs one table build, not a pass over
cells.

## atmosphere

One of five, selected by the seed or forced with ATMOS:

1. mist, `~` and `≈` drifting laterally with a slow vertical breath
2. fireflies, `· ∙ ◦` on slow elliptical orbits, brightness pulsing, below the horizon only
3. leaf fall, `◆ ◇ ·` falling with a sinusoidal lateral drift
4. rain, `│ ╵` falling fast on a fixed column
5. snow, `· ∙ ◦` falling slowly with a wide lateral swing

Mote count is `w*h/55` capped at 2600, so the cost is bounded at any grid size.

## glyph families

1. Trunk and heavy limb: `┃ │ ┼ ├ ┤`
2. Limb, shelf, prop root, horizon: `─ ╱ ╲ ╰ ╯ ┬ ┴ ╴ ╶`
3. Foliage and haze mass: `▓ ▒ ░`
4. Leaf, bead, star, mote: `◆ ◇ ● ◦ ∙ ·`
5. Weather: `~ ≈ │ ╵`

## knobs

| key | label | range | default |
| --- | --- | --- | ---: |
| DENSITY | trees per depth layer | 0.25 .. 2.5 | 1.0 |
| LAYERS | depth layers | 3 .. 6 | 4 |
| SWAY | sway amplitude in cells | 0.0 .. 4.0 | 1.1 |
| SPEED | time scale | 0.0 .. 3.0 | 1.0 |
| HUE | hue rotation deg | -180 .. 180 | 0 |
| ATMOS | 0 seed, 1 mist, 2 flies, 3 leaves, 4 rain, 5 snow | 0 .. 5 | 0 |
| ENERGY | crown energy | 0.3 .. 1.0 | 0.9 |
| FRUIT | fruit and bloom rate | 0.0 .. 1.0 | 0.12 |
| BRANCH | branch keep probability | 0.05 .. 1.0 | 0.6 |
| DETAIL | growth sample count factor | 0.2 .. 2.0 | 1.0 |
| CYCLE | day cycle seconds | 20 .. 60 | 42 |
| HORIZON | horizon height fraction | 0.2 .. 0.7 | 0.40 |
| BARE | bare trunk fraction of height | 0.08 .. 0.60 | 0.30 |

Positional order: `density layers sway speed hue atmos energy fruit branch detail cycle horizon`.

## render commands

```bash
ASCII_GRID_W=110 ASCII_GRID_H=36 ./target/release/ascii-renderer 7 opus-1-forest moss | sed 's/\x1b\[[0-9;]*m//g'
ASCII_GRID_W=110 ASCII_GRID_H=36 ASCII_T=18 ./target/release/ascii-renderer 3 opus-1-forest ember | sed 's/\x1b\[[0-9;]*m//g'
```

## perf receipt

`perf/knob_sweep.sh opus-1-forest 2000 1000 1`, full table in `perf/results/opus-1-forest.md`.

| knob at max | fps | avg ms | vs baseline |
| --- | ---: | ---: | ---: |
| baseline | 363.5 | 2.75 | 1.00x |
| DENSITY=2.5 | 327.4 | 3.05 | 1.11x |
| ENERGY=1 | 357.6 | 2.80 | 1.02x |
| DETAIL=2 | 362.0 | 2.76 | 1.00x |
| ATMOS=5 | 372.2 | 2.69 | 0.98x |
| BARE=0.6 | 378.0 | 2.65 | 0.96x |
| LAYERS=6 | 389.3 | 2.57 | 0.93x |
| HORIZON=0.7 | 409.9 | 2.44 | 0.89x |

Worst knob DENSITY=2.5 holds 331 fps at 2000x1000, 11x the 30 fps bar.

| layer | avg us | share of frame |
| --- | ---: | ---: |
| clear | 919.5 | 30.4% |
| trees | 505.1 | 16.7% |
| ground | 485.5 | 16.1% |
| sky | 99.7 | 3.3% |
| atmos | 83.9 | 2.8% |

The five timers cover every painter the mode owns. The residual 31 percent is the
blank-fill that `IterateFrameRenderer::render` performs on the shared grid before it
calls any mode, which no mode can wrap; at 2000x1000 that fixed 0.9 ms is a third of
this mode's whole frame.

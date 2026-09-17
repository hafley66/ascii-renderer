# reef

Subject: a coral reef seen through a water column. Colonies grow off a sand floor in
four silhouettes, under a depth gradient lit by tilted sun shafts and a caustic net,
with eelgrass, bubble vents and schooling fish.

Mode file: `src/modes/_57_reef.rs`, registered through `registered_modes()` alone.
`AnimKind::Iterate`, 11 knobs, 6 layer timers.

## Silhouettes

Each colony picks one of four forms by a hashed rotation of the colony index, so a
pane of four carries one of each and the assignment moves with the seed.

| form | growth | envelope | ramp thin to dense |
| --- | --- | --- | --- |
| fan | rim DLA, radial attachment bias | waist `0.30 + 0.82 * lift` | `.` `:` `*` `%` `#` `&` |
| stag | rim DLA, low coordination tolerance | waist `0.24 + 0.70 * lift` | `.` `:` `o` `O` `@` `#` |
| brain | filled half ellipse cut by grooves | dome, `sy` 0.96 | `.` `~` `=` `o` `@` `&` |
| tube | column cluster of mixed height | `sx` 0.82, `sy` 1.12 | `.` `o` `O` `\|` `#` `&` |

## Growth

A walker launches just outside the colony's current reach on the upper half of its
ellipse and random walks with a horizontal step preference. Attachment probability is
`STICK * coord[kin] * (0.34 + 0.66 * rise) * (0.45 + 0.85 * radial)`: the coordination
table falls off with neighbour count so dendrites stay open, `rise` pulls growth off
the buried base, and `radial` makes the current outline the sticky part, which is what
turns the aggregate bushy and branched instead of lumpy.

Attachment is rejected outside a per-colony waist that narrows toward the floor and
outside a per-column ragged edge `0.74 + 0.34 * hash(seed, tag, x)`, so the crown
breaks up instead of tracing the ellipse. A diagonal attachment also lays an elbow
cell, and one close pass fills any pocket with 4 or more kin (5 for a stag), which is
what makes an 8-connected aggregate read as coral mass at terminal cell aspect rather
than as speckle.

Glyph level mixes local density and subtree support:
`((kin / 8) * 0.70 + (level_of(weight) / 5) * 0.52) * 5`, so trunks and interiors land
on the dense end of the form's ramp and outer tips on the thin end. Hue lightens and
warms toward the tips by `0.24 + 0.44 * tip` in lightness and `+18 * tip` degrees.

## Water, floor and life

Water rows run hue 172 blue-green at the surface to 232 navy at the floor, lightness
0.255 down to 0.026, tinted 28 percent toward the theme background. Two to four tilted
shafts lift the background quadratically with height. The caustic net is
`sin(warp(x, y, t) * 3.4 + t * 0.6)` over a two octave warped field, strongest in the
upper third and cut off below two thirds of the column.

The sand band is the top 3 rows of the floor: ripple glyphs `.` `:` `~` on a hue 41
sand that sinks toward hue 30 and the water color below. Rocks sit on the floor line
in `o` `O` `&`. Eelgrass tufts of 2 to 4 blades rise 2 to 6 rows in `|` `/` `\` `)`,
picked by the per-row slope, and sway at `t > 0`.

Fish move as 2 to 4 schools, one heading and one shape each, members trailing the
leader with a hashed stagger. A ray silhouette crosses the pane on a 34 second cycle
at `t > 0`. Bubbles rise from 2 to 3 floor vents in wobbling columns, `.` young and
`o` old, and only once the clock runs.

## Glyph families

1. Coral, per form: `.` `:` `*` `%` `#` `&` `o` `O` `@` `~` `=` `|`
2. Caustic net and lit surface band: `-` `~` `=`
3. Seabed sand and ripples: `.` `:` `~`
4. Rocks: `o` `O` `&`
5. Eelgrass: `|` `/` `\` `)`
6. Plankton motes: `.` `` ` ``
7. Bubbles: `o` `.`
8. Fish: `><>` `><o>` `><=>` `><((>` and their mirrors, plus a 3 row ray

## Motion

Everything is a pure function of `t`, and the grown bed is cached on
`(seed, w, h, knobs)` so a frame never regrows the aggregate. `t = 0` is the static
frame: bubbles, the ray and grass sway are all gated on `t > 0`. Every per-bubble,
per-fish and per-tuft phase comes from `hash(seed, layer, index, salt)`, never from
the main rng.

## Knobs

| key | label | default | range |
| --- | --- | ---: | --- |
| COLONIES | coral colonies per 80 columns | 4 | 1 .. 12 |
| WALKERS | dla walker budget | 1400 | 40 .. 12000 |
| STICK | stickiness | 0.5 | 0.02 .. 1 |
| DEPTH | water column depth | 0.86 | 0.35 .. 0.98 |
| BUBBLES | bubble density | 0.7 | 0 .. 3 |
| FISH | fish per 80 columns | 7 | 0 .. 40 |
| SPREAD | colony spread | 0.86 | 0.2 .. 1 |
| CAUST | caustics and shafts | 0.7 | 0 .. 1.5 |
| GROW | colony size | 1.0 | 0.1 .. 1.5 |
| HUE | hue rotation deg | 0 | -180 .. 180 |
| SPEED | time scale | 1.0 | 0 .. 3 |

Positional order is the table order, starting at `args[4]`:
`reef [colonies] [walkers] [stickiness] [depth] [bubbles] [fish] [spread] [caustics]
[grow] [hue] [speed]`.

Colony hues cycle coral pink 348, orange 26, violet 288, gold 52, each with a
+-8 degree per-colony jitter and a +37 degree step once the cycle wraps.

## Knob sweep, 2000x1000, 2 s per run, dt 0.06, theme moss, release

| knob at max | frames | fps | avg ms | p50 ms | p99 ms | max ms | vs baseline |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| baseline | 166 | 82.9 | 12.06 | 11.83 | 14.73 | 16.13 | 1.00x |
| CAUST=1.5 | 154 | 76.9 | 13.01 | 12.99 | 13.51 | 13.59 | 1.08x |
| WALKERS=12000 | 163 | 81.1 | 12.33 | 11.89 | 16.19 | 17.00 | 1.02x |
| STICK=1 | 163 | 81.3 | 12.31 | 11.99 | 15.09 | 16.03 | 1.02x |
| SPEED=3 | 164 | 81.8 | 12.23 | 11.58 | 18.68 | 23.91 | 1.01x |
| SPREAD=1 | 164 | 81.8 | 12.22 | 11.57 | 16.27 | 58.64 | 1.01x |
| COLONIES=12 | 166 | 82.5 | 12.13 | 11.82 | 15.46 | 17.19 | 1.01x |
| BUBBLES=3 | 169 | 84.1 | 11.89 | 11.67 | 15.19 | 18.75 | 0.99x |
| FISH=40 | 171 | 85.3 | 11.73 | 11.53 | 14.07 | 34.54 | 0.97x |
| GROW=1.5 | 174 | 86.7 | 11.53 | 11.53 | 11.96 | 12.86 | 0.96x |
| HUE=180 | 175 | 87.5 | 11.43 | 11.46 | 11.87 | 12.24 | 0.95x |
| DEPTH=0.98 | 187 | 93.1 | 10.75 | 10.41 | 13.01 | 27.06 | 0.89x |

Worst knob: `CAUST=1.5`.

## Hotspots at CAUST=1.5, 116 frames, 57.7 fps

| layer | calls/frame | avg us | max us | share of frame |
| --- | ---: | ---: | ---: | ---: |
| caustics | 1.0 | 8806.9 | 22175.4 | 50.8% |
| water | 1.0 | 3915.3 | 18929.3 | 22.6% |
| seabed | 1.0 | 3047.4 | 17016.7 | 17.6% |
| coral_paint | 1.0 | 116.1 | 3952.5 | 0.7% |
| fish | 1.0 | 46.4 | 151.8 | 0.3% |
| bubbles | 1.0 | 13.0 | 76.1 | 0.1% |
| dla_grow | 1.0 | 0.0 | 0.0 | 0.0% |

Six timers attribute 92.1 percent of the frame. `dla_grow` reads as zero because the
bed is cached after the first frame; the whole cost of growth is paid once per
`(seed, w, h, knobs)`.

`perf/layer_coverage.sh` iterates `perf_sweep.rs` `NATIVE_MODES`, and reef left that
legacy list when it became a trait mode, so the coverage script reports 0 modes for a
`reef` filter. The hotspot table above is the attribution receipt; the registry gate
`every_registered_mode_that_renders_in_process_has_layer_timers` is what now proves
the timers fire.

Whole-process render, release: 80x24 p50 9.8 ms, min 7.4 ms. In-module `frame_cost` at
200x60 stays under the 6 ms release budget.

## Render commands

```bash
ASCII_GRID_W=100 ASCII_GRID_H=32 ./target/release/ascii-renderer 42 reef deep | sed 's/\x1b\[[0-9;]*m//g'
ASCII_GRID_W=100 ASCII_GRID_H=32 ASCII_T=9 ./target/release/ascii-renderer 7 reef moss | sed 's/\x1b\[[0-9;]*m//g'
perf/knob_sweep.sh reef 2000 1000 2
```

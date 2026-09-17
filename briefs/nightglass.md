# nightglass

Subject: rain on a window over a sleeping city, seen from inside. Behind the glass
a violet night sits over a warm band of light pollution, a stepped skyline carries
lit windows, and a field of bokeh lamps burns in warm amber, cool blue, rose and
blown-out white. On the glass: falling rain, a mist of condensation, a spray of
resolved beads, and one large drop — the subject — that refracts the brightest
cluster in the pane.

Every frame is a pure function of `(seed, dims, theme, t, knobs)`. There is no
retained state: element identities, animation phases and mist phases all come from
`hash(seed, layer, index, slot)`, a splitmix64 mix, so nothing simulates between
frames and nothing consumes a shared rng stream.

## What moves and why

| element | motion | period |
| --- | --- | --- |
| rain | wind-leaned lanes of dashes, each lane with its own speed, length and head phase | 2-4 s per lane crossing |
| running drops | slide to a per-drop stop height, fade in above the frame and out where they stop | 20-46 s; the subject runs for 96-156 s |
| droplet tracks | the chain of beads a drop leaves behind, jitter keyed to the pane row | follows the drop |
| beads | condensation breathes; beads swell, dry out and come back | 7-23 s, per-bead phase |
| mist | per-cell glitter whose phase and rate come from the cell's hash | 3-13 s |
| lamps | slow sway with parallax that grows with depth, plus a pulse; ~10% buzz like failing neon | 3-30 s, fast buzz at 21 rad/s |
| light pollution | the band and its district zones stay put; a passing car sweeps the pane | one pass per 17-43 s, absent the rest of the cycle |
| fog | two crossed sine fields drift on the glass | 50 s per period |

The subject drop is placed over a cluster the mode lays down for it (a tall neon
bar plus three small lamps), so the still frame has something to refract rather
than a blur. Knob rolls move every element, not just the shading: `LIGHTS`,
`BOKEH`, `DROPS`, `SLIDE` and `RAIN` change the population, and the tests require
400+ differing cells between any two of ten rolls.

## Knobs

| knob | range | default | what it changes |
| --- | --- | --- | --- |
| LIGHTS | 6..90 step 2 | 26 | lamp population; scaled down as the frame shrinks and as defocus grows |
| GLOW | 0..1.8 step 0.05 | 1.05 | halo gain and exposure |
| BOKEH | 0.35..2 step 0.05 | 1 | defocus scale: every scene element is sized in these units |
| DROPS | 0..2.2 step 0.1 | 1 | resolved beads and the mist glitter (0 is clear glass) |
| SLIDE | 0..2 step 0.1 | 1 | running drops, their tracks, and how much glass they wipe |
| RAIN | 0..2 step 0.1 | 0.9 | rain lanes that are wet, dash length and how hard a dash is lit |
| WIND | -1..1 step 0.05 | 0.3 | rain lean; past ~0.35 the dashes switch to `/` and `\` |
| HAZE | 0..1 step 0.05 | 0.42 | condensation fog: milky lift, strongest in the light |
| GLASS | 0..1.6 step 0.05 | 1 | lens power: refraction pull, gather, caustic and rim |
| TWINKLE | 0..1 step 0.05 | 0.55 | lamp pulse, neon buzz, and the frequency of glints on the glass |
| DRIFT | 0..2 step 0.05 | 0.6 | lamp sway with depth parallax |
| SWEEP | 0..1.5 step 0.05 | 0.8 | how often a passing light crosses outside |

Positional order: lights glow bokeh drops slide rain wind haze glass twinkle drift sweep.

## How a frame is built

Three O(W*H) passes over one reusable RGBA scratch, a bounded splat pass, then
bounded lens work:

1. `sky` — night gradient, light-pollution band with district zones, pane sheen, the passing car.
2. `city` — additive lamps (discs with a rim band about a cell thick, rain-smeared
   columns, blown-out points), then the two skyline ridges clip what stood behind
   them, then lit windows are drawn on top of the dark mass. Splat work is capped:
   a point lamp's bloom reach is bounded by a per-frame budget, so neither a large
   terminal nor a large defocus can make this pass unbounded.
3. `rain` — the dash field solved per cell: lane = floor of the wind-skewed
   coordinate, then one hash chain gives the lane its speed, length and head.
4. `compose` — one field cell plus its position become a drawable cell.
5. `beads`, `drops` — lenses that sample the composite through `comp`, so
   refraction moves light, fog and glyphs together. A drop pulls an inverted,
   shrunken sample, bent hardest near the rim; a lane-side shift keeps the
   displacement visible in a symmetric scene; a caustic burns in the lower half; a
   dark crown shades the top edge; a specular glint marks the crown.

The glyph channel carries two alphabets in one float: **positive is light** and
fills the cell with shade (`▒ ▓ █`, only for lamps bright enough to earn one),
**negative is rain** and draws a stroke (`╎ │ ┊`, or `/ \` under wind). Droplet
rims mark a sparse dither of `·`/`∙`, and the sheen pass stamps `· + ✦` glints. At
cell resolution a dotted ring around a disc reads as a rectangle, so discs carry
their edge in brightness alone.

Running drops wipe the glass: a bead already swept this cycle is not drawn, and
the drop lays its own small beads along the track, which is what makes a trail
read as a trail rather than as noise.

## Measured cost

`perf/knob_sweep.sh nightglass 2000 1000 2` (release, theme moss, dt 0.06):

| knob at max | fps | avg ms | vs baseline |
| --- | ---: | ---: | ---: |
| baseline | 33.0 | 30.31 | 1.00x |
| LIGHTS=90 | 27.5 | 36.38 | 1.20x |
| BOKEH=2 | 29.5 | 33.84 | 1.12x |
| SLIDE=2 | 31.7 | 31.51 | 1.04x |
| RAIN=2 | 31.8 | 31.40 | 1.04x |
| DROPS=2.2 | 32.2 | 31.02 | 1.02x |
| TWINKLE=1 | 32.3 | 30.94 | 1.02x |

The knob spread is 1.20x, and no knob is a cliff: the only knob that costs real
work is `LIGHTS`, which is meant to. Layer shares at the worst frame (LIGHTS=90,
2000x1000): city 31.1%, compose 19.3%, beads 18.4%, drops 14.6%, sky 9.2%, rain
4.6%, sheen 0.0%. `perf/layer_coverage.sh 400 120 3 moss nightglass` reports 7
layers, 97.6% attributed, not thin.

The receipt is `perf/results/nightglass.md`.

### Optimizations that landed, each measured at 2000x1000

1. District centres and the two skyline ridge tables are resolved once in `Look`
   instead of hashed per cell. `shade_roofs` runs over every cell, so hashing four
   times per cell there cost more than the whole sky pass; the ridge is now two
   table loads and two smoothsteps. Baseline 26.3 to 33.0 fps.
2. Lamp bloom is bounded two ways: a per-frame reach budget
   (`1.5 * cells / (4 * lamps)`, capped at 96 cells) for point lamps, whose halo
   fades to zero by construction so the cut is invisible; and the lamp count scales
   as `BOKEH^-2`, so a larger defocus moves the same amount of light around instead
   of spending more of it.
3. Beads are laid down until their lens area would pass 11% of the frame, so the
   `DROPS` knob costs the same on a 2000x1000 terminal as on an 80x24 one. This is
   what removed the previous worst knob: `BOKEH=2` went from 17.7 to 29.5 fps,
   with the bead layer from 27.6 ms to 6.7 ms.
4. `BOKEH=2` also used to blow up `drops` (7.4 ms) and `city` (8.0 ms); the caps
   above plus the drop radius clamp bring both under 6 ms.
5. Row passes shade on rayon above 20,480 cells; the splat pass and the lens
   passes stay serial because they write overlapping cells, and their work is
   bounded by the caps rather than by the frame.

The break-even for the row gate was taken from `prismata`'s measured value scaled
to this mode's cheaper body; the sweep above is the same code path at both sizes,
and 80x24 stays serial.

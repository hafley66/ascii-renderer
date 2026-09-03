# opus-1-quasicrystal

Subject: de Bruijn's multigrid construction, drawn as the rhombic tiling dual to N families of
parallel lines, so a quasiperiodic net with 5, 7, 8, 9, 11 or 13 fold symmetry fills the frame and
never repeats.

## The geometry

N line families, family m carrying the unit normal `e_m = (cos(2*pi*m/N + rot), sin(...))` and an
offset `gamma_m`. Line `(m, k)` is the set of points with `p . e_m = k + gamma_m`. Every cell of the
resulting arrangement gets an integer index vector `K`, and de Bruijn's dual map sends it to the
point `V = sum_m K_m e_m`. Each crossing of two lines `(j, kj)` and `(l, kl)` therefore dualizes to
one rhomb with corners `V`, `V + e_j`, `V + e_j + e_l`, `V + e_l`. N = 5 with the gammas summing to
zero is exactly Penrose; N = 8 is Ammann-Beenker; N = 12 is the Stampfli tiling. The renderer walks
crossings, not cells, so cost tracks the number of visible rhombs and never the arrangement size.

Each rhomb also carries a second coordinate, `V_perp = sum_m K_m f_m` with `f_m` the same directions
taken at a coprime multiple of the angle. That is the perpendicular space of the cut-and-project
picture. Vertices whose `|V_perp|` falls inside a small disk are the high-symmetry vertex class, and
they are the ones that get a star.

## What moves and why

Three motions, all slow, all pure functions of `t`:

| motion | mechanism | default period |
| --- | --- | --- |
| phason drift | every `gamma_m` slides at its own seeded rate | ~33 s per unit |
| lattice spin | `rot` advances, and `2*pi/N` is a symmetry, so the pattern closes | 48 s at N = 5 |
| light wave | a plane wave in dual space sets the shade level of each rhomb | ~26 s |

The drift is the interesting one. Sliding a gamma past an integer moves one line across a crossing,
and the dual tiling answers with a local rearrangement of three rhombs, the phason flip. Nothing
translates; the tiling reorganizes in place, everywhere at once, at a rate the eye reads as slow
crystalline breathing. The spin and the light wave are continuous and carry the frame between flips.

Secondary element: the lit worms. One line of the multigrid dualizes to a connected ladder of rhombs
crossing the whole tiling, so lighting a single line picks out a snake through a pattern that has no
straight lines of its own. `WORMS` of them burn at once in the accent color, each with a pulse
travelling down its length, and each jumps to a fresh line every dozen seconds.

Seed drives: symmetry order N (six choices, each a visibly different tiling), base hue, the gamma
offsets and their per family drift rates, the base rotation, the perp-space multiple, and the worm
line sequence.

## Glyph families

1. Rhomb interior shade ramp: ` `, `.`, `:`, `-`, `=`, `+`, `%`, `#`
2. Rhomb edges, chosen by the screen slope of the family direction: `-`, `/`, `\`, `|`
3. Star vertices of the high-symmetry class: `*`, `o`
4. Worm interiors and their travelling pulse: `~`, `+`, `#`
5. Worm rim accent: `,`, `` ` ``

## Knobs

| key | label | range | default | effect |
| --- | --- | --- | --- | --- |
| SPEED | time scale | 0..4 | 1.0 | multiplies every clock |
| HUE | hue rotation | 0..360 | 0 | rotates the class hues off the seed hue |
| SYM | symmetry order | 0..13 | 0 | 0 lets the seed choose from 5, 7, 8, 9, 11, 13 |
| SCALE | cells per edge | 5..40 | 9 | rhomb size, so the tile count |
| DRIFT | phason rate | 0..0.4 | 0.03 | gamma units per second |
| SPIN | lattice spin | 0..12 | 1.5 | degrees per second |
| SHADE | interior fill | 0..1 | 0.85 | how much of each rhomb is painted |
| WAVE | light wavelength | 2..40 | 11 | dual units per period of the light wave |
| WORMS | lit lines | 0..6 | 2 | how many worms burn at once |
| PULSE | pulse rate | 0..1 | 0.06 | worm pulse traversals per second |
| STARS | star threshold | 0..1 | 0.3 | perp radius admitting a vertex to the star class |
| BREATH | zoom breath | 0..0.35 | 0.12 | slow outward zoom amplitude |

Positional order: `speed hue sym scale drift spin shade wave worms pulse stars breath`.

## Render commands

```bash
ASCII_GRID_W=110 ASCII_GRID_H=36 ./target/release/ascii-renderer 7 opus-1-quasicrystal moss | sed 's/\x1b\[[0-9;]*m//g'
ASCII_GRID_W=110 ASCII_GRID_H=36 ASCII_T=12 ./target/release/ascii-renderer 7 opus-1-quasicrystal moss | sed 's/\x1b\[[0-9;]*m//g'
```

## Perf receipt

Filled in from `perf/knob_sweep.sh opus-1-quasicrystal 2000 1000 1`.

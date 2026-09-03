# opus-1-quasicrystal

Subject: de Bruijn's multigrid construction, drawn as the rhombic tiling dual to N families of
parallel lines, so a quasiperiodic net that never repeats fills the frame and reorganizes itself in
place while a light wave expands through it.

## The geometry

N line families, family m carrying the unit normal `e_m = (cos(pi*m/N + rot), sin(...))` and an
offset `gamma_m`. Line `(m, k)` is the set of points with `p . e_m = k + gamma_m`. Every cell of the
arrangement gets an integer index vector `K`, and de Bruijn's dual map sends it to the point
`V = sum_m (K_m + gamma_m) e_m`. Each crossing of two lines `(j, kj)` and `(l, kl)` therefore
dualizes to one rhomb with corners `V`, `V + e_j`, `V + e_j + e_l`, `V + e_l`. N = 4 is the
octagonal Ammann-Beenker tiling, N = 5 is decagonal Penrose, N = 6 dodecagonal. The renderer walks
crossings, not cells, so cost tracks visible rhombs and never the size of the arrangement. The
crossing enumeration is exact: for each `kj` the visible span of line j inside the cull disk gives a
closed interval of `kl`, so no crossing is tested and rejected.

Carrying `gamma_m` inside the dual sum is what makes drift continuous. When a `gamma` crosses an
integer every `K_m` shifts by one and the raw dual point jumps by `e_m`; the `+ gamma_m` term
cancels that jump exactly, so the picture only ever flips locally.

Each rhomb also carries `V_perp = sum_m (K_m + gamma_m) f_m`, with `f_m` the same directions taken
at an odd multiple `q` coprime to `2N`. That multiple makes `sum_m f_m e_m^T` vanish, so `V_perp`
is bounded: it is the perpendicular space of the cut-and-project picture, and it drives both the
per tile tone and the star vertex class.

Drift rates are projected onto the phason directions at build time (`sum_m rate_m e_m = 0`), so the
pattern reorganizes without translating.

## What moves and why

| motion | mechanism | default period |
| --- | --- | --- |
| phason drift | every `gamma_m` slides at its own seeded rate | 33 s per unit |
| lattice spin | `rot` advances; `pi/N` is a symmetry, so the pattern closes | 24 s at N = 5 |
| light wave | radial wave in dual space sets how much material each tile holds | 26 s |
| zoom breath | scale swells outward and returns | 37 s |

The drift is the interesting one. Sliding a gamma past an integer moves one line across a crossing,
and the dual tiling answers with a local rearrangement of rhombs, the phason flip. Nothing
translates; the tiling reorganizes in place, everywhere at once, at a rate the eye reads as slow
crystalline breathing. The light wave is a mass wave, not a tint: tiles in the trough empty out to
bare wireframe and tiles at the crest fill solid, so bands of material expand out of a bright core
and the wireframe voids between them show the raw net.

Secondary element: the lit worms. One line of the multigrid dualizes to a connected ladder of rhombs
crossing the whole tiling, so lighting a single line picks out a snake through a pattern that has no
straight lines of its own. `WORMS` of them burn at once in the accent color, each with a pulse
travelling down its length, and each jumps to a fresh line every 13 seconds.

Seed drives: symmetry order N from `[4, 5, 5, 6, 7, 9]`, base hue, the gamma offsets and their per
family drift rates, the base rotation, the light wave heading, the perpendicular space multiple,
and the worm line sequence.

## Glyph families

1. Rhomb interior mass ramp: ` `, `.`, `,`, `:`, `;`, `%`, `#`, `@`
2. Rhomb edges, chosen by the screen slope of the family direction: `-`, `/`, `\`, `|`
3. Star vertices of the high-symmetry perp class: `*`, `o`
4. Worm interiors and their travelling pulse: `~`, `#`, `%`
5. Worm rim dither: `,`

The mass ramp shares no glyph with the edge set, so the net stays readable through a filled tile.

## Knobs

| key | label | range | default | effect |
| --- | --- | --- | --- | --- |
| SPEED | time scale | 0..4 | 1.0 | multiplies every clock |
| HUE | hue rotation | 0..360 | 0 | rotates the class hues off the seed hue |
| SYM | symmetry order | 0..13 | 0 | 0 lets the seed choose; 3 and up forces N |
| SCALE | cells per edge | 5..40 | 12 | rhomb size, so the tile count |
| DRIFT | phason rate | 0..0.4 | 0.03 | gamma units per second |
| SPIN | spin deg/sec | 0..12 | 1.5 | lattice rotation rate |
| SHADE | interior fill | 0..1 | 0.85 | scales the mass ramp; 0 leaves bare wireframe |
| WAVE | light wavelength | 0.6..24 | 1.8 | dual units between mass bands |
| WORMS | lit lines | 0..6 | 2 | how many worms burn at once |
| PULSE | pulse rate | 0..1 | 0.06 | worm pulse traversals per second |
| STARS | star threshold | 0..1 | 0.3 | perp radius admitting a vertex to the star class |
| BREATH | zoom breath | 0..0.35 | 0.12 | outward zoom amplitude, never inward |

Positional order: `speed hue sym scale drift spin shade wave worms pulse stars breath`.

`MAX_RHOMBS` is a floor on the effective scale, not a knob: if the requested scale would put more
than 260k rhombs on the grid the scale is raised until it fits, so a huge grid at `SCALE=5` degrades
to coarser tiles instead of stalling.

## Render commands

```bash
ASCII_GRID_W=110 ASCII_GRID_H=36 ./target/release/ascii-renderer 7 opus-1-quasicrystal moss | sed 's/\x1b\[[0-9;]*m//g'
ASCII_GRID_W=110 ASCII_GRID_H=36 ASCII_T=12 ./target/release/ascii-renderer 7 opus-1-quasicrystal moss | sed 's/\x1b\[[0-9;]*m//g'
ASCII_GRID_W=110 ASCII_GRID_H=36 ASCII_T=30 ./target/release/ascii-renderer 3 opus-1-quasicrystal ember | sed 's/\x1b\[[0-9;]*m//g'
```

Frame cost, release, 200x60, 200 frames: avg 0.073 ms, worst 0.095 ms.

## Perf receipt

`perf/knob_sweep.sh opus-1-quasicrystal 2000 1000 1`, full output in
`perf/results/opus-1-quasicrystal.md`.

| knob at max | frames | fps | avg ms | p50 ms | p99 ms | max ms | vs baseline |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| baseline | 100 | 99.5 | 10.05 | 9.93 | 11.42 | 11.89 | 1.00x |
| SYM=13 | 85 | 84.4 | 11.84 | 11.75 | 12.69 | 12.76 | 1.18x |
| WAVE=40 | 94 | 93.7 | 10.67 | 10.18 | 16.98 | 17.46 | 1.06x |
| STARS=1 | 97 | 96.9 | 10.32 | 10.18 | 11.84 | 11.98 | 1.03x |
| SHADE=1 | 99 | 98.8 | 10.12 | 10.04 | 11.34 | 11.51 | 1.01x |
| DRIFT=0.4 | 100 | 99.1 | 10.10 | 9.92 | 11.53 | 19.28 | 1.00x |
| SPIN=12 | 100 | 99.2 | 10.08 | 10.01 | 11.09 | 11.75 | 1.00x |
| WORMS=6 | 100 | 99.5 | 10.05 | 9.87 | 11.86 | 14.71 | 1.00x |
| HUE=360 | 100 | 99.9 | 10.01 | 9.92 | 11.46 | 11.57 | 1.00x |
| PULSE=1 | 101 | 100.3 | 9.97 | 9.90 | 11.09 | 11.75 | 0.99x |
| BREATH=0.35 | 103 | 102.4 | 9.76 | 9.70 | 10.66 | 11.07 | 0.97x |
| SPEED=4 | 104 | 103.2 | 9.69 | 9.51 | 11.34 | 11.49 | 0.96x |
| SCALE=40 | 197 | 196.1 | 5.10 | 4.98 | 6.34 | 6.36 | 0.51x |

Worst knob `SYM=13` holds 84.4 fps at 2000x1000, 2.8x the 30 fps bar.

| layer | calls/frame | avg us | max us | share of frame |
| --- | ---: | ---: | ---: | ---: |
| shade | 1.0 | 5546.3 | 6384.7 | 46.8% |
| solve | 1.0 | 2565.0 | 2723.0 | 21.6% |
| edges | 1.0 | 2226.9 | 2577.7 | 18.8% |
| clear | 1.0 | 398.3 | 479.7 | 3.4% |
| stars | 1.0 | 120.8 | 145.2 | 1.0% |
| worms | 1.0 | 62.9 | 80.2 | 0.5% |

Six layers, 92.1% of the frame inside timers.

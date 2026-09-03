# opus-2-quasicrystal

Subject: a quasiperiodic crystal built by de Bruijn's multigrid, N families of evenly spaced parallel
lines laid over each other at equal angles, with faceted growth fronts blooming outward from the
nucleus and dissolve fronts trailing behind them.

What moves and why: three motions, all of them the geometry itself.

1. The growth band. A leading front sweeps from the nucleus out past the corners over CYCLE seconds
   and restarts; a dissolve front trails it by BAND of the sweep. Between the two the lattice is
   fully formed, ahead of the leading edge it is a vapor of loose threads, behind the dissolve edge
   it thins back out. The front is not a circle. Its radius is the polygonal norm `max_j |p . n_j|`
   taken over the same line normals that build the lattice, so the crystal grows with flat facets
   square to its own lattice planes and the facet count equals the symmetry order. FACET blends that
   polygon against a true round front. BLOOMS runs two or three fronts a phase apart so the frame is
   never empty and the rims interfere.
2. The turn. The whole pencil of lines rotates at TURN degrees per second. A rotation of 180/N maps
   the family set back onto itself, so at the default the composition returns to itself on roughly a
   45 second beat of wall clock and never repeats inside it.
3. The phason drift. Each family's offset gamma_j creeps at PHASON per second. In a quasicrystal a
   shift of the offsets is not a translation; it flips individual tiles from one type to another in
   place. Regions reorganize without anything sliding, which is the motion that separates a
   quasicrystal from a lattice.

Rates are sized for the demo clock, which advances t by 0.06 per frame at about 60 frames a second.
The 130 second CYCLE is therefore a 36 second bloom on the wall clock, and one frame at the default
dt changes about 1 percent of the cells.

Everything else is derived per cell from one incremental pass over the N families. The pass carries,
per family, the line index k_j and the fraction to the next line. The de Bruijn index sum of the k
tuple picks the tile class and its hue, so the class is flat across a whole mesh cell; a linear hash
of the same tuple picks the shade inside the class; the smallest fraction gives the web line and its
direction; the two smallest together give a node.

Glyph families:
1. Mosaic tile interiors, ramp indexed by class and shade. Solid `.` `:` `;` `=` `*` `8` `#` `%`,
   stipple `.` `,` `;` `:` `o` `O` `&` `@`, hatch `` ` `` `'` `"` `^` `~` `=` `%` `#`
2. Web lines, one glyph per family direction: `-` `\` `|` `/`
3. Nodes where two families cross: `+` `x`
4. Growth rim on the leading facet `@` `#` `%`, dissolve rim `:` `.`
5. Vapor threads and loose grains ahead of the front: the web glyphs dimmed, plus `.` `` ` `` `,`

Seed-driven choices (four, so two seeds never look alike):
- symmetry order N, drawn from 4, 5, 6, 7 weighted toward 5, when FOLDS is left at 0
- base hue and the fan of class hues across it
- glyph ramp: solid, stipple, or hatch
- nucleus offset from center, the initial gammas and their drift signs, and the phase of the bloom
  at t=0, so every seed opens on a different stage of the cycle

Knobs, in positional order:
- SPEED: clock multiplier (default 1)
- CYCLE: seconds for one growth sweep (default 130)
- FOLDS: symmetry order, 0 lets the seed pick (default 0, max 7)
- SCALE: line spacing in rows (default 9)
- LINEW: web half width in screen cells, corrected per family so every direction draws the same
  thickness and thinned automatically as the fold count rises (default 0.85)
- BAND: crystal band width, fraction of the sweep (default 0.72)
- EDGE: front rim softness (default 0.025)
- TURN: lattice rotation, degrees per second (default 0.22)
- PHASON: offset drift per second (default 0.012)
- FACET: 0 round front, 1 full polygon (default 0.85)
- DENSITY: mosaic ink inside the band (default 0.8)
- DUST: vapor thread and grain density outside it (default 0.3)
- HUE: hue spread across tile classes, degrees (default 46)
- GLOW: front rim brightness (default 0.85)
- ASPECT: columns per row (default 2)
- PHASE: bloom start offset, for picking a still frame (default 0)
- BLOOMS: growth fronts in flight (default 2)

Render commands:
```bash
ASCII_GRID_W=110 ASCII_GRID_H=36 ./target/release/ascii-renderer 7 opus-2-quasicrystal moss | sed 's/\x1b\[[0-9;]*m//g'
ASCII_GRID_W=110 ASCII_GRID_H=36 ASCII_T=45 ./target/release/ascii-renderer 7 opus-2-quasicrystal moss | sed 's/\x1b\[[0-9;]*m//g'
```

Measured frame time, release, 200x60, 200 frames: avg 0.206 ms, worst 0.239 ms.

## Perf receipt

`perf/knob_sweep.sh opus-2-quasicrystal 2000 1000 2`

| knob at max | fps | avg ms | p50 ms | p99 ms | vs baseline |
| --- | ---: | ---: | ---: | ---: | ---: |
| baseline | 35.3 | 28.35 | 27.98 | 31.73 | 1.00x |
| FOLDS=7 | 30.8 | 32.52 | 32.44 | 33.45 | 1.15x |
| DUST=1 | 32.2 | 31.07 | 30.99 | 31.59 | 1.10x |
| BAND=1 | 33.4 | 29.90 | 29.86 | 30.46 | 1.05x |
| BLOOMS=3 | 33.8 | 29.55 | 29.58 | 30.11 | 1.04x |
| HUE=180 | 35.1 | 28.51 | 28.52 | 29.01 | 1.01x |
| TURN=6 | 35.1 | 28.50 | 28.47 | 29.40 | 1.01x |
| GLOW=1.5 | 35.2 | 28.42 | 28.43 | 28.90 | 1.00x |
| PHASON=0.4 | 35.2 | 28.40 | 28.29 | 29.23 | 1.00x |
| PHASE=1 | 35.3 | 28.36 | 28.41 | 28.97 | 1.00x |
| DENSITY=1 | 35.3 | 28.34 | 28.03 | 32.43 | 1.00x |
| CYCLE=600 | 35.5 | 28.14 | 28.02 | 28.97 | 0.99x |
| SPEED=4 | 36.5 | 27.39 | 27.36 | 28.54 | 0.97x |
| ASPECT=4 | 39.8 | 25.12 | 25.10 | 25.53 | 0.89x |
| FACET=1 | 40.7 | 24.55 | 24.46 | 25.16 | 0.87x |
| LINEW=2.5 | 41.3 | 24.21 | 24.08 | 26.95 | 0.85x |
| SCALE=30 | 43.5 | 23.01 | 22.97 | 23.71 | 0.81x |
| EDGE=0.3 | 82.8 | 12.07 | 11.94 | 13.43 | 0.43x |

Worst knob FOLDS=7 at 30.8 fps, hotspots:

| layer | avg us | share of frame |
| --- | ---: | ---: |
| lattice | 31782 | 95.6% |
| clear | 455 | 1.4% |
| fronts | 38 | 0.1% |
| nucleus | 1 | 0.0% |

The lattice pass started at 12.4 fps on the worst knob. What bought the 2.5x, in order of size:
1. A per-frame lookup table over the radius for the whole front state. formed, both rim intensities,
   the vapor gate and the moat flag are functions of one scalar, so they cost a table load instead of
   two smoothsteps and a clamp per bloom per cell.
2. Folding the second full-frame pass back into the first, which also removed an 8 MB scratch buffer
   written and read once per frame, and let the rim, moat and blank cells return before the family
   loop runs at all.
3. Const-generic monomorphization on the fold count, so the per-family arrays are fixed size.
4. Replacing both radius norms with per-row upper envelopes of their defining lines: the polygonal
   norm from the 2N lattice normals, the round norm from 32 evenly spaced directions with a
   sec(pi/32) correction. Both collapse an N-long loop and a square root into one compare and one
   fused multiply-add per cell. `envelope_matches_brute_force` pins the first to 1e-3.
5. Maintaining the index sum and the tile hash incrementally instead of keeping a k array.
6. One hash per blank cell, sliced into three bit fields, instead of three hashes.

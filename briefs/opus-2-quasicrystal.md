# opus-2-quasicrystal

Subject: a quasiperiodic crystal built by de Bruijn's multigrid, N families of evenly spaced parallel
lines laid over each other at equal angles, with a faceted growth front blooming outward from the
nucleus and a dissolve front trailing behind it.

What moves and why: three motions, all of them the geometry itself.

1. The growth band. A leading front sweeps from the nucleus out past the corners over CYCLE seconds
   and wraps; a dissolve front trails it by BAND of the sweep. Between the two the lattice is fully
   formed, ahead of the leading edge it is loose dust, behind the dissolve edge it thins back out.
   The front is not a circle. Its radius is the polygonal norm `max_j |p . n_j|` taken over the same
   line normals that build the lattice, so the crystal grows with flat facets square to its own
   lattice planes and the facet count equals the symmetry order.
2. The turn. The whole pencil of lines rotates at TURN degrees per second. A rotation of 180/N
   degrees maps the family set back onto itself, so the composition returns to itself on a
   200-second beat at the default and never repeats inside it.
3. The phason drift. Each family's offset gamma_j creeps at PHASON per second. In a quasicrystal a
   uniform shift of the offsets is not a translation; it flips individual tiles from one type to
   another in place. Regions reorganize without anything sliding, which is the motion that separates
   a quasicrystal from a lattice.

Everything else is derived per cell from one incremental pass over the N families. For each cell the
pass carries, per family, the line index k_j and the fraction to the next line. The k tuple hashes to
a tile type, which picks the mosaic glyph and the hue; the smallest fraction gives the web line and
its direction; the two smallest together give a node; the largest |v_j| gives the facet radius.

Glyph families:
1. Mosaic tile interiors, ramp indexed by tile type and formation: `.` `:` `-` `=` `+` `*` `#` `%`,
   with a stipple variant `.` `,` `;` `o` `O` `&` `@` and a hatch variant `` ` `` `'` `^` `~` `8` `#`
2. Web lines, one per family direction: `-` `\` `|` `/`
3. Nodes where two families cross: `+` `x` `*`
4. Growth rim on the leading facet: `#` `%` `@`, dissolve rim: `:` `.`
5. Loose dust ahead of the front and in the dissolved core: `.` `` ` `` `,`

Seed-driven choices (four, so two seeds never look alike):
- symmetry order N, drawn from 5, 7, 9, 4, 6, 8 when the FOLDS knob is left at 0
- base hue and the per-family hue fan
- glyph ramp: solid, stipple, or hatch
- nucleus offset from center, initial gammas, and the phase of the growth band at t=0

Knobs, in positional order:
- SPEED: clock multiplier (default 1)
- CYCLE: seconds for one growth sweep (default 34)
- FOLDS: symmetry order, 0 lets the seed pick (default 0)
- SCALE: line spacing in rows (default 6.5)
- LINEW: web half width, fraction of spacing (default 0.1)
- BAND: crystal band width, fraction of the sweep (default 0.72)
- EDGE: front rim softness (default 0.07)
- TURN: lattice rotation, degrees per second (default 0.9)
- PHASON: offset drift per second (default 0.05)
- FACET: 0 round front, 1 full polygon (default 0.85)
- DENSITY: mosaic fill inside the band (default 0.55)
- DUST: loose grain density outside it (default 0.3)
- HUE: hue spread across tile types, degrees (default 46)
- GLOW: front rim brightness (default 0.85)
- ASPECT: columns per row (default 2)

Render commands:
```bash
ASCII_GRID_W=110 ASCII_GRID_H=36 ./target/release/ascii-renderer 7 opus-2-quasicrystal moss | sed 's/\x1b\[[0-9;]*m//g'
ASCII_GRID_W=110 ASCII_GRID_H=36 ASCII_T=12 ./target/release/ascii-renderer 7 opus-2-quasicrystal moss | sed 's/\x1b\[[0-9;]*m//g'
```

Perf receipt: pending.

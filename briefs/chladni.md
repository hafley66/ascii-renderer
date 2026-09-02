# chladni

Subject: a square metal plate driven from its center, with sand collecting along the nodal lines of the standing wave, and the drive frequency stepping through resonances so the sand figure dissolves and re-forms.

What moves and why: the resonance sequence is a seeded list of integer mode pairs (n, m). The clock holds each figure for DWELL seconds, then glides the pair to the next one over GLIDE seconds by sweeping n and m as real numbers, so the nodal net bends continuously from one figure into the next. Sand is dense where the plate is still (small |f|) and thins away from the node lines. At the antinodes the plate shakes hardest, so loose grains hop there: a hashed flicker at FLICKER changes per second, with density scaled by local amplitude. A drive dot pulses at the center.

Glyph families:
1. Sand ridge core on the node line: `#`, `%`
2. Sand slope beside the ridge: `+`, `=`, `:`
3. Loose grains hopping at the antinodes: `.`, `` ` ``, `~`
4. Plate rim: `+`, `-`, `|`
5. Center drive and its ring: `@`, `o`, `*`

Knobs:
- DWELL: seconds a figure holds (default 3)
- GLIDE: seconds to sweep between figures (default 2)
- ORDER: highest mode number in the pairs (default 7)

Added while building:
- SAND: nodal line width in plate units (default 0.02)
- SHAKE: loose grain density at the antinodes (default 0.3)
- FLICKER: grain hops per second (default 8)
- MARGIN: cells between plate rim and grid edge (default 1)
- LABEL: show the current mode pair inside the plate, 0 or 1 (default 1)
- ASPECT: plate columns per row (default 2)

Positional order: dwell glide order sand shake flicker margin label aspect.

Measured frame time, release, 200x60, 200 frames: avg 0.150 ms, worst 1.075 ms.

Render commands:
```bash
ASCII_GRID_W=110 ASCII_GRID_H=36 ./target/release/ascii-renderer 42 chladni moss | sed 's/\x1b\[[0-9;]*m//g'
ASCII_GRID_W=110 ASCII_GRID_H=36 ASCII_T=5 ./target/release/ascii-renderer 42 chladni moss | sed 's/\x1b\[[0-9;]*m//g'
```

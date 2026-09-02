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

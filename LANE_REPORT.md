# tree-of-life-5 lane report

Geometry: the tree lives in the Klein (projective) disk, where geodesics are straight
chords. Branches are recursive straight segments grown from the trunk; near the rim they
cluster, giving the hyperbolic perspective foreshortening. A rotating Euclidean spin moves
the whole scene per frame, and a rotating diameter geodesic seam sweeps the disk, splitting
it into an ethereal half (flat hollow glyphs, horocycle rings, motes riding rays) and a
living half (slope bark glyphs, swaying leaves, wind).

Knobs (registry keys, read via param_f32): DEPTH, SPREAD, LEN, SPIN, SEAM, MOTES, GLOW,
RINGS, WIND, SPEED. CLI positionals args[4..] = depth, spread, len, spin, seam, motes.

Measured frame time at 200x60: 0.404ms (under the 6ms budget); 200 frames, zero rng, zero
allocation per frame, geometry cached in a thread_local keyed by (w, h, seed, geometry knobs).
Crown segments at depth 8: 395 (test crown_has_hundreds_of_segments asserts > 300).

Render commands:
  ASCII_GRID_W=110 ASCII_GRID_H=36 ./.target-lane/release/ascii-renderer 42 tree-of-life-5 moss | sed 's/\x1b\[[0-9;]*m//g'
  ASCII_GRID_W=110 ASCII_GRID_H=36 ASCII_T=5 ./.target-lane/release/ascii-renderer 42 tree-of-life-5 moss | sed 's/\x1b\[[0-9;]*m//g'

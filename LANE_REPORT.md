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


---

# Lane Report: tree-of-life-6

Hyperbolic geometry in the Poincaré upper half-plane $\mathbb{H}^2$ using horocycle root coordinates, SL(2,R) parabolic and dilation flows, and a {5,4} hyperbolic geodesic lattice.
The canvas splits into an ethereal left half (pulsing glyphs, floating motes) and living right half (organic bark, breathing foliage).

## Knobs
- `DEPTH`: recursive tree branch depth (default: 7.0, range 3.0-10.0)
- `SPREAD`: branch angular divergence (default: 0.72, range 0.2-1.4)
- `ZOOM`: SL(2,R) hyperbolic dilation oscillation (default: 0.45, range 0.0-2.0)
- `FLOW`: SL(2,R) parabolic horocycle horizontal drift (default: 0.35, range -1.5-1.5)
- `WARP`: projection distortion strength (default: 0.25, range 0.0-1.0)
- `SPEED`: overall animation time multiplier (default: 1.0, range 0.05-4.0)
- `MOTES`: ethereal particle count (default: 50.0, range 0.0-300.0)
- `GLOW`: ethereal pulse intensity (default: 0.85, range 0.0-1.0)
- `LATTICE`: hyperbolic lattice visibility (default: 1.0, range 0.0-1.0)
- `SEAM`: dynamic wave speed across the dividing curve (default: 0.08, range -0.6-0.6)

## Performance
- Measured frame time at 200x60: ~1.621ms (< 6ms target).

## Render Commands
- `ASCII_GRID_W=110 ASCII_GRID_H=36 ./.target-lane/release/ascii-renderer 42 tree-of-life-6 moss | sed 's/\x1b\[[0-9;]*m//g'`
- `ASCII_GRID_W=110 ASCII_GRID_H=36 ASCII_T=5 ./.target-lane/release/ascii-renderer 42 tree-of-life-6 moss | sed 's/\x1b\[[0-9;]*m//g'`

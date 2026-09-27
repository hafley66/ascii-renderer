# breathhex prompt sequence

Verbatim user prompts and steering messages, in the order they were given.

## 1. User prompt

"i want highly dynamic breathing hexagon assymetric non euclidaen tiling"

## 2. Steering message (round 3)

"Ask it for breathing hexagons again, but don't have them be inside of a trapped circle."

## 3. Steering message (current round)

"This is a transport readiness probe..." (transport handshake only, no repository
inspection; response token "boop")

## 4. Steering message (current round, task body)

Earlier rounds put the tiling inside a Poincare disk: a circle with empty space
around it. This piece must not have that.

The tiling fills the entire terminal, edge to edge. There is no visible boundary
circle, no rim, no horizon, and no empty background around the tiling.

Ways to do it (offered, take any or a better one):

- Upper half-plane model. The screen is a window into H^2 that scrolls and zooms.
  Cells shrink toward one edge, and that edge is off-screen.
- Band model. A strip that runs past both screen edges and pans along the
  geodesic axis.
- Oversized Poincare disk. The disk is far larger than the screen, so every corner
  lies inside it. Breathing is a hyperbolic translation, a Mobius drift, rather
  than scaling.
- Non-hyperbolic. Spherical or elliptic hexagons, or a Euclidean hex grid pushed
  through a time-varying conformal map such as exp, a power, or a Joukowski map.
  It must still read as hexagons that breathe.

Asymmetric means breathing is not uniform: cells or regions inhale out of phase,
as a wave, or with rotated phase.

Scope: create src/modes/_133_breathhex.rs (mode breathhex) and
art/prompts/133_breathhex.md, and add its snapshots. Add a mod line and a
registry.add line to src/modes/mod.rs. Append breathhex to the end of the
CLAUDE.md modes list. Read no other file under src/modes/ except mod.rs. Do not
read tests/snapshot_modes.rs.

Gates for every save point: CARGO_BUILD_JOBS=4 cargo insta test --accept --
modes::_133_breathhex must pass; every row and column of the snapshot must
contain tiling with no blank border region; the release frame_cost test stays
under 6 ms at 200x60; commit with "feat(modes): breathhex - <what changed>" at
every working step, at least 3 commits.

## Design chosen

Non-hyperbolic option: a Euclidean hex grid pushed through a time-varying
conformal map w = exp(GAIN * (q + CURVE * e^{i*psi} * q^2)) whose shrinking
direction is aimed at an off-screen corner, so no boundary, rim, or horizon is
ever visible. Breathing is per-cell: each hexagon's wall radius follows
1 + BREATH * sin(TAU*RATE*t + ASYM*(site dot wave)/rh + seeded jitter), a
directional phase wave plus rotated per-cell offsets.
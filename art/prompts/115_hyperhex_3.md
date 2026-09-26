# 115 hyperhex-3 prompt sequence

User messages, verbatim, in order:

1. > how can i use it to go make art in a worktree taint and asll but asked not to look too hard at others
2. > try not to look at latest ones in psat 3 days, try to keep it efficient, have fun be avant garde but try not to be too influence buy the previous ones
3. > thats okay only ur stuff matters skip not ur stuff
4. > dont want u tainted
5. > i want highly dynamic breathing hexagon assymetric non euclidaen tiling

Brief: `briefs/boop-start.md` (this worktree's assigned brief).
6. > make beautiful art with ds41 art 1, commit at eachsave point, in fact, commit and save its work aand then copy it and tell it to work on the new copy to improve/change it, cooperate and making something _you_ like
7. > really stretch it and avoid boring.

Forked from 114 hyperhex-2 at 6b4a939.
8. > sure yes

## Round three (worktree ds41-art-2, branch from 68bbed0)

Coordinator direction, verbatim:

- > really stretch it and avoid boring
- > cooperate and making something _you_ like

Round-three changes:

- Corrected the fundamental hexagon radius to the true {6,4} Euclidean
  circumradius `tanh(acosh(cos(pi/4)/sin(pi/6))/2)`, and made the hexagon
  regular, so the reflection walk produces a real tiling instead of an
  overlapping thicket. `skew` now rigidly rotates the seed tile.
- Added a four-act slow cycle (`ACT`, default 40 s): calm, heartbeat, fracture,
  reassembly, with partition-of-unity crossfades.
- Contagion wound: two wave fronts spread from opposite sides of the reflection
  graph during the heartbeat and interfere where they meet.
- Ghost dual layer (`GHOST`): a faint web joining neighbouring cell centres,
  sampled one second behind the main view.
- Star horizon (`HORIZON`): drift stars outside the disk, plus an eclipse-corona
  rim bright where the cells are densest.
- Colour carries depth: warm at the centre, cold at the rim; the wound lands in
  an off-palette hue that appears nowhere else.

## Perf

Release bench, 60 frames t = 0..60, min of 8 rounds
(`CARGO_BUILD_JOBS=4 cargo test --release -- --ignored --nocapture modes::_115_hyperhex_3`):

| size | before ms/frame | after ms/frame |
|------|-----------------|----------------|
| 200x60 | 2.120 | 1.409 |
| 400x120 | 6.164 | 3.862 |

Hottest layer before: `fill` 1.014 ms @200x60 (48% of the frame), then `edges`
0.632 and `ground` 0.183. After: `edges` ~0.61, `fill` 0.528, `ground` 0.149,
`topology` ~0.

Changes:
- Cached the reflection-walk tiling in a per-thread `Rc<Vec<Tile>>` keyed by
  `(depth, skew, min_edge, scale)`; the `topology` layer now builds once per
  knob set.
- Replaced per-pixel `point_in_poly` fill with per-row even-odd crossing spans.
  Parity counts crossings strictly greater than the sample x, so the two-crossing
  fast path is `(xs[0] > px) != (xs[1] > px)` — a `px > xs[0]` span test is wrong
  when `px` lands exactly on a crossing.
- Hoisted each tile's `bright = 0.55 + 0.45*sin(band*TAU)` out of the per-pixel
  loop, and moved the `shown < 0.05` skip ahead of the `lerp_color`.
- `draw_geodesic` computed `cos`/`sin` twice per sample for the x and y
  projections; compute them once.
- Hoisted the constant ground dust foreground and precomputed the column/row
  radial terms.

Output is byte-identical: the 60-frame FNV checksum over plain chars is unchanged
(`3e9f131c1ba66867` @200x60, `5b459c0f3e4b3752` @400x120).

Session comparison (base `f08be93` vs HEAD, alternating in one session,
`--test-threads=1`, min of 3 runs):

| size | base ms/frame | HEAD ms/frame | speedup |
|------|---------------|---------------|---------|
| 200x60 | 1.959 | 1.386 | 1.41x |
| 400x120 | 5.689 | 3.800 | 1.50x |


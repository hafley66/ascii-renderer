# 114 hyperhex-2 prompt sequence

User messages, verbatim, in order:

1. > how can i use it to go make art in a worktree taint and asll but asked not to look too hard at others
2. > try not to look at latest ones in psat 3 days, try to keep it efficient, have fun be avant garde but try not to be too influence buy the previous ones
3. > thats okay only ur stuff matters skip not ur stuff
4. > dont want u tainted
5. > i want highly dynamic breathing hexagon assymetric non euclidaen tiling

Brief: `briefs/boop-start.md` (this worktree's assigned brief).
6. > make beautiful art with ds41 art 1, commit at eachsave point, in fact, commit and save its work aand then copy it and tell it to work on the new copy to improve/change it, cooperate and making something _you_ like
7. > really stretch it and avoid boring.

Forked from 113 hyperhex at f82eda5.

## Perf

Release bench, 60 frames t = 0..60, min of 8 rounds
(`CARGO_BUILD_JOBS=4 cargo test --release -- --ignored --nocapture modes::_114_hyperhex_2`):

| size | before ms/frame | after ms/frame |
|------|-----------------|----------------|
| 200x60 | 0.634 | 0.533 |
| 400x120 | 1.687 | 1.324 |

Hottest layer before: `edges` 0.351 ms @200x60 (55% of the frame), then `fill`
0.155 and `ground` 0.067. After: `edges` unchanged (~0.37), `fill` 0.101,
`ground` 0.044, `topology` ~0.

Changes:
- Cached the reflection-walk tiling in a per-thread `Rc<Vec<Tile>>` keyed by
  `(seed, depth, skew, min_edge, scale)`; the cheap `topology` layer now builds
  once per knob set instead of every frame.
- Replaced the per-pixel `point_in_poly` fill scan with per-row even-odd
  crossing spans. The parity must count crossings strictly greater than the
  sample x; using a `px > xc[0]` span test is wrong when `px` lands exactly on a
  crossing, so the two-crossing fast path is `(xs[0] > px) != (xs[1] > px)`.
- Hoisted the constant ground foreground and precomputed the column/row radial
  terms, as in hyperhex.

Output is byte-identical: the 60-frame FNV checksum over plain chars is unchanged
(`0aceff13967a8c62` @200x60, `a525fe18792ee160` @400x120).

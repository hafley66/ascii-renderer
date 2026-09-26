# 113 hyperhex prompt sequence

User messages, verbatim, in order:

1. > how can i use it to go make art in a worktree taint and asll but asked not to look too hard at others
2. > try not to look at latest ones in psat 3 days, try to keep it efficient, have fun be avant garde but try not to be too influence buy the previous ones
3. > thats okay only ur stuff matters skip not ur stuff
4. > dont want u tainted
5. > i want highly dynamic breathing hexagon assymetric non euclidaen tiling

Brief: `briefs/boop-start.md` (this worktree's assigned brief).

## Perf

Release bench, 60 frames t = 0..60, min of 8 rounds
(`CARGO_BUILD_JOBS=4 cargo test --release -- --ignored --nocapture modes::_113_hyperhex`):

| size | before ms/frame | after ms/frame |
|------|-----------------|----------------|
| 200x60 | 0.104 | 0.084 |
| 400x120 | 0.322 | 0.230 |

Hottest layer before: `ground` 0.066 ms @200x60 (63% of the frame).
After: `ground` 0.043 ms.

Change: hoisted the constant ground foreground `lerp_color(pal[0], pal[1], 0.5)`
out of the per-cell loop, and precomputed the column `dx*dx` and row `dy*dy`
terms so each cell does one add, one `sqrt` and one background lerp.

Output is byte-identical: the 60-frame FNV checksum over plain chars is unchanged
(`8d5c5f6243a32cc0` @200x60, `6c3bfcb2f295310d` @400x120).

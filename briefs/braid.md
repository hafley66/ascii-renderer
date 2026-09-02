# braid

Subject: a vertical plait of N colored ribbons tied by a seed-chosen braid word, crossing over and under each other as the whole braid scrolls downward.

What moves and why: the braid word is an infinite sequence of generators sigma_i^(+/-1) sampled at build time from the seed. The clock `t` scrolls the crossing sequence downward at SPEED rows per second, so new crossings appear at the top and fall off the bottom, like a plait being tied above the frame. Between crossings each ribbon rests in its lane and sways by a small per-ribbon phase, also fixed at build time. At a crossing the two ribbons swap lanes along diagonals; the over strand keeps its full brightness and glyph, the under strand darkens and breaks where the over strand passes.

Glyph families:
1. Ribbon core, straight run: `|`
2. Ribbon diagonals through a crossing: `/` and `\`
3. Crossing knot where the over strand passes: `X`, `+`
4. Ribbon edge and shading on the under strand: `:`, `.`
5. Background loom dust between lanes: `.`, `'`, `` ` ``

Knobs:
- STRANDS: number of ribbons (default 5)
- SPEED: scroll rate in rows per second (default 4)
- PITCH: rows between successive crossings (default 6)

Added while building:
- WIDTH: ribbon width in columns, odd (default 3)
- GAP: lane spacing in columns, clamped to fit the grid (default 8)
- CROSS: fraction of each pitch block spent on the diagonal (default 0.6)
- SWAY: lateral wobble amplitude in columns (default 1.0)
- DUST: background dust density (default 0.06)
- TWIST: probability a crossing follows the alternating over/under plait rule (default 0.85)
- FILL: probability each same-parity lane pair crosses at a step (default 0.75)

Positional order: strands speed pitch gap width cross sway dust twist fill.

Measured frame time, release, 200x60, 200 frames: avg 0.034 ms, worst 0.062 ms.

Render commands:
```bash
ASCII_GRID_W=110 ASCII_GRID_H=36 ./target/release/ascii-renderer 42 braid moss | sed 's/\x1b\[[0-9;]*m//g'
ASCII_GRID_W=110 ASCII_GRID_H=36 ASCII_T=5 ./target/release/ascii-renderer 42 braid moss | sed 's/\x1b\[[0-9;]*m//g'
```

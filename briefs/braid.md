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

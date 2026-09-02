# braid-2

Subject: a horizontal plait of twisted flat ribbons, each ribbon turning on its own axis so it shows its face and then its edge, with bright beads running along every ribbon so the permutation through the crossings can be followed by eye.

What moves and why: the braid word is a seeded sequence of lane masks (same-parity lane pairs crossing in parallel) built once from the seed. The clock scrolls the plait leftward at SPEED columns per second, so new crossings enter at the right edge. Each ribbon twists along its length with period TWIST columns and a per-ribbon phase: at full face it is WIDTH rows thick, at edge-on it collapses to one row. Beads travel rightward along each ribbon at PULSE columns per second, spaced BEADS columns apart, independent of the scroll; a bead keeps its ribbon through every crossing, so the eye can track one strand from left to right.

Glyph families:
1. Ribbon face fill: `#`, `=`
2. Ribbon edge-on and face rims: `-`, `~`
3. Diagonals through a crossing: `/`, `\`
4. Crossing knot on the over ribbon: `X`, `+`
5. Bead head and trail: `o`, `*`, `:`

Knobs:
- STRANDS: number of ribbons (default 5)
- SPEED: scroll rate in columns per second (default 6)
- PITCH: columns between successive crossing steps (default 12)

Added while building:
- GAP: lane spacing in rows, clamped to fit the grid (default 4)
- WIDTH: ribbon thickness in rows at full face, odd (default 3)
- CROSS: fraction of each pitch block spent on the diagonal (default 0.5)
- TWIST: twist period in columns; ribbon thins to one row at edge-on (default 28)
- PULSE: bead speed in columns per second, positive runs rightward (default 10)
- BEADS: bead spacing in columns along a ribbon (default 36)
- TRAIL: bead trail length in columns (default 7)
- SLIP: probability a crossing breaks the alternating over/under plait rule (default 0.15)
- FILL: probability each same-parity lane pair crosses at a step (default 0.75)

Positional order: strands speed pitch gap width cross twist pulse beads trail slip fill.

Measured frame time, release, 200x60, 200 frames: avg 0.047 ms, worst 0.067 ms.

Render commands:
```bash
ASCII_GRID_W=110 ASCII_GRID_H=36 ./target/release/ascii-renderer 42 braid-2 moss | sed 's/\x1b\[[0-9;]*m//g'
ASCII_GRID_W=110 ASCII_GRID_H=36 ASCII_T=5 ./target/release/ascii-renderer 42 braid-2 moss | sed 's/\x1b\[[0-9;]*m//g'
```

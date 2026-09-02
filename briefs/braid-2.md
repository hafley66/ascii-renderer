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

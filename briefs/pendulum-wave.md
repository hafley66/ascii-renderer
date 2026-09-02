# pendulum-wave

Subject: a row of pendulums hung from one beam, each one beat faster than its neighbor, so the bobs fall into snakes, then pairs, then apparent chaos, and line up again once every cycle.

What moves and why: pendulum i completes BASE + i swings per CYCLE seconds. Its string length follows from the period, so the row of strings shortens left to right and the bobs hang along a curve. All bobs start at full deflection at t = 0 and drift out of phase; the visible wave patterns are the beat frequencies between neighbors. The front view draws the beam, the strings, the bobs, and each bob's shadow on the floor. The top view gives every pendulum a horizontal lane and draws the bob with a fading trail of its last positions, so the envelope of the wave reads as a curve down the screen.

Glyph families:
1. Beam and hooks: `=`, `+`
2. Strings by slope: `|`, `/`, `\`
3. Bob and its rim: `@`, `(`, `)`, `O`
4. Floor and shadows: `-`, `=`, `~`
5. Trail in the top view: `*`, `:`, `.`

Knobs:
- COUNT: number of pendulums (default 15)
- CYCLE: seconds until all bobs realign (default 30)
- SWING: swing amplitude in radians (default 0.5)

Added while building:
- BASE: swings per cycle for the first pendulum; pendulum i does BASE + i (default 20)
- VIEW: 0 front view with strings, ghosts and shadows; 1 top view with trails; 2 phase waterfall (default 2)
- TRAIL: ghost or trail samples per bob (default 8)
- TAIL: seconds between ghost samples (default 0.03)
- ASPECT: columns per row for the swing in the front view (default 2)
- HUE: hue step between neighboring bobs in degrees (default 18)
- LINK: envelope line through the bobs, 1 top view, 2 front view too (default 0)
- ARC: dotted swing path guide (default 0)
- ROWDT: waterfall seconds per row (default 0.04)
- BANDS: waterfall quantized to the discrete pendulums, 0 or 1 (default 0)

Waterfall: column = pendulum index as a continuous chirp, row = time going down, glyph = cos of the phase.
Seed picks the bob hue and whether the longest pendulum hangs at the left or the right.
Positional order: count cycle base swing view trail tail aspect hue link arc rowdt bands.

Measured frame time, release, 200x60, 200 frames: avg 0.079 ms, worst 0.091 ms.

Render commands:
```bash
ASCII_GRID_W=110 ASCII_GRID_H=36 ASCII_T=19 ./target/release/ascii-renderer 42 pendulum-wave moss | sed 's/\x1b\[[0-9;]*m//g'
ASCII_GRID_W=110 ASCII_GRID_H=36 ASCII_T=7 ASCII_P_VIEW=0 ./target/release/ascii-renderer 42 pendulum-wave moss | sed 's/\x1b\[[0-9;]*m//g'
```

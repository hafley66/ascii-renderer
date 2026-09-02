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

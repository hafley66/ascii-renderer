# pendulum-wave

Subject: a row of pendulums hung from one beam, each one beat faster than its neighbor, so the bobs fall into snakes, then pairs, then apparent chaos, and line up again once every cycle.

What moves and why: pendulum i completes BASE + i swings per CYCLE seconds. Its string length follows from the period, so the row of strings shortens left to right and the bobs hang along a curve. All bobs start at full deflection at t = 0 and drift out of phase; the visible wave patterns are the beat frequencies between neighbors. The front view draws the beam, the strings, the bobs, and each bob's shadow on the floor. The top view gives every pendulum a horizontal lane and draws the bob with a fading trail of its last positions, so the envelope of the wave reads as a curve down the screen. When the grid gives a lane two rows or more, the lane packs into a band and the trail spreads over it as a ribbon.

Glyph families:
1. Beam and hooks: `=`, `+`
2. Strings by slope: `|`, `/`, `\`
3. Bob and its rim: `@`, `(`, `)`, `O`
4. Floor and shadows: `-`, `=`, `~`
5. Trail in the top view: `*`, `:`, `.`
6. Packed top-view band ramp: ` .:-=+*#%@`, lane rails `-`, packed bob `@`, `%`

Knobs:
- COUNT: number of pendulums (default 15)
- CYCLE: seconds until all bobs realign (default 120; pendulum 0 swings once per 6 s)
- SWING: swing amplitude in radians (default 0.5)

Added while building:
- BASE: swings per cycle for the first pendulum; pendulum i does BASE + i (default 20)
- VIEW: 0 front view with strings, ghosts and shadows; 1 top view with trails; 2 phase waterfall (default 2)
- TRAIL: ghost or trail samples per bob (default 8). TRAIL times TAIL is the history span, and in a packed top view that span is what a lane band covers top to bottom; the center-row trail is sub-sampled inside the same span until neighboring samples touch, so a wide grid draws a solid curve
- TAIL: seconds between ghost samples (default 0.1)
- ASPECT: columns per row for the swing in the front view, and the width-to-height ratio of the packed top-view bob (default 2)
- HUE: hue step between neighboring bobs in degrees (default 18)
- LINK: envelope line through the bobs, 1 top view, 2 front view too (default 0). In a packed top view the envelope is on by default and drawn opaque over the band; pass a negative LINK to suppress it there
- ARC: dotted swing path guide (default 0). In a packed top view it becomes a solid rail on each lane's top row instead of dots on the lane center
- ROWDT: waterfall seconds per row, so the field drifts down at 1/ROWDT rows per second (default 0.2)
- BANDS: waterfall quantized to the discrete pendulums, 0 or 1 (default 0)

Packed top view: a lane two rows or taller is drawn as a band instead of a single line. Row within the band is time going back, so the band top is now and the band bottom is TRAIL times TAIL seconds ago. Each band row draws the bob position at its time with a distance ramp out to about half the swing reach, dimming with age, which fills the lane; the lane center row keeps the bright sub-sampled trail and the bob. Bands five rows or taller draw the bob as an ellipse blob sized from the band height and ASPECT, shorter bands keep the `(@)` pill. Lane height comes from the grid, so nothing changes below two rows per lane and the 80x24 default picture is untouched.

Ink density in the top view at 2000x1000, seed 42, t = 7: 0.058 percent before lane packing, 39.7 percent after. Frame time for that grid stays under the waterfall's: 46 ms of process wall clock against 146 ms for VIEW=2.

Waterfall: column = pendulum index as a continuous chirp, row = time going down, glyph = cos of the phase.
Seed picks the bob hue and whether the longest pendulum hangs at the left or the right.
Positional order: count cycle base swing view trail tail aspect hue link arc rowdt bands.

Measured frame time, release, 200x60, 200 frames: avg 0.039 ms, worst 0.073 ms.

Render commands:
```bash
ASCII_GRID_W=110 ASCII_GRID_H=36 ASCII_T=76 ./target/release/ascii-renderer 42 pendulum-wave moss | sed 's/\x1b\[[0-9;]*m//g'
ASCII_GRID_W=110 ASCII_GRID_H=36 ASCII_T=7 ASCII_P_VIEW=0 ./target/release/ascii-renderer 42 pendulum-wave moss | sed 's/\x1b\[[0-9;]*m//g'
ASCII_GRID_W=400 ASCII_GRID_H=120 ASCII_T=7 ASCII_P_VIEW=1 ./target/release/ascii-renderer 42 pendulum-wave moss | sed 's/\x1b\[[0-9;]*m//g'
```

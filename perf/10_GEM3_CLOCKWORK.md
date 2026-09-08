# Gem 3 clockwork revision

Implements the armillary direction from `9_GEMINI_CREATIVE_REVIEW.md` in the
existing `gem-aetherium-3` mode. Gem 1 and Gem 2 retain their implementations.

The stationary engraved dial now surrounds two to four tumbling 3D gimbals,
a diameter-spanning rotating alidade, three counter-rotating gears, and planets
with paired moons. The RINGS knob controls the engraving count and gimbal count;
TILT controls ring/planet inclination; HARMONIC and PETALS alter ring contours.
Existing seed, speed, trail, comet, nebula, spread and tracer controls still apply.

A pitched/yawed circle supplies each ring's XYZ coordinates. Perspective maps
these to terminal coordinates. All moving marks are sorted from far to near;
nearer marks overwrite farther marks at crossings. Equal-depth marks retain
submission order. Background engravings are intentionally behind all moving ink.
Planet glyph offsets use terminal cells. Other geometry scales to the art grid.
All output glyphs remain ASCII and occupy one terminal column.

Maximum moving submissions per frame:

| Geometry | Marks |
| --- | ---: |
| Gimbals | 384 |
| Sighting arm | 385 |
| Gear teeth and spokes | 108 |
| Planets, tails and moons | 204 |
| Comets | 64 |
| Tracers | 24 |
| Core | 1 |
| Total | 1,170 |

The mark vector reserves 1,280 entries. Loop bounds limit submissions to 1,170;
an assertion checks the reserved limit. Sorting uses bounded additional scratch.
Static circle scratch remains capped at 4,096 triples. Work is
O(W*H + R*min(3*(W+H),4096) + M*log(M)); no simulation persists between frames.
Two fixed-input frames can change at most twice the submitted mark count.
Knob changes can also replace stationary geometry and exceed that motion bound.

The existing 20,000-byte real-encoder gate remains in the 100-frame all-maximum
regression. This is an output-size gate, not a claim about terminal painting FPS.
The older measurements in `7_GEM3.md` and `8_GEM3_RESULTS.json` describe the prior
249-mark implementation and are retained for comparison.

Validation results will be recorded after the focused tests and bounded playback.

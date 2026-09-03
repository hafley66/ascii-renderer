# perf-instr-modes-dir: measure_layer timers (pass 1 of 2)

Wrapped painters in `crate::_0_profile::measure_layer` for five registered modes:
`qwen-cathedral`, `glm-apotheosis`, `cosmograph`, `gem-aetherium`, `hyperloom`.
Zero behavior change: `cargo test --release` passes with no `.snap.new` files.

## qwen-cathedral

8 layers, summed share 97.4%

| layer | calls/frame | avg us | max us | share of frame |
| --- | ---: | ---: | ---: | ---: |
| background | 1.0 | 768.1 | 887.5 | 78.9% |
| structure | 1.0 | 88.0 | 116.2 | 9.0% |
| nave | 1.0 | 71.6 | 123.0 | 7.3% |
| fenestration | 1.0 | 16.7 | 36.7 | 1.7% |
| light | 1.0 | 3.3 | 10.8 | 0.3% |
| frame | 1.0 | 1.2 | 7.2 | 0.1% |
| flame | 1.0 | 0.8 | 5.4 | 0.1% |
| banners | 1.0 | 0.3 | 4.8 | 0.0% |

## glm-apotheosis

6 layers, summed share 98.0%

| layer | calls/frame | avg us | max us | share of frame |
| --- | ---: | ---: | ---: | ---: |
| background | 1.0 | 955.0 | 1063.1 | 77.9% |
| particles | 1.0 | 235.6 | 305.6 | 19.2% |
| mandala | 1.0 | 7.3 | 36.3 | 0.6% |
| rays | 1.0 | 3.0 | 13.0 | 0.2% |
| frame | 1.0 | 1.2 | 13.9 | 0.1% |
| figure | 1.0 | 0.1 | 0.5 | 0.0% |

## cosmograph

7 layers, summed share 99.3%

| layer | calls/frame | avg us | max us | share of frame |
| --- | ---: | ---: | ---: | ---: |
| nebula | 1.0 | 1458.3 | 1715.9 | 55.7% |
| starfield | 1.0 | 738.5 | 841.2 | 28.2% |
| aurora | 1.0 | 304.3 | 377.4 | 11.6% |
| orrery | 1.0 | 90.6 | 117.5 | 3.5% |
| bodies | 1.0 | 4.0 | 11.8 | 0.2% |
| spiral | 1.0 | 2.3 | 7.2 | 0.1% |
| frame | 1.0 | 1.2 | 3.4 | 0.0% |

## gem-aetherium

8 layers, summed share 98.4%

| layer | calls/frame | avg us | max us | share of frame |
| --- | ---: | ---: | ---: | ---: |
| background | 1.0 | 1847.2 | 2023.5 | 94.3% |
| gears | 1.0 | 23.8 | 44.1 | 1.2% |
| armillary | 1.0 | 20.6 | 37.5 | 1.1% |
| orrery | 1.0 | 18.2 | 44.6 | 0.9% |
| starfield | 1.0 | 5.5 | 23.3 | 0.3% |
| limb | 1.0 | 5.2 | 13.4 | 0.3% |
| comets | 1.0 | 4.2 | 21.0 | 0.2% |
| core | 1.0 | 2.9 | 12.7 | 0.1% |

## hyperloom

8 layers, summed share 98.8%

| layer | calls/frame | avg us | max us | share of frame |
| --- | ---: | ---: | ---: | ---: |
| spectral_field | 1.0 | 1405.2 | 2222.9 | 71.4% |
| threads | 1.0 | 368.6 | 1045.5 | 18.7% |
| knot_cages | 1.0 | 144.0 | 1743.7 | 7.3% |
| border | 1.0 | 13.3 | 78.6 | 0.7% |
| apertures | 1.0 | 9.3 | 23.5 | 0.5% |
| cards | 1.0 | 2.9 | 8.7 | 0.1% |
| shuttles | 1.0 | 2.3 | 8.6 | 0.1% |
| veil | 1.0 | 0.1 | 0.6 | 0.0% |

## Gates

`cargo test --release`: `test result: ok. 241 passed; 0 failed; 4 ignored` and `test result: ok. 132 passed; 0 failed; 0 ignored`

`.snap.new`: (empty)

`git status --short`: (clean after commit)

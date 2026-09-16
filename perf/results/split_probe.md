# split probe: prismata 366x199 (72834 cells), theme moss, seed 42, dt 0.06, 15 reps, release

| stage | median us | min us | convert us | emit us | bytes | cells changed | cells skipped | runs |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| render (whole mode) | 657.3 | 562.1 | - | - | - | - | - | - |
| blank grid build + row fill | 51.9 | 51.4 | - | - | - | - | - | - |
| encode full repaint | 2434.5 | 2334.3 | 1570.2 | 857.5 | 78021 | 72834 | 0 | 199 |
| encode delta over one dt step | 582.1 | 565.5 | 380.7 | 200.5 | 25421 | 2806 | 67204 | 2476 |
| encode identical frame | 304.0 | 290.8 | 261.2 | 42.2 | 0 | 0 | 72834 | 0 |

# split probe: prismata 366x199 (72834 cells), theme moss, seed 42, dt 0.06, 15 reps, release

| stage | median us | min us | convert us | emit us | bytes | cells changed | cells skipped | runs |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| render (whole mode) | 765.6 | 668.1 | - | - | - | - | - | - |
| blank grid build + row fill | 53.7 | 53.2 | - | - | - | - | - | - |
| encode full repaint | 2519.8 | 2345.2 | 1583.6 | 905.8 | 78021 | 72834 | 0 | 199 |
| encode delta over one dt step | 617.8 | 546.7 | 395.2 | 222.2 | 25421 | 2806 | 67204 | 2476 |
| encode identical frame | 303.8 | 283.0 | 260.2 | 42.5 | 0 | 0 | 72834 | 0 |

# split probe: prismata 366x199 (72834 cells), theme moss, seed 42, dt 0.06, 15 reps, release

| stage | median us | min us | convert us | emit us | bytes | cells changed | cells skipped | cells invisible | runs |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| render (whole mode) | 775.3 | 612.6 | - | - | - | - | - | - | - |
| blank grid build + row fill | 53.8 | 51.6 | - | - | - | - | - | - | - |
| encode full repaint | 2466.5 | 2358.0 | 1602.3 | 871.6 | 78021 | 72834 | 0 | 0 | 199 |
| encode delta over one dt step | 590.3 | 563.9 | 380.8 | 208.1 | 25421 | 2806 | 67204 | 2824 | 2476 |
| encode identical frame | 301.2 | 290.5 | 257.3 | 42.3 | 0 | 0 | 72834 | 0 | 0 |

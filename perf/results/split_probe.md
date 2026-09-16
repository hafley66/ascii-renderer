# split probe: prismata 366x199 (72834 cells), theme moss, seed 42, dt 0.06, 15 reps, release

| stage | median us | min us | convert us | emit us | bytes | cells changed | cells skipped | runs |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| render (whole mode) | 1259.2 | 826.5 | - | - | - | - | - | - |
| blank grid build + row fill | 58.1 | 53.7 | - | - | - | - | - | - |
| encode full repaint | 2263.4 | 2162.0 | 1319.9 | 938.5 | 78021 | 72834 | 0 | 199 |
| encode delta over one dt step | 788.6 | 687.5 | 444.7 | 312.7 | 25421 | 2806 | 67204 | 2476 |
| encode identical frame | 279.1 | 260.0 | 231.0 | 44.8 | 0 | 0 | 72834 | 0 |

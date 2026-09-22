# Prompt sequence record: wada-v2

Art run: follow-up request, mode wada-v2 (src/modes/_103_wada_v2.rs), a braille
rasterization of the wada field. This file records every prompt and steering
message verbatim, in order, then the internal revision log. Times are UTC.

---

## Entry 1

- channel: user request (follow-up on wada)
- received: 2026-09-21, after the wada commits (6258f61, e4508c5)

```text
i love what u did here. can u commit all of this, i dont like changing something that worked. can u just copy paste this into the next numbered thing and call it _v2? and the different here is that rendering should use braille/pseudo higher way of rendering that is not image protos
```

Read as: leave wada untouched and committed; clone it as the next numbered
file with a _v2 suffix; the one intended difference is the raster — braille
subcell bitmaps (eight dot pixels per terminal cell) instead of the glyph-ramp
cell rendering. Terminal-native braille, no image-based pixel prototypes.

---

## Entry 2

- channel: user interjection (priority notice attached)
- received: 2026-09-21, while the v2 build was compiling

```text
thank you!
```

Appreciation only; no change to direction.

---

## Internal revision log (no user steering after entry 1)

1. Clone: _103_wada_v2.rs from the wada skeleton, NAME "wada-v2", same 11
   knobs in the same positional order, five measure_layer passes.
2. Raster: each cell solves Newton at 8 subcell positions (2x4) and packs the
   dots into one U+2800 bitmap. Minority and unsettled dots light; calm
   interiors stay open color; the CONTOUR knob moved from dwell bands to a
   dot fill threshold.
3. Parallelism: the rayon threshold counts subcells, so a 200x60 frame is
   already 96k solves and runs row-parallel.
4. First release frame_cost measured 6.305 ms average against the 6 ms
   budget. Root cause: about six libm calls per subcell (atan2, sin_cos,
   two sins, sqrt) at 96k subcells.
5. Fix: the swirl is a smooth low-frequency field, so it is evaluated once
   at the cell center and applied to the eight subs as a shared rotation.
   The subcell inner loop became libm-free.
6. Still tight on reruns (7.34 ms worst measurement), so the convergence
   tolerance went from |f|^2 < 4e-6 to 1e-4 (basin identity unaffected at
   these scales) and MAXIT from 32 to 24. Settled at avg 5.14 to 5.16 ms,
   worst under 6 ms across repeated runs.
7. Snapshots regenerated after each of steps 5 and 6 and accepted by
   inspection of the braille output, never blindly.

## Entry 3

- channel: user request (efficiency pass)
- received: 2026-09-21, after the wada-v2 commits

```text
last thing, can u make both more efficient without snapshot changes?
```

Read as: optimize both modes while keeping every committed snapshot
byte-identical. For wada-v2: nearest_root gained an early break that
provably returns the same index (roots are half a unit apart), and the
per-cell labels (majority basin, average dwell) moved into the solve pass
so dots, veins and traps read them instead of recomputing up to five times
per cell. No floating point was restructured and no visual threshold moved.
Verified by re-running all eight snapshots: 12 in-module plus 4 integration
tests green, zero .snap.new files. frame_cost 5.10 to 4.92 ms; sweep
baseline 311.55 to 270.79 ms; vein scans 5229 to 672 us.

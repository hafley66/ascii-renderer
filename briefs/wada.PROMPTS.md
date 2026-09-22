# Prompt sequence record: wada

Art run: single-session request, mode wada (src/modes/_102_wada.rs).
This file records every prompt and steering message verbatim, in order, then the
internal revision log that steered the visual outcome. Times are UTC.

---

## Entry 1

- channel: user request (session start)
- received: 2026-09-21, before first implementation turn

```text
hello, please make really intense and avant garde art algorithm in this repo. you may read other impl's but be careful! they will bias you on this journey. some have great ideas, some do not (the more recent past day or 2 are rough dont look at them). pelase get ur first good few renders of max code complete and then test with similar perf harness inrepo
```

Constraints carried from the repo context files loaded with this request:
the add-mode skill (one file, generated registry, layer timers, snapshot tests,
perf receipt), perf/INSTRUMENT.md layer rules, and the AGENTS.md ground rule
that AI art carries its prompt sequence in the worktree.

Read for reference, per the skill: src/modes/_55_rosette.rs and
src/modes/_54_nightglass.rs (the named reference implementations). The files
dated one to two days back (_60_slice.rs through _64_slice_2.rs and the
_100/_101 lane files) were deliberately not opened, per the request.

---

## Entry 2

- channel: user message (mid-run)
- received: 2026-09-21, after the first preview gallery render

```text
hola
```

Greeting only; no change to direction.

---

## Internal revision log (no user steering after entry 1)

These are the decisions that shaped the final image, in order:

1. Concept: Mobius-warped Newton basins of z^order - c, where every basin
   boundary is shared (Wada property). Layers: basins / shade / veins / traps /
   grain, all under measure_layer.
2. Bug: power loop computed z^(order+1) with a z^order derivative factor, so
   basin labels flickered into thick vein blobs. Fixed to z^(order-1).
3. Bug: orbits diving near the origin hit fp->0 and catapulted past the escape
   radius into CHAOS, which then veined itself and its neighbors into noise.
   Fixed with a capped Newton step (STEP_CAP 1.8) that lands back in-plane.
4. Measurement (temporary stats test): chaos 1.0 percent but vein 42.8 percent
   under a naive any-foreign-neighbor rule; basin id rows showed genuine
   pixel-scale interleaving in the meet-zones (the central iris), not a bug.
5. Vein rule rewritten: only a clean single-direction split where both sides
   settled fast (SEAM_MAXIT = MAXIT/4) draws a vein; junction sparks were
   tried and removed because they fired all through the interleaved zones.
6. First color review (perf/previews/102_wada): fields too dark, meet-zones
   read as bright confetti. Rewrote the shade pass: calm interiors hold
   saturated color, interleaved web loses saturation instead of brightness.
7. Second color review: pale dashes stabbed through fields because fg/bg
   lightness tracked dwell. Flattened brightness against dwell; glyph density
   now carries the dwell contour.
8. Scars: veins and trap sparks were painted onto the near-black wall color.
   Both now keep the underlying cell background.
9. order=12 showed 10.2 percent dead cells (plunge returns outlasting the
   iteration budget). STEP_CAP tightened to 1.8 and MAXIT raised to 32, which
   cut it to 5.2 percent, and chaos cells were re-colored to hue-tinted char
   instead of punched-out black.
10. Final state accepted after the marked contact sheet review: saturated
    basin fields, dendrite coasts with lit # veins, trap sparks, engraved
    dwell contours. Snapshots accepted by inspection after that review.

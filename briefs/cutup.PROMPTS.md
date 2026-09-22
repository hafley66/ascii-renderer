# Prompt sequence record: cutup

Art run: mode cutup (src/modes/_104_cutup.rs), a woodblock landscape torn into
a cut-up collage with misregistered overprint. This file records every prompt
and steering message verbatim, in order, then the internal revision log.
Times are UTC.

---

## Entry 1

- channel: user request (new art)
- received: 2026-09-22

```text
hello! please make some avant garde art in this repo. try to keep it efficient, but really draw anything you want i do this to ping the default space of an ai model like yourself. have fun with it, some ground ruleS: go hard, go avant, or not, and then stress test to within acceptable levels. good luck! try to keep code consistent with that around it
```

Read as: one new piece in the repo's own idiom (a generated-registry mode),
free choice of subject, avant-garde license on the composition, efficiency
mattered, code style matched to neighboring modes, then a stress pass with
accepted bounds. Subject chosen here: the cut-up technique itself (Burroughs
and Gysin) applied to a woodblock landscape plate, with printmaking
misregistration as the second layer.

---

## Internal revision log (no user steering after entry 1)

1. Mode skeleton after _55_rosette.rs: trait mode, 11 registry knobs, splitmix
   hash/unit, thread-local scratch, each_row rayon threshold at 20_480 cells,
   six measure_layer passes (plate, cuts, backing, collage, overprint, grain).
2. Plate: analytic woodblock landscape in one pass per cell (sun disc with
   rim and rays, two triangular ridge crests with gouge noise, water strata
   with a carved sun column, hashed horizon line, stamped birds), colors from
   six block tones through a 10-level ink ramp.
3. Collage: wobbled vertical and horizontal cut curves (precomputed per row
   and column), fragments pasted at hashed offsets in hashed z order, optional
   mirroring and negative ink mixes, torn fiber along every rip, integer
   drift oscillation that rests at zero when time is zero.
4. Overprint: key block re-impressed off register over the reassembled sheet,
   solid mark over ink and ghost mark over bare paper, sliding with drift.
5. First visual review of the 80x24 snapshot: collage mechanics read, but the
   plate was hatch dither that read as speckle mush at small size, negative
   scraps rendered as heavy black blobs in the color-stripped snapshot, and
   5x3 default tears made each scrap too small to hold the landscape.
6. Art fix: ridges became solid silhouettes with sparse gouges (hatch removed),
   sun grew from 0.16 to 0.19 of height, clouds thinned to streaks, horizon
   gaps widened 0.12 to 0.18, negative ink printed on a light ramp instead of
   a dense one, and default tears dropped to 4x2 (15 scraps). Snapshots
   regenerated and re-reviewed after the change.
7. Second review: torn column seams, displaced mountain mass, broken horizon
   and negative scraps all read. The integration variant at 9x6 tears and
   shift 14 read as noise at 80x24, so it moved to 6x3 tears, shift 10,
   flip 0.8: shatter with legible scraps and visible negatives.
8. Real-terminal E2E (scripts/13_e2e.sh --headless --mode cutup --case all)
   failed the workflow case on "art pixels did not change during animation".
   Evidence: the suite compares terminal captures across 3 animation frames
   (dt 0.06, so about 0.18 s), and the drift oscillation rounded to integer
   cells needed about half a cell of travel before any cell moved, while the
   overprint slide only touches sparse key cells. Root cause: integer-rounded
   offsets, not slow rates.
9. Fix: fragment drift became continuous sub-cell offsets with nearest-cell
   sampling. The static frame is byte-identical (both t=0 snapshots passed
   unchanged through the refactor), motion is visible within three frames, and
   the E2E contract is now pinned in-module as motion_within_three_frames
   (t=0.19 differs from t=0; pre-fix the largest possible drift in that window
   was 0.33 cells, which rounds to zero everywhere). The t=6 and t=7
   snapshots regenerated and were re-reviewed.

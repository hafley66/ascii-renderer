# Instrumenting a mode for the knob sweep

Every native mode wraps its painters in `crate::_0_profile::measure_layer` so
`perf/knob_sweep.sh <mode>` prints a hotspot table. The timers cost nothing when no
capture is open and no `ASCII_PROFILE_LAYERS` is set.

## Pattern

```rust
use crate::_0_profile::measure_layer;

pub(crate) fn draw_thing(grid: &mut Grid, w: usize, h: usize, /* ... */) {
    measure_layer("thing", "clear", || {
        for row in grid.iter_mut().take(h) {
            row.fill(Cell::blank());
        }
    });
    let stars = measure_layer("thing", "stars", || paint_stars(grid, w, h, seed, t));
    measure_layer("thing", "trails", || paint_trails(grid, w, h, &stars, t));
    measure_layer("thing", "frame", || paint_frame(grid, w, h, palette));
}
```

Rules:

1. First argument is the registry mode name exactly (`"tree-of-life-4"`, `"gem-aetherium"`).
   Second is a snake_case phase name in paint order.
2. Wrap contiguous painter sections or single painter calls. Three to eight layers per mode.
   The layers must add up to at least 85 percent of the frame; the sweep prints each share.
3. `measure_layer` returns the closure's value, so a section that produces a value keeps it:
   `let x = measure_layer(..., || compute());`
4. Zero behavior change. `cargo test` must pass with no `.snap.new` files anywhere.
5. If the borrow checker rejects a wrap because the closure needs two mutable borrows, wrap a
   smaller section or hoist the section into a fn and wrap the call. Never restructure logic.
6. Comment rule: at most 2 consecutive comment lines anywhere. No em dashes. Never the words
   provenance, substrate, load-bearing, regime, signal.

## Validate one mode

```bash
cargo build --release
cargo test 2>&1 | grep -E "^test result"
ls src/snapshots/*.snap.new tests/snapshots/*.snap.new src/modes/snapshots/*.snap.new 2>/dev/null   # must print nothing
ASCII_PERF_MODE=<mode> ASCII_PERF_SECS=1 ASCII_PERF_WIDTH=400 ASCII_PERF_HEIGHT=120 \
  cargo test --release perf_knob_sweep -- --ignored --nocapture --test-threads=1 2>/dev/null \
  | sed -n '/^## hotspots/,$p'
```

The last command must print a layer table, never the line `no measure_layer timers fired`.

## Coverage

Rule 2 asks for 85 percent of the frame and neither `cargo test` gate checks it, because layers
nest and a share can read over 100 percent (`gem-aetherium-2` attributes 180 percent: its
`background` wraps `nebula` and `rays`). `perf/layer_coverage.sh` is the report that does:

```bash
perf/layer_coverage.sh                    # 400x120, 3 reps, whole roster
perf/layer_coverage.sh 400 120 3 moss tide  # filter by mode-name substring
```

It prints one row per mode: layers, calls per frame, attributed share, and whether the share is
thin (under 85 percent) or nested (over 100 percent), then names the layers of every mode worth
reading. It asserts nothing on purpose, so a thin mode is caught at review rather than being a
waived failure. The last full run is `perf/results/layer_coverage.md`: 37 of 78 modes were under
85 percent, and the worst were the tree and forest families, whose frames are dominated by the
shared `tree_draw`, `walker`, `sprites` and `scene` code that carries no timers of its own.

## Gate

`every_native_mode_has_layer_timers` in `src/perf_sweep.rs` runs with `cargo test`. It renders
every native mode on the `NATIVE_MODES` roster once with a capture open and fails naming any
mode with no timers. The roster is complete and a registered mode needs no entry in it:
`every_registered_mode_that_renders_in_process_has_layer_timers` iterates the registry instead,
so a new mode is checked without touching the roster.

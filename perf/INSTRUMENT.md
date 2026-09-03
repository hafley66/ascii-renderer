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

## Gate

`cargo test --release every_native_mode_has_layer_timers -- --ignored` renders every
native mode once with a capture open and lists the ones with no timers. It goes un-ignored
once the list is empty.

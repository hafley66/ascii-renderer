# Step 3: buffer hygiene on the frame path

Step 3 of `perf/16_LARGE_RENDER_PLAN.md`. The encoder wrote each frame's ANSI
payload into its own `Vec<u8>` and then copied it into the caller's `String`
through `std::str::from_utf8`, and the live loop inserted its frame prefixes with
`String::insert_str(0, ..)`. Both are per-frame O(payload) work on the hot path,
and both are gone: the diff is drawn straight into a caller-owned `Vec<u8>` and
the prefixes are written before the payload.

## What changed

- `AnsiFrameEncoder::encode(&mut self, grid, force_full, output: &mut Vec<u8>)`.
  It appends one frame's payload to `output` and reports `bytes` as the length it
  added, so the caller can write a prefix before it without a later memmove. The
  encoder's own `bytes: Vec<u8>` field is deleted, along with the
  `output.clear()` + `push_str(from_utf8(..))` pair that ended every frame.
- `src/morph.rs`: `frame_buffer` is a `Vec<u8>`; synchronized-output begin, the
  cursor-cancel prefix and the synchronized-output end are written in stream order
  around the payload instead of being inserted in front of a finished frame. The
  footer `write!` moves from `fmt::Write` to `io::Write`, and the options pane is
  appended with `extend_from_slice(pane_buffer.as_bytes())`.
- Other callers (`perf_sweep::perf_split_probe`, `_30_illuminarium`,
  `_52_gem_aetherium_3`, the `morph` and `gridio` tests) now pass a `Vec<u8>` and
  clear it per frame, which is the contract an appending writer needs.

Stream order is preserved exactly: `\x1b[?2026h` then `\x18` or `\x18\x1b[2J`
then the payload then the status row, the options pane and `\x1b[?2026l`.

## What it is worth, and why the plan overestimated it

The plan projected 0.2 to 0.6 ms per frame from the copy. Measured, the removed
work is much smaller and depends on how much of the payload is non-ASCII, because
std's UTF-8 validation has a word-at-a-time ASCII fast path and only leaves it for
non-ASCII chunks.

Direct measurement of the removed pair (`from_utf8` + `push_str` against a plain
copy) on 480 KB buffers, 500 iterations, median:

| payload | removed per frame |
| --- | ---: |
| 480 KB taken from the live prismata stream (0.2% non-ASCII) | 18.1 us |
| 480 KB synthetic, ASCII only | 15.9 us |
| 480 KB synthetic, 14% non-ASCII | 177.7 us |

Medians of 500 iterations; the first two rows are what the reproduce script at the
bottom prints, the third was measured with the same harness on a payload built from
multi-byte glyphs.

Real art payloads are ASCII-dominated: the `terminal.ansi.gz` of the prismata
`max-400x200` case is 12,352,446 bytes with 8,509 three-byte leads and a 0.002
non-ASCII fraction. That is why a mode drawing wide glyphs gains ten percent of
`emit` and a mode drawing ASCII loses only one or two.

## Probe

`perf_sweep::perf_split_probe`, 366x199, moss, dt 0.06, 15 reps, five alternating
rounds of the HEAD binary and this revision in one session, medians of the five
rounds. `emit` is the diff draw plus emission; `total` is `convert + emit`. Bytes,
changed cells and skipped cells are identical on every row of both binaries.

| mode | row | emit HEAD | emit this | total HEAD | total this | total delta |
| --- | --- | ---: | ---: | ---: | ---: | ---: |
| illuminarium | delta | 469.0 | 409.7 | 891.6 | 819.8 | -8.1% |
| illuminarium | full repaint | 1,258.7 | 1,085.5 | 2,652.7 | 2,461.2 | -7.2% |
| gem-aetherium-2 | delta | 428.2 | 386.2 | 843.0 | 793.0 | -5.9% |
| gem-aetherium-2 | full repaint | 1,111.3 | 1,003.4 | 2,366.9 | 2,265.2 | -4.3% |
| cosmograph | delta | 254.6 | 228.4 | 630.0 | 602.8 | -4.3% |
| cosmograph | full repaint | 1,161.9 | 1,017.5 | 2,471.5 | 2,354.6 | -4.7% |
| prismata (ASCII art) | delta | 209.4 | 205.4 | 591.6 | 585.6 | -1.0% |
| prismata (ASCII art) | full repaint | 885.2 | 858.0 | 2,459.1 | 2,403.2 | -2.3% |

`illuminarium`, `gem-aetherium-2` and `cosmograph` are the glyph-dense modes;
prismata's art is ASCII, which is the row that shows the mechanism rather than the
change. At `prismata` 800x240 the full repaint emits 201,390 bytes on both builds.

The two `insert_str(0, ..)` calls this removes are a second, smaller term: each
memmoved the whole composed frame (376 to 601 KB on the live prismata frames
below), which is roughly 15 us per move at copy bandwidth.

## Byte identity

- Probe, 16 modes at 366x199: the three encode rows report identical bytes, changed
  cells and skipped cells on HEAD and this revision in every row, including
  `gem-aetherium-2` and `cosmograph`, which draw zodiac wide glyphs.
- `morph::iterate_frame_tests::gem_bad_roll6_ansi_regression`, 60 frames: the
  observed comparator is `(5,124,966, 357,555, 0, 2,019,066, 2,976,090, 0)`, exactly
  what HEAD and both earlier encoder revisions observe in `18_split_probe.md`. This
  test is red at HEAD for a reason that predates this work (it expects 4,748,083
  foreground bytes and observes 0, so it never asserted what it intended).
- Live: E2E `max-400x200`, prismata, all twelve knobs at max, 366x199. Aligned by
  `frame_index`, the 15 frames shared by the HEAD run, the adapted revision and this
  one report identical `bytes`, `changed_cells` and `runs` with identical knobs:

| live, 366x199 max knobs, 15 shared frames | HEAD | adapted | this |
| --- | ---: | ---: | ---: |
| bytes, average | 484,713 | 484,713 | 484,713 |
| cells changed, average | 48,775 | 48,775 | 48,775 |
| `convert_us`, average | 1,465 | 1,608 | 1,631 |
| `emit_us`, average | 3,343 | 2,639 | 2,641 |
| `encoding_us`, average | 4,808 | 4,248 | 4,273 |
| `render_us`, average | 1,334 | 1,107 | 1,548 |
| frame `dur_us`, average | 21,909 | 17,523 | 19,922 |

The live `emit` column is flat between the adapted revision and this one because
prismata's payload is ASCII and the removed work is 15 to 20 us inside a 2,640 us
`emit`, while `render_us` moved 40 percent between those two runs on a shared host.
The live numbers are evidence of identity and of a passing pipeline, not of this
step's size; the probe table above is the size.

## Suite and E2E

- `cargo test --release`: 454 passed / 3 failed / 17 ignored in the bin target, the
  same three pre-existing failures (`gridio::ansi_frame_tests::
  animation_encoder_collapses_adjacent_rgb_levels`,
  `morph::iterate_frame_tests::gem_bad_roll6_ansi_regression`,
  `polytope::tests::snapshot_polytope_small`). Integration targets:
  `0_mode_generator` 1, `1_render_trace` 1, `2_input_replay` 1, `snapshot_modes`
  190 passed / 1 failed (`polytope_seed_42`, pre-existing and release-only). No
  `.snap.new` anywhere.
- E2E `max-400x200`, headless: 9 of 9 checks, twice. `e2e-1789597570940` with the
  default `gem-aetherium-2` mode and `e2e-1789597589345` with `prismata`; both
  artifacts are single-case runs, so the suite's other cases are unexecuted
  (`complete_suite: false`). Binary identity of the prismata run:
  sha256 `bd4a6fbce8eb094d3303582184669fcc928a3b7786c51d79c79e24ef7158f555`.

## Reproduce

```bash
# one binary per revision, alternated in one session; the test binary's file name
# hash depends on crate metadata, not on source, so copy it out before each rebuild
cargo test --release --no-run --message-format=json \
  | jq -r 'select(.executable!=null)|select(.target.name=="ascii-renderer")|.executable' | sort -u

for round in 1 2 3 4 5; do
  for bin in /tmp/probe-head /tmp/probe-rev4; do
    env ASCII_SPLIT_MODE=illuminarium ASCII_SPLIT_REPS=15 \
      "$bin" perf_split_probe --ignored --nocapture --test-threads=1
  done
done

# the removed work on a realistic payload: extract 480 KB from a live stream
# (terminal.ansi.gz of the prismata max-400x200 case), then
cat > /tmp/copy_cost.rs <<'EOF'
use std::time::Instant;
fn bench(label: &str, payload: &[u8]) {
    let (mut s, mut out) = (String::with_capacity(1 << 20), Vec::with_capacity(1 << 20));
    let (mut v, mut c) = (Vec::new(), Vec::new());
    for _ in 0..500 {
        s.clear();
        let t = Instant::now();
        s.push_str(std::str::from_utf8(payload).expect("utf8"));
        v.push(t.elapsed().as_secs_f64() * 1e6);
        std::hint::black_box(s.len());
        out.clear();
        let t = Instant::now();
        out.extend_from_slice(payload);
        c.push(t.elapsed().as_secs_f64() * 1e6);
        std::hint::black_box(out.len());
    }
    v.sort_by(|a, b| a.partial_cmp(b).unwrap());
    c.sort_by(|a, b| a.partial_cmp(b).unwrap());
    println!("{label}: removed {:.1} us per frame", v[250] - c[250]);
}
fn main() {
    let live = std::fs::read("/tmp/live-prismata.ansi").expect("live stream");
    bench("live 480 KB slice", &live[2_000_000..2_480_000]);
    bench("all-ascii synthetic", &b"\x1b[38;5;68m\x1b[1;42H*\x1b[39m\x1b[49m".repeat(480_000 / 26));
}
EOF
gunzip -c perf/results/e2e-1789597589345/max-400x200/terminal.ansi.gz > /tmp/live-prismata.ansi
rustc -O /tmp/copy_cost.rs -o /tmp/copy_cost && /tmp/copy_cost

# suite and live
cargo test --release --no-fail-fast
scripts/13_e2e.sh --headless --mode prismata --case max-400x200
```

# Function entry and exit recording

```bash
cargo build --release --locked
ASCII_FUNCTION_TRACE=perf/results/functions target/release/ascii-renderer 42 demo
```

Use the existing probe guard for live captures. Every process, including animation
workers and preview children, writes `functions-<pid>.ndjson.gz` in that directory.
`new` records mark function entry; `close` records mark completion and include
elapsed entered/idle span time. Records include source file, line, thread ID,
function name and timestamp. Arguments, grids and repeated ancestor lists are excluded.

The implementation uses `tracing::instrument`, `tracing-subscriber`,
`tracing-appender`, `flate2::write::GzEncoder` and `std::io::BufWriter`. Compression
runs in the appender's worker thread. The queue holds 4,096 records and the
file buffer holds 64 KiB. The library worker drains queued records in batches.
The queue is lossless: saturation blocks producers. Enabled tracing therefore
perturbs execution and its throughput is not an uninstrumented benchmark.

The scope is explicitly declared function bodies under `src`, including methods
and nested functions. Closures, dependency internals and compiler-generated
functions are outside that scope. Four `const fn` bodies cannot emit runtime
events. Subscriber initialization precedes recording; `main` has a manual span
after initialization. Forced termination and `process::exit` can leave spans
unclosed and buffered records unflushed, including an incomplete gzip trailer.
Normal return flushes through the
library's `WorkerGuard`. A shutdown record reports the dropped-record count.

`perf/results/23_function_trace/coverage.json` records the insertion inventory.
The generated registry receives its attribute through the registry generator.

Validate paired records and identical art using the existing watchdog:

```bash
python3 scripts/5_probe_guard.py \
  --state perf/results/23_function_trace/guard.json \
  --artifact-dir perf/results/23_function_trace \
  --max-seconds 15 --max-owned-mib 256 --max-artifact-mib 32 \
  -- python3 scripts/15_test_function_trace.py perf/results/23_function_trace/check
```

#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
build_artifacts="perf/results/gem-build-$(date +%s)"
mkdir -p "$build_artifacts"
echo "Build log: $build_artifacts/build.log"
exec python3 scripts/5_probe_guard.py \
    --state "$build_artifacts/guard.ndjson" --artifact-dir "$build_artifacts" \
    --max-seconds 900 --max-owned-mib 1024 --max-artifact-mib 32 \
    -- /usr/sbin/taskpolicy -b /usr/bin/nice -n 19 \
    cargo build --release --locked -j 1 --features gem-lab-only --bin gem-render-lab --example 1_native_terminal \
    > "$build_artifacts/build.log" 2>&1

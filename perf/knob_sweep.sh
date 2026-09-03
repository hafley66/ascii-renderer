#!/usr/bin/env bash
# Sweep every registry knob of one mode to its max at a large grid, report fps,
# then print in-process layer hotspots for the slowest knob.
# usage: perf/knob_sweep.sh <mode> [width height secs dt theme]
set -euo pipefail
mode="${1:?mode name}"
width="${2:-2000}"
height="${3:-1000}"
secs="${4:-5}"
dt="${5:-0.06}"
theme="${6:-moss}"
out="perf/results/${mode}.md"
mkdir -p perf/results
env \
  ASCII_PERF_MODE="$mode" \
  ASCII_PERF_WIDTH="$width" \
  ASCII_PERF_HEIGHT="$height" \
  ASCII_PERF_SECS="$secs" \
  ASCII_PERF_DT="$dt" \
  ASCII_PERF_THEME="$theme" \
  cargo test --release --quiet perf_knob_sweep -- --ignored --nocapture --test-threads=1 2>/dev/null \
  | sed -n '/^# knob sweep/,$p' | sed '/^test result/,$d' | grep -v '^\.$' | tee "$out"
echo "wrote $out"

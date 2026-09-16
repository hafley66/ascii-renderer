#!/usr/bin/env bash
# Measure where one frame's cost sits at a grid size: whole-mode render, blank
# grid build, then full / delta / identical encoder stages with the per-cell
# conversion split from the diff and emission.
# usage: perf/split_probe.sh [mode] [width height reps dt theme]
set -euo pipefail
mode="${1:-prismata}"
width="${2:-366}"
height="${3:-199}"
reps="${4:-9}"
dt="${5:-0.06}"
theme="${6:-moss}"
out="${7:-perf/results/split_probe.md}"
mkdir -p perf/results
env \
  ASCII_SPLIT_MODE="$mode" \
  ASCII_SPLIT_WIDTH="$width" \
  ASCII_SPLIT_HEIGHT="$height" \
  ASCII_SPLIT_REPS="$reps" \
  ASCII_SPLIT_DT="$dt" \
  ASCII_SPLIT_THEME="$theme" \
  cargo test --release --quiet perf_split_probe -- --ignored --nocapture --test-threads=1 2>/dev/null \
  | sed -n '/^# split probe/,$p' | sed '/^test result/,$d' | grep -v '^\.$' | tee "$out"
echo "wrote $out"

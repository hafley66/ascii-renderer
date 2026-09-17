#!/usr/bin/env bash
# Report-only layer coverage: per mode, how many measure_layer timers fire and what
# share of the render call they attribute. The add-mode skill asks for 3 to 8 layers
# covering at least 85 percent of the frame; this is what shows whether a mode meets
# that, so a thin mode is caught at review rather than at the next perf question.
# usage: perf/layer_coverage.sh [width height reps theme] [filter] [out]
set -euo pipefail
width="${1:-400}"
height="${2:-120}"
reps="${3:-3}"
theme="${4:-moss}"
filter="${5:-}"
# A filtered run is a scratch question, not the roster receipt, so it lands in /tmp.
if [[ -n "${6:-}" ]]; then
  out="$6"
elif [[ -z "$filter" ]]; then
  out="perf/results/layer_coverage.md"
else
  out="/tmp/layer_coverage-${filter//[^A-Za-z0-9_-]/_}.md"
fi
mkdir -p perf/results "$(dirname "$out")"
env \
  ASCII_LAYER_WIDTH="$width" \
  ASCII_LAYER_HEIGHT="$height" \
  ASCII_LAYER_REPS="$reps" \
  ASCII_LAYER_THEME="$theme" \
  ASCII_LAYER_FILTER="$filter" \
  cargo test --release --quiet layer_coverage_report -- --ignored --nocapture --test-threads=1 2>/dev/null \
  | sed -n '/^# layer coverage/,$p' | sed '/^test result/,$d' | grep -v '^\.$' | tee "$out"
echo "wrote $out"

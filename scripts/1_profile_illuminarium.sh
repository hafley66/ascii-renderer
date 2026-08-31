#!/usr/bin/env bash
set -euo pipefail

profile_width="${1:-320}"
profile_height="${2:-100}"
profile_frames="${3:-600}"
profile_fps="${4:-60}"
profile_time_step="${5:-0.06}"

env \
  ASCII_PROFILE=1 \
  ASCII_PROFILE_EVERY="$profile_frames" \
  ASCII_PERF_WIDTH="$profile_width" \
  ASCII_PERF_HEIGHT="$profile_height" \
  ASCII_PERF_FRAMES="$profile_frames" \
  ASCII_PERF_FPS="$profile_fps" \
  ASCII_PERF_DT="$profile_time_step" \
  cargo test --release --quiet perf_illuminarium_generation_and_terminal_encoding \
    -- --ignored --nocapture

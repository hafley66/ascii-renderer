#!/bin/bash
# Rebuild the viewer and screenshot the clay panel at a few yaws. Usage: docs/maha_rig/shot.sh [outdir] [yaws...]
set -e
cd "$(dirname "$0")/../.."
out=${1:-/tmp/maha_shots}; shift || true
yaws=${@:-0 45 90 180}
mkdir -p "$out"
cargo run -q --release --example 3_maha_rig -- viewer 0.12 >/dev/null
C="/Applications/Google Chrome.app/Contents/MacOS/Google Chrome"
for y in $yaws; do
  timeout 60 "$C" --headless=new --use-angle=swiftshader --enable-unsafe-swiftshader --user-data-dir="$out/prof" \
    --window-size=1000,700 --timeout=9000 --screenshot="$out/yaw_$y.png" "file://$PWD/docs/maha_rig/viewer.html?yaw=$y" >/dev/null 2>&1 || true
done
ls "$out"/yaw_*.png

#!/bin/bash
# Rebuild the viewer and screenshot the clay panel at a few yaws. Usage: [CAM="&dist=12&ty=9"] docs/maha_rig/shot.sh [outdir] [yaws...]
set -e
cd "$(dirname "$0")/../.."
out=${1:-/tmp/maha_shots}; shift || true
yaws=${@:-0 45 90 180}
mkdir -p "$out"
cargo run -q --release --example 3_maha_rig -- viewer 0.12 >/dev/null
C="/Applications/Google Chrome.app/Contents/MacOS/Google Chrome"
for y in $yaws; do
  timeout 60 "$C" --headless=new --use-angle=swiftshader --enable-unsafe-swiftshader --user-data-dir="$out/prof" \
    --window-size=1000,700 --timeout=9000 --screenshot="$out/yaw_$y.png" "file://$PWD/docs/maha_rig/viewer.html?yaw=$y$CAM" >/dev/null 2>&1 || true
done
ls "$out"/yaw_*.png
# Side-by-side strip of the clay panels only.
args=(); f=""; n=0
for y in $yaws; do args+=(-i "$out/yaw_$y.png"); f+="[$n:v]crop=440:560:32:66[c$n];"; n=$((n+1)); done
ins=""; for i in $(seq 0 $((n-1))); do ins+="[c$i]"; done
ffmpeg -loglevel error -y "${args[@]}" -filter_complex "${f}${ins}hstack=inputs=$n" "$out/strip.png" 2>/dev/null || cp "$out/yaw_$(echo $yaws | cut -d' ' -f1).png" "$out/strip.png"
echo "$out/strip.png"

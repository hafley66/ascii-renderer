#!/usr/bin/env bash
set -euo pipefail

repo_dir=$(cd "$(dirname "$0")/.." && pwd)
"$repo_dir/scripts/0_generate_modes.sh" --watch &
watch_pid=$!
trap 'kill "$watch_pid" 2>/dev/null || true' EXIT INT TERM

cd "$repo_dir"
cargo run -- "$@"

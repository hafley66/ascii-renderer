#!/usr/bin/env bash

set -u

mode=${1:-}
animation_flag=${2:-}

repo_root=$(git rev-parse --show-toplevel 2>/dev/null) || {
    printf 'run inside the ascii-renderer repository\n' >&2
    exit 2
}
cd "$repo_root" || exit 2

if [[ ! "$mode" =~ ^[a-z0-9][a-z0-9+-]*$ ]]; then
    printf 'usage: %s MODE [--animated]\n' "$0" >&2
    exit 2
fi

count_literal() {
    local needle=$1
    local file=$2
    local matches
    matches=$(rg -n -F "$needle" "$file" 2>/dev/null || true)
    if [[ -z "$matches" ]]; then
        printf '0'
    else
        printf '%s\n' "$matches" | wc -l | tr -d ' '
    fi
}

dispatch_count=$(count_literal "mode == \"$mode\"" src/cli.rs)
demo_matches=$(rg -n "^[[:space:]]*\"${mode}\",?$" src/opts.rs 2>/dev/null || true)
if [[ -z "$demo_matches" ]]; then
    demo_count=0
else
    demo_count=$(printf '%s\n' "$demo_matches" | wc -l | tr -d ' ')
fi
snapshot_count=$(count_literal "\"$mode\"" tests/snapshot_modes.rs)
form_count=$(count_literal "\"$mode\"" src/registry.rs)
iterate_matches=$(rg -n "^[[:space:]]*\"${mode}\" =>" src/morph.rs 2>/dev/null || true)
if [[ -z "$iterate_matches" ]]; then
    iterate_count=0
else
    iterate_count=$(printf '%s\n' "$iterate_matches" | wc -l | tr -d ' ')
fi

printf 'mode\t%s\n' "$mode"
printf 'dispatch refs\t%s\tsrc/cli.rs\n' "$dispatch_count"
printf 'demo refs\t%s\tsrc/opts.rs\n' "$demo_count"
printf 'snapshot refs\t%s\ttests/snapshot_modes.rs\n' "$snapshot_count"
printf 'form refs\t%s\tsrc/registry.rs\n' "$form_count"
printf 'native iterate refs\t%s\tsrc/morph.rs\n' "$iterate_count"

status=0
if [[ "$dispatch_count" -lt 1 || "$demo_count" -lt 1 || "$snapshot_count" -lt 1 ]]; then
    status=1
fi
if [[ "$animation_flag" == "--animated" && ( "$form_count" -lt 1 || "$iterate_count" -lt 1 ) ]]; then
    status=1
fi

exit "$status"

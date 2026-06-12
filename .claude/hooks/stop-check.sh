#!/bin/bash
# Stop hook: don't end a turn with a broken tree.
# Runs cargo test; on failure exits 2 with the failure summary so Claude
# keeps working instead of stopping. Guards against infinite stop loops via
# stop_hook_active. Fail-open on unexpected input or missing tools.

input="$(cat)"

# Without jq we cannot honor the stop_hook_active loop guard, so fail open
# rather than risk re-firing exit 2 forever on a persistently failing tree.
command -v jq >/dev/null 2>&1 || exit 0
active="$(printf '%s' "$input" | jq -r '.stop_hook_active // false' 2>/dev/null)"
[ "$active" = "true" ] && exit 0

command -v cargo >/dev/null 2>&1 || exit 0
root="${CLAUDE_PROJECT_DIR:-$(pwd)}"
cd "$root" 2>/dev/null || exit 0
[ -f Cargo.toml ] || exit 0

test_out="$(cargo test --quiet 2>&1)"
test_status=$?
if [ "$test_status" -ne 0 ]; then
  printf '%s\n' "$test_out" | tail -30 >&2
  echo "cargo test is failing — fix the tree before ending the turn." >&2
  exit 2
fi

exit 0

#!/bin/bash
# PostToolUse hook (matcher: Edit|Write).
#   *.rs              → format the edited file, then clippy with -D warnings;
#                       failures surface back to Claude (exit 2) so warnings
#                       are fixed immediately, not at commit time.
#   queries/**/*.scm  → run the query snapshot/contract tests (cargo test query_).
# Fail-open (exit 0) on unexpected input or missing tools so a hook bug never
# bricks the session.

input="$(cat)"
command -v jq >/dev/null 2>&1 || exit 0

file="$(printf '%s' "$input" | jq -r '.tool_input.file_path // empty' 2>/dev/null)"
[ -n "$file" ] || exit 0

root="${CLAUDE_PROJECT_DIR:-$(pwd)}"
cd "$root" 2>/dev/null || exit 0
command -v cargo >/dev/null 2>&1 || exit 0
[ -f Cargo.toml ] || exit 0

case "$file" in
  *.rs)
    cargo fmt -- "$file" 2>/dev/null
    clippy_out="$(cargo clippy --quiet -- -D warnings 2>&1)"
    clippy_status=$?
    if [ "$clippy_status" -ne 0 ]; then
      printf '%s\n' "$clippy_out" | tail -20 >&2
      echo "clippy failed (-D warnings) after editing $file — fix the warnings above now." >&2
      exit 2
    fi
    ;;
  *queries/*.scm)
    test_out="$(cargo test --quiet -p repomap query_ 2>&1)"
    test_status=$?
    if [ "$test_status" -ne 0 ]; then
      printf '%s\n' "$test_out" | tail -30 >&2
      echo "Query tests failed after editing $file — fix the query or update the snapshot/fixture." >&2
      exit 2
    fi
    ;;
esac

exit 0

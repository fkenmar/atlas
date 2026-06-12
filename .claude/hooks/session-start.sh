#!/bin/bash
# SessionStart hook: open every session with milestone context.
# Prints STATUS.md and the last 3 ADR titles to stdout (added to Claude's
# context). Fail-open: never block a session over a missing file.

cat >/dev/null 2>&1 # drain the hook JSON from stdin; not needed here

root="${CLAUDE_PROJECT_DIR:-$(pwd)}"

if [ -f "$root/STATUS.md" ]; then
  echo "=== STATUS.md (current milestone — keep the board fresh) ==="
  cat "$root/STATUS.md"
  echo
fi

if [ -d "$root/docs/adr" ]; then
  echo "=== Last 3 ADRs (docs/adr/ — append-only) ==="
  ls "$root/docs/adr" 2>/dev/null | LC_ALL=C sort | tail -3 | while IFS= read -r f; do
    title="$(head -1 "$root/docs/adr/$f" 2>/dev/null)"
    echo "- ${title:-$f}"
  done
fi

exit 0

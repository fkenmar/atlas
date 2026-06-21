# Generating a map section for CLAUDE.md / AGENTS.md

Agent context files like `CLAUDE.md` and `AGENTS.md` are **hand-written project
guidance** — atlas doesn't replace them. But a current repo map is useful context
to include, and atlas can keep a *delimited section* of one of these files fresh
without touching your manual instructions.

The pattern: a fenced map between two HTML-comment markers that a script
regenerates, leaving everything outside the markers alone. The section looks
like this in your file (a fenced `# atlas: …` map sitting between the markers):

    <!-- atlas:start (generated — do not edit inside this block) -->
    ```
    # atlas: your-repo (3740 LOC, 16 files) | budget 1024 | rendered 1012 tok
    ...
    ```
    <!-- atlas:end -->

## Manual approach

Add the two markers anywhere in your `AGENTS.md`, then paste a map between them:

```sh
atlas . --budget 1024
```

Re-paste when the structure changes. That's it — everything outside the markers
is yours.

## Scripted approach (non-destructive)

This script regenerates **only** the delimited block. It appends the block the
first time and replaces it in place afterward, so your hand-written content is
never overwritten. It creates the file if it doesn't exist, and never rewrites an
existing file's manual text.

```sh
#!/usr/bin/env sh
# Refresh the atlas block in an agent file (default: AGENTS.md). Usage:
#   ./atlas-section.sh            # updates AGENTS.md
#   ./atlas-section.sh CLAUDE.md
set -e
FILE="${1:-AGENTS.md}"
START="<!-- atlas:start (generated — do not edit inside this block) -->"
END="<!-- atlas:end -->"

# Render the map into a fenced, delimited block.
{
  printf '%s\n' "$START"
  printf '```\n'
  atlas . --budget 1024
  printf '```\n'
  printf '%s\n' "$END"
} > .atlas-block.tmp

if [ -f "$FILE" ] && grep -qF "$START" "$FILE"; then
  # Replace the existing block, leaving all other text untouched.
  awk -v start="$START" -v end="$END" '
    $0 == start { while ((getline line < ".atlas-block.tmp") > 0) print line; skip=1; next }
    $0 == end   { skip=0; next }
    !skip       { print }
  ' "$FILE" > "$FILE.tmp" && mv "$FILE.tmp" "$FILE"
else
  # No block yet: append one (creates the file if absent). Never rewrites
  # existing manual content.
  { [ -f "$FILE" ] && printf '\n'; cat .atlas-block.tmp; } >> "$FILE"
fi
rm -f .atlas-block.tmp
```

Running it twice is idempotent: the second run replaces the block rather than
duplicating it. Because it only ever rewrites text **between the markers**, it's
safe to run against a file full of hand-written guidance.

> **Don't point it at a file with no markers expecting a merge.** On a
> marker-less file it *appends* a new block (safe, non-destructive). If you'd
> rather it refuse than append, add a guard, or add the markers yourself first.

## Keeping it fresh

Wire the script into the same automation as the rest of your map workflow:

- run it from a [pre-commit hook](pre-commit.md) so the section updates on commit;
- or in [CI](ci-recipes.md) with a freshness check on the agent file;
- see the [agent cookbook](agent-cookbook.md) for how agents consume the map.

If you'd rather scaffold these integration files from scratch, that's the job of
`atlas init` (tracked in [#52](https://github.com/fkenmar/atlas/issues/52)).

---
description: The Sunday 20-minute ritual — bench, milestone audit, plan the week
---
The Sunday ritual, in order, sized to 20 minutes total:

1. Run /bench.
2. Run /milestone.
3. Summarize what changed since the last review: `git log --oneline` since the previous weekly-review commit (or the last 7 days if none), grouped by pipeline stage — one line per theme, not per commit. Include the week's new benchmark/history.md rows (the measured deltas) and anything added to ideas.md "Tried and reverted".
4. Propose the coming week's board for ~10 hours of work:
   - **NOW** — fits in the week, ordered; items that close ❌ exit criteria come first.
   - **NEXT** — queued behind NOW.
   - **NOT-YET** — parked, each tagged with its milestone.
5. Wait for my approval of the proposed board, then update STATUS.md (board + any exit-criteria checkboxes the /milestone audit justified) and commit it as the week's review marker.

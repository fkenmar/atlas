---
name: benchmark-protocol
description: Use for benchmark work — running or interpreting ./benchmark/run.sh, recording or questioning baselines, baseline.json, adding benchmark tasks, or any "is this ranking/budgeting change actually better?" question.
---

# The atlas benchmark protocol

The agent-task benchmark is the arbiter of every ranking/budgeting change (PRD §8): 10 Claude Code tasks on a pinned ~50k-LOC repo, measuring **exploration tokens** and **turns-to-completion** with the map in context vs. without. v0.1 target: ≥25% reduction in exploration tokens. If a change doesn't move these numbers, it doesn't matter how clever it is.

## How run.sh works

`./benchmark/run.sh [tasks…]` iterates `benchmark/tasks/*.yaml` (or just the ones you pass), runs each task's two arms — **without-map** (plain session) and **with-map** (atlas output injected at session start) — and writes one result JSON per invocation to `benchmark/results/run-<stamp>.local.json` (gitignored). Real agent-session execution is the marked M0 integration point in the script; until it lands, run.sh emits the result schema with nulls so reporting tooling can be built against real shapes.

## Task format (one YAML per task)

```yaml
id:       unique slug, matches the filename
title:    one line
prompt:   the verbatim task given to the agent
repo:     { url, rev }   # pinned rev — comparisons across revs are invalid
success:  { check, anti_pattern }   # objective pass criterion + what counts as failure
notes:    scoring guidance, pitfalls
```

Good tasks are (1) realistic agent asks, (2) objectively checkable, and (3) sensitive to repo knowledge — an agent that knows the structure should win measurably.

## Baselines and why baseline.json is edit-protected

`benchmark/baseline.json` holds the recorded no-map numbers every run is compared against. It is regenerated **only** by `./benchmark/run.sh --record-baseline` — a PreToolUse hook blocks hand edits, because a baseline you can casually edit is not a baseline. Re-record (and say so in the commit) when the task set, target repo rev, or measurement method changes; never to make a delta look better.

## Statistical hygiene

- **3 runs minimum per arm; report the MEDIAN**, never a single run or the mean.
- Note variance. >15% spread between repeats means the comparison is suspect — investigate (model nondeterminism, repo state leak, task ambiguity) before concluding anything.
- Compare like with like: same model, same pinned repo rev, same task set. Anything else is a new baseline, not a delta.

## The rule

A ranking or budgeting PR without a benchmark delta is not done (CLAUDE.md). Run `/bench`, report the per-task table and the aggregate vs. the ≥25% target, and put the delta in the PR/commit description.

## The comprehension gate

`./benchmark/comprehension.sh` measures understanding: read-only Q&A sessions against the pinned repo, scored against verified answer keys (`comprehension/questions-*.yaml`). Hard gate: **with-map accuracy ≥ without-map accuracy** — a drop means the map misleads (faster but wronger) and the change reverts regardless of token wins. Run it whenever map content or rendering changes. Answer-key rule: never commit an expected answer you haven't verified against the pinned clone.

## The ledger

Every measured change appends one row to `benchmark/history.md` (medians, pass rate, Δ vs the previous comparable row), committed alongside the change — the self-improvement loop (docs/SELF_IMPROVEMENT.md) does this in its record step. "Comparable" means same arm, model, repo rev, task set, and metric version; anything else starts a new comparison chain, noted in the row.

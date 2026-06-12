# repomap agent-task benchmark

The arbiter of every ranking/budgeting change (PRD §8): does a repomap in context actually reduce agent exploration? If a change doesn't move these numbers, it doesn't ship as an improvement.

## What it measures

10 Claude Code tasks (growing from the 2 committed examples) on a pinned ~50k-LOC target repo. Each task runs in two arms:

- **without-map** (baseline): plain Claude Code session.
- **with-map**: `repomap` output for the target repo injected into context at session start.

(A preliminary with-map probe with the naive unbudgeted map was run 2026-06-12 — turns −41–43%, tokens up from cache re-reads of the oversized map; see STATUS.md. The official comparison starts when M1 budgeting makes the arm fair.)

Per arm, per run, we record:

- **exploration tokens** — tokens spent reading/searching before the first correct edit;
- **turns-to-completion**;
- whether the task's **success criterion** passed.

v0.1 target: **≥25% reduction in exploration tokens** (aggregate across tasks).

### M0 metric operationalization

The PRD-pure definition ("tokens before the first correct edit") needs transcript analysis; until that lands, the M0 proxy recorded by run.sh is **total input-side tokens processed per session** (fresh input + cache creation + cache reads from the headless `claude -p` usage report) plus `num_turns`. Both arms are measured identically, sessions run in `acceptEdits` permission mode (file edits allowed, no shell — same constraints for both arms), so comparisons are valid; only the absolute numbers will shift when the metric is refined. Recorded baselines state model and turn cap in their `environment` field — a baseline is only comparable against runs with the same settings.

## Protocol

- **3 runs minimum per arm per task; the MEDIAN is the reported number.** Never a single run, never the mean.
- Report variance; >15% spread between repeats makes the comparison suspect — investigate before concluding.
- Compare like with like: same model, same pinned repo rev, same task set. Changing any of those means re-recording the baseline, not comparing across it.
- Local results go to `results/run-<stamp>.local.json` (gitignored); the recorded no-map baseline lives in `baseline.json`.

## Files

- `run.sh` — the runner (real headless execution since M0). For each task it shallow-clones the pinned repo into `.work/cache/`, copies a fresh working tree per run, drives `claude -p` (model/runs/turn-cap via `BENCH_MODEL`/`BENCH_RUNS`/`BENCH_MAX_TURNS`), records tokens/turns/cost, and evaluates the task's `success.cmd`. The `with_map` arm exists but is off by default until budgeting lands (M1) — injecting the unbudgeted naive map would not be a fair or realistic arm. Requires: `claude` CLI (logged in), git, jq, python3 + PyYAML.
- `tasks/*.yaml` — one task per file: `id`, `title`, `prompt` (verbatim agent task), `repo {url, rev}` (pinned), `success {check, cmd, anti_pattern}` (`cmd` is the machine-checkable form run.sh executes in the working clone), `notes`.
- `baseline.json` — recorded no-map numbers, with the schema documented inside it. Regenerated **only** by `./run.sh --record-baseline`; a PreToolUse hook blocks hand edits, because a baseline you can casually edit is not a baseline.
- `results/` — per-invocation outputs; `*.local.json` is gitignored.

## Adding a task

Copy an existing YAML. Good tasks are:

1. **realistic** — something a user would actually ask an agent;
2. **objectively checkable** — the success criterion is a test, a diff property, or a grep, not an opinion;
3. **knowledge-sensitive** — an agent that knows the repo structure should win measurably (tasks where grep finds the answer in one shot don't discriminate).

The committed example tasks target pytest 8.2.0 as a stand-in until the M0 target-repo decision is made; re-target them when it is (and re-record the baseline — see Protocol).

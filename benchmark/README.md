# atlas agent-task benchmark

The arbiter of every ranking/budgeting change (PRD §8): does an atlas map in context actually reduce agent exploration? If a change doesn't move these numbers, it doesn't ship as an improvement.

## Reproduce the headline number

The README's claim — **~65% fewer tokens at identical accuracy** — is the
comprehension benchmark, and you can run it yourself. It's the trustworthy,
low-variance signal atlas stands behind (edit-task token deltas are too noisy to
headline — see the variance section below).

```sh
cargo build --release          # builds target/release/atlas (the with-map arm)
./benchmark/comprehension.sh   # 20 verified questions, both arms, read-only
```

Requirements: the `claude` CLI (logged in), `git`, `jq`, and `python3` with
PyYAML. Knobs: `BENCH_MODEL` (default `claude-sonnet-4-6`), `BENCH_QLIMIT=N` for a
quick smoke run, `ATLAS_BIN` to point at a different binary.

**What you should see** (the recorded run, `results/comprehension-20260617-084740`):

| Arm         | Accuracy | Median tokens | Median turns |
| ----------- | -------- | ------------- | ------------ |
| without map | 20/20    | 85,670        | 3            |
| with map    | 20/20    | 29,781        | 1            |

Same accuracy, **−65.2% tokens**, 3 → 1 turns, at the shipped default 2,048-token
budget. The questions ("which class is the central config object?", "name the
path-normalization helper") and answer keys live in
[`comprehension/`](comprehension/), each verified against the pinned clone before
commit. The run is a live agent against a real repo, so exact tokens vary
slightly run to run — but comprehension is constrained and stable (unlike the
60–140% swings on edit tasks), which is why this is the number we publish.

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

### Metric operationalization (M1 refinement — results schema_version 3)

The reported number is now **`exploration_tokens`: the input-side tokens (fresh input + cache creation + cache reads) the agent processes *up to and including the turn of its first file edit*** — computed by `metric.py` from the `--output-format stream-json` transcript. This isolates the exploration phase the map is meant to shrink and, critically, stops counting at the first edit, so editing/verification/retry turns and a 30-turn-cap blowout no longer dominate the number (the failure mode that made the M0 proxy too noisy — see history.md, the 2026-06-16 checkpoint). A session that never edits has `exploration_tokens == total_tokens` (it explored the whole time). `total_tokens` (the old whole-session sum) and `turns_to_first_edit` are recorded alongside for context.

Both arms are measured identically in `acceptEdits` permission mode (file edits allowed, no shell), so comparisons are valid. **This metric is not comparable to the old M0 proxy** (results schema ≤ 2 / `baseline.json` schema 1): the baseline must be re-recorded with `run.sh --record-baseline` before the new numbers can be compared against it. Recorded baselines state model and turn cap in their `environment` field — a baseline is only comparable against runs with the same settings.

The **old M0 proxy** was total input-side tokens across the *whole* session plus `num_turns` — accurate but turn-cap-dominated, so a long verification phase could swamp the exploration signal.

## Protocol

- **3 runs minimum per arm per task; the MEDIAN is the reported number.** Never a single run, never the mean.
- **Medians are over passing runs only.** A run that edited early but failed its success criterion is not a valid exploration sample (the first edit was wrong), and counting it would bias exploration tokens downward. Every run — pass or fail — is kept in the `per_run` array for diagnostics; a failed/malformed session is recorded `null`, never `0`.
- Report variance; >15% spread between passing repeats makes the comparison suspect — investigate before concluding.
- Compare like with like: same model, same pinned repo rev, same task set, **and the same metric generation** (results `schema_version` / `metric` must match the baseline's — the exploration metric is not comparable to the old whole-session proxy). Changing any of those means re-recording the baseline, not comparing across it.
- Local results go to `results/run-<stamp>.local.json` (gitignored); the recorded no-map baseline lives in `baseline.json`.

### What this suite can and can't claim (variance reality — settled 2026-06-17, N=5)

The N=5 run (history.md) is decisive: **edit-task `exploration_tokens` is too noisy to support a quantified claim.** Across N=5 the `without_map` arm swings **127–139%** around its median (one task ranged 784k → 3.5M tokens), and *turns* and *msgs-to-edit* are no calmer (2–5× per-run swings). The agent's nondeterministic path, not the map, dominates the number. Therefore:

- **Edit-task token deltas are reported, not claimed.** Any aggregate whose passing-run spread exceeds ~30% is **INCONCLUSIVE** — record it, never headline it. The +78% / +53.9% from the N=3 run did **not** survive N=5 (it was one lucky `without_map` blowup); treat single-run extremes as noise, not signal.
- **The comprehension benchmark is the primary worthiness gate.** Constrained "locate the structural element" Q&A (20 verified questions) is low-variance and tests the value prop directly: does the map let the agent answer *as accurately* using *fewer tokens*? A download-worthy claim rests here (accuracy held at materially fewer tokens), **not** on edit-task token medians.
- **Turns is a secondary, still-soft signal** — directionally the map cuts turns ~25%, but the per-run spread means cite it as a tendency, not a number.
- **To make the edit benchmark claim-worthy:** lower-variance task design + a bigger suite (10+ tasks) + trimmed-mean statistics. Until then it *characterizes behavior* (helps find-the-thing, hurts multi-site edits) — it does not *quantify* a win.

No public / README / marketing token-reduction number ships without a result that clears this gate.

## Files

- `run.sh` — the runner (real headless execution since M0). For each task it shallow-clones the pinned repo into `.work/cache/`, copies a fresh working tree per run, drives `claude -p` (model/runs/turn-cap via `BENCH_MODEL`/`BENCH_RUNS`/`BENCH_MAX_TURNS`), records tokens/turns/cost, and evaluates the task's `success.cmd`. The `with_map` arm exists but is off by default until budgeting lands (M1) — injecting the unbudgeted naive map would not be a fair or realistic arm. Requires: `claude` CLI (logged in), git, jq, python3 + PyYAML.
- `tasks/*.yaml` — one task per file: `id`, `title`, `prompt` (verbatim agent task), `repo {url, rev}` (pinned), `success {check, cmd, anti_pattern}` (`cmd` is the machine-checkable form run.sh executes in the working clone), `notes`.
- `baseline.json` — recorded no-map numbers, with the schema documented inside it. Regenerated **only** by `./run.sh --record-baseline`; a PreToolUse hook blocks hand edits, because a baseline you can casually edit is not a baseline.
- `history.md` — append-only ledger: one row per measured change with its medians and Δ vs the previous comparable row, committed alongside the change (the self-improvement loop appends these — docs/SELF_IMPROVEMENT.md).
- `comprehension.sh` + `comprehension/` — the understanding benchmark (section above); results land in `results/comprehension-*.local.json`.
- `results/` — per-invocation outputs; `*.local.json` is gitignored.

## Comprehension benchmark — understanding, not just speed

Token and turn savings are worthless if the map makes agents **faster but wronger** — answering from signatures without verifying, or trusting a stale/misranked map. The comprehension benchmark guards that axis:

- `comprehension/questions-<repo>-<rev>.yaml` — repo-understanding questions ("which class is the central config object?", "name the existing path-normalization helper") with answer keys of exact identifiers/paths, **every entry verified against the pinned clone before commit**.
- `comprehension.sh` — runs each question as one read-only headless session per arm (no edit permissions, identical constraints), scores answers by substring match (`any`/`all`), reports per-arm accuracy plus median tokens/turns. `BENCH_QLIMIT=N` for smoke runs.
- **The bar (hard gate in the self-improvement loop):** with-map accuracy ≥ without-map accuracy. An accuracy drop is a regression that no token win can buy back. Ideally the map *improves* accuracy — it puts the answer's location in context.
- When the change is map content or rendering (extraction, ranking, budgeting, render), the loop runs this alongside the edit-task benchmark; competitor arms (below) apply here too once M1 lands.

### Coverage proxy — the free inner loop (`coverage.py`)

`comprehension.sh` is billed (one `claude -p` session per question per arm), so it can't run on every ranking tweak. `coverage.py` is its **deterministic, zero-cost proxy**: for each question it checks whether the answer key (`expect`, scored by the same `any`/`all` rule) appears in the atlas map itself — "is the answer in the map?" The comprehension runs established that an answer in the map ⇒ a one-turn correct answer, so coverage tracks the accuracy axis without an LLM.

```
python3 benchmark/coverage.py --budget 2048,3072          # full set, sweep budgets
python3 benchmark/coverage.py comprehension/questions-smoke.yaml -v
```

It reproduces the recorded numbers exactly — **12/20 @ 2048, 17/20 @ 3072** (matching the symbol-index result) — in milliseconds. Use it as the fast inner loop when tuning ranking/budgeting (e.g. per-symbol ranking): keep what raises coverage, then confirm the keep/revert at a milestone with the billed `comprehension.sh`. It does **not** replace the billed run — an answer present in the map could still be misread — it makes the iteration *between* runs cheap and objective.

## Competitive arms — the next protocol step (post-M1)

Beating our own no-map baseline is necessary but not sufficient: repomap has to beat the **existing alternatives** (PRD §12) on the same task suite, same protocol, same model. Once the M1 budgeted map exists, each competitor becomes one more arm per task — the injection mechanism is identical (context prepended at session start), only the artifact changes:

| Arm | Artifact injected | What it tests |
|---|---|---|
| `repomap` | M1 budgeted map (default 2,048 tok) | our product |
| `aider_map` | Aider's repo-map output for the same repo (extracted via Aider, same token ballpark) | the proven incumbent — **the bar that matters most** |
| `ctags` | universal-ctags symbol dump, trimmed to the same token budget | flat symbol list, no graph/ranking |
| `file_tree` | bare directory listing, same budget | cheap control: is structure alone enough? |
| `without_map` | nothing | the recorded baseline |

**The bar:** at equal injected-context budget, repomap must beat every competitor arm on median exploration tokens AND turns, and `aider_map` specifically must not beat us on either — if it does, that's a ranking-quality gap to close before v0.1 ships (PRD §10 lists this exact risk). Results get the same 3-run/median/variance hygiene as everything else, and the comparison table goes in the README the day we have it.

## Adding a task

Copy an existing YAML. Good tasks are:

1. **realistic** — something a user would actually ask an agent;
2. **objectively checkable** — the success criterion is a test, a diff property, or a grep, not an opinion;
3. **knowledge-sensitive** — an agent that knows the repo structure should win measurably (tasks where grep finds the answer in one shot don't discriminate).

The committed example tasks target pytest 8.2.0 as a stand-in until the M0 target-repo decision is made; re-target them when it is (and re-record the baseline — see Protocol).

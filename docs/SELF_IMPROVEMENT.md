# The self-improvement loop

repomap develops itself in a measured loop instead of ad-hoc prompting: each
iteration picks one item off the board, implements it, proves the tree is
healthy, **measures whether it actually helped**, and either keeps it (with
the stats recorded) or reverts it (with the failed approach logged). The
human's job shrinks to the decisions only a human can make.

## How to run it

- `/improve` — one iteration.
- `/loop /improve` — continuous, self-paced iterations in a session; the
  loop ends itself at a stop condition (below).
- `/weekly-review` — the human checkpoint: audits what the loop did, re-aims
  the board for the coming week.

## One iteration

1. **Orient.** Read STATUS.md. Pick exactly ONE item: an unmet milestone
   exit criterion first, else the top NOW item. Small enough to land,
   verify, and measure in one iteration — split it if it isn't.
2. **Snapshot the before-stats** for whatever the item can affect:
   - always: `cargo test` count green, clippy clean;
   - extraction changes: current snapshot files;
   - perf-relevant changes: `time target/release/repomap <pinned pytest clone>`;
   - agent-behavior changes (ranking/budgeting/extraction/render): the
     relevant medians from benchmark/history.md.
3. **Implement.** Tests first where the change is testable (extraction →
   fixture + snapshot; budget → ladder-order unit tests). Honor CLAUDE.md
   conventions; new dependencies are a human gate — pause and ask.
4. **Gate.** `cargo fmt && cargo clippy --all-targets -- -D warnings &&
   cargo test`. Red gate = fix or revert; never proceed to measurement on a
   broken tree.
5. **Measure.** Only what the change can affect:
   - perf: re-time the pytest map; compare to NFR-1 (≤2 s cold) and the
     previous timing;
   - agent behavior: `./benchmark/run.sh` (3 runs/arm, medians) and compare
     against benchmark/baseline.json AND the last history.md row;
   - **understanding**: any change to what the map contains or how it reads
     (extraction, ranking, budgeting/degradation, rendering) also runs
     `./benchmark/comprehension.sh` — the map must not make agents faster
     but wronger;
   - everything else: the gate itself is the measurement.
6. **Decide and record.**
   - **Improved or neutral-by-design** (e.g. scaffolding for a later step —
     say so explicitly): update STATUS.md board, append the stats row to
     benchmark/history.md (if measured), add the CHANGELOG line with the
     delta, commit — one commit per iteration, stats in the message.
   - **Regressed**: `git restore` the change, append the failed approach +
     numbers to ideas.md under "Tried and reverted" so future iterations
     don't retry it blind. A revert is a successful iteration — the loop
     learned something for a few dollars.
7. **Report.** One block: item, before → after stats, Δ%, keep/revert, and
   what the next iteration should pick up.

## What counts as "improved"

- **Benchmark:** lower median exploration tokens AND turns at an equal or
  better success-criterion pass rate, per benchmark/README.md hygiene
  (3 runs minimum, medians, variance flagged >15%). One metric up and one
  down = not an improvement; say which and why before keeping.
- **Understanding (hard gate):** with-map comprehension accuracy ≥
  without-map accuracy on the question set. Any accuracy drop is a
  regression no token or turn win can buy back — revert or fix.
- **Perf:** lower cold/warm wall time on the pinned repo, release build.
- **Extraction:** snapshot diffs reviewed line-by-line; more correct
  symbols, not just more symbols.
- Compare like with like: same model, same pinned repo rev, same task set,
  same metric version — anything else is a new baseline, not a delta.

## Cost guards

Benchmark iterations cost real money (~$0.3–1.5 per edit-task session; a
3-run two-task arm ≈ $3–5; a full comprehension pass — 10 read-only Q&A
sessions per arm — ≈ $1–4). The loop runs benchmarks only when the change
can plausibly move agent behavior or map content, never for
refactors/docs/CI. If an iteration would push the session's total benchmark
spend past **$15**, pause and ask — that is a human gate, not a judgment
call.

## Stop conditions (end the loop, report, hand back)

- All current-milestone exit criteria met → report milestone complete; the
  human decides to advance (PRD §9 sequencing is a product decision).
- Blocked on a human gate: new dependency, scope question (/scope-check),
  baseline re-record, >$15 benchmark spend, or anything destructive.
- Two consecutive reverted iterations on the same item → stop and describe
  the blocker instead of burning a third attempt.

## The ledger

benchmark/history.md is the append-only stats ledger — one row per measured
change, committed with it. STATUS.md carries only the latest result;
history.md is how "how much have we improved since week 1?" stays a
30-second lookup instead of an archaeology dig.

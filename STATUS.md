# STATUS

## Current milestone: M0 — Foundation ✅ COMPLETE (2026-06-12)

**Exit criteria (PRD §9):**
- [x] Cargo workspace with tree-sitter + Python grammar wired *(tree-sitter 0.26.9 + tree-sitter-python 0.25.0; lib+bin layout; queries embedded; snapshot tests)*
- [x] Naive full map runs end-to-end on one real repo *(deepfake_detector: 7 py files / 1,658 LOC in 0.22 s, vendored junk excluded; pytest 8.2.0: 264 files / 92,156 LOC in 0.56 s cold — NFR-1 reference point)*
- [x] Agent benchmark harness built; baseline (no-map) numbers recorded in benchmark/baseline.json *(real headless `claude -p` runner; baseline recorded 2026-06-12, claude-sonnet-4-6, 3 runs/task, 6/6 success-criteria passes)*

Next milestone: **M1 — Core (v0.1 alpha)**: TS/JS + Rust grammars; import linking; PageRank; tiktoken budgeting; md + json renderers; cache; gitignore (2 wks). Burn-down runs through the self-improvement loop — `/improve` or `/loop /improve` (docs/SELF_IMPROVEMENT.md); measured changes append to benchmark/history.md.

## M1 core built — 2026-06-16 (autonomous session)

The full pipeline now runs end-to-end (`atlas [PATH] --budget --focus --lang --no-private --format md|json`). **All M1 functional requirements done:** FR-1 (TS+Rust grammars), FR-3/FR-11 (tiktoken `cl100k_base` budget + degradation ladder), FR-4 (personalized PageRank), FR-5 (md + json), FR-6 (bincode content-hash cache), FR-7 (`.gitignore`/`.atlasignore`), FR-12. **NFR-1 cold:** 0.25 s on pytest 92 k LOC (8× under the 2 s target; warm-path wall-clock verification still pending). **Remaining for M1 exit:** the *benchmark-shows-a-measurable-win* criterion (first fair with-map vs without-map checkpoint **in flight**), warm-path timing, optional rayon. Dogfood self-map of atlas's own source: 3.7 k LOC → ~1.4 k tokens at full detail. Quality fixes baked in: test-code excluded from extraction, ranking de-biased against symbol count, per-file one-line rung, language-aware visibility. Full history in CHANGELOG.md.

## Usability pass — post-worthiness (2026-06-17)

The comprehension worthiness gate **PASSED** (−30.1% tokens at 20/20 accuracy), so usability
work is justified. Cleared the audit's top recommendations (workflow w6xlnc3es) to make atlas
safe for real users / downloads:
- **Silent-failure class killed** (src/cli.rs): nonexistent path, file-as-path, empty /
  0-supported-files dir, `--budget 0`, all-unresolved `--focus`, and `--lang` mismatch now each
  emit an actionable stderr line + proper exit code (was: bare header at exit 0). `--lang <bad>`
  lists valid extensions; `--focus` accepts CSV; planned `serve`/`diff` say "not available yet."
- **`--help` teaches** — EXAMPLES block + long_about; dropped stale "json lands in later rungs"
  jargon; short flags `-b/-f/-l`.
- **Honest degraded header** (markdown.rs): cryptic `public-only` → `public API only
  (--no-private)` vs `private symbols … omitted to fit budget — raise --budget`, distinguishing
  user choice from budget pressure (new `requested_no_private` flag on BudgetedMap).
- **Self-ignoring cache**: atlas writes `.atlas/.gitignore` (`*`) so the cache never clutters
  `git status`.
- **README**: first-success check, "what it maps," troubleshooting, budget-degradation
  explainer, one-line installer, fixed `--focus` example, honest 30%-at-equal-accuracy headline.
Gate green throughout (69 tests). Lower-priority audit items remain (always-on map legend,
`-o/--output`, `--for-agent` preamble, shell completions [needs clap_complete dep approval]).

## GitHub issue triage + NFR-1 warm path — 2026-06-16

Filed the remaining roadmap as 14 GitHub issues (fkenmar/atlas) and organized them under
milestones **M1 — Core** (#1–#6, #14), **M2 — Integration** (#7–#9, #13), **M3 — Breadth**
(#10–#12). Triage outcome:

- **#2 NFR-1 warm path — VERIFIED, closed.** Measured on pytest 8.2.0 (256 files / 92k LOC,
  1.8× the 50k-LOC spec): **cold 668 ms** (clean run, atlas cache cleared, incl. render — 3×
  under the 2 s target) → **warm median 83 ms** (min 82, max 87; n=7) — **8× speedup**, well
  under the ≤200 ms warm target even at nearly 2× the spec repo size. Cache hit confirmed
  (single bincode blob under `.atlas/cache`). NFR-1 now verified on **both** halves.
- **#5 exploration-token metric — DONE, closed.** Already implemented (`metric.py` →
  tokens-up-to-first-edit in `run.sh`) and documented in benchmark/README.md.
- **#3 reverse-references — implementation DONE** (commits 144051b/d29a67d/0c846db = class
  fields; a42c7f9 = `used by` edges); only the billed benchmark validation remains, folded
  into #1.
- **#1 (N≥5 win confirmation)** is the one decisive blocker — billed (~$3–5) and tied to the
  paused 80% goal; left to the maintainer's go-ahead. **#14 release** gated on it.
- **#4 rayon** deferred (new-dep gate; cold path already 3× under target, low value now).
- M2/M3 epics (#7–#13) intentionally deferred behind M1 exit.

**Shipped this session:** cut **`v0.1.0-alpha`** GitHub pre-release (notes + verified
NFR-1 numbers; honestly labeled "M1 win pending #1"; no crates.io per maintainer).
Packaging: trimmed the published crate (`cargo package` 81 → 41 files via Cargo.toml
`exclude`) and set up **cargo-dist 0.32** (`dist-workspace.toml` + `.github/workflows/release.yml`)
— on every version tag, CI cross-builds macOS/Linux/Windows (x86_64+arm64) + a `curl|sh`
installer onto the GitHub release. (M2 item #9 pulled forward at the maintainer's request;
completes on its first tagged matrix run.) Verified locally via `dist plan` + dist-profile
host build. **RL** evaluated and parked (ideas.md) — does not fit the structural-only scope.

## Ship-prep + rename — 2026-06-16 (repomap → atlas)

Renamed the project to **atlas** end-to-end (crate, binary, map header, CLI messages, cache dir `.repomap`→`.atlas`, ignore file `.atlasignore`) — the binary is now `atlas`, `cargo install --path .` works. Made it usable for a general audience: rewrote README (problem-first, real example output, simple install/usage), added MIT `LICENSE`, added `repository`/`readme`/`keywords`/`categories` to Cargo.toml, added 10 GitHub topics for discoverability. Gate green (68 tests, clippy clean). **80% token-reduction goal paused at the measured ~70%** per maintainer ("stop around 70% for now"); the N≥5 benchmark to confirm the aggregate remains the open decisive measurement when the goal resumes. Local git remote still points at the old `RepoBrain.git` (push works via GitHub redirect; rewrite to `atlas.git` is a one-liner the maintainer can run). A tagged `v0.1.0` GitHub release is the natural next shipping step (gated on the M1 benchmark-win criterion).

## Board

| NOW | NEXT | NOT-YET |
|---|---|---|
| ~~TS/JS grammar (tree-sitter-typescript)~~ ✅ done 2026-06-16 | ~~Incremental cache + warm path~~ ✅ done 2026-06-16 (FR-6) | MCP server (M2) |
| ~~Rust grammar (tree-sitter-rust)~~ ✅ done 2026-06-16 | rayon parallel parse (M1) | --watch daemon (M2) |
| ~~Import linking → index-based graph (ADR 0002)~~ ✅ done 2026-06-16 | clap CLI: --budget/--format/--focus (M1; opens the CI self-map gate) | --focus personalization (M2) |
| ~~PageRank over the graph~~ ✅ done 2026-06-16 | ~~.gitignore/.repomapignore in discover (FR-7)~~ ✅ done 2026-06-16 | cargo-dist packaging (M2) |
| ~~tiktoken budgeting + degradation ladder~~ ✅ code done 2026-06-16 (bench owed at integration) | Refine exploration-token metric toward PRD definition (tokens before first correct edit) | Tier 2 grammars, XML renderer, repomap diff (M3) |
| ~~clap CLI + full pipeline wired (discover→…→render)~~ ✅ done 2026-06-16 | | |
| ~~Exclude inline #[cfg(test)] code from extraction~~ ✅ done 2026-06-16 (self-map: 2036 tok degraded → 1749 tok at FULL detail, 16/16 files) | | |
| Checkpoint benchmark (pytest with-map vs baseline) ← next | | |
| | Re-record baseline when with-map arm goes live (variance notes now auto-recorded by run.sh) | More benchmark tasks (target: 10) + decide long-term target repo (pytest 8.2.0 is the M0 stand-in) |
| | Competitive benchmark arms (post-M1): same suite vs Aider repo-map / ctags / file-tree control at equal budget — repomap must beat them all, Aider especially (protocol: benchmark/README.md §Competitive arms) | |

**Worthiness gate — PASSED (2026-06-17, comprehension run-004810, 20 verified questions):** with_map vs without_map both **20/20 accuracy (100%, zero regressions)**; tokens **85,817 → 59,916 = −30.1%**; turns 3→2. This is the trustworthy, low-variance signal — the map locates structural elements with ~30% fewer tokens at identical accuracy. **This is atlas's defensible value claim** (README cites it). The edit-task token deltas below stay INCONCLUSIVE (60–140% variance), but the worthiness question is settled: atlas earns its place in an agent's context, so usability/ship work is justified.

**Last benchmark result (2026-06-17 — DECISIVE N=5, run-232455, first run with the reverse-ref/field lever, spend $14.12):** the M1 "measurable benchmark win" exit criterion is **NOT met on edit-task tokens.** with_map vs without_map exploration tokens: **task 02 (find-the-thing) +22.9%**, **task 01 (multi-site edit) −66.8%**, **aggregate +8.9%** (sum of medians, below the ≥25% bar). The earlier N=3 +53.9%/+78% **did not hold up** — it was largely an artifact of one run's without_map blowup. Variance is still **64–139%** even at N=5, so medians are soft; the only robust signals are the task split (helps find-the-thing, hurts multi-site) and **turns −25%** in both arms. The reverse-ref/field lever helped task 01 (−186.7% → −66.8% vs run-101017) but did not flip it positive — keep it. The **80% goal is out of reach with this approach**; the prerequisite for any trustworthy token verdict is killing the variance (task redesign / trimmed means / larger N), not more map content. README's headline claim was corrected to the supported numbers (turns −25%, comprehension −45% at equal accuracy). #1 stays open; v0.1.0 (#14) stays gated — the alpha label was right. See benchmark/history.md + results/run-20260616-232455.local.json.

**Prior benchmark result (2026-06-16 — refined metric + density-improved map, run-101017, N=3 clean):** 0/12 capped, 12/12 pass (the cap fix worked). Same-run exploration-token reduction with_map vs without_map: **task 02 (locate-a-utility) +78.1%** (within 2pt of the 80% goal, 19% variance — the cleanest arm); **task 01 (multi-site edit) −186.7%** (bimodal, 84% variance); **aggregate +53.9%** (clears ≥25%, ~26pt below 80%). The density wins (footer + resolved imports) flipped task 02 from −24% on the earlier noisy run to +78%. Pattern: the map strongly helps "find the existing thing" tasks, hurts "find all sites" (multi-site) tasks — which need reverse-reference info. Variance still needs N≥5 to trust per-task numbers. Prior checkpoint below.

**Earlier checkpoint (2026-06-16 — first fair with_map, N=2 PRELIMINARY, OLD whole-session metric):** Comprehension gate **PASS** — with_map 6/6 accuracy at 29,668 tok / 1 turn vs without_map 6/6 at 54,300 tok / 2 turns (equal accuracy, ~45% fewer tokens, half the turns). Edit-task: task 01 −31.9%, task 02 +24.4%, aggregate **−15.5% vs baseline.json (FAIL ≥25%)** but **−26.0% same-run vs a fresh without_map arm (PASS)**. The disagreement is a protocol/metric + stale-baseline issue (baseline's without_map 369k did not reproduce → 793k today; turn-cap runs dominate the `cache_read` token proxy), not a map-quality failure — see benchmark/history.md. **Next (some are human gates):** re-record the without_map baseline, run N≥3 odd-count, refine the exploration-token metric, replace the non-discriminating task 02. Spend $6.11.

**Preliminary with-map probe (2026-06-12, NOT the official comparison — naive unbudgeted ~81k-token map injected):** turns dropped 41–43% (task 01: 22 → 13 median; task 02: 14 → 8 median; 6/6 passes), but total tokens and cost ROSE (~2.2–3.1× tokens, ~2.7–3.4× cost) because 92% of the with-map token bill is cache-rereading the oversized map each turn. Conclusion: the map's navigational value is real and already beats the ≥25% target on turns; the token win requires the M1 budget stage (a ~2k map would extrapolate to roughly ~35–40% token reduction at the observed turn counts). This is the strongest evidence yet that budgeting is the load-bearing feature.

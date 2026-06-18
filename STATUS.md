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

## pip / pipx distribution (2026-06-17)

`pip install atlas-map` / `pipx install atlas-map` — the Python-native audience (aider/Claude-Code/Cursor
users) can now install atlas without a Rust toolchain or `curl | sh`, including on Windows and in
locked-down/corp environments the shell installer can't reach. Decided after a 6-agent research workflow
(pip = strong audience fit, conda = skip: narrow audience + heavy feedstock maintenance).
- **How:** maturin `bindings = "bin"` wraps the *same* compiled binary into a platform wheel — no Python at
  runtime, the `atlas` command just lands on PATH (the ruff/uv/ast-grep pattern). `pyproject.toml` +
  `.github/workflows/pypi.yml` (5 platform wheels mirroring the cargo-dist triples + an sdist source
  fallback), publishing via PyPI **Trusted Publishing (OIDC, no token)** on each release tag. Separate from
  the cargo-dist-generated `release.yml`; strictly additive to curl|sh / archives / `cargo install`.
- **Verified locally end-to-end:** `maturin build` → `atlas_map-0.1.0a0-py3-none-macosx_11_0_arm64.whl`;
  fresh-venv `pip install` drops `atlas` on PATH; `atlas --version` + a real map both run; sdist includes
  `Cargo.lock` + `queries/` so source-build fallback works.
- **Dist name** is `atlas-map` (bare `atlas` taken on PyPI; verified `atlas-map`/`atlasmap`/`atlas-cli`
  free); command stays `atlas`. Alpha ⇒ version normalizes to `0.1.0a0`, so users need `--pre` until a
  stable `0.1.0`.
- **⚠ Maintainer action required before the first publish works:** on PyPI, add a *pending Trusted
  Publisher* — project `atlas-map`, owner `fkenmar`, repo `atlas`, workflow `pypi.yml`, environment `pypi`
  (and create a GitHub environment named `pypi`). Until then the README's `pip install` line is staged, not
  live; the wheels publish automatically on the next release tag once the publisher is configured.

## Symbol index — comprehension token win (2026-06-17, ADR 0004)

**−65.2% tokens at 20/20 accuracy** on the comprehension benchmark (run-084740), at the
shipped default 2,048 budget — more than doubling the prior −30.1%, and meeting the "~70%
fewer tokens at identical accuracy" goal as far as the harness allows (the agent runtime's
~28–30k/turn fixed overhead caps map-side reduction near ~65%).

- **What landed:** when files overflow the budget (rung 3), atlas now reserves 40% of the
  budget for a compact `path: TypeA, TypeB` **symbol index** of the collapsed/degraded files'
  navigable declarations — type-first, ranked, capped per file (8 types / 2 funcs), greedily
  fit by binary search. Purely additive; off entirely when a repo fits in full. src/budget.rs
  + render/markdown.rs + render/json.rs (additive `symbol_index` array, schema v1 unchanged —
  additive keys don't bump per the refined contract). 62 lib tests, clippy clean.
- **Why it works:** a free A/B smoke proved answer-in-map ⇒ 1-turn answer (~30k tok) vs grep ⇒
  3 turns (~90k). The old footer erased the tail to `src/* (65)`; only 8/20 answers were in the
  default map. The index took default coverage **8 → 12/20** (3,072 → 17/20), flipping the
  *median* question to a one-turn answer. Median turns 3 → 1.
- **Result of the validation the maintainer authorized (~$6–7):** accuracy held 20/20 in BOTH
  arms — the terser map never made the agent wronger (the hard gate). This is now the README
  headline candidate, superseding the −30.1% figure.
- **Known gaps / next lever:** `CaptureManager` (low PageRank, hook-invoked) and `CallInfo`
  (rank 43, past 2,048 index depth) still miss at the default budget (both clear at 3,072) —
  better per-symbol ranking is the follow-up. See benchmark/history.md + ADR 0004.

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

## `atlas diff` — structural delta between two trees (2026-06-18, closes #12)

`atlas diff <old> <new>` emits a deterministic structural delta: added/removed
files, and per common file the added/removed symbols, changed signatures, and
added/removed import edges. Runs on raw parse output (no rank/budget, so every
change is reported), not the budgeted map. Design recorded in **ADR 0005**:
- **Input = two paths** (not git revisions) — the general primitive; compare
  revisions by materializing them (`git worktree add`). Avoids the `git2` dep gate;
  a revision shorthand is a deferred follow-up.
- **CLI = git-style router**, not a clap subcommand — `run()` intercepts `diff`
  before `Cli::parse()`, so the flat map `Cli` and all its tests stay untouched.
- New `src/diff.rs` (engine) + `src/render/diff.rs` (Markdown). Symbol identity is
  `(kind, name)`; overload buckets and kind-changes (free fn → method) fall back to
  per-signature add/remove.
No new dependency, no benchmark impact (separate path, never touches rank/budget).
Hardened by an adversarial review workflow (7 confirmed findings fixed): the diff
renderer now carries the FR-12 skipped/unwired footer per side (a one-sided
unparseable file no longer reads as a phantom add/remove); `--no-private` suppresses
all-private added/removed files (consistent with the changed-file path); path errors
share the map command's actionable messaging via a `canonicalize_root` helper.
Gate green: 95 lib tests + a `tests/diff_cli.rs` binary integration suite (4),
clippy clean. M3 board NOT-YET cell now reads just "Tier 2 grammars" — both XML
(#11) and diff (#12) have shipped.

## XML renderer — first M3 deliverable (2026-06-18, closes #11)

`atlas --format xml` now joins md/json — a third output format for
prompt-injection-safe wrapping in Claude prompts (PRD §6/FR-5). Well-formed XML
where signatures/paths are escaped per the XML 1.0 spec so embedded source can't
break out; describes the *same* logical schema as the JSON renderer (shares
`SCHEMA_VERSION` + the kind/visibility/detail vocabulary, made `pub(crate)`), so
the two structured formats don't drift. No new dependency (hand-rolled escaper,
matching `json_str`); not a ranking/budget change, so no `/bench`. TDD'd (genuine
RED) and hardened by an adversarial review workflow (8 confirmed findings fixed):
- **Blocker fixed** — the escaper now drops the full set of code points illegal in
  the XML 1.0 `Char` production (C0 controls, U+FFFE/U+FFFF, the BMP gap), not
  just `<0x20`. A real Python source file with U+FFFE in a default-arg string
  previously yielded malformed XML; now it parses (verified end-to-end with
  Python's expat).
- Attribute tab/newline/CR → numeric refs (`&#9;` …) so they survive XML
  attribute-value normalization (JSON-parity, no silent corruption).
- Collapsed-dir attribute renamed `path`→`dir` to mirror the JSON key.
Gate green: 79 lib tests (+9 xml: schema/escaping/determinism/adversarial
round-trip), clippy clean, real-parser validation on the 5,285-LOC self-map.
M3 board cell trimmed to "Tier 2 grammars, atlas diff."

## Board

| NOW | NEXT | NOT-YET |
|---|---|---|
| ~~TS/JS grammar (tree-sitter-typescript)~~ ✅ done 2026-06-16 | ~~Incremental cache + warm path~~ ✅ done 2026-06-16 (FR-6) | MCP server (M2) |
| ~~Rust grammar (tree-sitter-rust)~~ ✅ done 2026-06-16 | rayon parallel parse (M1) | --watch daemon (M2) |
| ~~Import linking → index-based graph (ADR 0002)~~ ✅ done 2026-06-16 | clap CLI: --budget/--format/--focus (M1; opens the CI self-map gate) | --focus personalization (M2) |
| ~~PageRank over the graph~~ ✅ done 2026-06-16 | ~~.gitignore/.repomapignore in discover (FR-7)~~ ✅ done 2026-06-16 | cargo-dist packaging (M2) |
| ~~tiktoken budgeting + degradation ladder~~ ✅ code done 2026-06-16 (bench owed at integration) | Refine exploration-token metric toward PRD definition (tokens before first correct edit) | Tier 2 grammars (M3) |
| ~~clap CLI + full pipeline wired (discover→…→render)~~ ✅ done 2026-06-16 | | |
| ~~Exclude inline #[cfg(test)] code from extraction~~ ✅ done 2026-06-16 (self-map: 2036 tok degraded → 1749 tok at FULL detail, 16/16 files) | | |
| Checkpoint benchmark (pytest with-map vs baseline) ← next | | |
| | Re-record baseline when with-map arm goes live (variance notes now auto-recorded by run.sh) | More benchmark tasks (target: 10) + decide long-term target repo (pytest 8.2.0 is the M0 stand-in) |
| | Competitive benchmark arms (post-M1): same suite vs Aider repo-map / ctags / file-tree control at equal budget — repomap must beat them all, Aider especially (protocol: benchmark/README.md §Competitive arms) | |

**Worthiness gate — PASSED, then doubled.** Superseded 2026-06-17 by the **symbol index (ADR 0004): −65.2% tokens at 20/20 accuracy** (run-084740) — see the dedicated section above; this is now atlas's defensible value claim. Original gate (run-004810, 20 verified questions): with_map vs without_map both **20/20 accuracy (100%, zero regressions)**; tokens **85,817 → 59,916 = −30.1%**; turns 3→2 — the trustworthy, low-variance signal that justified usability/ship work. The edit-task token deltas below stay INCONCLUSIVE (60–140% variance), but the worthiness question is settled: atlas earns its place in an agent's context.

**Last benchmark result (2026-06-17 — DECISIVE N=5, run-232455, first run with the reverse-ref/field lever, spend $14.12):** the M1 "measurable benchmark win" exit criterion is **NOT met on edit-task tokens.** with_map vs without_map exploration tokens: **task 02 (find-the-thing) +22.9%**, **task 01 (multi-site edit) −66.8%**, **aggregate +8.9%** (sum of medians, below the ≥25% bar). The earlier N=3 +53.9%/+78% **did not hold up** — it was largely an artifact of one run's without_map blowup. Variance is still **64–139%** even at N=5, so medians are soft; the only robust signals are the task split (helps find-the-thing, hurts multi-site) and **turns −25%** in both arms. The reverse-ref/field lever helped task 01 (−186.7% → −66.8% vs run-101017) but did not flip it positive — keep it. The **80% goal is out of reach with this approach**; the prerequisite for any trustworthy token verdict is killing the variance (task redesign / trimmed means / larger N), not more map content. README's headline claim was corrected to the supported numbers (turns −25%, comprehension −45% at equal accuracy). #1 stays open; v0.1.0 (#14) stays gated — the alpha label was right. See benchmark/history.md + results/run-20260616-232455.local.json.

**Prior benchmark result (2026-06-16 — refined metric + density-improved map, run-101017, N=3 clean):** 0/12 capped, 12/12 pass (the cap fix worked). Same-run exploration-token reduction with_map vs without_map: **task 02 (locate-a-utility) +78.1%** (within 2pt of the 80% goal, 19% variance — the cleanest arm); **task 01 (multi-site edit) −186.7%** (bimodal, 84% variance); **aggregate +53.9%** (clears ≥25%, ~26pt below 80%). The density wins (footer + resolved imports) flipped task 02 from −24% on the earlier noisy run to +78%. Pattern: the map strongly helps "find the existing thing" tasks, hurts "find all sites" (multi-site) tasks — which need reverse-reference info. Variance still needs N≥5 to trust per-task numbers. Prior checkpoint below.

**Earlier checkpoint (2026-06-16 — first fair with_map, N=2 PRELIMINARY, OLD whole-session metric):** Comprehension gate **PASS** — with_map 6/6 accuracy at 29,668 tok / 1 turn vs without_map 6/6 at 54,300 tok / 2 turns (equal accuracy, ~45% fewer tokens, half the turns). Edit-task: task 01 −31.9%, task 02 +24.4%, aggregate **−15.5% vs baseline.json (FAIL ≥25%)** but **−26.0% same-run vs a fresh without_map arm (PASS)**. The disagreement is a protocol/metric + stale-baseline issue (baseline's without_map 369k did not reproduce → 793k today; turn-cap runs dominate the `cache_read` token proxy), not a map-quality failure — see benchmark/history.md. **Next (some are human gates):** re-record the without_map baseline, run N≥3 odd-count, refine the exploration-token metric, replace the non-discriminating task 02. Spend $6.11.

**Preliminary with-map probe (2026-06-12, NOT the official comparison — naive unbudgeted ~81k-token map injected):** turns dropped 41–43% (task 01: 22 → 13 median; task 02: 14 → 8 median; 6/6 passes), but total tokens and cost ROSE (~2.2–3.1× tokens, ~2.7–3.4× cost) because 92% of the with-map token bill is cache-rereading the oversized map each turn. Conclusion: the map's navigational value is real and already beats the ≥25% target on turns; the token win requires the M1 budget stage (a ~2k map would extrapolate to roughly ~35–40% token reduction at the observed turn counts). This is the strongest evidence yet that budgeting is the load-bearing feature.

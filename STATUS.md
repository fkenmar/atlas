# STATUS

## Current milestone: M0 — Foundation ✅ COMPLETE (2026-06-12)

**Exit criteria (PRD §9):**
- [x] Cargo workspace with tree-sitter + Python grammar wired *(tree-sitter 0.26.9 + tree-sitter-python 0.25.0; lib+bin layout; queries embedded; snapshot tests)*
- [x] Naive full map runs end-to-end on one real repo *(deepfake_detector: 7 py files / 1,658 LOC in 0.22 s, vendored junk excluded; pytest 8.2.0: 264 files / 92,156 LOC in 0.56 s cold — NFR-1 reference point)*
- [x] Agent benchmark harness built; baseline (no-map) numbers recorded in benchmark/baseline.json *(real headless `claude -p` runner; baseline recorded 2026-06-12, claude-sonnet-4-6, 3 runs/task, 6/6 success-criteria passes)*

Next milestone: **M1 — Core (v0.1 alpha)**: TS/JS + Rust grammars; import linking; PageRank; tiktoken budgeting; md + json renderers; cache; gitignore (2 wks). Burn-down runs through the self-improvement loop — `/improve` or `/loop /improve` (docs/SELF_IMPROVEMENT.md); measured changes append to benchmark/history.md.

## M1 core built — 2026-06-16 (autonomous session)

The full pipeline now runs end-to-end (`repomap [PATH] --budget --focus --lang --no-private --format md|json`). **All M1 functional requirements done:** FR-1 (TS+Rust grammars), FR-3/FR-11 (tiktoken `cl100k_base` budget + degradation ladder), FR-4 (personalized PageRank), FR-5 (md + json), FR-6 (bincode content-hash cache), FR-7 (`.gitignore`/`.repomapignore`), FR-12. **NFR-1 cold:** 0.25 s on pytest 92 k LOC (8× under the 2 s target; warm-path wall-clock verification still pending). **Remaining for M1 exit:** the *benchmark-shows-a-measurable-win* criterion (first fair with-map vs without-map checkpoint **in flight**), warm-path timing, optional rayon. Dogfood self-map of repomap's own source: 3.7 k LOC → ~1.4 k tokens at full detail. Quality fixes baked in: test-code excluded from extraction, ranking de-biased against symbol count, per-file one-line rung, language-aware visibility. Full history in CHANGELOG.md.

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

**Last benchmark result:** baseline recorded 2026-06-12 (without-map arm, claude-sonnet-4-6, max 30 turns, 3 runs/task, medians): 01-add-field-three-sites = 902,555 tokens / 22 turns; 02-reuse-existing-utility = 369,461 tokens / 14 turns; all 6 runs passed their success criteria. High run variance observed (task 01 tokens 663k–1,216k; task 02 122k–393k — well over the 15% threshold), recorded here because this baseline predates run.sh's automatic variance notes; with-map comparisons start in M1 when budgeting makes the arm fair.

**Preliminary with-map probe (2026-06-12, NOT the official comparison — naive unbudgeted ~81k-token map injected):** turns dropped 41–43% (task 01: 22 → 13 median; task 02: 14 → 8 median; 6/6 passes), but total tokens and cost ROSE (~2.2–3.1× tokens, ~2.7–3.4× cost) because 92% of the with-map token bill is cache-rereading the oversized map each turn. Conclusion: the map's navigational value is real and already beats the ≥25% target on turns; the token win requires the M1 budget stage (a ~2k map would extrapolate to roughly ~35–40% token reduction at the observed turn counts). This is the strongest evidence yet that budgeting is the load-bearing feature.

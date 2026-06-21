# atlas roadmap

Organized by **user outcome**, not by component — what each pile of work is
*for*. Issues are linked under the outcome they serve; this is a guide, not an
exhaustive list of the [issue tracker](https://github.com/fkenmar/atlas/issues).

atlas is **alpha**. The line below marks what's shipped vs. planned; the
alpha → beta → stable bar is defined in
[`docs/release-readiness.md`](docs/release-readiness.md), and what's a stable
contract vs. what may still change is in [`docs/stability.md`](docs/stability.md).

Legend: ✅ shipped · 🚧 in progress · ⬜ planned.

## Trust the output

The benchmark claim must be believable and the formats must be dependable.

- ✅ Reproducible comprehension benchmark (~65% fewer tokens at equal accuracy) — [`benchmark/README.md`](benchmark/README.md)
- ✅ Machine-readable JSON Schemas for map and diff — #57, #58
- ✅ Pre-1.0 stability & deprecation policy — #97
- ⬜ Public benchmark case study for the README claim — #40
- ⬜ Per-language extraction-accuracy report — #111, backed by a real-world fixture corpus — #110
- ⬜ Competitive benchmark arms (Aider repo-map / ctags / file-tree at equal budget) — #13
- ⬜ Large-repo performance matrix — #73

## Install anywhere

Make atlas trivial to get, on any platform and in locked-down environments.

- ✅ pip/pipx (`atlas-map`), prebuilt binaries (macOS/Linux/Windows), `cargo install` from source
- ✅ Offline / proxy / air-gapped install guidance — [`docs/install-offline.md`](docs/install-offline.md)
- 🚧 PyPI Trusted Publishing — #29
- ⬜ crates.io publish — #37 · Homebrew — #38 · Docker image — #67 · npm/npx wrapper — #51
- ⬜ Packaged shell completions — #61 · man page — #62
- ⬜ Supply chain: checksums & signatures — #63 · SBOM — #66 · SLSA provenance — #65
- ⬜ Install smoke tests — #43 · binary-size/startup tracking in CI — #114

## Drop into any agent

The map is only useful if it reaches the agent with minimal friction.

- ✅ Agent integration cookbook — #39 · `CLAUDE.md`/`AGENTS.md` snippets — #87 · editor task snippets — #88
- ✅ `atlas-orient` agent skill — [`skills/`](skills/)
- ✅ MCP stdio server (`get_map`, `get_symbol`, …) — [`docs/CLAUDE_CODE_MCP.md`](docs/CLAUDE_CODE_MCP.md)
- ✅ Budget presets (`--preset`) — #104
- 🚧 `atlas doctor` install/repo diagnostics — #47
- ⬜ MCP conformance tests against real clients — #101
- ⬜ Selectable tokenizer profiles (Claude/OpenAI/generic) — #103
- ⬜ Reusable GitHub Action — #68 · `atlas init` scaffolding — #52 · config file (`.atlas.toml`) — #59

## Review & edit workflows

Help agents (and humans) with change-centric tasks, not just first-look orientation.

- ✅ Structural `atlas diff` (trees or git revisions) + `--exit-code` CI gate
- ✅ `atlas --check` committed-map freshness gate
- ⬜ Git-aware focus from changed files — #105 · changed-map mode (files + dependents) — #108
- ⬜ PR summary from diff — #70 · PR comment mode — #86
- ⬜ Warning mode for unresolved import edges — #93 · compression-ratio report — #74

## Cover your languages

- ✅ Python, TypeScript/JavaScript, Rust, Go, Java, C/C++ — see the [language matrix](docs/languages.md)
- ⬜ Tier 2 signature-accuracy audit — #22
- ⬜ Framework-flavored examples (no semantic analysis) — #112
- ⬜ Language-request template + prioritization rubric — #109

## Live context

- ⬜ `--watch` daemon for incremental re-mapping — #8, with a concrete behavior contract — #28

## Core map quality (benchmark-gated)

Ranking/budgeting changes ship only with a measured win — see [`docs/SELF_IMPROVEMENT.md`](docs/SELF_IMPROVEMENT.md).

- ✅ Collapsed-tail symbol index (ADR 0004) · stable anchors / progressive disclosure (ADR 0009)
- ⬜ Better default-budget ranking for known comprehension misses — #30
- ⬜ Evaluate adaptive default budget — #79
- ⬜ Grow the benchmark suite to 10 balanced tasks — #6

---

<sub>This roadmap is updated as major issues close. Outcomes are stable; the
issues under them will churn. For day-to-day status see [`STATUS.md`](STATUS.md);
for shipped changes see [`CHANGELOG.md`](CHANGELOG.md).</sub>

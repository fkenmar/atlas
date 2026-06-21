# atlas documentation

Everything under `docs/` (plus the key top-level docs), grouped by what you're
trying to do. New to atlas? Start with the [README](../README.md) and the
[60-second quickstart](../README.md#60-second-quickstart).

## Getting started

- [README](../README.md) — install, quickstart, usage, why it works.
- [FAQ](FAQ.md) — common product/workflow questions.
- [Windows guide](windows.md) — PATH, PowerShell, completions.
- [Offline / proxy / corporate install](install-offline.md) — restricted networks.
- [Migration from repomap](MIGRATION.md) — old names → atlas equivalents.

## Using & tuning the map

- [Language support matrix](languages.md) — what's extracted per language, with caveats.
- [`.atlasignore` & monorepo tuning](monorepos.md) — ignore syntax, `--focus`/`--lang`, big repos.
- [Exit codes & error taxonomy](exit-codes.md) — the `0/1/2` contract for scripting.
- [Stability & deprecation policy](stability.md) — what's a stable contract pre-1.0 vs what may change.
- JSON Schemas: [map output](../schemas/atlas-map.schema.json) · [diff output](../schemas/atlas-diff.schema.json) — versioned contracts for `--format json`.
- [Example gallery](../examples/gallery/) — real maps for Python / TypeScript / mixed repos.

## Using atlas with an AI agent

- [Agent integration cookbook](agent-cookbook.md) — per-agent stdout/file recipes + flag guidance.
- [`atlas-orient` agent skill](../skills/README.md) — drop-in skill that tells an agent to map first.
- [Generating CLAUDE.md / AGENTS.md sections](agent-files.md) — keep a delimited map block fresh.
- [Editor task snippets](editor-snippets.md) — VS Code / JetBrains / shell.
- [Claude Code MCP setup](CLAUDE_CODE_MCP.md) — the `atlas serve --mcp` route.
- [Prompt-injection threat model](prompt-injection.md) — Markdown vs XML, safe wrappers.

## Keeping a committed map fresh

- [Pre-commit hook](pre-commit.md) — regenerate `atlas-map.md` on commit (or `atlas --check` to gate it).
- [CI recipes](ci-recipes.md) — artifact upload + freshness check.
- [CI gate for API changes](ci-diff-gate.md) — `atlas diff --exit-code`.

## Why atlas / positioning

- [Comparison guide](comparison.md) — vs Aider repo-map, ctags, SCIP, concat packers.
- [PRD](PRD.md) — scope, requirements, non-goals, risks.

## Architecture decisions (ADRs)

- [0002 — index-based graph](adr/0002-index-based-graph.md)
- [0003 — budget packing ladder](adr/0003-budget-packing-ladder.md)
- [0004 — symbol index for the collapsed tail](adr/0004-symbol-index-for-collapsed-tail.md)
- [0005 — structural diff between trees](adr/0005-structural-diff-between-trees.md)
- [0008 — MCP server over stdio](adr/0008-mcp-server-over-stdio.md)
- [0009 — stable symbol anchors / progressive disclosure](adr/0009-stable-symbol-anchors-progressive-disclosure.md)
- (Full list in [`adr/`](adr/).)

## Privacy & security

- [Privacy model](PRIVACY.md) — local/offline operation, cache, map sensitivity.
- [Security policy](../SECURITY.md) — supported versions, disclosure path.

## Project & process

- [STATUS](../STATUS.md) — current milestone and board.
- [CHANGELOG](../CHANGELOG.md) — user-facing changes by release.
- [CONTRIBUTING](../CONTRIBUTING.md) — build/test, conventions, good first issues.
- [Benchmark protocol](../benchmark/README.md) & [history](../benchmark/history.md) — what claims are supported.
- [Self-improvement loop](SELF_IMPROVEMENT.md) — the measured keep-or-revert loop.

## Releasing & launch (maintainer)

- [Releasing & release notes](RELEASING.md) — cutting a tag, polished notes.
- [Release-readiness gates](release-readiness.md) — alpha/beta/stable bars.
- [Launch checklist](LAUNCH_CHECKLIST.md) — metadata, trust files, claim checks.
- [Submissions playbook](submissions.md) & [ready-to-open packets](submission-drafts.md) — directories & awesome-lists.
- [Social launch copy](social-launch.md) — X / LinkedIn drafts.
- [Post-launch outreach](post-launch-outreach.md) · [Adoption metrics](ADOPTION_METRICS.md) · [Release-notes draft](release-notes-draft.md)

---

<sub>Agents: machine-oriented entry points are [`llms.txt`](../llms.txt) (short)
and [`llms-full.txt`](../llms-full.txt) (expanded). Keep this index in sync when
adding or removing a doc.</sub>

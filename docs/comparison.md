# Comparison guide

Use this page for honest positioning. It is not a substitute for a fresh
competitor benchmark.

## Short version

atlas is for giving an AI coding agent a compact structural index before it
starts opening files: signatures, types, imports, reverse dependencies, ranking,
and a token budget. It is not a full source packer, semantic search engine, LSP,
or editor integration.

## Adjacent tools

| Tool or pattern | Best at | Difference from atlas |
|---|---|---|
| Full repo-to-prompt packers | Sending many files into one prompt or artifact | atlas omits function bodies and spends the budget on navigable structure. |
| Repomix-style packaging | Collecting repository contents for LLM ingestion | atlas is a graph-ranked structural map, not a content bundle. |
| Gitingest-style URL conversion | Quick web-to-text repository snapshots | atlas is local, repeatable, budgeted, and meant for iterative coding-agent use. |
| Aider repo-map | Proven embedded repo-map workflow inside Aider | atlas is a standalone Rust CLI/library/MCP server usable with any agent. |
| ctags / universal-ctags | Broad symbol extraction | atlas adds import graph ranking, token budgeting, multiple renderers, and agent-oriented output. |
| tree-sitter CLI | Parser primitives and AST inspection | atlas turns parser output into a stable product workflow. |
| Sourcegraph / SCIP / LSPs | Deep code intelligence, search, and navigation | atlas deliberately avoids server infrastructure and semantic analysis. |
| `find` / `tree` / `cat` | Raw file discovery and source inspection | atlas helps decide what to inspect first; source remains the truth before editing. |

## Claims that are safe today

- atlas runs locally, offline, and has no product telemetry.
- atlas output contains structural information only: signatures, types, imports,
  reverse dependencies, and rank/budget metadata.
- The comprehension benchmark is the strongest current public evidence:
  20/20 accuracy in both arms, median tokens 85,670 -> 29,781 (-65.2%), and
  median turns 3 -> 1 at the default 2,048-token budget.
- `atlas diff` reports structural deltas between directories or git revisions,
  with Markdown, JSON, and XML output.
- `atlas serve --mcp` exposes read-only `get_map` and `get_symbol` tools over
  stdio, confined by `--root`.

## Claims to avoid

- Do not claim atlas replaces reading code. It tells an agent where to look.
- Do not claim a broad edit-task token reduction. Existing edit-task runs are
  high variance and not headline-worthy.
- Do not claim semantic understanding, type checking, rename safety, or
  go-to-definition accuracy. Those are LSP/compiler jobs.
- Do not imply competitor superiority without a fresh equal-budget benchmark.

## Practical positioning copy

Short:

> atlas is a fast Rust repo map for AI coding agents: it extracts signatures,
> types, imports, and reverse dependencies, then packs the most useful structure
> into a token budget so the agent stops exploring blind.

Benchmark-aware:

> On atlas' constrained comprehension benchmark, the default map kept accuracy
> at 20/20 while cutting median tokens from 85,670 to 29,781. Edit-task token
> deltas are still too noisy for a headline claim.

## Differentiation checklist

- Fixed token budget instead of "as much repo text as fits."
- Parser-grade signatures rather than regex snippets.
- Import/reverse-dependency graph ranking instead of file-order output.
- Markdown for humans/agents, JSON/XML for tooling.
- Structural diff for review and CI use.
- MCP stdio server for clients that can call tools directly.

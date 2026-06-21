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

## Decision matrix

| Dimension | atlas | Aider repo-map | ctags | tree-sitter CLI | Sourcegraph / SCIP | Concat / Repomix-style packers |
|---|---|---|---|---|---|---|
| Standalone CLI | Yes | No, embedded in Aider | Yes | Yes | Usually server/toolchain | Yes |
| Structural graph | Imports and reverse dependencies | Yes, inside Aider | No | No | Yes | No |
| Ranking | PageRank-style file importance plus focus boosts | Yes | No | No | Search/index ranking, not a small prompt map | Usually file order/filter order |
| Token budgeting | Fixed budget with graceful degradation | Yes, Aider-specific | No | No | Not the core workflow | Usually size limits or manual filters |
| Output formats | Markdown, JSON, XML | Aider internal prompt format | Tags text | AST/query output | APIs/UI indexes | Text/XML/Markdown bundles vary by tool |
| Install friction | Single binary, pipx/PyPI wrapper, release archives | Install Aider/Python app | Native packages common | Native binary | Service/index setup | Usually npm/Python/CLI |
| Agent integration | Any agent via file/pipe; MCP stdio server | Aider only | Manual prompt glue | Manual prompt glue | API/editor integration | Any agent via attached text |

## Claims that are safe today

- atlas runs locally, offline, and has no product telemetry.
- atlas output contains structural information only: signatures, types, imports,
  reverse dependencies, and rank/budget metadata.
- The comprehension benchmark is the strongest current public evidence:
  20/20 accuracy in both arms, median tokens 85,670 -> 29,781 (-65.2%), and
  median turns 3 -> 1 at the default 2,048-token budget.
- **Head-to-head vs Aider repo-map (equal budget, fresh — 2026-06-21).** Same
  20-question comprehension benchmark, matched map size (~2k tokens: atlas 2,040,
  Aider 1,942), claude-sonnet-4-6, pytest 8.2.0. All arms 20/20 accuracy. Median
  tokens: without-map 86,525; **Aider repo-map 59,452 (-31.3%); atlas 29,937
  (-65.4%)** -- atlas answers at ~half Aider's token cost. atlas's map held 12/20
  answer keys vs Aider's 2/20; Aider also overshot its 2,048-token budget ~2x
  (4,126 actual) and still reached only 5/20. Aider spends budget on test/doc
  files and function-body snippets; atlas ranks and surfaces the symbol index.
  (N=1/arm; comprehension is low-variance. See benchmark/history.md.)
- `atlas diff` reports structural deltas between directories or git revisions,
  with Markdown, JSON, and XML output.
- `atlas serve --mcp` exposes read-only map, symbol lookup, anchor-index, and
  anchor-expansion tools over stdio, confined by `--root`.

## Claims to avoid

- Do not claim atlas replaces reading code. It tells an agent where to look.
- Do not claim a broad edit-task token reduction. Existing edit-task runs are
  high variance and not headline-worthy.
- Do not claim semantic understanding, type checking, rename safety, or
  go-to-definition accuracy. Those are LSP/compiler jobs.
- Do not imply competitor superiority beyond what a fresh equal-budget benchmark
  shows. The Aider head-to-head above is backed (matched ~2k tokens, both 20/20
  accuracy); ctags, tree-sitter CLI, and concat packers are not yet measured.

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

# Post-launch outreach plan

The goal is to explain atlas clearly to people who already care about coding
agents, not to ask for stars or carpet-bomb communities. Post once per channel,
follow the rules, disclose maintainer affiliation, and treat replies as product
research.

Launch URL placeholder: `https://github.com/fkenmar/atlas`

## Outreach list

| Channel | Submission rules | Launch URL | Timing | Owner | Status |
|---|---|---|---|---|---|
| Hacker News / Show HN | Use `Show HN: atlas - a fast Rust repo map for AI coding agents`; be present for comments; do not ask for votes. | GitHub repo or release page | Launch day, morning US Eastern | Kenmar | Draft |
| r/rust | Only post if framed as a Rust CLI/release with implementation detail; avoid generic self-promo tone. | GitHub repo | After HN or next day | Kenmar | Draft |
| r/ClaudeAI or client-specific communities | Focus on agent workflow and MCP, not broad marketing. Check each community's self-promo rules first. | README MCP section or docs/CLAUDE_CODE_MCP.md | After install path is verified | Kenmar | Draft |
| MCP/client Discords or forums | Share the MCP setup doc when relevant to tool/server channels; answer setup questions. | docs/CLAUDE_CODE_MCP.md | After tagged release | Kenmar | Draft |
| Awesome lists | Submit only to lists that accept tools for coding agents, Rust CLIs, or MCP servers; follow each list's PR template. | GitHub repo | Week 1 follow-up | Kenmar | Draft |
| Personal/LinkedIn/X post | Tell the build story and benchmark caveat honestly; link to repo and release. | GitHub repo | Launch day or day 2 | Kenmar | Draft |

## Show HN draft

Title:

```text
Show HN: atlas - a fast Rust repo map for AI coding agents
```

Body:

```text
Hi HN, I built atlas, a local CLI that turns a codebase into a compact map for
AI coding agents.

It extracts signatures, types, imports, and reverse dependencies, ranks files by
the repo graph, and packs the result into a token budget. The point is to give
an agent a navigation index before it starts opening files. It does not include
function bodies and it runs fully offline.

Install:

pipx install --pre atlas-map

Example:

atlas . --for-agent -o atlas-map.md
atlas diff HEAD~1 HEAD
atlas serve --mcp --root .

The strongest benchmark result so far is a constrained comprehension benchmark:
20/20 accuracy with and without the map, with median tokens 85,670 -> 29,781 at
the default 2,048-token budget. Edit-task token deltas are still too noisy for a
headline claim, so I am not claiming that.

I would especially love feedback from people using Claude Code, Codex, Cursor,
Aider, or MCP clients: does this fit your workflow, and what would make it less
annoying to keep fresh?
```

## Reddit / community draft

```text
I shipped an alpha of atlas, a Rust CLI that generates a token-budgeted
structural map of a repo for AI coding agents.

It outputs signatures, types, imports, reverse dependencies, and a collapsed
symbol index; no function bodies. It is local/offline, supports Markdown/JSON/XML,
has `atlas diff`, and can run as a read-only MCP stdio server.

Repo: https://github.com/fkenmar/atlas
MCP setup: https://github.com/fkenmar/atlas/blob/main/docs/CLAUDE_CODE_MCP.md

Useful feedback: install friction, unsupported languages, bad/missing symbols,
and whether the map actually saves your agent from opening the wrong files.
```

## Awesome-list PR draft

```text
Add atlas, a local Rust CLI/MCP server that generates token-budgeted structural
repo maps for AI coding agents. It extracts signatures, types, imports, and
reverse dependencies without function bodies, with Markdown/JSON/XML output and
structural diff support.
```

## Objection and follow-up handling

Track recurring comments as product evidence:

- "How is this different from `cat` or a file tree?" -> improve
  [docs/comparison.md](comparison.md) and README examples.
- "Where are line numbers/token counts/filtering?" -> open focused issues with
  concrete examples.
- "It missed a symbol" -> ask for a minimal snippet and file a query extraction
  bug.
- "The map made my agent wrong" -> request the map, prompt, and target snippet;
  treat as a benchmark or safety issue.
- "MCP setup is confusing" -> update [docs/CLAUDE_CODE_MCP.md](CLAUDE_CODE_MCP.md)
  and the example config.

Repeated objections become FAQ/docs issues. Reproducible bugs become bug issues.
Small high-impact fixes become release candidates.

## Seven-day public metrics checklist

No product telemetry. Use public-source metrics only; the maintained collection
guide is [docs/ADOPTION_METRICS.md](ADOPTION_METRICS.md).

Day 0:

- [ ] Record GitHub stars, forks, watchers, open issues, and release downloads.
- [ ] Record PyPI project page availability and install status.
- [ ] Record top launch comments and objections.

Day 1:

- [ ] Reply to substantive comments.
- [ ] Open issues for reproducible bugs and confusing docs.
- [ ] Check failed installs by platform from issue reports only.

Day 3:

- [ ] Summarize repeated objections into README/FAQ changes.
- [ ] Triage small fixes for a patch release.
- [ ] Re-check release downloads and PyPI stats if available.

Day 7:

- [ ] Compare public metrics against Day 0.
- [ ] Write a short launch retro: channels, useful feedback, bugs, next release
      candidates.
- [ ] Decide whether to submit to additional communities or pause outreach.

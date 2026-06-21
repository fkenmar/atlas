# Agent integration cookbook

atlas writes a Markdown (or JSON/XML) map to **stdout** or a **file**. That's the
whole integration surface — anything that can read a file or accept piped input
can use it. These recipes work *today* with the standalone CLI, no MCP required.
(For the Claude Code MCP server, see [`CLAUDE_CODE_MCP.md`](CLAUDE_CODE_MCP.md).)

## The two core patterns

**1. Save a file, attach it.** Best for chat-style and editor agents:

```sh
atlas . -o atlas-map.md
```

Then attach / `@`-mention `atlas-map.md` in your prompt. Regenerate when the
structure changes (re-runs are warm-cached, ~80 ms).

**2. Pipe it inline.** Best for terminal agents that read stdin:

```sh
{ echo "Repo map:"; atlas .; echo; echo "Task: add a --verbose flag"; } | your-agent
```

`--for-agent` prepends a one-line note telling the model to treat the map as a
navigation index, not as source:

```sh
atlas . --for-agent -o atlas-map.md
```

## Choosing flags for a prompt

| Goal                                            | Flag                              |
| ----------------------------------------------- | --------------------------------- |
| Bias the map toward the files you're editing    | `--focus src/auth --focus src/api`|
| One stack of a polyglot repo                     | `--lang ts,tsx` or `--lang py`    |
| More/less detail (default 2,048 tokens)          | `--budget 4096` / `--budget 1024` |
| Public API surface only                          | `--no-private`                    |
| A parser-safe boundary for untrusted repos       | `--format xml` (see [prompt-injection.md](prompt-injection.md)) |
| Machine consumption (your own tooling)           | `--format json`                   |

Rule of thumb: **`--focus` the task, `--budget` the context window.** For a
focused edit, `atlas . --focus <area>` puts the relevant files at the top so they
survive the budget; raise `--budget` only if your model has the room.

## Per-agent notes

> These are generic "attach a Markdown file / pipe stdin" patterns — atlas has no
> agent-specific integration code. Anything below works because the map is just
> text.

- **Claude Code** — `atlas . -o atlas-map.md`, then `@atlas-map.md` in your
  prompt. Or use the MCP server for a live `get_map` tool
  ([`CLAUDE_CODE_MCP.md`](CLAUDE_CODE_MCP.md)).
- **Codex CLI / other terminal agents** — use the inline pipe pattern, or save
  the file and reference its path in the prompt.
- **Cursor / Windsurf** — `atlas . -o atlas-map.md` and `@`-mention the file in
  the chat. Keep it in the repo so it's always available.
- **GitHub Copilot Chat** — save `atlas-map.md` and attach it (or open it as the
  active file) so it's in context.
- **Generic / any chat model** — paste the contents of `atlas-map.md`. Prefer
  `--format xml` and the [safe wrapper](prompt-injection.md) if the repo isn't
  fully trusted.

## Keeping the map fresh

Regenerate when the structure changes — new files, moved modules, changed public
signatures. Day-to-day body edits don't change the map. Automate it:

- **Pre-commit hook** — regenerate `atlas-map.md` before each commit:
  [`docs/pre-commit.md`](pre-commit.md).
- **CI** — upload the map as an artifact or check a committed map is current:
  [`docs/ci-recipes.md`](ci-recipes.md).
- **Agent context files** — point `CLAUDE.md` / `AGENTS.md` at the map:
  [`docs/agent-files.md`](agent-files.md).
- **From your editor** — one-keystroke regeneration:
  [`docs/editor-snippets.md`](editor-snippets.md).

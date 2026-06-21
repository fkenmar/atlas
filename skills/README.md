# atlas agent skill

`atlas-orient` is a drop-in **skill** that tells your AI coding agent to orient with
an [atlas](https://github.com/fkenmar/atlas) structural map before it explores a
repo — the "install a skill into your agent" distribution model, for *navigation*
instead of behavior. It's pure instructions (no code, no hooks); atlas itself stays
a read-only CLI/MCP tool.

## Install

First install atlas so the `atlas` command is on your PATH:

```
pipx install --pre atlas-map      # or any method in the atlas README
```

Then add the skill to your agent:

- **Claude Code** — copy the `atlas-orient/` directory into your project's
  `.claude/skills/` (or `~/.claude/skills/` to enable it for every project). The
  agent discovers it automatically and invokes it when starting work in an
  unfamiliar repo.
- **Codex CLI / other skill-aware agents** — copy `atlas-orient/SKILL.md` into the
  agent's skills directory.
- **Cursor · Windsurf · Cline · Copilot, or any agent without a skills system** —
  paste the body of `atlas-orient/SKILL.md` into your rules file
  (`.cursor/rules`, `.windsurfrules`, `.clinerules`) or your `AGENTS.md` /
  `CLAUDE.md`. The same guidance also ships as a ready-to-paste block in
  [`../examples/AGENTS.md`](../examples/AGENTS.md).

For the MCP route — where the agent pulls maps and expands symbols as tool calls
instead of reading a file — see [`../docs/CLAUDE_CODE_MCP.md`](../docs/CLAUDE_CODE_MCP.md).

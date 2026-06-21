# Editor task snippets

atlas is a CLI, and a full IDE plugin is **out of scope** (CLI + MCP only). But
most editors can run a shell command on a keystroke, which is all you need to
regenerate a map without leaving your editor. These are optional, copy-paste
snippets — not an extension to install.

## VS Code — tasks

Add to `.vscode/tasks.json` (create it if absent). The JSON below is valid
`tasks.json` v2.0.0:

```json
{
  "version": "2.0.0",
  "tasks": [
    {
      "label": "atlas: map repo",
      "type": "shell",
      "command": "atlas",
      "args": [".", "--budget", "2048", "-o", "atlas-map.md"],
      "problemMatcher": [],
      "presentation": { "reveal": "silent" }
    },
    {
      "label": "atlas: map focused on current folder",
      "type": "shell",
      "command": "atlas",
      "args": [".", "--focus", "${relativeFileDirname}", "-o", "atlas-map.md"],
      "problemMatcher": []
    }
  ]
}
```

Run either with **Terminal → Run Task…**, or bind one to a key in
`keybindings.json`:

```json
{
  "key": "ctrl+alt+m",
  "command": "workbench.action.tasks.runTask",
  "args": "atlas: map repo"
}
```

## JetBrains (IntelliJ, PyCharm, GoLand, …) — External Tool

**Settings → Tools → External Tools → +**, then:

- **Program:** `atlas`
- **Arguments:** `. --budget 2048 -o atlas-map.md`
- **Working directory:** `$ProjectFileDir$`

For a focused map of the file you're in, use Arguments
`. --focus $FileDirRelativeToProjectRoot$ -o atlas-map.md`. Invoke it from
**Tools → External Tools**, or assign a shortcut under **Keymap**.

## Generic shell aliases

For any terminal-driven editor (Vim, Emacs, Helix) or just your shell. Add to
`~/.bashrc` / `~/.zshrc`:

```sh
alias amap='atlas . --budget 2048 -o atlas-map.md'
# Focused map: amapf src/auth
amapf() { atlas . --focus "$1" -o atlas-map.md; }
```

Vim/Neovim can call it directly: `:!atlas . -o atlas-map.md`. In Emacs:
`M-! atlas . -o atlas-map.md`.

---

These pair naturally with the [agent cookbook](agent-cookbook.md) (what to do
with the map) and the [quickstart](../README.md#60-second-quickstart). For
hands-off regeneration, prefer a [pre-commit hook](pre-commit.md) or
[CI](ci-recipes.md).

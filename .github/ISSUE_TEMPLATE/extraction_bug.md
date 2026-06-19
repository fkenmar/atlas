---
name: Symbol or import extraction bug
about: Report a wrong or missing symbol, signature, field, or import edge
title: ''
labels: bug
assignees: ''
---

**Language / extension**

Example: Python `.py`, Rust `.rs`, TypeScript `.ts`, Go `.go`, Java `.java`,
C/C++ `.c` / `.h` / `.cpp`.

**atlas version**

```
atlas --version
```

**Command run**

```
atlas . --budget 2048
```

**Minimal source snippet**

Paste the smallest file or snippet that reproduces the wrong/missing extraction.
Remove private implementation details if needed, but keep the declaration/import
syntax intact.

```text

```

**Expected map output**

What symbol, signature, field, or import edge should atlas show?

**Actual map output**

What did atlas show instead?

**Path / ignore checks**

- [ ] The file extension is supported by atlas.
- [ ] The file is under the path passed to atlas.
- [ ] The file is not excluded by `.gitignore`, `.atlasignore`, or a built-in
      vendored/build directory skip.
- [ ] `atlas . --lang <ext>` still reproduces the issue.

**Want to submit a fix?**

Extraction rules live in `queries/<lang>/tags.scm`. Add or update a fixture
under `tests/queries/fixtures/` and a snapshot in `tests/queries/snapshots/`.
See [CONTRIBUTING.md](../../CONTRIBUTING.md) for the local test gate.

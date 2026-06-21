# Language support matrix

atlas extracts symbols with [tree-sitter](https://tree-sitter.github.io/) queries
(`queries/<lang>/tags.scm`) and links files with a best-effort, **syntactic**
import resolver (`src/link.rs`). This page documents what each language yields,
where support is approximate, and where to look in the tests.

All extraction is structural: signatures, types, and import/reference edges —
never function bodies.

## Extensions → language

| Language          | Extensions                                           | Grammar                 |
| ----------------- | ---------------------------------------------------- | ----------------------- |
| Python            | `.py`, `.pyi`                                        | tree-sitter-python      |
| TypeScript / JS   | `.ts`, `.tsx`, `.mts`, `.cts`, `.js`, `.jsx`, `.mjs`, `.cjs` | tree-sitter-typescript  |
| Rust              | `.rs`                                                | tree-sitter-rust        |
| Go                | `.go`                                                | tree-sitter-go          |
| Java              | `.java`                                              | tree-sitter-java        |
| C                 | `.c`, `.h`                                           | tree-sitter-c           |
| C++               | `.cc`, `.cpp`, `.cxx`, `.hpp`, `.hh`                 | tree-sitter-cpp         |

JavaScript is parsed with the **TypeScript** grammar (a superset), so JSX/TSX and
plain JS share one extractor.

## Extracted symbol kinds

A `●` means the language's query emits that kind. Kinds map to `SymbolKind` in
`src/parse.rs`.

| Language | Function | Method | Class | Interface | Enum | Type alias | Constant | Module | Field |
| -------- | :------: | :----: | :---: | :-------: | :--: | :--------: | :------: | :----: | :---: |
| Python   |    ●     |   ●    |   ●   |           |      |            |    ●     |        |   ●   |
| TS / JS  |    ●     |   ●    |   ●   |     ●     |  ●   |     ●      |    ●     |        |   ●   |
| Rust     |    ●     |   ●    |   ●¹  |     ●²    |  ●   |     ●      |    ●     |   ●    |   ●   |
| Go       |    ●     |   ●    |   ●³  |     ●     |      |            |    ●     |        |   ●   |
| Java     |          |   ●    |   ●   |     ●     |  ●   |            |          |        |   ●   |
| C        |    ●     |        |   ●⁴  |           |  ●   |     ●      |          |        |   ●   |
| C++      |    ●     |   ●    |   ●   |           |  ●   |     ●      |          |   ●⁵   |   ●   |

¹ Rust `struct`/`union` map to the `class` kind. ² Rust `trait` maps to
`interface`. ³ Go `struct` maps to `class`. ⁴ C `struct`/`union` map to `class`.
⁵ C++ `namespace` maps to `module`.

Python additionally recognizes `@dataclass`, `@property`, and `@staticmethod`
decorators when shaping a symbol's signature.

## Visibility rules

Visibility drives `--no-private` and the budget ladder's first rung (drop private
symbols). Each language uses its real rule, not a single convention
(`visibility_of` in `src/parse.rs`):

| Language | Public when…                                                | Private when…                                  |
| -------- | ----------------------------------------------------------- | ---------------------------------------------- |
| Python   | name does **not** start with `_`                            | leading `_`                                    |
| Rust     | declaration starts with `pub` (`pub`, `pub(crate)`, …)      | no `pub` keyword                               |
| TS / JS  | no `private`/`protected` member modifier                    | `private` or `protected` modifier before name  |
| Java     | no `private`/`protected` modifier (public + package-private)| `private` or `protected` modifier              |
| Go       | identifier's first letter is uppercase (exported)           | lowercase first letter                         |
| C / C++  | external linkage                                             | `static` free function/file-scope var; C++ class members under a `private:`/`protected:` section |

> **Rust caveat:** trait *method signatures* carry no `pub` keyword (they're as
> visible as the trait), so they read as private here. It's a known
> over-restriction that only bites under a tight budget with `--no-private`.

## Import / linking behavior

Resolution is best-effort and **syntactic** — no build-system or module-tree
resolution. An import that doesn't resolve simply produces no edge; the file is
still mapped. (`FileIndex::resolve` in `src/link.rs`.)

| Language | What's resolved                                                       | Known gaps                                          |
| -------- | --------------------------------------------------------------------- | --------------------------------------------------- |
| Python   | dotted module paths, by longest path then unique basename             | leading-dot relative imports (`from . import x`)    |
| TS / JS  | relative specifiers (`./x`, `../y/z`) against the importer's dir       | bare/package specifiers (`react`, `@scope/pkg`)     |
| Rust     | `use crate::a::b::C` — longest path prefix, then top-segment basename  | no module-tree resolution; aliases dropped          |
| Go       | quoted import paths, longest suffix then final package segment         | external-only module paths                          |
| Java     | resolved like Python dotted paths                                      | same as Python; classpath not consulted             |
| C / C++  | `#include "..."` relative to the includer, then relative to root       | `<system>` includes are intentionally not captured  |

## Caveats worth knowing

- **C vs C++ header ambiguity.** A `.h` is mapped to the **C** grammar. In a
  mixed C/C++ tree a `.h` may really be C++, but the C grammar parses the common
  subset, so most declarations still extract. Use `.hpp`/`.hh` for
  unambiguously-C++ headers.
- **Best-effort linking.** Cross-package, aliased, or build-tool-mapped imports
  may not produce an edge. This lowers a file's linked centrality but never drops
  it from the map.
- **Structural only.** atlas never reports call-graph or type-inference results —
  it's not an LSP. References are syntactic call/import captures used for
  ranking.

## Test & fixture coverage

Every wired language has a fixture and two snapshot tests, so changes to a query
are caught:

| Language | Query file                  | Fixture                            | Tests                                            |
| -------- | --------------------------- | ---------------------------------- | ------------------------------------------------ |
| Python   | `queries/python/tags.scm`   | `tests/queries/fixtures/python.py` | `query_python_fixture_snapshot`, `query_python_tags_contract` |
| TS / JS  | `queries/typescript/tags.scm` | `tests/queries/fixtures/typescript.ts` | `query_typescript_fixture_snapshot`, `query_typescript_tags_contract` |
| Rust     | `queries/rust/tags.scm`     | `tests/queries/fixtures/rust.rs`   | `query_rust_fixture_snapshot`, `query_rust_tags_contract` |
| Go       | `queries/go/tags.scm`       | `tests/queries/fixtures/go.go`     | `query_go_fixture_snapshot`, `query_go_tags_contract` |
| Java     | `queries/java/tags.scm`     | `tests/queries/fixtures/java.java` | `query_java_fixture_snapshot`, `query_java_tags_contract` |
| C        | `queries/c/tags.scm`        | `tests/queries/fixtures/c.c`       | `query_c_fixture_snapshot`, `query_c_tags_contract` |
| C++      | `queries/cpp/tags.scm`      | `tests/queries/fixtures/cpp.cpp`   | `query_cpp_fixture_snapshot`, `query_cpp_tags_contract` |

`query_fixtures_exist_for_every_wired_language` enforces that every language has a
fixture. Run the smallest relevant slice with `cargo test query_` before the full
gate. The capture-tag contract for queries lives in
`.claude/skills/tree-sitter-queries/SKILL.md`.

---

<sub>**Maintainer note:** when a query, visibility rule, or resolver changes —
or a new language is wired — update the relevant row(s) here alongside the
README "What it maps" section so the docs don't drift.</sub>

---
name: tree-sitter-queries
description: Use when working on tree-sitter extraction — any task touching queries/**/*.scm or tags.scm files, adding language support, fixing missing or wrong symbols in the map, writing snapshot tests for queries, or looking up grammar node names.
---

# Writing tree-sitter extraction queries for repomap

## Anatomy of a tags.scm

Each `queries/<lang>/tags.scm` is a tree-sitter query file: S-expression patterns matched against the parse tree, with `@capture` names marking what to extract. Patterns are independent — a node can match several. The files are embedded at compile time via `include_str!` in `src/lang/<lang>.rs`, so an .scm edit requires a recompile to take effect (the query tests do this for you).

Every pattern has a two-level shape:

```scheme
(declaration_node
  name: (identifier_node) @name) @definition.kind
```

The **outer** capture (`@definition.*` / `@reference.*`) marks the whole node — its source span becomes the signature text. The **inner** `@name` marks the identifier. Both are required: a definition without `@name` is dropped by src/parse.rs.

## The capture-name contract (what src/parse.rs expects)

- `@definition.<kind>` where `<kind>` ∈ `function`, `method`, `class`, `interface`, `enum`, `type`, `constant`, `module`. Cross-language normalization: Rust `struct` → `class`, `trait` → `interface`.
- `@reference.call` — call site; becomes a caller → callee graph edge.
- `@reference.import` — import; becomes a file → file graph edge.
- `@name` — the identifier inside the enclosing capture.

Any other capture name is silently ignored — the contract test (`cargo test query_`) rejects unknown names so this fails loudly instead.

## Finding node names

Never guess. In order of convenience:

1. `cargo run --example dump-ast path/to/file.py` — prints the named AST for any source file (wired in M0).
2. The grammar crate's `node-types.json` (vendored source under `~/.cargo/registry/src/`).
3. The tree-sitter playground (https://tree-sitter.github.io/tree-sitter/playground) for interactive experimenting.
4. Aider's published query files — the reference baseline (PRD §10), but our capture contract differs; translate, don't paste.

## Snapshot-test workflow (required for every query change)

1. Add or extend a construct in `tests/queries/fixtures/<lang>.<ext>` that exercises exactly what you changed.
2. Add or update a test named `query_*` in `tests/queries/` (the `query_` prefix is what the post-edit hook runs on .scm edits).
3. `cargo test query_` — green before you report done. The hook re-runs this automatically whenever a `queries/**/*.scm` file is edited.

## Worked example: Python functions, classes, imports

```scheme
; Functions (module-level and nested; `async def` shares this node kind).
(function_definition
  name: (identifier) @name) @definition.function

; Methods: a function_definition directly inside a class body — matched
; separately so parse.rs can qualify the name as Class.method.
(class_definition
  body: (block
    (function_definition
      name: (identifier) @name) @definition.method))

; Decorated definitions: tree-sitter wraps the def in decorated_definition,
; so a bare (function_definition) pattern still matches the inner node, but
; capturing through the wrapper keeps the decorator in the signature span.
(decorated_definition
  definition: (function_definition
    name: (identifier) @name) @definition.function)

; Classes.
(class_definition
  name: (identifier) @name) @definition.class

; Imports: both forms. The captured @name is the module path — link.rs
; resolves it to a file, best-effort.
(import_statement
  name: (dotted_name) @name) @reference.import
(import_from_statement
  module_name: (dotted_name) @name) @reference.import

; Call sites: bare calls and attribute calls (obj.method(...)).
(call
  function: (identifier) @name) @reference.call
(call
  function: (attribute
    attribute: (identifier) @name)) @reference.call
```

The live version of this is `queries/python/tags.scm`; its fixture is `tests/queries/fixtures/python.py`.

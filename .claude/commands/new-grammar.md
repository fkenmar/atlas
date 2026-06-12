---
description: Scaffold support for a new language
argument-hint: <language>
---
Scaffold language support for: $ARGUMENTS

0. **Scope check first**: if $ARGUMENTS is not in the PRD's Tier 1/2 list (TypeScript/JavaScript, Python, Rust; Go, Java, C/C++, OCaml), stop — languages beyond the tier list are a non-goal. Add the request to ideas.md and report that instead.
1. Add the tree-sitter grammar crate to Cargo.toml, pinned — **ask me before adding it** (CLAUDE.md dependency rule).
2. Delegate to the **grammar-engineer** subagent: create `queries/$ARGUMENTS/tags.scm` following the capture contract (tree-sitter-queries skill), covering the language's declaration forms and known edge cases.
3. Create `src/lang/$ARGUMENTS.rs` embedding the query via `include_str!`, and wire it into the registry in src/lang/mod.rs: `Language` enum variant, `from_extension` arm(s), `tags_query` arm.
4. Add a fixture at `tests/queries/fixtures/` exercising every construct the query extracts, plus `query_*` snapshot tests.
5. Done = `cargo test query_` green, `cargo clippy -- -D warnings` clean, and a sample $ARGUMENTS file maps end-to-end. Then update the STATUS.md board.

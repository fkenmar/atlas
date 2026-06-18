//! C language support: grammar handle (tree-sitter-c, M3 Tier 2) plus the
//! embedded extraction query. Edge cases owned here: a `function_definition`
//! nests its name inside a `function_declarator`; struct/enum tags appear both
//! inline and via `typedef`; `#include` is a preprocessor node, not a statement.

/// Extraction query, embedded at compile time (PRD §7.2).
pub const TAGS_QUERY: &str = include_str!("../../queries/c/tags.scm");

#[cfg(test)]
mod tests {
    // Per-construct extraction tests run via tests/query_snapshots.rs.
}

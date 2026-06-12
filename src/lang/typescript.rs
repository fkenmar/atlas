//! TypeScript/JavaScript language support: grammar handles
//! (tree-sitter-typescript: ts + tsx dialects, M1) plus the embedded
//! extraction query. Edge cases owned here: function overloads, ambient
//! (`declare`) declarations, arrow functions bound to `const`, `export`
//! wrappers.

/// Extraction query, embedded at compile time (PRD §7.2).
pub const TAGS_QUERY: &str = include_str!("../../queries/typescript/tags.scm");

#[cfg(test)]
mod tests {
    // Per-construct extraction tests run via tests/query_snapshots.rs.
}

//! C++ language support: grammar handle (tree-sitter-cpp, M3 Tier 2) plus the
//! embedded extraction query. A superset of C: adds `class_specifier`,
//! namespaces, and member functions declared in-class
//! (`field_declaration` with a `function_declarator`) vs. defined out-of-class
//! (`function_definition` with a qualified `::` name).

/// Extraction query, embedded at compile time (PRD §7.2).
pub const TAGS_QUERY: &str = include_str!("../../queries/cpp/tags.scm");

#[cfg(test)]
mod tests {
    // Per-construct extraction tests run via tests/query_snapshots.rs.
}

//! Java language support: grammar handle (tree-sitter-java, M3 Tier 2) plus the
//! embedded extraction query. Edge cases owned here: methods and fields live
//! inside `class_body`/`interface_body`; visibility is by `public`/`protected`/
//! `private` modifier; enums and interfaces are first-class declarations.

/// Extraction query, embedded at compile time (PRD §7.2).
pub const TAGS_QUERY: &str = include_str!("../../queries/java/tags.scm");

#[cfg(test)]
mod tests {
    // Per-construct extraction tests run via tests/query_snapshots.rs.
}

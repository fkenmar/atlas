//! Python language support: grammar handle (tree-sitter-python, M0) plus
//! the embedded extraction query. Edge cases owned here: decorated
//! definitions, `async def`, properties/staticmethods, nested defs.

/// Extraction query, embedded at compile time (PRD §7.2). Capture-name
/// contract documented in the file itself and in the tree-sitter-queries
/// skill.
pub const TAGS_QUERY: &str = include_str!("../../queries/python/tags.scm");

#[cfg(test)]
mod tests {
    // Per-construct extraction tests run via tests/query_snapshots.rs.
}

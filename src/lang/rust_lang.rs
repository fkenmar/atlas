//! Rust language support: grammar handle (tree-sitter-rust, M1) plus the
//! embedded extraction query. Named `rust_lang` to avoid clashing with the
//! `rust` keyword-adjacent module ecosystem conventions. Edge cases owned
//! here: methods inside `impl` blocks, trait method signatures (no body),
//! `macro_rules!` definitions, `pub(crate)` visibility.

/// Extraction query, embedded at compile time (PRD §7.2).
pub const TAGS_QUERY: &str = include_str!("../../queries/rust/tags.scm");

#[cfg(test)]
mod tests {
    // Per-construct extraction tests run via tests/query_snapshots.rs.
}

//! Go language support: grammar handle (tree-sitter-go, M3 Tier 2) plus the
//! embedded extraction query. Edge cases owned here: methods carry a receiver
//! (`func (s *Service) Run()`) and parse as `method_declaration`; `type … struct`
//! and `type … interface` map to class/interface; exported-ness is by leading
//! capital letter, not a keyword.

/// Extraction query, embedded at compile time (PRD §7.2).
pub const TAGS_QUERY: &str = include_str!("../../queries/go/tags.scm");

#[cfg(test)]
mod tests {
    // Per-construct extraction tests run via tests/query_snapshots.rs.
}

//! Markdown renderer — the default output, optimized for LLM readability
//! (example layout: PRD §5.3).
//!
//! M0 ships [`render_naive_map`]: the full unranked, unbudgeted map.
//! The [`super::Renderer`] impl over a budgeted map lands in M1 alongside
//! rank/budget. Output is deterministic: inputs arrive sorted from
//! discover/parse and are emitted in order (NFR-4).

use std::fmt::Write as _;

use crate::parse::{ParseOutcome, SymbolKind};

pub struct MarkdownRenderer;

impl super::Renderer for MarkdownRenderer {
    fn render(&self, _map: &crate::budget::BudgetedMap) -> String {
        todo!(
            "M1: budgeted markdown rendering (header budget figures, rank order, collapse footer)"
        )
    }
}

/// M0 naive full map: every extracted symbol from every parsed file, no
/// ranking, no budgeting. Proves the pipeline end-to-end and feeds the
/// benchmark's with-map arm until budgeting lands.
pub fn render_naive_map(repo_name: &str, outcome: &ParseOutcome) -> String {
    let stats = outcome.stats;
    let mut out = String::new();
    let _ = writeln!(
        out,
        "# repomap: {repo_name} ({} LOC, {} files) | naive full map (M0 — no ranking/budgeting yet)",
        stats.total_lines, stats.parsed_files
    );

    for (file, parsed) in &outcome.files {
        if parsed.symbols.is_empty() && parsed.imports.is_empty() {
            continue;
        }
        out.push('\n');
        let _ = writeln!(out, "## {}", file.rel);
        for symbol in &parsed.symbols {
            let indent = if symbol.kind == SymbolKind::Method {
                "    "
            } else {
                ""
            };
            let _ = writeln!(out, "{indent}{}", symbol.signature);
        }
        if !parsed.imports.is_empty() {
            let _ = writeln!(out, "imports: {}", parsed.imports.join(", "));
        }
    }

    // FR-12 footer: skipped and not-yet-wired files are reported, never
    // silently dropped.
    if stats.skipped_files > 0 || stats.unwired_files > 0 {
        out.push('\n');
        let _ = writeln!(
            out,
            "[{} unparseable file(s) skipped; {} file(s) in languages not yet wired (TS/JS, Rust land in M1)]",
            stats.skipped_files, stats.unwired_files
        );
    }
    out
}

#[cfg(test)]
mod tests {
    // Golden-output determinism tests land with the M1 budgeted renderer;
    // the M0 naive path is covered by tests/query_snapshots.rs end-to-end.
}

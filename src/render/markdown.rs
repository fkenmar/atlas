//! Markdown renderer — the default output, optimized for LLM readability
//! (example layout: PRD §5.3). [`render`] emits a budgeted map: header with
//! the budget/rendered figures, files in rank order, then the collapsed
//! directory-skeleton footer. Output is deterministic — the budget stage
//! delivers files and symbols already ordered (NFR-4).
//!
//! [`render_naive_map`] is the M0 full unranked/unbudgeted map, kept for the
//! CLI until the budgeted pipeline is wired (M1 integration).

use std::fmt::Write as _;

use crate::budget::{BudgetedMap, Detail};
use crate::parse::{ParseOutcome, SymbolKind};

pub struct MarkdownRenderer;

impl super::Renderer for MarkdownRenderer {
    fn render(&self, map: &BudgetedMap) -> String {
        render(map)
    }
}

/// Render a budgeted map to Markdown. Used both for final output and, by the
/// budget stage, to measure exact token counts of candidate maps.
pub fn render(map: &BudgetedMap) -> String {
    let mut out = String::new();
    let degraded = match map.detail {
        Detail::Full => "",
        Detail::NoPrivate => " | public-only",
        Detail::NoParams => " | public-only, params elided",
    };
    let _ = writeln!(
        out,
        "# repomap: {} ({} LOC, {} files) | budget {} | rendered {} tok{degraded}",
        map.repo_name, map.total_loc, map.total_files, map.target_tokens, map.rendered_tokens
    );

    for file in &map.files {
        out.push('\n');
        if file.imported_by > 0 {
            let _ = writeln!(
                out,
                "## {} (#{} — imported by {} file(s))",
                file.rel, file.rank, file.imported_by
            );
        } else {
            let _ = writeln!(out, "## {} (#{})", file.rel, file.rank);
        }
        for symbol in &file.symbols {
            let indent = if symbol.kind == SymbolKind::Method {
                "    "
            } else {
                ""
            };
            let _ = writeln!(out, "{indent}{}", symbol.signature);
        }
        if !file.imports.is_empty() {
            let _ = writeln!(out, "imports: {}", file.imports.join(", "));
        }
    }

    // Directory-skeleton footer: low-rank files dropped to fit, never silently
    // lost (PRD §5.1 — the skeleton is always retained).
    if !map.collapsed.is_empty() {
        let total: usize = map.collapsed.iter().map(|c| c.count).sum();
        let groups: Vec<String> = map
            .collapsed
            .iter()
            .map(|c| format!("{}/* ({})", c.dir, c.count))
            .collect();
        out.push('\n');
        let _ = writeln!(
            out,
            "[{total} low-rank file(s) collapsed: {}]",
            groups.join(", ")
        );
    }

    // FR-12 footer: skipped and not-yet-wired files reported, never dropped.
    if map.skipped_files > 0 || map.unwired_files > 0 {
        out.push('\n');
        let _ = writeln!(
            out,
            "[{} unparseable file(s) skipped; {} file(s) in languages not yet wired]",
            map.skipped_files, map.unwired_files
        );
    }
    out
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

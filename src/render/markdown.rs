//! Markdown renderer — the default output, optimized for LLM readability
//! (example layout: PRD §5.3). [`render`] emits a budgeted map: header with
//! the budget/rendered figures, files in rank order, then the collapsed
//! directory-skeleton footer. Output is deterministic — the budget stage
//! delivers files and symbols already ordered (NFR-4).
//!
//! [`render_naive_map`] is the M0 full unranked/unbudgeted map, kept for the
//! CLI until the budgeted pipeline is wired (M1 integration).

use std::fmt::Write as _;

use crate::api::SymbolIndexMap;
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
    // Word the header honestly: when the user asked for `--no-private` say so;
    // when the budget forced symbols out, name the action and the lever
    // (`--budget`) instead of the cryptic bare "public-only".
    let degraded = match (map.detail, map.requested_no_private) {
        (Detail::Full, _) => "",
        (Detail::NoPrivate, true) => " | public API only (--no-private)",
        (Detail::NoPrivate, false) => {
            " | private symbols omitted to fit budget — raise --budget for full detail"
        }
        (Detail::NoParams, true) => {
            " | public API only, parameter names omitted to fit budget — raise --budget"
        }
        (Detail::NoParams, false) => {
            " | private symbols + parameter names omitted to fit budget — raise --budget"
        }
    };
    let _ = writeln!(
        out,
        "# atlas: {} ({} LOC, {} files) | budget {} | rendered {} tok{degraded}",
        map.repo_name, map.total_loc, map.total_files, map.target_tokens, map.rendered_tokens
    );

    for file in &map.files {
        out.push('\n');
        // Rung-3 one-line summary: a file too large to show in full still earns
        // a single line recording its existence and symbol count.
        if file.one_line {
            let _ = writeln!(
                out,
                "## {} (#{}, {} symbol(s) — collapsed to fit)",
                file.rel,
                file.rank,
                file.symbols.len()
            );
            continue;
        }
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
            let indent = if matches!(symbol.kind, SymbolKind::Method | SymbolKind::Field) {
                "    "
            } else {
                ""
            };
            let _ = writeln!(out, "{indent}{}", symbol.signature);
        }
        // Partial rung: the file's lower-ranked symbols were dropped to fit.
        if file.omitted > 0 {
            let _ = writeln!(out, "… ({} more symbol(s))", file.omitted);
        }
        if !file.imports.is_empty() {
            let _ = writeln!(out, "imports: {}", file.imports.join(", "));
        }
        // Reverse deps — the edit sites for a change to this file's API.
        if !file.used_by.is_empty() {
            let _ = writeln!(out, "used by: {}", file.used_by.join(", "));
        }
    }

    // Symbol index: the navigable declarations of files that didn't fit in
    // full, as stable ADR 0009 anchors (grouped by file, rank order). Lets an
    // agent locate/expand the long tail without grepping, at a fraction of a
    // full block's cost. Entries arrive already ordered (file rank, then source
    // order), so runs from the same file are contiguous and group cleanly.
    if !map.symbol_index.is_empty() {
        out.push('\n');
        out.push_str(
            "---\nsymbol index (PARTIAL — more symbols by file, not exhaustive; if the \
             symbol you need isn't listed here, grep the source instead of guessing. \
             Anchor = `path#name`; expand it for the full signature):\n",
        );
        let mut i = 0;
        while i < map.symbol_index.len() {
            let rel = &map.symbol_index[i].rel;
            let mut names: Vec<&str> = Vec::new();
            while i < map.symbol_index.len() && &map.symbol_index[i].rel == rel {
                // Show the bare name (or `name@line`): the path is already the line
                // prefix, so repeating it inside every anchor is pure token bloat
                // (it halved the comprehension win, 65% -> 30%). The anchor stays
                // derivable as `path#name` for expand_symbol.
                let anchor = map.symbol_index[i].anchor.as_str();
                let display = anchor
                    .strip_prefix(rel.as_str())
                    .and_then(|s| s.strip_prefix('#'))
                    .unwrap_or(anchor);
                names.push(display);
                i += 1;
            }
            let _ = writeln!(out, "{rel}: {}", names.join(", "));
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

/// Render the ADR 0009 progressive-disclosure symbol index: stable anchors only,
/// no signatures. The API uses this text for exact budget measurement; MCP can
/// return structured JSON from the same entries.
pub fn render_symbol_index(index: &SymbolIndexMap) -> String {
    let mut out = String::new();
    let _ = writeln!(
        out,
        "# atlas symbol index: {} ({} LOC, {} files) | budget {} | rendered {} tok",
        index.repo_name,
        index.total_loc,
        index.total_files,
        index.target_tokens,
        index.rendered_tokens
    );
    for entry in &index.entries {
        let _ = writeln!(out, "{} {}", entry.kind, entry.anchor);
    }
    if index.skipped_files > 0 || index.unwired_files > 0 {
        out.push('\n');
        let _ = writeln!(
            out,
            "[{} unparseable file(s) skipped; {} file(s) in languages not yet wired]",
            index.skipped_files, index.unwired_files
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
        "# atlas: {repo_name} ({} LOC, {} files) | naive full map (M0 — no ranking/budgeting yet)",
        stats.total_lines, stats.parsed_files
    );

    for (file, parsed) in &outcome.files {
        if parsed.symbols.is_empty() && parsed.imports.is_empty() {
            continue;
        }
        out.push('\n');
        let _ = writeln!(out, "## {}", file.rel);
        for symbol in &parsed.symbols {
            let indent = if matches!(symbol.kind, SymbolKind::Method | SymbolKind::Field) {
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

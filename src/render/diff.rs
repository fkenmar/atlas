//! Markdown renderer for a structural diff (ADR 0005). Deterministic: the
//! [`StructuralDiff`] arrives already sorted from the diff stage, so this just
//! walks it. `+`/`-`/`~` mark added/removed/changed declarations.

use std::fmt::Write as _;

use crate::diff::StructuralDiff;
use crate::parse::ParseStats;

/// Render a structural diff to Markdown. `old_label`/`new_label` are the two
/// trees as the user named them (shown in the header). `old_stats`/`new_stats`
/// drive the FR-12 footer: a file that's unparseable on only one side would
/// otherwise appear as a phantom add/remove, so each side's skip counts are
/// reported (mirroring the map renderer).
pub fn render(
    diff: &StructuralDiff,
    old_label: &str,
    new_label: &str,
    old_stats: &ParseStats,
    new_stats: &ParseStats,
) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "# atlas diff: {old_label} → {new_label}");

    if diff.is_empty() {
        out.push('\n');
        let _ = writeln!(out, "No structural changes.");
        push_skip_footer(&mut out, "old", old_stats);
        push_skip_footer(&mut out, "new", new_stats);
        return out;
    }

    if !diff.added_files.is_empty() {
        out.push('\n');
        let _ = writeln!(out, "## Added files");
        for f in &diff.added_files {
            let _ = writeln!(
                out,
                "+ {} ({}, {} symbol(s))",
                f.rel, f.lang, f.symbol_count
            );
        }
    }

    if !diff.removed_files.is_empty() {
        out.push('\n');
        let _ = writeln!(out, "## Removed files");
        for f in &diff.removed_files {
            let _ = writeln!(
                out,
                "- {} ({}, {} symbol(s))",
                f.rel, f.lang, f.symbol_count
            );
        }
    }

    if !diff.changed_files.is_empty() {
        out.push('\n');
        let _ = writeln!(out, "## Changed files");
        for fd in &diff.changed_files {
            out.push('\n');
            let _ = writeln!(out, "### {}", fd.rel);
            for c in &fd.changed {
                let _ = writeln!(
                    out,
                    "~ {} {}: {} → {}",
                    c.kind, c.name, c.old_signature, c.new_signature
                );
            }
            for s in &fd.added {
                let _ = writeln!(out, "+ {}", s.signature);
            }
            for s in &fd.removed {
                let _ = writeln!(out, "- {}", s.signature);
            }
            if !fd.added_imports.is_empty() {
                let _ = writeln!(out, "imports +: {}", fd.added_imports.join(", "));
            }
            if !fd.removed_imports.is_empty() {
                let _ = writeln!(out, "imports -: {}", fd.removed_imports.join(", "));
            }
        }
    }

    // FR-12: report each side's unparseable / not-yet-wired files so a one-sided
    // skip isn't mistaken for a real add/remove.
    push_skip_footer(&mut out, "old", old_stats);
    push_skip_footer(&mut out, "new", new_stats);
    out
}

/// Append the FR-12 skip/unwired footer for one side, only when nonzero.
fn push_skip_footer(out: &mut String, side: &str, stats: &ParseStats) {
    if stats.skipped_files > 0 || stats.unwired_files > 0 {
        out.push('\n');
        let _ = writeln!(
            out,
            "[{side} tree: {} unparseable file(s) skipped; {} file(s) in languages not yet wired]",
            stats.skipped_files, stats.unwired_files
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diff::{FileDelta, FileSummary, SymbolChange, SymbolLine};

    fn sample() -> StructuralDiff {
        StructuralDiff {
            added_files: vec![FileSummary {
                rel: "b.py".to_string(),
                lang: "python",
                symbol_count: 2,
            }],
            removed_files: vec![FileSummary {
                rel: "a.py".to_string(),
                lang: "python",
                symbol_count: 1,
            }],
            changed_files: vec![FileDelta {
                rel: "x.py".to_string(),
                added: vec![SymbolLine {
                    kind: "function",
                    name: "h".to_string(),
                    signature: "def h()".to_string(),
                }],
                removed: vec![SymbolLine {
                    kind: "function",
                    name: "g".to_string(),
                    signature: "def g()".to_string(),
                }],
                changed: vec![SymbolChange {
                    kind: "function",
                    name: "f".to_string(),
                    old_signature: "def f(x)".to_string(),
                    new_signature: "def f(x, y)".to_string(),
                }],
                added_imports: vec!["c".to_string()],
                removed_imports: vec!["a".to_string()],
            }],
        }
    }

    fn empty_diff() -> StructuralDiff {
        StructuralDiff {
            added_files: vec![],
            removed_files: vec![],
            changed_files: vec![],
        }
    }

    #[test]
    fn header_and_empty_message() {
        let out = render(
            &empty_diff(),
            "old",
            "new",
            &ParseStats::default(),
            &ParseStats::default(),
        );
        assert!(out.contains("# atlas diff: old → new"));
        assert!(out.contains("No structural changes."));
    }

    #[test]
    fn skip_footer_reports_unparseable_files_per_side() {
        let new_stats = ParseStats {
            skipped_files: 2,
            unwired_files: 1,
            ..Default::default()
        };
        let out = render(
            &empty_diff(),
            "old",
            "new",
            &ParseStats::default(),
            &new_stats,
        );
        // Only the side with skips gets a footer line.
        assert!(out.contains(
            "[new tree: 2 unparseable file(s) skipped; 1 file(s) in languages not yet wired]"
        ));
        assert!(!out.contains("[old tree:"));
    }

    #[test]
    fn renders_all_sections() {
        let out = render(
            &sample(),
            "HEAD~1",
            "HEAD",
            &ParseStats::default(),
            &ParseStats::default(),
        );
        assert!(out.contains("# atlas diff: HEAD~1 → HEAD"));
        assert!(out.contains("## Added files"));
        assert!(out.contains("+ b.py (python, 2 symbol(s))"));
        assert!(out.contains("## Removed files"));
        assert!(out.contains("- a.py (python, 1 symbol(s))"));
        assert!(out.contains("## Changed files"));
        assert!(out.contains("### x.py"));
        assert!(out.contains("~ function f: def f(x) → def f(x, y)"));
        assert!(out.contains("+ def h()"));
        assert!(out.contains("- def g()"));
        assert!(out.contains("imports +: c"));
        assert!(out.contains("imports -: a"));
    }

    #[test]
    fn is_deterministic() {
        let s = ParseStats::default();
        assert_eq!(
            render(&sample(), "a", "b", &s, &s),
            render(&sample(), "a", "b", &s, &s)
        );
    }
}

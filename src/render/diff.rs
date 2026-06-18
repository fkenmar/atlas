//! Markdown renderer for a structural diff (ADR 0005). Deterministic: the
//! [`StructuralDiff`] arrives already sorted from the diff stage, so this just
//! walks it. `+`/`-`/`~` mark added/removed/changed declarations.

use std::fmt::Write as _;

use crate::diff::{FileDelta, FileSummary, KindChange, StructuralDiff, SymbolChange, SymbolLine};
use crate::parse::ParseStats;
use crate::render::json::json_str;
use crate::render::xml::xml_escape;

/// Version stamped into the structured diff output (`atlas diff --format
/// json|xml`). Independent of the map's schema version — the diff is a separate
/// contract — so programmatic consumers can detect future shape changes. Bumped
/// on any breaking change to the diff schema (see the release-process skill).
pub const DIFF_SCHEMA_VERSION: u32 = 1;

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
            for k in &fd.kind_changed {
                let _ = writeln!(
                    out,
                    "± {}: {} {} → {} {}",
                    k.name, k.old_kind, k.old_signature, k.new_kind, k.new_signature
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

/// Render a structural diff as JSON (deterministic; mirrors the diff's logical
/// schema). Hand-rolled like the map's JSON renderer — no serde.
pub fn render_json(
    diff: &StructuralDiff,
    old_label: &str,
    new_label: &str,
    old_stats: &ParseStats,
    new_stats: &ParseStats,
) -> String {
    let added = json_arr(&diff.added_files, file_summary_json);
    let removed = json_arr(&diff.removed_files, file_summary_json);
    let changed = json_arr(&diff.changed_files, file_delta_json);
    let mut out = String::new();
    out.push_str("{\n");
    let _ = writeln!(out, "  \"version\": {DIFF_SCHEMA_VERSION},");
    let _ = writeln!(out, "  \"old\": {},", json_str(old_label));
    let _ = writeln!(out, "  \"new\": {},", json_str(new_label));
    let _ = writeln!(out, "  \"added_files\": {added},");
    let _ = writeln!(out, "  \"removed_files\": {removed},");
    let _ = writeln!(out, "  \"changed_files\": {changed},");
    let _ = writeln!(
        out,
        "  \"skipped\": {{\"old\": {}, \"new\": {}}}",
        stats_json(old_stats),
        stats_json(new_stats)
    );
    out.push_str("}\n");
    out
}

fn json_arr<T>(items: &[T], f: fn(&T) -> String) -> String {
    if items.is_empty() {
        return "[]".to_string();
    }
    format!("[{}]", items.iter().map(f).collect::<Vec<_>>().join(", "))
}

fn file_summary_json(f: &FileSummary) -> String {
    format!(
        "{{\"path\": {}, \"lang\": {}, \"symbols\": {}}}",
        json_str(&f.rel),
        json_str(f.lang),
        f.symbol_count
    )
}

fn file_delta_json(fd: &FileDelta) -> String {
    let changed = json_arr(&fd.changed, |c: &SymbolChange| {
        format!(
            "{{\"kind\": {}, \"name\": {}, \"old_sig\": {}, \"new_sig\": {}}}",
            json_str(c.kind),
            json_str(&c.name),
            json_str(&c.old_signature),
            json_str(&c.new_signature)
        )
    });
    let kind_changed = json_arr(&fd.kind_changed, |k: &KindChange| {
        format!(
            "{{\"name\": {}, \"old_kind\": {}, \"new_kind\": {}, \
             \"old_sig\": {}, \"new_sig\": {}}}",
            json_str(&k.name),
            json_str(k.old_kind),
            json_str(k.new_kind),
            json_str(&k.old_signature),
            json_str(&k.new_signature)
        )
    });
    let added = json_arr(&fd.added, symbol_line_json);
    let removed = json_arr(&fd.removed, symbol_line_json);
    let added_imports = json_arr(&fd.added_imports, |i: &String| json_str(i));
    let removed_imports = json_arr(&fd.removed_imports, |i: &String| json_str(i));
    format!(
        "{{\"path\": {}, \"changed\": {changed}, \"kind_changed\": {kind_changed}, \
         \"added\": {added}, \"removed\": {removed}, \
         \"added_imports\": {added_imports}, \"removed_imports\": {removed_imports}}}",
        json_str(&fd.rel)
    )
}

fn symbol_line_json(s: &SymbolLine) -> String {
    format!(
        "{{\"kind\": {}, \"name\": {}, \"sig\": {}}}",
        json_str(s.kind),
        json_str(&s.name),
        json_str(&s.signature)
    )
}

fn stats_json(st: &ParseStats) -> String {
    format!(
        "{{\"skipped_files\": {}, \"unwired_files\": {}}}",
        st.skipped_files, st.unwired_files
    )
}

/// Render a structural diff as well-formed XML (deterministic; signatures/paths
/// escaped per XML 1.0 via the shared escaper).
pub fn render_xml(
    diff: &StructuralDiff,
    old_label: &str,
    new_label: &str,
    old_stats: &ParseStats,
    new_stats: &ParseStats,
) -> String {
    let mut out = String::new();
    out.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    let _ = writeln!(
        out,
        "<atlas-diff version=\"{DIFF_SCHEMA_VERSION}\" old=\"{}\" new=\"{}\">",
        xml_escape(old_label, true),
        xml_escape(new_label, true)
    );

    xml_file_list(&mut out, "added-files", &diff.added_files);
    xml_file_list(&mut out, "removed-files", &diff.removed_files);

    if diff.changed_files.is_empty() {
        out.push_str("  <changed-files/>\n");
    } else {
        out.push_str("  <changed-files>\n");
        for fd in &diff.changed_files {
            xml_file_delta(&mut out, fd);
        }
        out.push_str("  </changed-files>\n");
    }

    out.push_str("  <skipped>\n");
    xml_stats(&mut out, "old", old_stats);
    xml_stats(&mut out, "new", new_stats);
    out.push_str("  </skipped>\n");
    out.push_str("</atlas-diff>\n");
    out
}

fn xml_file_list(out: &mut String, tag: &str, files: &[FileSummary]) {
    if files.is_empty() {
        let _ = writeln!(out, "  <{tag}/>");
        return;
    }
    let _ = writeln!(out, "  <{tag}>");
    for f in files {
        let _ = writeln!(
            out,
            "    <file path=\"{}\" lang=\"{}\" symbols=\"{}\"/>",
            xml_escape(&f.rel, true),
            xml_escape(f.lang, true),
            f.symbol_count
        );
    }
    let _ = writeln!(out, "  </{tag}>");
}

fn xml_file_delta(out: &mut String, fd: &FileDelta) {
    let _ = writeln!(out, "    <file path=\"{}\">", xml_escape(&fd.rel, true));
    for c in &fd.changed {
        let _ = writeln!(
            out,
            "      <changed kind=\"{}\" name=\"{}\" old-sig=\"{}\" new-sig=\"{}\"/>",
            xml_escape(c.kind, true),
            xml_escape(&c.name, true),
            xml_escape(&c.old_signature, true),
            xml_escape(&c.new_signature, true)
        );
    }
    for k in &fd.kind_changed {
        let _ = writeln!(
            out,
            "      <kind-changed name=\"{}\" old-kind=\"{}\" new-kind=\"{}\" \
             old-sig=\"{}\" new-sig=\"{}\"/>",
            xml_escape(&k.name, true),
            xml_escape(k.old_kind, true),
            xml_escape(k.new_kind, true),
            xml_escape(&k.old_signature, true),
            xml_escape(&k.new_signature, true)
        );
    }
    for s in &fd.added {
        xml_symbol_line(out, "added", s);
    }
    for s in &fd.removed {
        xml_symbol_line(out, "removed", s);
    }
    xml_import_list(out, "added-imports", &fd.added_imports);
    xml_import_list(out, "removed-imports", &fd.removed_imports);
    out.push_str("    </file>\n");
}

fn xml_symbol_line(out: &mut String, tag: &str, s: &SymbolLine) {
    let _ = writeln!(
        out,
        "      <{tag} kind=\"{}\" name=\"{}\" sig=\"{}\"/>",
        xml_escape(s.kind, true),
        xml_escape(&s.name, true),
        xml_escape(&s.signature, true)
    );
}

fn xml_import_list(out: &mut String, tag: &str, imports: &[String]) {
    if imports.is_empty() {
        let _ = writeln!(out, "      <{tag}/>");
        return;
    }
    let _ = writeln!(out, "      <{tag}>");
    for i in imports {
        let _ = writeln!(out, "        <import>{}</import>", xml_escape(i, false));
    }
    let _ = writeln!(out, "      </{tag}>");
}

fn xml_stats(out: &mut String, side: &str, st: &ParseStats) {
    let _ = writeln!(
        out,
        "    <{side} skipped-files=\"{}\" unwired-files=\"{}\"/>",
        st.skipped_files, st.unwired_files
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diff::{FileDelta, FileSummary, KindChange, SymbolChange, SymbolLine};

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
                kind_changed: vec![KindChange {
                    name: "k".to_string(),
                    old_kind: "function",
                    new_kind: "method",
                    old_signature: "def k()".to_string(),
                    new_signature: "def k(self)".to_string(),
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
        assert!(out.contains("± k: function def k() → method def k(self)"));
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

    #[test]
    fn json_carries_the_full_delta() {
        let s = ParseStats::default();
        let out = render_json(&sample(), "old", "new", &s, &s);
        assert!(out.contains("\"version\": 1"), "{out}");
        assert!(out.contains("\"old\": \"old\""), "{out}");
        assert!(out.contains("\"new\": \"new\""), "{out}");
        assert!(
            out.contains("\"path\": \"b.py\", \"lang\": \"python\", \"symbols\": 2"),
            "{out}"
        );
        assert!(out.contains("\"path\": \"a.py\""), "{out}");
        assert!(
            out.contains(
                "\"kind\": \"function\", \"name\": \"f\", \
                 \"old_sig\": \"def f(x)\", \"new_sig\": \"def f(x, y)\""
            ),
            "{out}"
        );
        assert!(out.contains("\"sig\": \"def h()\""), "{out}");
        assert!(out.contains("\"sig\": \"def g()\""), "{out}");
        assert!(
            out.contains(
                "\"name\": \"k\", \"old_kind\": \"function\", \"new_kind\": \"method\", \
                 \"old_sig\": \"def k()\", \"new_sig\": \"def k(self)\""
            ),
            "{out}"
        );
        assert!(out.contains("\"added_imports\": [\"c\"]"), "{out}");
        assert!(out.contains("\"removed_imports\": [\"a\"]"), "{out}");
        assert!(out.contains("\"skipped\""), "{out}");
    }

    #[test]
    fn json_escapes_and_is_deterministic() {
        let mut d = sample();
        d.changed_files[0].added[0].signature = "def h(s=\"x\")".to_string();
        let s = ParseStats::default();
        let out = render_json(&d, "a", "b", &s, &s);
        assert!(out.contains("\"sig\": \"def h(s=\\\"x\\\")\""), "{out}");
        assert_eq!(
            render_json(&sample(), "a", "b", &s, &s),
            render_json(&sample(), "a", "b", &s, &s)
        );
    }

    #[test]
    fn xml_is_well_formed_and_escaped() {
        let mut d = sample();
        // A signature with XML metacharacters must be escaped, not break out.
        d.changed_files[0].changed[0].new_signature = "fn f<T>() -> &T".to_string();
        let s = ParseStats::default();
        let out = render_xml(&d, "old", "new", &s, &s);
        assert!(
            out.contains("<?xml version=\"1.0\" encoding=\"UTF-8\"?>"),
            "{out}"
        );
        assert!(
            out.contains("<atlas-diff version=\"1\" old=\"old\" new=\"new\">"),
            "{out}"
        );
        assert!(
            out.contains("new-sig=\"fn f&lt;T&gt;() -&gt; &amp;T\""),
            "{out}"
        );
        assert!(
            out.contains(
                "<kind-changed name=\"k\" old-kind=\"function\" new-kind=\"method\" \
                 old-sig=\"def k()\" new-sig=\"def k(self)\"/>"
            ),
            "{out}"
        );
        assert!(out.trim_end().ends_with("</atlas-diff>"), "{out}");
        assert_eq!(
            render_xml(&sample(), "a", "b", &s, &s),
            render_xml(&sample(), "a", "b", &s, &s)
        );
    }
}

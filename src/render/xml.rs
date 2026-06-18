//! XML renderer — well-formed XML for prompt-injection-safe wrapping in Claude
//! prompts (PRD §6) and for XML-native programmatic consumers. Describes the
//! *same* logical schema as the JSON renderer (`render/json.rs`): it reuses
//! [`SCHEMA_VERSION`] and the shared kind/visibility/detail vocabulary so the
//! two structured formats never drift. Code text (signatures, paths) is escaped
//! per the XML 1.0 spec so embedded source can't break out of the structure.
//!
//! Output is deterministic (NFR-4): files and symbols arrive already ordered
//! from the budget stage, and `f64` scores use a fixed-precision format.

use std::fmt::Write as _;

use crate::budget::{BudgetedFile, BudgetedMap, RenderedSymbol};
use crate::render::json::{detail_name, kind_name, visibility_name, SCHEMA_VERSION};

pub struct XmlRenderer;

impl super::Renderer for XmlRenderer {
    fn render(&self, map: &BudgetedMap) -> String {
        render(map)
    }
}

/// Render a budgeted map as XML (schema version [`SCHEMA_VERSION`], shared with
/// the JSON renderer).
pub fn render(map: &BudgetedMap) -> String {
    let mut out = String::new();
    out.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    let _ = writeln!(out, "<atlas version=\"{SCHEMA_VERSION}\">");
    let _ = writeln!(
        out,
        "  <repo name=\"{}\" loc=\"{}\" files=\"{}\"/>",
        xml_escape(&map.repo_name, true),
        map.total_loc,
        map.total_files
    );
    let _ = writeln!(
        out,
        "  <budget target=\"{}\" rendered=\"{}\" detail=\"{}\"/>",
        map.target_tokens,
        map.rendered_tokens,
        detail_name(map.detail)
    );

    if map.files.is_empty() {
        out.push_str("  <files/>\n");
    } else {
        out.push_str("  <files>\n");
        for file in &map.files {
            render_file(&mut out, file);
        }
        out.push_str("  </files>\n");
    }

    // Directory-skeleton footer: low-rank files dropped to fit (PRD §5.1).
    if map.collapsed.is_empty() {
        out.push_str("  <collapsed/>\n");
    } else {
        out.push_str("  <collapsed>\n");
        for c in &map.collapsed {
            let _ = writeln!(
                out,
                // Attribute name mirrors the JSON renderer's `dir` key so the
                // two structured formats don't drift (`<dir>` is the element,
                // `dir=` the same logical field json.rs emits).
                "    <dir dir=\"{}\" count=\"{}\"/>",
                xml_escape(&c.dir, true),
                c.count
            );
        }
        out.push_str("  </collapsed>\n");
    }

    // Symbol index: navigable declarations of files that didn't fit in full
    // (ADR 0004). Entries arrive already ordered.
    if map.symbol_index.is_empty() {
        out.push_str("  <symbol_index/>\n");
    } else {
        out.push_str("  <symbol_index>\n");
        for s in &map.symbol_index {
            let _ = writeln!(
                out,
                "    <symbol name=\"{}\" kind=\"{}\" path=\"{}\"/>",
                xml_escape(&s.name, true),
                kind_name(s.kind),
                xml_escape(&s.rel, true)
            );
        }
        out.push_str("  </symbol_index>\n");
    }

    // FR-12 footer: skipped and not-yet-wired files reported, never dropped.
    let _ = writeln!(
        out,
        "  <skipped_files>{}</skipped_files>",
        map.skipped_files
    );
    let _ = writeln!(
        out,
        "  <unwired_files>{}</unwired_files>",
        map.unwired_files
    );
    out.push_str("</atlas>\n");
    out
}

fn render_file(out: &mut String, file: &BudgetedFile) {
    let _ = writeln!(
        out,
        "    <file path=\"{}\" lang=\"{}\" rank=\"{}\" score=\"{:.6}\" \
         imported_by=\"{}\" one_line=\"{}\" omitted=\"{}\">",
        xml_escape(&file.rel, true),
        xml_escape(file.lang, true),
        file.rank,
        file.score,
        file.imported_by,
        file.one_line,
        file.omitted
    );

    if file.symbols.is_empty() {
        out.push_str("      <symbols/>\n");
    } else {
        out.push_str("      <symbols>\n");
        for s in &file.symbols {
            render_symbol(out, s);
        }
        out.push_str("      </symbols>\n");
    }

    if file.imports.is_empty() {
        out.push_str("      <imports/>\n");
    } else {
        out.push_str("      <imports>\n");
        for i in &file.imports {
            let _ = writeln!(out, "        <import>{}</import>", xml_escape(i, false));
        }
        out.push_str("      </imports>\n");
    }

    // Reverse deps — the edit sites for a change to this file's API.
    if file.used_by.is_empty() {
        out.push_str("      <used_by/>\n");
    } else {
        out.push_str("      <used_by>\n");
        for u in &file.used_by {
            let _ = writeln!(out, "        <ref>{}</ref>", xml_escape(u, false));
        }
        out.push_str("      </used_by>\n");
    }

    out.push_str("    </file>\n");
}

fn render_symbol(out: &mut String, s: &RenderedSymbol) {
    // Metadata as attributes; the signature as escaped text content so embedded
    // source (with `<`, `>`, `&`) stays well-formed and can't break out.
    let _ = writeln!(
        out,
        "        <symbol kind=\"{}\" name=\"{}\" line=\"{}\" visibility=\"{}\">{}</symbol>",
        kind_name(s.kind),
        xml_escape(&s.name, true),
        s.line,
        visibility_name(s.visibility),
        xml_escape(&s.signature, false)
    );
}

/// Escape a string for inclusion in XML 1.0. `&`, `<`, `>` are always escaped;
/// in `attr` mode `"` is too (attributes are double-quoted) and tab/newline/CR
/// become numeric refs so they survive attribute-value normalization (XML 1.0
/// §3.3.3) — in text content they stay literal. Any code point outside the XML
/// 1.0 `Char` production (C0 controls, U+FFFE/U+FFFF, the BMP gap above
/// U+FFFD) is dropped, so the output is always well-formed.
pub(crate) fn xml_escape(s: &str, attr: bool) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' if attr => out.push_str("&quot;"),
            '\t' if attr => out.push_str("&#9;"),
            '\n' if attr => out.push_str("&#10;"),
            '\r' if attr => out.push_str("&#13;"),
            '\t' | '\n' | '\r' => out.push(c),
            c if is_xml_char(c) => out.push(c),
            _ => {}
        }
    }
    out
}

/// Whether `c` is allowed by the XML 1.0 `Char` production. Tab/newline/CR
/// (also legal) are handled before this in [`xml_escape`]; lone surrogates
/// can't occur in a Rust `char`, so the practical rejects here are C0 controls,
/// U+FFFE/U+FFFF, and the BMP gap between U+D7FF and U+E000.
fn is_xml_char(c: char) -> bool {
    matches!(c as u32, 0x20..=0xD7FF | 0xE000..=0xFFFD | 0x10000..=0x10FFFF)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::budget::{CollapsedDir, Detail, IndexedSymbol};
    use crate::parse::{SymbolKind, Visibility};

    fn sample_map() -> BudgetedMap {
        BudgetedMap {
            repo_name: "demo".to_string(),
            target_tokens: 2048,
            rendered_tokens: 120,
            total_loc: 5000,
            total_files: 12,
            detail: Detail::Full,
            requested_no_private: false,
            files: vec![BudgetedFile {
                rel: "src/auth.py".to_string(),
                lang: "python",
                rank: 1,
                score: 0.0410,
                imported_by: 3,
                imports: vec!["src/db.py".to_string()],
                used_by: vec!["src/api.py".to_string()],
                symbols: vec![RenderedSymbol {
                    kind: SymbolKind::Method,
                    name: "login".to_string(),
                    signature: "def login(user: str) -> bool".to_string(),
                    visibility: Visibility::Public,
                    line: 12,
                }],
                one_line: false,
                omitted: 0,
            }],
            collapsed: vec![CollapsedDir {
                dir: "tests".to_string(),
                count: 4,
            }],
            symbol_index: vec![IndexedSymbol {
                name: "Widget".to_string(),
                kind: SymbolKind::Class,
                rel: "src/widget.py".to_string(),
            }],
            skipped_files: 1,
            unwired_files: 2,
        }
    }

    #[test]
    fn schema_shape_is_stable() {
        let xml = render(&sample_map());
        assert!(xml.contains("<?xml version=\"1.0\" encoding=\"UTF-8\"?>"));
        assert!(xml.contains("<atlas version=\"1\">"));
        assert!(xml.contains("<repo name=\"demo\" loc=\"5000\" files=\"12\"/>"));
        assert!(xml.contains("<budget target=\"2048\" rendered=\"120\" detail=\"full\"/>"));
        assert!(xml.contains(
            "<file path=\"src/auth.py\" lang=\"python\" rank=\"1\" score=\"0.041000\" \
             imported_by=\"3\" one_line=\"false\" omitted=\"0\">"
        ));
        // The signature's `->` is escaped in text content.
        assert!(xml.contains(
            "<symbol kind=\"method\" name=\"login\" line=\"12\" visibility=\"public\">\
             def login(user: str) -&gt; bool</symbol>"
        ));
        assert!(xml.contains("<import>src/db.py</import>"));
        assert!(xml.contains("<ref>src/api.py</ref>"));
        assert!(xml.contains("<dir dir=\"tests\" count=\"4\"/>"));
        assert!(xml.contains("<symbol name=\"Widget\" kind=\"class\" path=\"src/widget.py\"/>"));
        assert!(xml.contains("<skipped_files>1</skipped_files>"));
        assert!(xml.contains("<unwired_files>2</unwired_files>"));
        assert!(xml.trim_end().ends_with("</atlas>"));
    }

    #[test]
    fn text_escaping_leaves_quotes_but_escapes_angles_and_amp() {
        assert_eq!(xml_escape("a<b>&\"c", false), "a&lt;b&gt;&amp;\"c");
    }

    #[test]
    fn attribute_escaping_also_escapes_quotes() {
        assert_eq!(xml_escape("a<b>&\"c", true), "a&lt;b&gt;&amp;&quot;c");
    }

    #[test]
    fn illegal_control_chars_are_dropped() {
        // NUL and a C0 control are illegal in XML 1.0 and removed; tab survives.
        assert_eq!(xml_escape("a\u{0}b\u{7}c\td", false), "abc\td");
    }

    #[test]
    fn illegal_high_chars_are_dropped_but_u_fffd_survives() {
        // U+FFFE / U+FFFF are outside the XML 1.0 Char production → dropped;
        // U+FFFD (the replacement char) is the legal upper bound → kept.
        assert_eq!(xml_escape("a\u{FFFE}b\u{FFFF}c", false), "abc");
        assert_eq!(xml_escape("a\u{FFFD}b", false), "a\u{FFFD}b");
    }

    #[test]
    fn attribute_whitespace_uses_numeric_refs_text_keeps_literal() {
        // In attributes, tab/newline/CR must be numeric refs to survive
        // attribute-value normalization; in text content they stay literal.
        assert_eq!(xml_escape("a\tb\nc\rd", true), "a&#9;b&#10;c&#13;d");
        assert_eq!(xml_escape("a\tb\nc\rd", false), "a\tb\nc\rd");
    }

    #[test]
    fn adversarial_input_stays_well_formed_and_escaped() {
        let mut map = sample_map();
        // Attribute-context special chars + an illegal XML char in a path.
        map.files[0].rel = "src/a<b>&\"\u{FFFE}c.py".to_string();
        // A structural-breakout attempt smuggled through a symbol name (attr)
        // and a signature (text content).
        map.files[0].symbols[0].name = "ev<il>&\"".to_string();
        map.files[0].symbols[0].signature =
            "fn pwn() </symbol></file></atlas><inject/>".to_string();
        let xml = render(&map);

        // The illegal code point is gone — the document parses.
        assert!(!xml.contains('\u{FFFE}'));
        // No injected structure leaked: exactly one real </atlas>, one real
        // </symbol>, and the fake element stayed inert text.
        assert_eq!(xml.matches("</atlas>").count(), 1);
        assert_eq!(xml.matches("</symbol>").count(), 1);
        assert!(!xml.contains("<inject/>"));
        assert!(xml.contains("&lt;inject/&gt;"));
        // Attribute special chars are entity-escaped (illegal char dropped).
        assert!(xml.contains("path=\"src/a&lt;b&gt;&amp;&quot;c.py\""));
        assert!(xml.contains("name=\"ev&lt;il&gt;&amp;&quot;\""));
    }

    #[test]
    fn is_deterministic() {
        assert_eq!(render(&sample_map()), render(&sample_map()));
    }

    #[test]
    fn empty_collections_render_as_empty_elements() {
        let mut map = sample_map();
        map.files.clear();
        map.collapsed.clear();
        map.symbol_index.clear();
        let xml = render(&map);
        assert!(xml.contains("<files/>"));
        assert!(xml.contains("<collapsed/>"));
        assert!(xml.contains("<symbol_index/>"));
    }
}

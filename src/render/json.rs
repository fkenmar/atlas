//! JSON renderer — stable, versioned schema for programmatic consumers
//! (PRD §7.3). A *breaking* shape change — removing, renaming, or retyping an
//! existing field — bumps `SCHEMA_VERSION` and, pre-1.0, the crate minor
//! version (see the release-process skill). Purely *additive* keys (e.g.
//! `symbol_index`, ADR 0004) don't: consumers ignore unknown keys, so the
//! version still means "every field you knew about is still here, unchanged".
//!
//! Serialized by hand (no serde dependency yet — added with the MCP server in
//! M2): the schema is small and fixed, and hand-rolling keeps the dependency
//! surface minimal while still escaping strings per the JSON spec. Output is
//! deterministic — files/symbols arrive already ordered from the budget stage
//! (NFR-4), and `f64` scores use a fixed-precision format.

use std::fmt::Write as _;

use crate::budget::{BudgetedFile, BudgetedMap, Detail, RenderedSymbol};
use crate::parse::{SymbolKind, Visibility};

/// Version stamped into every JSON output as `"version"`.
pub const SCHEMA_VERSION: u32 = 1;

pub struct JsonRenderer;

impl super::Renderer for JsonRenderer {
    fn render(&self, map: &BudgetedMap) -> String {
        render(map)
    }
}

/// Render a budgeted map as JSON (schema version [`SCHEMA_VERSION`]).
pub fn render(map: &BudgetedMap) -> String {
    let files: Vec<String> = map.files.iter().map(render_file).collect();
    let collapsed: Vec<String> = map
        .collapsed
        .iter()
        .map(|c| format!("{{\"dir\": {}, \"count\": {}}}", json_str(&c.dir), c.count))
        .collect();
    let symbol_index: Vec<String> = map
        .symbol_index
        .iter()
        .map(|s| {
            format!(
                "{{\"name\": {}, \"kind\": {}, \"path\": {}, \"anchor\": {}, \"line\": {}}}",
                json_str(&s.name),
                json_str(kind_name(s.kind)),
                json_str(&s.rel),
                json_str(&s.anchor),
                s.line
            )
        })
        .collect();

    let mut out = String::new();
    out.push_str("{\n");
    let _ = writeln!(out, "  \"version\": {SCHEMA_VERSION},");
    let _ = writeln!(
        out,
        "  \"repo\": {{\"name\": {}, \"loc\": {}, \"files\": {}}},",
        json_str(&map.repo_name),
        map.total_loc,
        map.total_files
    );
    let _ = writeln!(
        out,
        "  \"budget\": {{\"target\": {}, \"rendered\": {}, \"detail\": {}}},",
        map.target_tokens,
        map.rendered_tokens,
        json_str(detail_name(map.detail))
    );
    let _ = writeln!(out, "  \"files\": [{}],", join_indented(&files, 4));
    let _ = writeln!(out, "  \"collapsed\": [{}],", join_indented(&collapsed, 4));
    let _ = writeln!(
        out,
        "  \"symbol_index\": [{}],",
        join_indented(&symbol_index, 4)
    );
    let _ = writeln!(
        out,
        "  \"skipped_files\": {}, \"unwired_files\": {}",
        map.skipped_files, map.unwired_files
    );
    out.push_str("}\n");
    out
}

fn render_file(file: &BudgetedFile) -> String {
    let symbols: Vec<String> = file.symbols.iter().map(render_symbol).collect();
    let imports: Vec<String> = file.imports.iter().map(|i| json_str(i)).collect();
    let used_by: Vec<String> = file.used_by.iter().map(|i| json_str(i)).collect();
    format!(
        "{{\"path\": {}, \"lang\": {}, \"rank\": {}, \"score\": {:.6}, \"imported_by\": {}, \"one_line\": {}, \"omitted\": {}, \"symbols\": [{}], \"imports\": [{}], \"used_by\": [{}]}}",
        json_str(&file.rel),
        json_str(file.lang),
        file.rank,
        file.score,
        file.imported_by,
        file.one_line,
        file.omitted,
        symbols.join(", "),
        imports.join(", "),
        used_by.join(", "),
    )
}

fn render_symbol(s: &RenderedSymbol) -> String {
    format!(
        "{{\"kind\": {}, \"name\": {}, \"sig\": {}, \"line\": {}, \"visibility\": {}}}",
        json_str(kind_name(s.kind)),
        json_str(&s.name),
        json_str(&s.signature),
        s.line,
        json_str(visibility_name(s.visibility)),
    )
}

pub(crate) fn kind_name(kind: SymbolKind) -> &'static str {
    match kind {
        SymbolKind::Function => "function",
        SymbolKind::Method => "method",
        SymbolKind::Class => "class",
        SymbolKind::Interface => "interface",
        SymbolKind::Enum => "enum",
        SymbolKind::TypeAlias => "type",
        SymbolKind::Constant => "constant",
        SymbolKind::Module => "module",
        SymbolKind::Field => "field",
    }
}

pub(crate) fn visibility_name(v: Visibility) -> &'static str {
    match v {
        Visibility::Public => "public",
        Visibility::Private => "private",
    }
}

pub(crate) fn detail_name(d: Detail) -> &'static str {
    match d {
        Detail::Full => "full",
        Detail::NoPrivate => "public-only",
        Detail::NoParams => "public-only-params-elided",
    }
}

/// Join JSON array items onto their own indented lines, or render `[]` empty.
fn join_indented(items: &[String], indent: usize) -> String {
    if items.is_empty() {
        return String::new();
    }
    let pad = " ".repeat(indent);
    let mut out = String::from("\n");
    out.push_str(
        &items
            .iter()
            .map(|item| format!("{pad}{item}"))
            .collect::<Vec<_>>()
            .join(",\n"),
    );
    out.push_str("\n  ");
    out
}

/// Escape a string as a JSON string literal (RFC 8259).
pub(crate) fn json_str(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => {
                let _ = write!(out, "\\u{:04x}", c as u32);
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::budget::{CollapsedDir, IndexedSymbol};

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
                anchor: "src/widget.py#Widget".to_string(),
                line: 7,
            }],
            skipped_files: 1,
            unwired_files: 2,
        }
    }

    #[test]
    fn schema_shape_is_stable() {
        let json = render(&sample_map());
        // Spot-check the documented schema keys and values.
        assert!(json.contains("\"version\": 1"));
        assert!(json.contains("\"repo\": {\"name\": \"demo\", \"loc\": 5000, \"files\": 12}"));
        assert!(json.contains("\"target\": 2048, \"rendered\": 120"));
        assert!(json.contains("\"path\": \"src/auth.py\", \"lang\": \"python\""));
        assert!(json.contains("\"kind\": \"method\", \"name\": \"login\""));
        assert!(json.contains("\"line\": 12, \"visibility\": \"public\""));
        assert!(json.contains("\"dir\": \"tests\", \"count\": 4"));
        assert!(
            json.contains("\"name\": \"Widget\", \"kind\": \"class\", \"path\": \"src/widget.py\", \"anchor\": \"src/widget.py#Widget\", \"line\": 7")
        );
        assert!(json.contains("\"skipped_files\": 1, \"unwired_files\": 2"));
    }

    #[test]
    fn strings_are_escaped() {
        assert_eq!(json_str("a\"b\\c\n"), "\"a\\\"b\\\\c\\n\"");
    }

    #[test]
    fn is_deterministic() {
        assert_eq!(render(&sample_map()), render(&sample_map()));
    }

    #[test]
    fn empty_collections_render_as_empty_arrays() {
        let mut map = sample_map();
        map.files.clear();
        map.collapsed.clear();
        map.symbol_index.clear();
        let json = render(&map);
        assert!(json.contains("\"files\": [],"));
        assert!(json.contains("\"collapsed\": [],"));
        assert!(json.contains("\"symbol_index\": [],"));
    }
}

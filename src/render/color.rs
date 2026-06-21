//! ANSI colorization of the rendered Markdown map — a *display-only* layer.
//!
//! Applied by the CLI only when stdout is a terminal (and `--color` allows it);
//! the map artifact piped into a file or an agent's context stays plain. This
//! is deliberately decoupled from [`super::markdown::render`]: the budget stage
//! counts tokens by rendering the *plain* map, so colorizing there would inflate
//! the token count and break determinism (NFR-4). Color is a pure transform of
//! the already-rendered text, so the bytes that matter — the piped map — are
//! untouched.

const RESET: &str = "\x1b[0m";
const BOLD: &str = "\x1b[1m";
const DIM: &str = "\x1b[2m";
const CYAN: &str = "\x1b[36m";
const BLUE: &str = "\x1b[34m";
const MAGENTA: &str = "\x1b[35m";

/// Leading declaration keywords worth tinting in a signature line. Only a
/// *leading* run is colored (never a substring), so a keyword appearing inside
/// a type name is left alone.
const KEYWORDS: &[&str] = &[
    "pub",
    "fn",
    "def",
    "class",
    "struct",
    "impl",
    "trait",
    "enum",
    "interface",
    "type",
    "const",
    "static",
    "let",
    "async",
    "mod",
    "function",
    "export",
    "default",
    "abstract",
    "public",
    "private",
    "protected",
    "var",
    "declare",
];

/// Colorize the plain Markdown map for terminal display. Operates line by line,
/// classifying each by the renderer's known shapes (header, file heading,
/// signature, imports/used-by, symbol index, footers). Unrecognized lines pass
/// through untouched, so this degrades gracefully if the renderer changes.
pub fn colorize(plain: &str) -> String {
    let mut out = String::with_capacity(plain.len() + plain.len() / 4);
    // The symbol index is a run of `path: A, B` lines after its header; track it
    // so those aren't mistaken for signatures.
    let mut in_index = false;
    for line in plain.split_inclusive('\n') {
        let (body, nl) = match line.strip_suffix('\n') {
            Some(b) => (b, "\n"),
            None => (line, ""),
        };
        if body.is_empty() {
            in_index = false;
            out.push_str(nl);
            continue;
        }
        if body.starts_with("# atlas:") {
            // Top header: bold cyan, with the degradation note (after " | ...")
            // left in the same line — kept simple, the whole line reads as the
            // banner.
            push_styled(&mut out, &format!("{BOLD}{CYAN}"), body, nl);
        } else if let Some(rest) = body.strip_prefix("## ") {
            // File heading: `## path (#rank — ...)`. Path bold blue, the
            // parenthetical rank/metadata dimmed.
            out.push_str("## ");
            match rest.split_once(" (") {
                Some((path, meta)) => {
                    out.push_str(&format!("{BOLD}{BLUE}{path}{RESET} {DIM}({meta}{RESET}"));
                }
                None => out.push_str(&format!("{BOLD}{BLUE}{rest}{RESET}")),
            }
            out.push_str(nl);
        } else if body == "---" {
            push_styled(&mut out, DIM, body, nl);
        } else if body.starts_with("symbol index (") {
            in_index = true;
            push_styled(&mut out, DIM, body, nl);
        } else if in_index {
            // `path: TypeA, TypeB` — tint the path, dim the names.
            match body.split_once(": ") {
                Some((path, names)) => {
                    out.push_str(&format!("{BLUE}{path}{RESET}: {DIM}{names}{RESET}"));
                }
                None => out.push_str(body),
            }
            out.push_str(nl);
        } else if body.starts_with("imports:")
            || body.starts_with("used by:")
            || body.starts_with("… (")
            || body.starts_with('[')
        {
            // Dependency lines, the "N more" elision, and the collapsed/skipped
            // footers — supporting detail, dimmed.
            push_styled(&mut out, DIM, body, nl);
        } else {
            // A signature line: tint a leading run of declaration keywords.
            out.push_str(&color_signature(body));
            out.push_str(nl);
        }
    }
    out
}

/// Wrap `body` in `style … RESET`, preserving the trailing newline.
fn push_styled(out: &mut String, style: &str, body: &str, nl: &str) {
    out.push_str(style);
    out.push_str(body);
    out.push_str(RESET);
    out.push_str(nl);
}

/// Tint the leading whitespace-preserving run of declaration keywords magenta;
/// leave the symbol name and the rest of the signature at the default color.
fn color_signature(body: &str) -> String {
    let indent_len = body.len() - body.trim_start().len();
    let (indent, rest) = body.split_at(indent_len);
    let mut kw_end = 0;
    for tok in rest.split_inclusive(' ') {
        let word = tok.trim_end();
        if KEYWORDS.contains(&word) {
            kw_end += tok.len();
        } else {
            break;
        }
    }
    if kw_end == 0 {
        return body.to_string();
    }
    let (kw, tail) = rest.split_at(kw_end);
    // Trailing space (if any) stays outside the color span to keep it tidy.
    let kw_trimmed = kw.trim_end();
    let space = &kw[kw_trimmed.len()..];
    format!("{indent}{MAGENTA}{kw_trimmed}{RESET}{space}{tail}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_passthrough_when_no_known_shapes() {
        let s = "just some text\nmore text\n";
        // Text with no leading keywords or markers is unchanged.
        assert_eq!(colorize(s), s);
    }

    #[test]
    fn header_and_file_heading_are_styled() {
        let plain = "# atlas: demo (10 LOC, 2 files) | budget 600 | rendered 590 tok\n\n## cache.rs (#1 — imported by 1 file(s))\n";
        let out = colorize(plain);
        assert!(out.contains(CYAN), "header tinted");
        assert!(out.contains(BLUE), "file path tinted");
        // The plain text is still present between the escape codes.
        assert!(out.contains("cache.rs"));
        assert!(out.contains("# atlas: demo"));
    }

    #[test]
    fn signature_leading_keywords_tinted_not_substrings() {
        // `pub fn` tinted; the name and a keyword-like substring inside a type
        // are NOT.
        let out = colorize("pub fn open(&Path) -> Cache\n");
        assert!(out.contains(MAGENTA));
        let idx_kw = out.find("pub").unwrap();
        let idx_name = out.find("open").unwrap();
        let idx_reset = out.find(RESET).unwrap();
        // RESET falls between the keyword run and the name.
        assert!(idx_kw < idx_reset && idx_reset < idx_name);
    }

    #[test]
    fn indented_method_keeps_indent_before_color() {
        let out = colorize("    pub fn get(&mut self)\n");
        assert!(
            out.starts_with("    "),
            "indent preserved before the color code"
        );
    }

    #[test]
    fn symbol_index_entries_styled_signatures_not() {
        let plain = "symbol index (other defined symbols — anchors only):\n_pytest/runner.py: _pytest/runner.py#CallInfo, _pytest/runner.py#TestReport\n";
        let out = colorize(plain);
        assert!(out.contains(BLUE), "index path tinted");
        // The anchors are present.
        assert!(out.contains("_pytest/runner.py#CallInfo"));
    }

    #[test]
    fn blank_line_resets_index_state() {
        // After the index ends (blank line), a `path:`-looking line is NOT
        // treated as an index entry. Here a normal signature with a colon-ish
        // shape still colorizes as a signature (leading `const`).
        let plain = "symbol index (x):\na.py: A\n\nconst X: u32\n";
        let out = colorize(plain);
        assert!(out.contains(&format!("{MAGENTA}const")));
    }

    #[test]
    fn is_deterministic() {
        let plain = "# atlas: d (1 LOC, 1 files) | budget 9 | rendered 9 tok\n\n## a.rs (#1)\npub struct A\n";
        assert_eq!(colorize(plain), colorize(plain));
    }
}

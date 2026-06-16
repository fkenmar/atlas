//! Language registry: maps file extensions to tree-sitter grammars and
//! their embedded extraction queries (`queries/<lang>/tags.scm`).
//!
//! Tier 1 (FR-1): TypeScript/JavaScript, Python, Rust. Python is wired (M0);
//! TS/JS and Rust grammars land in M1.
//! Tier 2 (FR-2, M3): Go, Java, C/C++, OCaml — added via /new-grammar.

pub mod python;
pub mod rust_lang;
pub mod typescript;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Language {
    Python,
    TypeScript,
    Rust,
}

impl Language {
    /// Detect a Tier 1 language from a file extension; `None` means the
    /// file is not mapped.
    pub fn from_extension(ext: &str) -> Option<Language> {
        match ext {
            "py" | "pyi" => Some(Language::Python),
            "ts" | "tsx" | "mts" | "cts" | "js" | "jsx" | "mjs" | "cjs" => {
                Some(Language::TypeScript)
            }
            "rs" => Some(Language::Rust),
            _ => None,
        }
    }

    /// The tree-sitter grammar for this language, if it is wired yet.
    /// `None` languages are discovered and counted but not parsed (their
    /// grammar crates land in M1).
    pub fn grammar(&self) -> Option<tree_sitter::Language> {
        match self {
            Language::Python => Some(tree_sitter::Language::new(tree_sitter_python::LANGUAGE)),
            Language::TypeScript => Some(tree_sitter::Language::new(
                tree_sitter_typescript::LANGUAGE_TYPESCRIPT,
            )),
            Language::Rust => Some(tree_sitter::Language::new(tree_sitter_rust::LANGUAGE)),
        }
    }

    /// Lowercase display name (also used in the JSON schema's `lang` field).
    pub fn name(&self) -> &'static str {
        match self {
            Language::Python => "python",
            Language::TypeScript => "typescript",
            Language::Rust => "rust",
        }
    }

    /// The tags.scm query source for this language, embedded at compile
    /// time so the binary stays self-contained (PRD §7.2).
    pub fn tags_query(&self) -> &'static str {
        match self {
            Language::Python => python::TAGS_QUERY,
            Language::TypeScript => typescript::TAGS_QUERY,
            Language::Rust => rust_lang::TAGS_QUERY,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Language;

    #[test]
    fn maps_tier1_extensions() {
        assert_eq!(Language::from_extension("py"), Some(Language::Python));
        assert_eq!(Language::from_extension("tsx"), Some(Language::TypeScript));
        assert_eq!(Language::from_extension("mjs"), Some(Language::TypeScript));
        assert_eq!(Language::from_extension("rs"), Some(Language::Rust));
        assert_eq!(Language::from_extension("ml"), None); // Tier 2, M3
        assert_eq!(Language::from_extension("md"), None);
    }

    #[test]
    fn python_grammar_is_wired() {
        assert!(Language::Python.grammar().is_some());
    }
}

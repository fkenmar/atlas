//! Language registry: maps file extensions to tree-sitter grammars and
//! their embedded extraction queries (`queries/<lang>/tags.scm`).
//!
//! Tier 1 (FR-1): TypeScript/JavaScript, Python, Rust — all wired.
//! Tier 2 (FR-2, M3): Go, Java, C, C++ — wired (#10); OCaml remains, added via
//! /new-grammar.

pub mod c;
pub mod cpp;
pub mod go;
pub mod java;
pub mod python;
pub mod rust_lang;
pub mod typescript;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Language {
    Python,
    TypeScript,
    Rust,
    Go,
    Java,
    C,
    Cpp,
}

impl Language {
    /// Detect a supported language from a file extension; `None` means the
    /// file is not mapped.
    pub fn from_extension(ext: &str) -> Option<Language> {
        match ext {
            "py" | "pyi" => Some(Language::Python),
            "ts" | "tsx" | "mts" | "cts" | "js" | "jsx" | "mjs" | "cjs" => {
                Some(Language::TypeScript)
            }
            "rs" => Some(Language::Rust),
            "go" => Some(Language::Go),
            "java" => Some(Language::Java),
            // C headers (.h) are mapped to C; in mixed C/C++ trees a `.h` is
            // ambiguous, but the C grammar parses the common subset of both.
            "c" | "h" => Some(Language::C),
            "cc" | "cpp" | "cxx" | "hpp" | "hh" => Some(Language::Cpp),
            _ => None,
        }
    }

    /// The tree-sitter grammar for this language. Every variant is wired, so
    /// this is always `Some`; the `Option` is kept for forward compatibility
    /// with languages discovered before their grammar is added.
    pub fn grammar(&self) -> Option<tree_sitter::Language> {
        match self {
            Language::Python => Some(tree_sitter::Language::new(tree_sitter_python::LANGUAGE)),
            Language::TypeScript => Some(tree_sitter::Language::new(
                tree_sitter_typescript::LANGUAGE_TYPESCRIPT,
            )),
            Language::Rust => Some(tree_sitter::Language::new(tree_sitter_rust::LANGUAGE)),
            Language::Go => Some(tree_sitter::Language::new(tree_sitter_go::LANGUAGE)),
            Language::Java => Some(tree_sitter::Language::new(tree_sitter_java::LANGUAGE)),
            Language::C => Some(tree_sitter::Language::new(tree_sitter_c::LANGUAGE)),
            Language::Cpp => Some(tree_sitter::Language::new(tree_sitter_cpp::LANGUAGE)),
        }
    }

    /// Lowercase display name (also used in the JSON schema's `lang` field).
    pub fn name(&self) -> &'static str {
        match self {
            Language::Python => "python",
            Language::TypeScript => "typescript",
            Language::Rust => "rust",
            Language::Go => "go",
            Language::Java => "java",
            Language::C => "c",
            Language::Cpp => "cpp",
        }
    }

    /// The tags.scm query source for this language, embedded at compile
    /// time so the binary stays self-contained (PRD §7.2).
    pub fn tags_query(&self) -> &'static str {
        match self {
            Language::Python => python::TAGS_QUERY,
            Language::TypeScript => typescript::TAGS_QUERY,
            Language::Rust => rust_lang::TAGS_QUERY,
            Language::Go => go::TAGS_QUERY,
            Language::Java => java::TAGS_QUERY,
            Language::C => c::TAGS_QUERY,
            Language::Cpp => cpp::TAGS_QUERY,
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

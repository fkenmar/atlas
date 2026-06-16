//! Stage 1 — discover: walk the repository tree and emit the source files
//! to parse, in a deterministic (sorted) order.
//!
//! M0 scope: built-in vendored-path defaults and hidden-directory skipping.
//! Full .gitignore/.atlasignore handling (FR-7) lands in M1 with the rest
//! of the core. Symlinks are never followed (cycle safety).

use std::path::{Path, PathBuf};

use crate::lang::Language;

/// Directory names that are never descended into, regardless of ignore
/// files (PRD §5.1: vendored/binary-path heuristics).
const VENDORED_DIRS: &[&str] = &[
    "node_modules",
    "target",
    "dist",
    "build",
    "out",
    "venv",
    "env",
    "__pycache__",
    "site-packages",
    "vendor",
    "third_party",
    "coverage",
];

/// A source file selected for parsing.
#[derive(Clone, Debug)]
pub struct SourceFile {
    /// Absolute path, used for reading.
    pub path: PathBuf,
    /// Root-relative path with `/` separators, used for display and sorting.
    pub rel: String,
    pub lang: Language,
}

/// Walk `root` and return every Tier 1 source file, sorted by relative path
/// (NFR-4: deterministic output starts with deterministic input order).
/// Hidden directories (`.git`, `.venv`, …) and [`VENDORED_DIRS`] are
/// skipped; unreadable directories are skipped silently in M0.
pub fn discover(root: &Path) -> Vec<SourceFile> {
    let ignore = IgnoreRules::load(root);
    let mut out = Vec::new();
    let mut stack = vec![root.to_path_buf()];

    while let Some(dir) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let Ok(file_type) = entry.file_type() else {
                continue;
            };
            if file_type.is_symlink() {
                continue;
            }
            let path = entry.path();
            let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
                continue;
            };
            let rel = path
                .strip_prefix(root)
                .unwrap_or(&path)
                .to_string_lossy()
                .replace(std::path::MAIN_SEPARATOR, "/");

            if file_type.is_dir() {
                if name.starts_with('.')
                    || VENDORED_DIRS.contains(&name)
                    || ignore.ignored(&rel, true)
                {
                    continue;
                }
                stack.push(path);
            } else if file_type.is_file() && !name.starts_with('.') {
                let Some(lang) = path
                    .extension()
                    .and_then(|e| e.to_str())
                    .and_then(Language::from_extension)
                else {
                    continue;
                };
                if ignore.ignored(&rel, false) {
                    continue;
                }
                out.push(SourceFile { path, rel, lang });
            }
        }
    }

    out.sort_by(|a, b| a.rel.cmp(&b.rel));
    out
}

/// Minimal `.gitignore` / `.atlasignore` matcher (FR-7). Supports the common
/// forms: comments, blank lines, `dir/` (directory-only), `*` segment globs,
/// basename patterns (no `/`, match any path component) and root-relative path
/// patterns (containing `/`). Not handled in v1: negation (`!`), `**`, and
/// nested ignore files below the root — the built-in [`VENDORED_DIRS`] still
/// apply regardless. Only the root-level ignore files are read.
struct IgnoreRules {
    rules: Vec<Rule>,
}

struct Rule {
    /// Pattern with any leading/trailing `/` stripped.
    pattern: String,
    /// Source line ended in `/` — matches directories only.
    dir_only: bool,
    /// Pattern contained `/` — matched against the path, not a bare basename.
    anchored: bool,
}

impl IgnoreRules {
    fn load(root: &Path) -> IgnoreRules {
        let mut rules = Vec::new();
        for name in [".gitignore", ".atlasignore"] {
            let Ok(content) = std::fs::read_to_string(root.join(name)) else {
                continue;
            };
            for raw in content.lines() {
                let line = raw.trim();
                if line.is_empty() || line.starts_with('#') || line.starts_with('!') {
                    continue; // comment / blank / unsupported negation
                }
                let dir_only = line.ends_with('/');
                let trimmed = line.trim_end_matches('/');
                let anchored = trimmed.contains('/');
                let pattern = trimmed.trim_start_matches('/').to_string();
                if !pattern.is_empty() {
                    rules.push(Rule {
                        pattern,
                        dir_only,
                        anchored,
                    });
                }
            }
        }
        IgnoreRules { rules }
    }

    fn ignored(&self, rel: &str, is_dir: bool) -> bool {
        self.rules.iter().any(|r| {
            if r.dir_only && !is_dir {
                return false;
            }
            if r.anchored {
                anchored_match(&r.pattern, rel)
            } else {
                rel.split('/').any(|seg| glob_match(&r.pattern, seg))
            }
        })
    }
}

/// Path match: the pattern's `/`-segments must glob-match a leading run of the
/// path's segments, so `src/gen` ignores `src/gen` and everything under it.
fn anchored_match(pattern: &str, rel: &str) -> bool {
    let pats: Vec<&str> = pattern.split('/').collect();
    let segs: Vec<&str> = rel.split('/').collect();
    pats.len() <= segs.len() && pats.iter().zip(&segs).all(|(p, s)| glob_match(p, s))
}

/// Glob-match one path segment: `*` matches any run of characters (a path
/// separator never appears within a segment). No `?` / `[...]` in v1.
fn glob_match(pattern: &str, text: &str) -> bool {
    if !pattern.contains('*') {
        return pattern == text;
    }
    let parts: Vec<&str> = pattern.split('*').collect();
    let mut pos = 0;
    for (i, part) in parts.iter().enumerate() {
        if part.is_empty() {
            continue;
        }
        let rest = &text[pos..];
        if i == 0 {
            if !rest.starts_with(part) {
                return false;
            }
            pos += part.len();
        } else if i == parts.len() - 1 {
            return rest.ends_with(part);
        } else {
            match rest.find(part) {
                Some(idx) => pos += idx + part.len(),
                None => return false,
            }
        }
    }
    true
}

#[cfg(test)]
mod tests {
    // The walk itself is covered by the integration test in
    // tests/discover_walk.rs against the committed fixture tree.
    use super::{anchored_match, glob_match, IgnoreRules, Rule};

    fn rule(pattern: &str, dir_only: bool, anchored: bool) -> Rule {
        Rule {
            pattern: pattern.to_string(),
            dir_only,
            anchored,
        }
    }

    #[test]
    fn glob_segment_matching() {
        assert!(glob_match("exact", "exact"));
        assert!(!glob_match("exact", "other"));
        assert!(glob_match("*.pyc", "foo.pyc"));
        assert!(!glob_match("*.pyc", "foo.py"));
        assert!(glob_match("test_*", "test_thing"));
        assert!(!glob_match("test_*", "thing_test"));
        assert!(glob_match("*", "anything"));
        assert!(glob_match("a*c", "abc"));
        assert!(glob_match("a*c", "ac"));
        assert!(!glob_match("a*c", "abx"));
    }

    #[test]
    fn anchored_path_matching() {
        assert!(anchored_match("src/generated", "src/generated"));
        // Everything under an ignored dir is ignored too.
        assert!(anchored_match("src/generated", "src/generated/mod.rs"));
        assert!(!anchored_match("src/generated", "src/other"));
        // Globs work per segment.
        assert!(anchored_match("build/*.o", "build/x.o"));
        assert!(!anchored_match("a/b/c", "a/b")); // pattern longer than path
    }

    #[test]
    fn ignore_rules_dispatch() {
        let rules = IgnoreRules {
            rules: vec![
                rule("node_modules", false, false), // basename, anywhere
                rule("*.pyc", false, false),
                rule("src/generated", false, true), // anchored path
                rule("build", true, false),         // directory-only basename
            ],
        };
        assert!(rules.ignored("pkg/node_modules/x.js", false));
        assert!(rules.ignored("a/b/foo.pyc", false));
        assert!(!rules.ignored("a/b/foo.py", false));
        assert!(rules.ignored("src/generated", true));
        assert!(rules.ignored("src/generated/code.rs", false));
        assert!(rules.ignored("build", true)); // dir matches dir-only rule
        assert!(!rules.ignored("build", false)); // file does not
        assert!(!rules.ignored("src/main.rs", false));
    }
}

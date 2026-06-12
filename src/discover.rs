//! Stage 1 — discover: walk the repository tree and emit the source files
//! to parse, in a deterministic (sorted) order.
//!
//! M0 scope: built-in vendored-path defaults and hidden-directory skipping.
//! Full .gitignore/.repomapignore handling (FR-7) lands in M1 with the rest
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
            if file_type.is_dir() {
                if name.starts_with('.') || VENDORED_DIRS.contains(&name) {
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
                let rel = path
                    .strip_prefix(root)
                    .unwrap_or(&path)
                    .to_string_lossy()
                    .replace(std::path::MAIN_SEPARATOR, "/");
                out.push(SourceFile { path, rel, lang });
            }
        }
    }

    out.sort_by(|a, b| a.rel.cmp(&b.rel));
    out
}

#[cfg(test)]
mod tests {
    // The walk itself is covered by the integration test in
    // tests/discover_walk.rs against the committed fixture tree.
}

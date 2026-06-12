//! Incremental cache (FR-6): parse results keyed on (file path, content
//! hash, grammar version), serialized with bincode into `.repomap/cache`.
//! Only changed files re-parse; a grammar crate version bump invalidates
//! everything. The cache is always safe to delete.

use std::path::Path;

pub struct Cache;

impl Cache {
    /// Open (or create) the cache under `<repo_root>/.repomap/cache`.
    pub fn open(_repo_root: &Path) -> Cache {
        todo!("M1: bincode-backed cache keyed on (path, content hash, grammar version)")
    }
}

#[cfg(test)]
mod tests {
    // Hit/miss and invalidation tests land with the M1 implementation.
}

//! Incremental cache (FR-6): parse results keyed on (file path, content
//! hash, cache version), serialized with bincode into `.repomap/cache`.
//! Only changed files re-parse; a cache-version bump invalidates everything.
//! The cache is purely an optimization and is always safe to delete: every
//! I/O or decode failure degrades silently to a cold parse, never an error.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::parse::ParsedFile;

/// Bumped whenever extraction output or the grammar set changes, so a stale
/// cache from an older repomap is discarded wholesale rather than returning
/// out-of-date symbols. (Tie to grammar crate versions once they're surfaced.)
const CACHE_VERSION: u32 = 1;

#[derive(bincode::Encode, bincode::Decode)]
struct CacheEntry {
    content_hash: u64,
    parsed: ParsedFile,
}

#[derive(bincode::Encode, bincode::Decode)]
struct CacheFile {
    version: u32,
    entries: BTreeMap<String, CacheEntry>,
}

/// Content-hash-keyed parse cache under `<repo_root>/.repomap/cache`.
pub struct Cache {
    path: PathBuf,
    /// Entries loaded from disk (read for hits).
    loaded: BTreeMap<String, CacheEntry>,
    /// Entries to persist — every file touched this run, so deleted files are
    /// pruned on save.
    fresh: BTreeMap<String, CacheEntry>,
    enabled: bool,
}

impl Cache {
    /// Open (or start fresh) the cache under `<repo_root>/.repomap/cache`.
    pub fn open(repo_root: &Path) -> Cache {
        let path = repo_root.join(".repomap").join("cache");
        let loaded = load(&path).unwrap_or_default();
        Cache {
            path,
            loaded,
            fresh: BTreeMap::new(),
            enabled: true,
        }
    }

    /// A no-op cache: every lookup misses, nothing is written. Lets the
    /// uncached parse path share one code path with the cached one.
    pub fn disabled() -> Cache {
        Cache {
            path: PathBuf::new(),
            loaded: BTreeMap::new(),
            fresh: BTreeMap::new(),
            enabled: false,
        }
    }

    /// Return the cached parse for `rel` if present with a matching content
    /// hash, carrying it into the fresh set so it survives the next save.
    pub fn get(&mut self, rel: &str, content_hash: u64) -> Option<ParsedFile> {
        if !self.enabled {
            return None;
        }
        let entry = self.loaded.get(rel)?;
        if entry.content_hash != content_hash {
            return None;
        }
        let parsed = entry.parsed.clone();
        self.fresh.insert(
            rel.to_string(),
            CacheEntry {
                content_hash,
                parsed: parsed.clone(),
            },
        );
        Some(parsed)
    }

    /// Record a freshly-parsed file for persistence.
    pub fn insert(&mut self, rel: &str, content_hash: u64, parsed: &ParsedFile) {
        if !self.enabled {
            return;
        }
        self.fresh.insert(
            rel.to_string(),
            CacheEntry {
                content_hash,
                parsed: parsed.clone(),
            },
        );
    }

    /// Persist the fresh entries. Best-effort — any failure is ignored.
    pub fn save(self) {
        if !self.enabled {
            return;
        }
        let file = CacheFile {
            version: CACHE_VERSION,
            entries: self.fresh,
        };
        let Ok(bytes) = bincode::encode_to_vec(&file, bincode::config::standard()) else {
            return;
        };
        if let Some(parent) = self.path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let _ = std::fs::write(&self.path, bytes);
    }
}

/// Hash file content for the cache key. `DefaultHasher` has a fixed seed, so
/// this is stable across runs and platforms (NFR-4-friendly).
pub fn content_hash(content: &str) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    content.hash(&mut hasher);
    hasher.finish()
}

/// Load and version-check the cache file; `None` (→ cold parse) on any miss.
fn load(path: &Path) -> Option<BTreeMap<String, CacheEntry>> {
    let bytes = std::fs::read(path).ok()?;
    let (file, _): (CacheFile, usize) =
        bincode::decode_from_slice(&bytes, bincode::config::standard()).ok()?;
    if file.version != CACHE_VERSION {
        return None; // stale → discard wholesale
    }
    Some(file.entries)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse::ParsedFile;

    fn parsed(marker: &str) -> ParsedFile {
        ParsedFile {
            symbols: Vec::new(),
            imports: vec![marker.to_string()],
            references: Vec::new(),
            lines: 1,
        }
    }

    #[test]
    fn content_hash_is_stable_and_distinguishing() {
        assert_eq!(content_hash("abc"), content_hash("abc"));
        assert_ne!(content_hash("abc"), content_hash("abd"));
    }

    #[test]
    fn disabled_cache_always_misses() {
        let mut c = Cache::disabled();
        c.insert("a.py", 1, &parsed("x"));
        assert!(c.get("a.py", 1).is_none());
    }

    #[test]
    fn round_trips_through_disk() {
        let dir = std::env::temp_dir().join(format!("repomap-cache-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();

        // First run: miss, insert, save.
        {
            let mut c = Cache::open(&dir);
            assert!(c.get("a.py", 42).is_none());
            c.insert("a.py", 42, &parsed("hello"));
            c.save();
        }
        // Second run: hit with the same hash.
        {
            let mut c = Cache::open(&dir);
            let hit = c.get("a.py", 42).expect("cached entry should load");
            assert_eq!(hit.imports, vec!["hello".to_string()]);
            // A changed hash misses.
            assert!(c.get("a.py", 99).is_none());
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn save_prunes_files_not_seen_this_run() {
        let dir = std::env::temp_dir().join(format!("repomap-cache-prune-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();

        {
            let mut c = Cache::open(&dir);
            c.insert("a.py", 1, &parsed("a"));
            c.insert("b.py", 2, &parsed("b"));
            c.save();
        }
        // Second run only touches a.py → b.py is pruned.
        {
            let mut c = Cache::open(&dir);
            assert!(c.get("a.py", 1).is_some());
            c.save();
        }
        {
            let mut c = Cache::open(&dir);
            assert!(c.get("a.py", 1).is_some());
            assert!(
                c.get("b.py", 2).is_none(),
                "unseen file should have been pruned"
            );
        }
        let _ = std::fs::remove_dir_all(&dir);
    }
}

//! Structural diff between two parsed trees (ADR 0005). Reuses the parse
//! output of both trees and never ranks or budgets, so every changed
//! declaration is reported. Output is deterministic (NFR-4): all collections
//! are built from BTreeMap/BTreeSet, so they arrive sorted.

use std::collections::{BTreeMap, BTreeSet};

use crate::discover::SourceFile;
use crate::parse::{ParsedFile, Symbol, Visibility};
use crate::render::json::kind_name;

/// Options controlling what the diff considers.
pub struct DiffOptions {
    /// Drop private symbols from both sides before comparing.
    pub no_private: bool,
}

/// A structural delta between two trees: file-level adds/removes and, for files
/// present in both, the symbol- and import-edge-level changes.
#[derive(Debug, PartialEq, Eq)]
pub struct StructuralDiff {
    pub added_files: Vec<FileSummary>,
    pub removed_files: Vec<FileSummary>,
    pub changed_files: Vec<FileDelta>,
    /// Files detected as renamed/moved — a removed and an added file with an
    /// identical, uniquely-matched symbol set (#106). Conservative: any
    /// ambiguity keeps them as separate add/remove.
    pub moved_files: Vec<MovedFile>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct MovedFile {
    pub old_rel: String,
    pub new_rel: String,
    pub lang: &'static str,
    pub symbol_count: usize,
}

#[derive(Debug, PartialEq, Eq)]
pub struct FileSummary {
    pub rel: String,
    pub lang: &'static str,
    pub symbol_count: usize,
    /// How many of `symbol_count` are public — drives file-level severity
    /// (a public-bearing file add/remove is more significant). (#107)
    pub public_count: usize,
}

/// Heuristic significance of a structural change (#107). Not a type-checker
/// guarantee — a conservative prioritization aid for reviewers and CI.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    /// Public surface removed or its signature changed — likely API-breaking.
    Breaking,
    /// Public surface added or reclassified — worth a look, not breaking.
    Notable,
    /// Import/include edge changes only.
    Informational,
    /// Private-only changes — internal, no external surface affected.
    Internal,
}

impl Severity {
    pub fn as_str(self) -> &'static str {
        match self {
            Severity::Breaking => "breaking",
            Severity::Notable => "notable",
            Severity::Informational => "informational",
            Severity::Internal => "internal",
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct FileDelta {
    pub rel: String,
    pub added: Vec<SymbolLine>,
    pub removed: Vec<SymbolLine>,
    pub changed: Vec<SymbolChange>,
    /// Declarations that kept their name but changed kind (e.g. free fn →
    /// method), paired from added/removed (ADR 0006).
    pub kind_changed: Vec<KindChange>,
    pub added_imports: Vec<String>,
    pub removed_imports: Vec<String>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct SymbolLine {
    pub kind: &'static str,
    pub name: String,
    pub signature: String,
    pub visibility: &'static str,
}

#[derive(Debug, PartialEq, Eq)]
pub struct SymbolChange {
    pub kind: &'static str,
    pub name: String,
    pub old_signature: String,
    pub new_signature: String,
    pub visibility: &'static str,
}

#[derive(Debug, PartialEq, Eq)]
pub struct KindChange {
    pub name: String,
    pub old_kind: &'static str,
    pub new_kind: &'static str,
    pub old_signature: String,
    pub new_signature: String,
    pub visibility: &'static str,
}

impl SymbolLine {
    fn is_public(&self) -> bool {
        self.visibility == "public"
    }

    /// Severity of this symbol's add (`removed == false`) or removal (#107):
    /// removing public surface is breaking, adding it is notable, anything
    /// private is internal.
    pub fn severity(&self, removed: bool) -> Severity {
        match (self.is_public(), removed) {
            (true, true) => Severity::Breaking,
            (true, false) => Severity::Notable,
            (false, _) => Severity::Internal,
        }
    }
}

impl SymbolChange {
    /// A signature change to public surface is breaking; private is internal.
    pub fn severity(&self) -> Severity {
        if self.visibility == "public" {
            Severity::Breaking
        } else {
            Severity::Internal
        }
    }
}

impl KindChange {
    /// A reclassification of public surface is notable; private is internal.
    pub fn severity(&self) -> Severity {
        if self.visibility == "public" {
            Severity::Notable
        } else {
            Severity::Internal
        }
    }
}

impl FileSummary {
    /// A file carrying public symbols is breaking on removal, notable on add;
    /// a private-only file is internal either way.
    pub fn severity(&self, removed: bool) -> Severity {
        match (self.public_count > 0, removed) {
            (true, true) => Severity::Breaking,
            (true, false) => Severity::Notable,
            (false, _) => Severity::Internal,
        }
    }
}

impl StructuralDiff {
    /// True when nothing changed structurally between the two trees.
    pub fn is_empty(&self) -> bool {
        self.moved_files.is_empty()
            && self.added_files.is_empty()
            && self.removed_files.is_empty()
            && self.changed_files.is_empty()
    }
}

/// Compute the structural diff from `old` to `new` (the parse output of each
/// tree, as returned by [`crate::parse::parse_all`]).
pub fn diff(
    old: &[(SourceFile, ParsedFile)],
    new: &[(SourceFile, ParsedFile)],
    opts: &DiffOptions,
) -> StructuralDiff {
    let old_idx = index(old);
    let new_idx = index(new);

    let mut added_files = Vec::new();
    let mut removed_files = Vec::new();
    let mut changed_files = Vec::new();

    // Removed: present in old, gone in new (BTreeMap iteration → sorted by rel).
    for (rel, &(lang, pf)) in &old_idx {
        if !new_idx.contains_key(rel.as_str()) {
            removed_files.extend(file_summary(rel, lang, pf, opts));
        }
    }
    // Added + changed: walk new (sorted by rel).
    for (rel, &(lang, new_pf)) in &new_idx {
        match old_idx.get(rel.as_str()) {
            None => added_files.extend(file_summary(rel, lang, new_pf, opts)),
            Some(&(_, old_pf)) => {
                if let Some(delta) = file_delta(rel, old_pf, new_pf, opts) {
                    changed_files.push(delta);
                }
            }
        }
    }

    // Pair renamed/moved files (identical, uniquely-matched symbol sets) out of
    // the add/remove lists (#106).
    let moved_files = detect_file_moves(
        &mut removed_files,
        &mut added_files,
        &old_idx,
        &new_idx,
        opts,
    );

    StructuralDiff {
        added_files,
        removed_files,
        changed_files,
        moved_files,
    }
}

/// A file's fingerprint for move detection: its sorted visible `(kind, name,
/// signature)` set, joined. Two files with the same nonempty fingerprint are
/// candidate rename/move partners.
fn file_fingerprint(pf: &ParsedFile, opts: &DiffOptions) -> Vec<String> {
    let mut fp: Vec<String> = group(pf, opts)
        .into_iter()
        .flat_map(|((kind, name), bucket)| {
            bucket
                .sigs
                .into_iter()
                .map(move |sig| format!("{kind}\u{1}{name}\u{1}{sig}"))
        })
        .collect();
    fp.sort();
    fp
}

/// Detect renamed/moved files: a removed file and an added file whose symbol
/// fingerprints are identical, nonempty, and unique on both sides (#106). Paired
/// files are removed from `removed`/`added` and returned as moves. Conservative:
/// a fingerprint shared by more than one file on either side is left as
/// add/remove (ambiguous).
fn detect_file_moves(
    removed: &mut Vec<FileSummary>,
    added: &mut Vec<FileSummary>,
    old_idx: &BTreeMap<String, (&'static str, &ParsedFile)>,
    new_idx: &BTreeMap<String, (&'static str, &ParsedFile)>,
    opts: &DiffOptions,
) -> Vec<MovedFile> {
    // Fingerprint each candidate; count how often each appears per side.
    let fp_for = |idx: &BTreeMap<String, (&'static str, &ParsedFile)>, rel: &str| {
        idx.get(rel).map(|(_, pf)| file_fingerprint(pf, opts))
    };
    let mut removed_counts: BTreeMap<Vec<String>, usize> = BTreeMap::new();
    for f in removed.iter() {
        if let Some(fp) = fp_for(old_idx, &f.rel) {
            if !fp.is_empty() {
                *removed_counts.entry(fp).or_default() += 1;
            }
        }
    }
    let mut added_counts: BTreeMap<Vec<String>, usize> = BTreeMap::new();
    for f in added.iter() {
        if let Some(fp) = fp_for(new_idx, &f.rel) {
            if !fp.is_empty() {
                *added_counts.entry(fp).or_default() += 1;
            }
        }
    }

    let mut moves: Vec<MovedFile> = Vec::new();
    let mut moved_removed: BTreeSet<String> = BTreeSet::new();
    let mut moved_added: BTreeSet<String> = BTreeSet::new();
    for r in removed.iter() {
        let Some(fp) = fp_for(old_idx, &r.rel) else {
            continue;
        };
        if fp.is_empty() || removed_counts.get(&fp) != Some(&1) || added_counts.get(&fp) != Some(&1)
        {
            continue;
        }
        // Unique on both sides — find the single added partner with this fp.
        if let Some(a) = added
            .iter()
            .find(|a| fp_for(new_idx, &a.rel).as_ref() == Some(&fp))
        {
            moved_removed.insert(r.rel.clone());
            moved_added.insert(a.rel.clone());
            moves.push(MovedFile {
                old_rel: r.rel.clone(),
                new_rel: a.rel.clone(),
                lang: r.lang,
                symbol_count: r.symbol_count,
            });
        }
    }
    removed.retain(|f| !moved_removed.contains(&f.rel));
    added.retain(|f| !moved_added.contains(&f.rel));
    moves.sort_by(|a, b| a.new_rel.cmp(&b.new_rel));
    moves
}

/// Index a tree's parse output by relative path (sorted, deterministic).
fn index(files: &[(SourceFile, ParsedFile)]) -> BTreeMap<String, (&'static str, &ParsedFile)> {
    files
        .iter()
        .map(|(sf, pf)| (sf.rel.clone(), (sf.lang.name(), pf)))
        .collect()
}

/// Summarize an added/removed file, or `None` when `--no-private` leaves it
/// with no visible symbols — so it stays consistent with the changed-file path,
/// which likewise suppresses a file whose only delta is private.
fn file_summary(
    rel: &str,
    lang: &'static str,
    pf: &ParsedFile,
    opts: &DiffOptions,
) -> Option<FileSummary> {
    let visible: Vec<&Symbol> = visible(pf, opts).collect();
    let symbol_count = visible.len();
    if opts.no_private && symbol_count == 0 {
        return None;
    }
    let public_count = visible
        .iter()
        .filter(|s| matches!(s.visibility, Visibility::Public))
        .count();
    Some(FileSummary {
        rel: rel.to_string(),
        lang,
        symbol_count,
        public_count,
    })
}

/// Symbols of `pf` that the diff considers, honoring `--no-private`.
fn visible<'a>(pf: &'a ParsedFile, opts: &DiffOptions) -> impl Iterator<Item = &'a Symbol> {
    let no_private = opts.no_private;
    pf.symbols
        .iter()
        .filter(move |s| !no_private || matches!(s.visibility, Visibility::Public))
}

/// Group a file's visible symbols by `(kind, name)` → its sorted signatures.
/// Keyed for deterministic iteration; sorting the signatures makes the
/// set comparison order-independent.
///
/// Identity is `(kind, name)` (ADR 0005). A declaration that keeps its name but
/// changes *kind* — e.g. a Rust free `fn` moved into an `impl` becomes a
/// `method` — therefore shows as a removed + added pair, not a `~ changed`
/// line, the same family as the overload fallback. Broadening identity to pair
/// these is a deferred v2; `kind_change_reports_remove_and_add` pins the
/// current behavior.
fn group(pf: &ParsedFile, opts: &DiffOptions) -> BTreeMap<(&'static str, String), Bucket> {
    let mut m: BTreeMap<(&'static str, String), Bucket> = BTreeMap::new();
    for s in visible(pf, opts) {
        let bucket = m
            .entry((kind_name(s.kind), s.name.clone()))
            .or_insert_with(|| Bucket {
                sigs: Vec::new(),
                visibility: "private",
            });
        bucket.sigs.push(s.signature.clone());
        if matches!(s.visibility, Visibility::Public) {
            bucket.visibility = "public";
        }
    }
    for b in m.values_mut() {
        b.sigs.sort();
    }
    m
}

/// A `(kind, name)` bucket: its sorted signatures plus the bucket's visibility
/// (public if any member is public — the conservative choice for severity).
struct Bucket {
    sigs: Vec<String>,
    visibility: &'static str,
}

/// Compute one common file's delta, or `None` if nothing changed.
fn file_delta(
    rel: &str,
    old_pf: &ParsedFile,
    new_pf: &ParsedFile,
    opts: &DiffOptions,
) -> Option<FileDelta> {
    let old_syms = group(old_pf, opts);
    let new_syms = group(new_pf, opts);

    let mut added = Vec::new();
    let mut removed = Vec::new();
    let mut changed = Vec::new();

    let keys: BTreeSet<&(&'static str, String)> = old_syms.keys().chain(new_syms.keys()).collect();
    for key in keys {
        let kind = key.0;
        let name = key.1.as_str();
        match (old_syms.get(key), new_syms.get(key)) {
            (None, Some(news)) => emit_bucket(&mut added, kind, name, news),
            (Some(olds), None) => emit_bucket(&mut removed, kind, name, olds),
            (Some(olds), Some(news)) => {
                if olds.sigs == news.sigs {
                    // Unchanged (signature-identical). A visibility-only change
                    // is not separately flagged in v1 (#107) — documented limit.
                } else if olds.sigs.len() == 1 && news.sigs.len() == 1 {
                    changed.push(SymbolChange {
                        kind,
                        name: name.to_string(),
                        old_signature: olds.sigs[0].clone(),
                        new_signature: news.sigs[0].clone(),
                        visibility: news.visibility,
                    });
                } else {
                    // Overload bucket: fall back to per-signature add/remove.
                    let oset: BTreeSet<&String> = olds.sigs.iter().collect();
                    let nset: BTreeSet<&String> = news.sigs.iter().collect();
                    for sig in news.sigs.iter().filter(|s| !oset.contains(s)) {
                        added.push(line(kind, name, sig, news.visibility));
                    }
                    for sig in olds.sigs.iter().filter(|s| !nset.contains(s)) {
                        removed.push(line(kind, name, sig, olds.visibility));
                    }
                }
            }
            (None, None) => unreachable!("key came from one of the two maps"),
        }
    }

    // Pair uniquely-named add/remove entries of differing kinds into a single
    // kind-change (ADR 0006); the rest stay as add/remove.
    let kind_changed = pair_kind_changes(&mut added, &mut removed);

    let old_imp: BTreeSet<&String> = old_pf.imports.iter().collect();
    let new_imp: BTreeSet<&String> = new_pf.imports.iter().collect();
    let added_imports: Vec<String> = new_imp
        .difference(&old_imp)
        .map(|s| s.to_string())
        .collect();
    let removed_imports: Vec<String> = old_imp
        .difference(&new_imp)
        .map(|s| s.to_string())
        .collect();

    if added.is_empty()
        && removed.is_empty()
        && changed.is_empty()
        && kind_changed.is_empty()
        && added_imports.is_empty()
        && removed_imports.is_empty()
    {
        None
    } else {
        Some(FileDelta {
            rel: rel.to_string(),
            added,
            removed,
            changed,
            kind_changed,
            added_imports,
            removed_imports,
        })
    }
}

fn emit_bucket(dst: &mut Vec<SymbolLine>, kind: &'static str, name: &str, bucket: &Bucket) {
    for sig in &bucket.sigs {
        dst.push(line(kind, name, sig, bucket.visibility));
    }
}

/// Pair a uniquely-named removed symbol with a uniquely-named added symbol of a
/// *different* kind into one [`KindChange`] (ADR 0006), removing both from the
/// add/remove lists. Conservative: only 1↔1 unique-name matches are paired, so
/// an unrelated add and remove that share a name are never mis-merged.
fn pair_kind_changes(
    added: &mut Vec<SymbolLine>,
    removed: &mut Vec<SymbolLine>,
) -> Vec<KindChange> {
    let counts = |v: &[SymbolLine]| {
        let mut m: BTreeMap<String, usize> = BTreeMap::new();
        for s in v {
            *m.entry(s.name.clone()).or_default() += 1;
        }
        m
    };
    let added_counts = counts(added);
    let removed_counts = counts(removed);

    let mut paired: BTreeSet<String> = BTreeSet::new();
    let mut changes: Vec<KindChange> = Vec::new();
    for r in removed.iter() {
        if added_counts.get(&r.name) != Some(&1) || removed_counts.get(&r.name) != Some(&1) {
            continue;
        }
        if let Some(a) = added.iter().find(|a| a.name == r.name) {
            if a.kind != r.kind && paired.insert(r.name.clone()) {
                changes.push(KindChange {
                    name: r.name.clone(),
                    old_kind: r.kind,
                    new_kind: a.kind,
                    old_signature: r.signature.clone(),
                    new_signature: a.signature.clone(),
                    visibility: a.visibility,
                });
            }
        }
    }
    added.retain(|s| !paired.contains(&s.name));
    removed.retain(|s| !paired.contains(&s.name));
    changes.sort_by(|a, b| a.name.cmp(&b.name));
    changes
}

fn line(kind: &'static str, name: &str, sig: &str, visibility: &'static str) -> SymbolLine {
    SymbolLine {
        kind,
        name: name.to_string(),
        signature: sig.to_string(),
        visibility,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lang::Language;
    use crate::parse::{ParsedFile, Symbol, SymbolKind, Visibility};

    fn sf(rel: &str, lang: Language) -> SourceFile {
        SourceFile {
            path: rel.into(),
            rel: rel.to_string(),
            lang,
        }
    }

    fn sym(kind: SymbolKind, name: &str, sig: &str, vis: Visibility) -> Symbol {
        Symbol {
            name: name.to_string(),
            kind,
            signature: sig.to_string(),
            line: 1,
            visibility: vis,
        }
    }

    fn file(rel: &str, symbols: Vec<Symbol>, imports: Vec<&str>) -> (SourceFile, ParsedFile) {
        (
            sf(rel, Language::Python),
            ParsedFile {
                symbols,
                imports: imports.into_iter().map(String::from).collect(),
                ..Default::default()
            },
        )
    }

    fn pubf(name: &str, sig: &str) -> Symbol {
        sym(SymbolKind::Function, name, sig, Visibility::Public)
    }

    #[test]
    fn added_and_removed_files() {
        let old = vec![file("a.py", vec![pubf("f", "def f()")], vec![])];
        let new = vec![file("b.py", vec![pubf("g", "def g()")], vec![])];
        let d = diff(&old, &new, &DiffOptions { no_private: false });
        assert_eq!(d.added_files.len(), 1);
        assert_eq!(d.added_files[0].rel, "b.py");
        assert_eq!(d.added_files[0].symbol_count, 1);
        assert_eq!(d.removed_files.len(), 1);
        assert_eq!(d.removed_files[0].rel, "a.py");
        assert!(d.changed_files.is_empty());
    }

    #[test]
    fn changed_signature_detected() {
        let old = vec![file("x.py", vec![pubf("f", "def f(x)")], vec![])];
        let new = vec![file("x.py", vec![pubf("f", "def f(x, y)")], vec![])];
        let d = diff(&old, &new, &DiffOptions { no_private: false });
        assert_eq!(d.changed_files.len(), 1);
        let fd = &d.changed_files[0];
        assert!(fd.added.is_empty() && fd.removed.is_empty());
        assert_eq!(fd.changed.len(), 1);
        assert_eq!(fd.changed[0].name, "f");
        assert_eq!(fd.changed[0].old_signature, "def f(x)");
        assert_eq!(fd.changed[0].new_signature, "def f(x, y)");
    }

    #[test]
    fn added_and_removed_symbols() {
        let old = vec![file(
            "x.py",
            vec![pubf("f", "def f()"), pubf("g", "def g()")],
            vec![],
        )];
        let new = vec![file(
            "x.py",
            vec![pubf("f", "def f()"), pubf("h", "def h()")],
            vec![],
        )];
        let d = diff(&old, &new, &DiffOptions { no_private: false });
        assert_eq!(d.changed_files.len(), 1);
        let fd = &d.changed_files[0];
        assert_eq!(
            fd.added.iter().map(|s| s.name.as_str()).collect::<Vec<_>>(),
            vec!["h"]
        );
        assert_eq!(
            fd.removed
                .iter()
                .map(|s| s.name.as_str())
                .collect::<Vec<_>>(),
            vec!["g"]
        );
        assert!(fd.changed.is_empty());
    }

    #[test]
    fn import_edges_diff_sorted() {
        let old = vec![file("x.py", vec![pubf("f", "def f()")], vec!["a", "b"])];
        let new = vec![file("x.py", vec![pubf("f", "def f()")], vec!["b", "c"])];
        let d = diff(&old, &new, &DiffOptions { no_private: false });
        assert_eq!(d.changed_files.len(), 1);
        let fd = &d.changed_files[0];
        assert_eq!(fd.added_imports, vec!["c".to_string()]);
        assert_eq!(fd.removed_imports, vec!["a".to_string()]);
    }

    #[test]
    fn no_private_filters_symbols() {
        let old = vec![file("x.py", vec![pubf("f", "def f()")], vec![])];
        let new = vec![file(
            "x.py",
            vec![
                pubf("f", "def f()"),
                sym(SymbolKind::Function, "h", "def h()", Visibility::Private),
            ],
            vec![],
        )];
        // With no_private, the new private `h` is invisible → no change at all.
        let hidden = diff(&old, &new, &DiffOptions { no_private: true });
        assert!(hidden.is_empty());
        // Without it, `h` shows as an added symbol.
        let shown = diff(&old, &new, &DiffOptions { no_private: false });
        assert_eq!(shown.changed_files.len(), 1);
        assert_eq!(shown.changed_files[0].added.len(), 1);
    }

    #[test]
    fn unique_kind_change_is_paired() {
        // A symbol that keeps its name but changes kind (free fn → method) is
        // paired into one kind-change entry, not a remove + add (ADR 0006).
        let old = vec![file("x.py", vec![pubf("helper", "def helper()")], vec![])];
        let new = vec![file(
            "x.py",
            vec![sym(
                SymbolKind::Method,
                "helper",
                "def helper(self)",
                Visibility::Public,
            )],
            vec![],
        )];
        let d = diff(&old, &new, &DiffOptions { no_private: false });
        assert_eq!(d.changed_files.len(), 1);
        let fd = &d.changed_files[0];
        assert!(fd.added.is_empty(), "{fd:?}");
        assert!(fd.removed.is_empty(), "{fd:?}");
        assert!(fd.changed.is_empty());
        assert_eq!(fd.kind_changed.len(), 1);
        let kc = &fd.kind_changed[0];
        assert_eq!(kc.name, "helper");
        assert_eq!(kc.old_kind, "function");
        assert_eq!(kc.new_kind, "method");
        assert_eq!(kc.old_signature, "def helper()");
        assert_eq!(kc.new_signature, "def helper(self)");
    }

    #[test]
    fn ambiguous_same_name_is_not_paired() {
        // Two same-named removed symbols (different kinds) make the match
        // ambiguous → stay as plain add/remove, never a kind-change.
        let old = vec![file(
            "x.py",
            vec![
                pubf("dup", "def dup()"),
                sym(SymbolKind::Class, "dup", "class dup", Visibility::Public),
            ],
            vec![],
        )];
        let new = vec![file(
            "x.py",
            vec![sym(
                SymbolKind::Method,
                "dup",
                "def dup(self)",
                Visibility::Public,
            )],
            vec![],
        )];
        let d = diff(&old, &new, &DiffOptions { no_private: false });
        let fd = &d.changed_files[0];
        assert!(fd.kind_changed.is_empty(), "{fd:?}");
        // The added method and both removed decls stay as add/remove.
        assert_eq!(fd.added.len(), 1);
        assert_eq!(fd.removed.len(), 2);
    }

    #[test]
    fn no_private_hides_all_private_added_file() {
        let old: Vec<(SourceFile, ParsedFile)> = vec![];
        let new = vec![file(
            "secret.py",
            vec![sym(
                SymbolKind::Function,
                "_helper",
                "def _helper()",
                Visibility::Private,
            )],
            vec![],
        )];
        // Under --no-private the all-private added file has no public surface to
        // report, so it is suppressed (consistent with the changed-file path).
        assert!(diff(&old, &new, &DiffOptions { no_private: true }).is_empty());
        // Without the flag it is reported, its private symbol counted.
        let shown = diff(&old, &new, &DiffOptions { no_private: false });
        assert_eq!(shown.added_files.len(), 1);
        assert_eq!(shown.added_files[0].rel, "secret.py");
        assert_eq!(shown.added_files[0].symbol_count, 1);
    }

    #[test]
    fn identical_trees_yield_empty_diff() {
        let tree = vec![file("x.py", vec![pubf("f", "def f()")], vec!["a"])];
        let d = diff(&tree, &tree, &DiffOptions { no_private: false });
        assert!(d.is_empty());
    }

    #[test]
    fn is_deterministic() {
        let old = vec![
            file("b.py", vec![pubf("f", "def f()")], vec![]),
            file("a.py", vec![pubf("g", "def g(x)")], vec![]),
        ];
        let new = vec![
            file("a.py", vec![pubf("g", "def g(x, y)")], vec![]),
            file("c.py", vec![pubf("h", "def h()")], vec![]),
        ];
        let opts = DiffOptions { no_private: false };
        assert_eq!(diff(&old, &new, &opts), diff(&old, &new, &opts));
    }

    #[test]
    fn identical_file_at_new_path_is_a_move() {
        let syms = || vec![pubf("f", "def f()"), pubf("g", "def g()")];
        let old = vec![file("a.py", syms(), vec![])];
        let new = vec![file("b.py", syms(), vec![])];
        let d = diff(&old, &new, &DiffOptions { no_private: false });
        assert!(d.added_files.is_empty(), "{d:?}");
        assert!(d.removed_files.is_empty(), "{d:?}");
        assert_eq!(d.moved_files.len(), 1);
        assert_eq!(d.moved_files[0].old_rel, "a.py");
        assert_eq!(d.moved_files[0].new_rel, "b.py");
        assert_eq!(d.moved_files[0].symbol_count, 2);
    }

    #[test]
    fn ambiguous_identical_files_are_not_moved() {
        // Two old files share one symbol set → ambiguous → kept as add/remove.
        let old = vec![
            file("a.py", vec![pubf("f", "def f()")], vec![]),
            file("a2.py", vec![pubf("f", "def f()")], vec![]),
        ];
        let new = vec![file("b.py", vec![pubf("f", "def f()")], vec![])];
        let d = diff(&old, &new, &DiffOptions { no_private: false });
        assert!(d.moved_files.is_empty(), "{d:?}");
        assert_eq!(d.removed_files.len(), 2);
        assert_eq!(d.added_files.len(), 1);
    }

    #[test]
    fn unrelated_add_remove_is_not_a_move() {
        let old = vec![file("a.py", vec![pubf("f", "def f()")], vec![])];
        let new = vec![file("b.py", vec![pubf("g", "def g()")], vec![])];
        let d = diff(&old, &new, &DiffOptions { no_private: false });
        assert!(d.moved_files.is_empty(), "{d:?}");
        assert_eq!(d.removed_files.len(), 1);
        assert_eq!(d.added_files.len(), 1);
    }
}

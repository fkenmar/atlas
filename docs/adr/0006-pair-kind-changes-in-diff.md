# ADR 0006 — Pair unique kind-changes in the diff into a single entry

## Context

ADR 0005 set the diff's symbol identity to `(kind, name)`, with the documented
consequence that a declaration which keeps its name but changes *kind* shows as a
removed + added pair, not a `~ changed` line. The common case is a Rust free
`fn` moved into an `impl` (Function → Method) or a Python `def` becoming a
`class` — a single refactor that reads as a delete + create, which the #12 review
flagged (the behavior was pinned by a test as intentional, not incidental).

Broadening identity to ignore kind would wrongly merge a genuinely-deleted
function with an unrelated new method of the same name, and would break the
overload fallback. So identity stays `(kind, name)`; the fix is a narrow,
post-hoc pairing.

## Decision

After the per-file added/removed/changed sets are computed, run a second pass:
for a `name` that appears **exactly once in added and exactly once in removed**
with **different kinds**, move the pair out of added/removed into a new
`kind_changed` list as a single entry recording `name`, `old_kind → new_kind`,
and `old_sig → new_sig`. Any name with more than one added or removed entry (the
overload/ambiguous case) keeps the conservative add/remove split unchanged.

`kind_changed` is a **new, additive** field on `FileDelta` — it does not alter
the existing `changed` list (still same-kind signature changes) or the diff JSON
schema's existing keys, so `DIFF_SCHEMA_VERSION` (ADR/#16) stays 1; the new
`kind_changed` array/`<kind-changed>` element is additive. The Markdown renderer
marks it with `±` (`± name: old_kind old_sig → new_kind new_sig`).

## Consequences

- A move/reclassification refactor now reads as one entry instead of an
  unexplained delete + create — the headline improvement.
- The pairing is deliberately conservative: it fires only on a 1↔1 unique-name
  match, so it can't mis-pair an unrelated add and remove that happen to share a
  name. Ambiguous cases stay as add/remove (same family as the overload
  fallback), preserving ADR 0005's identity rule.
- Determinism holds: the pairing keys off the already-sorted added/removed Vecs
  and emits `kind_changed` in sorted `name` order.
- Three renderers (md/json/xml) gain a small additive branch; the structured
  schemas stay backward-compatible (additive keys don't bump the version).
- Supersedes the "kind change → remove + add" consequence in ADR 0005 for the
  unique-name case only; the overload/duplicate-name fallback there still stands.

# Stability & deprecation policy (pre-1.0)

atlas is **alpha**. Pre-1.0, breaking changes are *allowed* — but they will never
be *silent or accidental*. This page says what you can rely on, what may change,
and how changes are announced, surface by surface. If you depend on atlas in
automation, **pin a version**.

The short version: structured, versioned outputs (JSON/XML) are the most stable
contract; the human-facing Markdown map is the least. Everything is governed by
semver-with-an-asterisk: until 1.0, a **minor** bump (0.x) may carry a breaking
change, always recorded in [`CHANGELOG.md`](../CHANGELOG.md).

## Surfaces

### JSON / XML output — versioned contract (most stable)

- `atlas --format json` and `--format xml` carry an explicit integer
  `version`/`SCHEMA_VERSION`; `atlas diff --format json|xml` carries an
  independent `DIFF_SCHEMA_VERSION`. Both are **1** today.
- The shape is pinned by machine-readable JSON Schemas:
  [`schemas/atlas-map.schema.json`](../schemas/atlas-map.schema.json) and
  [`schemas/atlas-diff.schema.json`](../schemas/atlas-diff.schema.json), and a
  test keeps the renderers and schemas in agreement.
- **Additive** fields (new optional keys) bump the schema `version` and, pre-1.0,
  the crate minor. **Removing or renaming** a field is a breaking change, called
  out in the changelog and the schema version.
- Build against these formats if you need stability. Validate the `version` field
  and treat unknown fields leniently.

### CLI flags & exit codes — stable, changes announced

- Flag names, defaults, and the [`0/1/2` exit-code contract](exit-codes.md) are
  treated as a stable interface; changes are noted in the changelog.
- When a flag is renamed, the old name is kept as a hidden alias for at least one
  minor release where practical, emitting a one-line deprecation notice to
  **stderr** (never stdout, so piped map output is unaffected).
- New flags are additive and don't change existing behavior.

### Markdown map output — readability-first, not a parsing contract

- The default Markdown map is tuned for humans and agents to *read*, and may
  change between releases (new header fields, a legend, density tweaks) without a
  version bump.
- **Don't parse Markdown for tooling** — use `--format json`/`xml`, which are
  versioned. A committed `atlas-map.md` can still drift release to release; the
  [`atlas --check`](pre-commit.md) gate compares against the *current* binary, so
  pin the atlas version in CI to keep it stable.

### Cache format — internal, no contract

- The `.atlas/` parse cache (bincode, content-hash keyed) is an internal
  implementation detail. It is version-keyed and rebuilt automatically when the
  format or atlas version changes, and is always safe to delete
  (`atlas cache clean --force`). Don't depend on its contents or layout.

### Config files — when they land

- Config-file support (`.atlas.toml`, [#59](https://github.com/fkenmar/atlas/issues/59))
  isn't shipped yet. When it is, it will follow this policy: documented keys,
  CLI-overrides-config precedence, and additive evolution with deprecation
  notices for renamed keys.

### Rust library API — not a supported surface pre-1.0

- atlas ships as a **CLI/MCP tool**. The library crate's internal modules
  (`src/*`) are not a supported public API before 1.0 and may change freely.
  Depend on the binary's CLI/MCP/JSON surfaces, not on `atlas` as a library.

## How changes are announced

1. **`CHANGELOG.md`** is the source of truth — every breaking change is listed
   under its release, with migration notes.
2. **Schema `version` bumps** signal output-shape changes for JSON/XML consumers.
3. **stderr deprecation notices** flag a renamed/removed flag during its grace
   release.
4. **Release notes** summarize the above for each tag.

## What "alpha" means for you

- Try it, build workflows on the JSON/XML schemas, and pin a version.
- Expect the Markdown format and internal heuristics (ranking, budget ladder) to
  keep improving — that's the point of the [benchmark loop](SELF_IMPROVEMENT.md).
- Report a surprise: if something changed without a changelog entry or a version
  bump, that's a bug — please
  [open an issue](https://github.com/fkenmar/atlas/issues/new).

The 1.0 line will freeze these surfaces under standard semver; until then, this
policy is the contract.

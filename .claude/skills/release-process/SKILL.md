---
name: release-process
description: Use for releases and versioning — version bumps, tagging, CHANGELOG entries, cargo-dist builds, Homebrew or crates.io publishing, or any "ship v0.x" request.
---

# Releasing repomap

## Semver policy (pre-1.0)

- **JSON schema changes = MINOR bump.** Programmatic consumers parse the JSON output; `SCHEMA_VERSION` in src/render/json.rs bumps with the crate minor version. Added fields count as schema changes.
- New flags, languages, or features = minor. Bugfix-only = patch.
- Breaking CLI changes (flag rename/removal) = minor pre-1.0, but called out at the top of the release notes — pipe consumers break silently otherwise.

## cargo-dist workflow

- One-time setup (M2): `cargo dist init` — configures targets (macOS arm64/x64, Linux x64/arm64 musl), generates the release GitHub Actions workflow, the Homebrew formula, and the curl-pipe installer script.
- Per release: `cargo dist build` locally first to confirm all four artifacts build; the real artifacts are produced by CI when the version tag is pushed.
- The generated workflow publishes GitHub release assets; Homebrew tap and installer script update from the same run.

## The release checklist

Executed by the **release-manager** agent — delegate to it rather than running ad hoc:

1. STATUS.md milestone exit criteria all checked — otherwise the release is refused.
2. Clean tree; fmt + clippy `-D warnings` + tests green.
3. Version bump per the policy above.
4. CHANGELOG.md entry (Added/Changed/Fixed; benchmark delta for ranking/budgeting changes).
5. README benchmark numbers current — stale numbers block release.
6. `cargo dist build` artifacts verified for all four targets.
7. Commit, tag `vX.Y.Z`, draft notes; publish only after maintainer confirmation.

## crates.io

- `cargo publish --dry-run` first; check the package file list doesn't leak benchmark results or .claude/.
- `cargo publish` only after the maintainer confirms — same rule as pushing the tag.
- The crate must build with only Cargo.toml dependencies (queries/ are include_str!'d, so they ship inside the crate — keep them in the package include list).

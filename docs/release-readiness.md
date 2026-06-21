# Release readiness gates

atlas uses release channels to keep public claims honest while the product is
still moving quickly. The channel is a user-facing promise: do not promote a
release under a stronger channel until the matching gate is true or the release
notes explicitly call out the exception.

Current channel: **alpha**.

These gates are intentionally lightweight for a single-maintainer project:
documented caveats and release-note waivers are acceptable until a channel
promises durability.

## Channel definitions

| Channel | User promise | Appropriate release type |
|---|---|---|
| Alpha | Core workflow works and is tested, but CLI flags, output shape, install paths, and docs can still change. | Pre-release tags and `--pre` package installs. |
| Beta | A stranger can install it through documented channels, use the supported workflows, and understand known limits without maintainer help. Breaking changes are still possible but announced. | Public pre-1.0 releases intended for regular external users. |
| Stable | Install, output schemas, support process, security posture, and compatibility policy are durable enough for downstream tooling and CI. | Non-prerelease tags and package installs. |

## Alpha gate

Alpha is allowed when all of these are true:

- Install channels: at least one documented install path works for a fresh user,
  and any staged-but-not-live channels are clearly labeled.
- Docs: README explains status, core usage, supported languages, troubleshooting,
  privacy/offline behavior, and how to file extraction bugs.
- Benchmark evidence: any headline claim is reproducible and limited to the
  benchmark that actually supports it.
- Language accuracy: supported languages have query fixtures and snapshot tests;
  known caveats are documented.
- Output/schema stability: output can still change, but JSON/XML changes remain
  additive unless release notes say otherwise.
- Security docs: `SECURITY.md`, privacy docs, and local/read-only behavior are
  documented.
- Support process: issue templates exist for bugs/features or release notes say
  support is maintainer-direct during alpha.

## Beta gate

Beta requires alpha plus:

- Install channels: PyPI/pipx, GitHub release archives, and at least one
  source/build install path are smoke-tested for the release.
- Docs: README, install docs, MCP docs, FAQ, comparison guide, and release notes
  describe the same shipped feature set.
- Benchmark evidence: the README claim has a reproducible case study, and any
  broader performance claims are backed by committed runs.
- Language accuracy: a real-world fixture corpus and per-language extraction
  accuracy report exist for the supported languages.
- Output/schema stability: public map and diff JSON schemas are documented, and
  pre-1.0 breaking-change/deprecation policy is published.
- Security docs: binary verification docs exist; supply-chain provenance gaps are
  either closed or explicitly listed in release notes.
- Support process: early-adopter feedback path is documented, and issue triage
  expectations are clear.

## Stable gate

Stable requires beta plus:

- Install channels: primary install paths work without prerelease flags, and
  release artifacts have checksums/signatures or equivalent verification.
- Docs: docs are version-aware enough that users can distinguish `main`, latest
  prerelease, and stable package behavior.
- Benchmark evidence: benchmark wording has survived at least one release cycle
  without caveat churn, and large-repo performance data covers the supported
  language mix.
- Language accuracy: extraction behavior has a published compatibility policy;
  known parser gaps are tracked as bugs, not hidden caveats.
- Output/schema stability: map and diff schemas are treated as compatibility
  contracts; breaking changes require a planned release note and migration path.
- Security docs: SBOM/provenance/attestation posture is decided and documented.
- Support process: discussions or another public feedback channel exists, and
  bug/security response expectations are clear.

## Open blockers

These are the current issue links to check before promoting the channel. The
exact list can change; refresh it before a release candidate.

| Area | Blocks beta | Blocks stable |
|---|---|---|
| Install channels | [#29 PyPI Trusted Publishing](https://github.com/fkenmar/atlas/issues/29), [#37 crates.io](https://github.com/fkenmar/atlas/issues/37), [#43 install smoke tests](https://github.com/fkenmar/atlas/issues/43) | [#38 Homebrew](https://github.com/fkenmar/atlas/issues/38), [#61 completions](https://github.com/fkenmar/atlas/issues/61), [#62 man page](https://github.com/fkenmar/atlas/issues/62) |
| Docs and release policy | [#40 benchmark case study](https://github.com/fkenmar/atlas/issues/40), [#56 release announcement template](https://github.com/fkenmar/atlas/issues/56), [#97 pre-1.0 stability policy](https://github.com/fkenmar/atlas/issues/97), [#113 docs by release channel](https://github.com/fkenmar/atlas/issues/113) | [#98 changelog/release-note automation](https://github.com/fkenmar/atlas/issues/98), [#113 docs by release channel](https://github.com/fkenmar/atlas/issues/113) |
| Benchmark and performance evidence | [#6 grow benchmark suite](https://github.com/fkenmar/atlas/issues/6), [#13 competitive benchmark arms](https://github.com/fkenmar/atlas/issues/13), [#73 large-repo performance matrix](https://github.com/fkenmar/atlas/issues/73), [#74 output-size report](https://github.com/fkenmar/atlas/issues/74) | [#73 large-repo performance matrix](https://github.com/fkenmar/atlas/issues/73) |
| Language accuracy | [#110 real-world fixture corpus](https://github.com/fkenmar/atlas/issues/110), [#111 extraction accuracy report](https://github.com/fkenmar/atlas/issues/111), [#112 framework-aware examples](https://github.com/fkenmar/atlas/issues/112) | [#109 future-language request rubric](https://github.com/fkenmar/atlas/issues/109), [#111 extraction accuracy report](https://github.com/fkenmar/atlas/issues/111) |
| Output/schema stability | [#57 map JSON Schema](https://github.com/fkenmar/atlas/issues/57), [#58 diff JSON Schema](https://github.com/fkenmar/atlas/issues/58), [#103 selectable token budgets](https://github.com/fkenmar/atlas/issues/103) | [#57 map JSON Schema](https://github.com/fkenmar/atlas/issues/57), [#58 diff JSON Schema](https://github.com/fkenmar/atlas/issues/58), [#97 pre-1.0 stability policy](https://github.com/fkenmar/atlas/issues/97) |
| Security and supply chain | [#63 checksums/signatures](https://github.com/fkenmar/atlas/issues/63), [#66 SBOM](https://github.com/fkenmar/atlas/issues/66) | [#65 attestations/provenance](https://github.com/fkenmar/atlas/issues/65), [#66 SBOM](https://github.com/fkenmar/atlas/issues/66) |
| Support and client confidence | [#96 feedback template/discussions](https://github.com/fkenmar/atlas/issues/96), [#101 MCP conformance tests](https://github.com/fkenmar/atlas/issues/101) | [#96 feedback template/discussions](https://github.com/fkenmar/atlas/issues/96), [#101 MCP conformance tests](https://github.com/fkenmar/atlas/issues/101) |

## Release go/no-go

For every release candidate:

1. Pick the intended channel before writing release notes.
2. Run the matching gate above.
3. If a blocker is intentionally waived, name it in the release notes with the
   expected user impact.
4. Keep README status, `CHANGELOG.md`, and the GitHub release label aligned with
   the selected channel.

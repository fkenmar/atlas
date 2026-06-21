# Security policy

atlas is a local, read-only developer tool. It does not make network calls during
normal map generation and has no product telemetry.

## Supported versions

Security fixes are accepted for the latest published alpha and `main`. Pre-1.0
interfaces may change, but security-relevant fixes should be backported when a
published release is likely to remain in use.

## Reporting a vulnerability

Please report vulnerabilities privately through GitHub's security advisory flow
for `fkenmar/atlas` when available. If that is not available to you, contact the
maintainer through GitHub and avoid posting exploit details publicly until there
is a fix or mitigation.

Useful reports include:

- atlas version or commit SHA.
- Operating system and install path.
- Exact command or MCP request.
- Minimal repository or file snippet that reproduces the issue.
- Expected impact, especially path traversal, unwanted file reads/writes,
  malformed output that escapes JSON/XML boundaries, or denial of service on
  ordinary source files.

## Scope

In scope:

- Reading files outside an intended root.
- MCP `--root` confinement bypasses.
- Crashes or unbounded resource use on ordinary source input.
- Malformed JSON/XML output that can break downstream parsers.
- Binary distribution, installer-script, PyPI wheel, and GitHub release artifact
  concerns.
- GitHub Actions workflow issues that could publish the wrong artifact or expose
  release credentials.
- Local code parsing issues that can read, write, or execute beyond the
  requested repository map operation.

Out of scope:

- Issues requiring a malicious local user who already controls the repository
  being mapped.
- The model behavior of downstream AI agents.
- Vulnerabilities in unrelated tools that consume atlas output.

## Disclosure expectations

The maintainer will acknowledge a credible report, investigate, and coordinate a
fix before public disclosure. If the issue is not accepted as a vulnerability,
the maintainer will explain why and may convert it into a normal bug or hardening
issue.

## Hardening guidance

- **Using maps in LLM prompts.** A map is untrusted data derived from source
  code. See [`docs/prompt-injection.md`](docs/prompt-injection.md) for the threat
  model, the XML-escaping guarantees, and safe prompt wrappers.
- **Restricted-network install.** atlas runs offline after install; for proxy,
  air-gapped, and mirror setups see [`docs/install-offline.md`](docs/install-offline.md).

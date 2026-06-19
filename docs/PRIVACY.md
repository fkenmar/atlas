# Privacy and offline operation

atlas is local-first. Normal map generation reads files from the repository path
you pass in, parses them locally, and writes the map to stdout or to the output
path you request.

## Network and telemetry

- atlas does not phone home.
- atlas does not collect product telemetry.
- atlas does not send repository contents, file names, symbols, or usage data to
  the maintainer.
- Normal CLI commands make no network calls. Release installers, package
  managers, and GitHub Actions workflows may use the network to download or
  publish artifacts, but the `atlas` binary's map/diff/MCP operations are local.

If a future feature adds network access, this document and the README must be
updated before that feature ships.

## Local data written by atlas

atlas may write a parse cache under `.atlas/` at the repository root when cache
mode is enabled by the CLI. The cache stores parser output derived from local
source files so warm runs can skip unchanged files. It is local to your checkout
and safe to delete.

Inspect or remove it with:

```sh
atlas cache info
atlas cache clean --force
```

The MCP server builds maps without writing the parse cache into the target repo.

## Map output sensitivity

An atlas map does not include function bodies, but it is still source-derived.
It may contain:

- file paths;
- symbol names;
- function and method signatures;
- type and field names;
- import and reverse-dependency relationships.

Treat generated maps, JSON, XML, and diffs with the same care as other
repository-derived artifacts. Do not paste or publish a map from a private repo
unless you would also share its API surface and file layout.

## Related docs

- [Security policy](../SECURITY.md)
- [Launch checklist](LAUNCH_CHECKLIST.md)
- [Adoption metrics](ADOPTION_METRICS.md)

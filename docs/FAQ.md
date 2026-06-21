# FAQ

Common questions about atlas, in one place. Deeper detail lives in the linked
docs — this page points rather than duplicates.

## What is atlas, in one line?

A CLI that compiles a repo into a compact, ranked, **structural** map (every
signature, type, and import edge — no function bodies) sized to a token budget,
so an AI coding agent gets its bearings without reading every file. See the
[README](../README.md).

## Why not just use grep / ctags / repomix / aider's repo-map?

Different jobs:

- **grep / ripgrep** finds text you already know to search for. atlas gives an
  agent the *shape* of a repo it has never seen.
- **ctags** lists symbols but doesn't rank them or fit a token budget — you get
  everything or nothing.
- **repomix / concat packers** paste whole files (bodies included); that's the
  token cost atlas exists to avoid.
- **aider's repo-map** is the closest relative — atlas is that idea unbundled
  into a standalone tool with explicit budgeting and a diff mode.

For a point-by-point comparison against Aider repo-map, ctags, tree-sitter CLI,
and SCIP/Sourcegraph, see [`docs/comparison.md`](comparison.md).

## Does my source code leave my machine?

No. atlas runs entirely locally, makes no network calls, and collects no
telemetry. It only reads files and writes a map. See
[`docs/PRIVACY.md`](PRIVACY.md). (The map you *generate* is yours to paste into
an agent — that's a choice you make, not something atlas does.)

## Why does the output omit function bodies?

Because bodies are the expensive part and rarely what an agent needs to
*navigate*. Signatures, types, and import edges tell it where things are and how
they connect; it opens the actual source before editing. Dropping bodies is what
makes a whole-repo map fit in ~2,000 tokens.

## How does the token budget work?

atlas targets a budget (default **2,048 tokens**, set with `--budget`). When a
repo doesn't fit, it **degrades in steps** instead of truncating: drop private
symbols, then elide parameter detail, then collapse the lowest-ranked files to a
single line. Files are kept in PageRank order, so the most central code survives
longest. Token counts are exact BPE (cl100k). Use `--focus` to protect the paths
you care about. More: the "Why it works" section of the
[README](../README.md#why-it-works).

## When should I commit the map vs. regenerate it?

Both are fine:

- **Commit `atlas-map.md`** so every contributor and agent starts oriented, and
  refresh it in a pre-commit hook or CI.
- **Regenerate on demand** when you just need a fresh map for a session — warm
  re-runs are cache-backed and finish in roughly ~80 ms, so it's cheap.

## How often should I regenerate it?

Whenever the *structure* changes — new files, moved modules, changed public
signatures. Day-to-day edits inside function bodies don't change the map.
`atlas diff` shows exactly what moved between two revisions if you want to gate
on it.

## What languages are supported?

Python, TypeScript/JavaScript, Rust, Go, Java, and C/C++. Full extension list
and per-language caveats: the README's "What it maps" section. Unsupported files
are skipped and counted, never fatal.

## Is it production-ready?

It's **alpha**. The core works end-to-end and is benchmark-tested, but the CLI
and output format may still change — pin a version if you depend on the output.
See [release readiness gates](release-readiness.md), [STATUS.md](../STATUS.md),
and [CHANGELOG.md](../CHANGELOG.md).

## What are the actual benchmark numbers?

In our agent-task benchmark, agents given an atlas map answered structural
questions using **~65% fewer tokens at identical accuracy** (20/20 correct with
and without the map), typically in a single turn instead of three. On open-ended
edit tasks the map cuts turns too, though token savings vary by task. The
methodology and ledger live in [benchmark/README.md](../benchmark/README.md) and
[benchmark/history.md](../benchmark/history.md).

## A symbol is wrong or missing — is that a bug?

Usually yes, a tree-sitter extraction bug. Please
[open an issue](https://github.com/fkenmar/atlas/issues/new?template=extraction_bug.md)
with a minimal snippet. If a *file* is missing, it's more often an unsupported
language or an ignored directory — see
[`docs/monorepos.md`](monorepos.md) and the
[README troubleshooting](../README.md#troubleshooting).

---

<sub>**Maintainer note:** when language support changes (a Tier promotion or a
new grammar), update the "What languages are supported?" answer here, the
README "What it maps" section, and the language docs together so they don't
drift.</sub>

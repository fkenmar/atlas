# Launch checklist

This checklist is for release candidates and public launch passes. It keeps repo
metadata, trust files, preview assets, and claims in sync.

## Release candidate

- [ ] `cargo fmt --all`
- [ ] `cargo clippy --all-targets -- -D warnings`
- [ ] `cargo test`
- [ ] Confirm README install commands match the release tag.
- [ ] Confirm `CHANGELOG.md` has the release notes.
- [ ] Confirm benchmark wording matches `benchmark/history.md`.
- [ ] Confirm no headline edit-task token claim is used unless a fresh benchmark
      clears the protocol in `benchmark/README.md`.
- [ ] Confirm README badges point at the release, CI, license, and supported
      language set users will see on `main`.

## Install channels

| Channel | Go/no-go check |
|---|---|
| PyPI / pipx | `pipx install --pre atlas-map` works in a fresh env, or the release notes clearly say the wheel is pending. |
| GitHub release artifacts | The release page has archives for the intended macOS, Linux, and Windows targets plus generated notes. |
| curl installer | The README installer URL matches the tag and downloads from the GitHub release. |
| `cargo install` | Crate metadata is current; if crates.io is not enabled for this release, docs say so. |
| Homebrew | If enabled, formula tap and install command are tested; if not enabled, README does not imply it is available. |

## Release artifacts and notes

- [ ] Run the release workflow plan/check job before tagging.
- [ ] Confirm generated release notes include the merged issues and do not
      overstate benchmark claims.
- [ ] Confirm binary names, archive names, checksums, and installer text match
      the release tag.
- [ ] Smoke test at least one binary archive locally with `atlas --version` and
      `atlas . --budget 600`.
- [ ] Smoke test `atlas diff HEAD~1 HEAD` on this repository.
- [ ] For pre-1.0 releases, include an alpha/stability note in release notes and
      README status text.

## GitHub repository metadata

Owner: the maintainer of `fkenmar/atlas` owns GitHub settings. Re-check these at
release-candidate freeze, immediately after trust files land on the default
branch, and after any homepage or docs-site change.

Live snapshot checked 2026-06-19:

- Description: `Fast Rust repo map for AI coding agents: token-budgeted signatures, imports, diff, and MCP.`
- Homepage: unset.
- Topics: includes `ai-agents`, `cli`, `code-map`, `coding-agents`,
  `developer-tools`, `llm`, `rust`, `static-analysis`, `tokens`,
  `tree-sitter`, `claude`, `chatgpt`, `mcp`, and related discovery terms.
- Public community profile: 71% before `SECURITY.md` and
  `CODE_OF_CONDUCT.md` land on the default branch.

Re-check commands:

```sh
gh repo view fkenmar/atlas --json description,homepageUrl,repositoryTopics
gh api repos/fkenmar/atlas/community/profile --jq '{health_percentage, files}'
```

## Social preview

- [ ] Confirm [assets/social-preview.svg](../assets/social-preview.svg) is the
      source of truth.
- [ ] Confirm [assets/social-preview.png](../assets/social-preview.png) is
      1280x640.
- [ ] Upload the PNG in GitHub repository settings under the social preview
      image control. GitHub does not expose this through the normal `gh repo
      edit` flow, so treat the UI upload as a maintainer-owned launch step.
- [ ] Re-open a repo link in an incognito browser or chat preview debugger after
      GitHub has cached the new image.

## Homepage

- [ ] Leave homepage unset until a stable launch page exists.
- [ ] When a GitHub Pages site or canonical release page exists, set it with:

```sh
gh repo edit fkenmar/atlas --homepage URL
```

- [ ] Link the homepage back to install docs and `llms.txt`.

## Community profile

- [ ] `README.md`
- [ ] `LICENSE`
- [ ] `CONTRIBUTING.md`
- [ ] `.github/ISSUE_TEMPLATE/bug_report.md`
- [ ] `.github/ISSUE_TEMPLATE/feature_request.md`
- [ ] `.github/PULL_REQUEST_TEMPLATE.md`
- [ ] `SECURITY.md`
- [ ] `CODE_OF_CONDUCT.md`

After merge to `main`, rerun the community profile command above and record the
new score in the launch issue or release checklist.

## Rollback and hotfix

- [ ] If install artifacts are broken, mark the GitHub release as pre-release or
      draft again while a fix is prepared.
- [ ] If a bad wheel was published, publish a patch/pre-release rather than
      deleting artifacts users may have cached.
- [ ] If README install commands are wrong, patch README first and link the
      correction from the release notes.
- [ ] If a security-sensitive packaging issue is found, follow
      [SECURITY.md](../SECURITY.md) and avoid public exploit detail until a fix
      or mitigation exists.

## Launch issue closure

- [ ] #117: `llms.txt` and `llms-full.txt` exist and are linked.
- [ ] #118: preview PNG exists; metadata checked; settings owner and re-check
      cadence documented.
- [ ] #119: outreach list, channel copy, objection tracking, and 7-day metrics
      checklist exist.

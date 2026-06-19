# Non-telemetry adoption metrics

atlas does not collect runtime analytics. Use public-source metrics for roadmap
reviews and launch retros instead of adding tracking to the binary.

## Metrics to collect

| Metric | Source | Manual command or location | Notes |
|---|---|---|---|
| GitHub stars, forks, watchers | GitHub repository metadata | `gh repo view fkenmar/atlas --json stargazerCount,forkCount,watchers` | Lightweight interest signal, not a product goal by itself. |
| Open issues and PRs | GitHub issues/PRs | `gh issue list --repo fkenmar/atlas --state open`; `gh pr list --repo fkenmar/atlas --state open` | Separate bug reports from roadmap ideas. |
| Release downloads | GitHub releases | `gh release list --repo fkenmar/atlas`; `gh api repos/fkenmar/atlas/releases --jq '.[] | {tag_name, assets: [.assets[] | {name, download_count}]}'` | Useful after tagged releases with binary assets. |
| PyPI downloads | Public PyPI stats provider | `pypistats recent atlas-map` if `pypistats` is installed | Third-party stats can lag and may be unavailable. |
| crates.io downloads | crates.io page/API | Check the crate page once publishing is enabled | Keep marked N/A until crates.io publishing is live. |
| Homebrew installs | Homebrew analytics | Check formula analytics once a formula exists | Keep marked N/A until Homebrew distribution is live. |
| Docs/landing traffic | GitHub Pages or host analytics | Host dashboard if a landing page exists | Do not add client-side analytics just for this. |
| Community feedback | HN, Reddit, MCP/client communities, issues | Summarize repeated objections in the launch retro | Convert repeated objections into docs/issues. |

## Collection cadence

Launch week:

- Day 0: record baseline public metrics before posting.
- Day 1: triage bugs and confusing docs from comments.
- Day 3: group repeated objections into FAQ/docs/backlog items.
- Day 7: write a short launch retro with public metrics and qualitative themes.

Ongoing:

- Monthly while the project is in alpha.
- Before and after tagged releases.
- Before prioritizing distribution or integration work.

## Roadmap use

Use these metrics to spot friction, not to chase vanity numbers:

- Many install questions -> improve install docs or release artifacts.
- Repeated missing-symbol reports -> prioritize query fixtures and language
  extraction tests.
- MCP setup confusion -> improve `docs/CLAUDE_CODE_MCP.md`.
- High release downloads but low issue activity -> add first-success prompts and
  contribution paths.

Do not embed tracking in the atlas binary to answer these questions.

## Related docs

- [Privacy and offline operation](PRIVACY.md)
- [Launch checklist](LAUNCH_CHECKLIST.md)
- [Post-launch outreach plan](post-launch-outreach.md)

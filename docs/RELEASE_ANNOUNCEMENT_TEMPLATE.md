# Release announcement template

Fill-in-the-blanks copy for announcing a release, so promotion is consistent and
never overstates the benchmark. Replace every `{{PLACEHOLDER}}`; delete variants
you don't use.

**Before publishing:** run the [launch checklist](LAUNCH_CHECKLIST.md), confirm
the benchmark wording matches [`benchmark/history.md`](../benchmark/history.md),
and confirm install commands match the tag. Mechanics of cutting the tag are in
[RELEASING.md](RELEASING.md). For richer, ready-to-edit drafts see
[social-launch.md](social-launch.md) (X/LinkedIn) and
[release-notes-draft.md](release-notes-draft.md) (a value-first GitHub note).

> **Honest-claim guardrails (keep these true):**
> - Headline only the **comprehension** benchmark (accuracy-gated, low-variance).
> - Do **not** headline edit-task token deltas (too noisy). Turns may be cited as
>   a tendency (~−25%), not a number.
> - Say it's **alpha**; state the offline/no-telemetry guarantee.

Placeholders used below: `{{VERSION}}` · `{{DATE}}` · `{{TOKENS_BEFORE}}` →
`{{TOKENS_AFTER}}` (`{{PCT}}`) · `{{ACCURACY}}` · `{{TURNS_BEFORE}}`→`{{TURNS_AFTER}}`
· `{{MODEL}}` · `{{TARGET_REPO}}` · `{{LANGS}}` · `{{HIGHLIGHTS}}`.

---

## Variant A — GitHub release notes

```markdown
## atlas {{VERSION}}

Give your AI coding agent a map of your repo so it stops burning tokens just
finding its way around. atlas compiles a codebase into a compact, ranked,
token-budgeted structural map — signatures, types, and imports, no bodies.

### Why it helps
In a comprehension benchmark ({{MODEL}}, {{TARGET_REPO}}), agents answered with
**identical accuracy ({{ACCURACY}})** using **{{PCT}} fewer tokens**
({{TOKENS_BEFORE}} → {{TOKENS_AFTER}}), typically in {{TURNS_AFTER}} turn(s)
instead of {{TURNS_BEFORE}}. Reproduce it:
https://github.com/fkenmar/atlas/blob/main/benchmark/README.md#reproduce-the-headline-number
Edit-task token deltas are too noisy to headline; we don't claim them.

### What's new in {{VERSION}}
{{HIGHLIGHTS}}

### Install
    pipx install --pre atlas-map        # or: pip install --pre atlas-map
    # prebuilt macOS/Linux/Windows binaries are attached below

Languages: {{LANGS}}. Local, offline, no telemetry. **Alpha** — pin a version if
you depend on the output.
```

## Variant B — short social post (X / Mastodon / LinkedIn teaser)

```text
atlas {{VERSION}} is out — a repo map for AI coding agents.

Same answers, {{PCT}} fewer tokens ({{TOKENS_BEFORE}}→{{TOKENS_AFTER}}, {{ACCURACY}}
accuracy) in our comprehension benchmark. Signatures + imports, no bodies. Rust,
local, offline. Alpha.

pipx install --pre atlas-map
https://github.com/fkenmar/atlas
```

(Longer, threaded X/LinkedIn versions live in [social-launch.md](social-launch.md).)

## Variant C — forum / Hacker News (Show HN style)

```text
Show HN: atlas {{VERSION}} – a token-budgeted repo map for AI coding agents

I built atlas to stop my coding agent from spending most of its tokens just
figuring out where things are. It compiles a repo into a ranked, ~2,048-token
structural map (every signature, type, and import edge; no function bodies) that
the agent reads up front.

Honest benchmark: on a comprehension suite ({{MODEL}}, {{TARGET_REPO}}), accuracy
held at {{ACCURACY}} while median tokens dropped {{TOKENS_BEFORE}} → {{TOKENS_AFTER}}
({{PCT}}) and turns {{TURNS_BEFORE}} → {{TURNS_AFTER}}. Harness + the verified
questions are in the repo so you can rerun it. On open-ended edit tasks the token
numbers are too noisy to claim a win, so I don't — I'll only say turns trend down.

It's a single Rust binary: Markdown/JSON/XML output, a structural diff, and an
MCP server. Local, offline, no telemetry. Languages: {{LANGS}}. It's alpha.

    pipx install --pre atlas-map

Repo: https://github.com/fkenmar/atlas
Feedback from folks using Claude Code / Cursor / Codex / Aider especially welcome.
```

---

<sub>Keep this template in sync with the README's headline numbers and
`benchmark/history.md`. When language support or install channels change, update
the `{{LANGS}}` / install lines here too.</sub>

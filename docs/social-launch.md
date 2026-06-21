# Social launch copy (X / LinkedIn)

Copy-paste drafts for the personal launch posts in
[`post-launch-outreach.md`](post-launch-outreach.md). Tell the build story, lead
with the honest number, and disclose that you're the maintainer. **Don't** ask
for stars/upvotes — let the tool earn them.

> Honesty guardrails (keep these true as the project moves):
> - The headline is the **comprehension** benchmark: same accuracy, ~65% fewer
>   tokens (85,670 → 29,781), 1 turn vs 3.
> - **Don't** claim an edit-task token win — that signal is too noisy (60–140%
>   run-to-run variance). You *can* say turns trend down ~25%.
> - It's **alpha**; say so.

## One-tweet version

```
I kept watching my coding agent burn thousands of tokens just figuring out where
things are in a repo.

So I built atlas: it compiles your codebase into a ranked, ~2k-token map
(signatures + imports, no bodies) the agent reads in one shot.

Same answers, ~65% fewer tokens. Rust, local, alpha 👇
github.com/fkenmar/atlas
```

## X / Twitter thread

```
1/ Coding agents spend most of their tokens on one boring thing: exploring.
Opening file after file just to learn where things live, before they do any
real work.

I built a small tool to skip that. It's called atlas. 🧵

2/ atlas compiles a whole repo into a compact, *ranked* map: every function
signature, type, and import edge — and zero function bodies.

It ranks files by how central they are (PageRank over the import graph) and packs
the most important ones into a token budget (~2,048 by default).

3/ The agent reads that map once and gets its bearings immediately, instead of
grepping around to build a mental model.

Think aider's repo-map idea, unbundled into a standalone CLI you can point at any
agent — or run as an MCP server.

4/ Does it actually help? I built a benchmark to keep myself honest.

On a comprehension test (20 verified questions, real repo), agents answered with
the SAME accuracy — 20/20 both ways — using ~65% fewer tokens: 85,670 → 29,781.
And in 1 turn instead of 3.

5/ The honest caveat: on open-ended *edit* tasks, the token numbers are too noisy
to claim a win (the agent's own randomness swings them 60–140%). Turns trend down
~25%, but I'm not going to dress up noise as a result.

The harness + questions are in the repo — run it yourself.

6/ It's a single fast Rust binary. Local, offline, no telemetry. Markdown / JSON
/ XML output, a structural `diff` mode, and a read-only MCP server.
Python, TS/JS, Rust, Go, Java, C/C++.

    pipx install --pre atlas-map
    atlas .

7/ It's alpha — the core works and is benchmark-tested, but the output may still
change.

I'd love feedback from anyone using Claude Code, Cursor, Codex, or Aider: does
this fit your workflow, and what would make it less annoying to keep fresh?

github.com/fkenmar/atlas
```

## LinkedIn post

```
I built a small tool to stop my AI coding agent from wasting tokens.

Here's the problem I kept hitting: when an agent works in an unfamiliar
codebase, most of its effort goes to *exploration* — opening file after file
just to figure out where things are — before it writes a single useful line.

So I built atlas. It compiles a repo into a compact, ranked map: every function
signature, type, and import edge, with no function bodies. It scores files by how
central they are to the codebase and packs the most important ones into a token
budget (~2,000 by default). The agent reads that once and starts oriented.

To avoid fooling myself, I made it benchmark-driven. The result I'll stand
behind: on a comprehension benchmark (20 verified questions against a real repo),
agents answered with identical accuracy — 20/20 with and without the map — using
about 65% fewer tokens, and usually in one turn instead of three.

The honest part: on open-ended edit tasks, the token numbers are too noisy to
claim a win, so I don't. The harness and the questions are in the repo if you
want to check the math yourself.

atlas is a single Rust binary — local, offline, no telemetry — with Markdown /
JSON / XML output, a structural diff mode, and an MCP server for agents that
support it. It supports Python, TypeScript/JavaScript, Rust, Go, Java, and C/C++.
It's alpha, and I'm building in the open.

Disclosure: I'm the author. If you work with coding agents, I'd genuinely value
your feedback on whether this fits your workflow.

    pipx install --pre atlas-map

GitHub: https://github.com/fkenmar/atlas
```

## Reminders before you post

- **Be present.** Block the hour after posting to answer replies — that's where
  the real value (and credibility) is.
- **One post per channel.** Cross-posting the same link everywhere in an hour
  reads as spam.
- **Lead with the problem,** not the feature list. People share things that name
  a pain they feel.
- **Pin the benchmark link.** Reproducibility is your trust anchor with a
  skeptical audience.

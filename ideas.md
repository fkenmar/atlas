# ideas.md — the scope-creep parking lot

Out-of-scope ideas land here instead of in code. The non-goals in CLAUDE.md
(semantic search/embeddings, LSP-style code intelligence, IDE plugins/GUI,
code editing/generation, languages beyond the Tier 1/2 list) are a contract:
when a session produces an idea that violates it — or one that's simply not
on the current milestone — append it below with a one-line rationale and move
on. `/scope-check` files entries here automatically. Reviewed during
`/weekly-review`; nothing here is a commitment.

Format: `- **idea** — why it's parked (date)`

## Parked

- **Reinforcement learning for ranking/budgeting** — evaluated 2026-06-16 at the
  maintainer's request ("incorporate RL *only if it helps and fits*"). Verdict:
  **does not fit.** The one place RL is even conceptually relevant is learning the
  rank/pack policy against the benchmark's reward signal (exploration-token
  reduction / comprehension accuracy). It's the wrong tool here because: (1) it
  collides with the **structural-only, no-ML scope contract** (PRD §3.2) and the
  **determinism + explainability** posture — a trained policy is an opaque shipped
  artifact, the opposite of a debuggable hand-tuned ranker agents must trust; (2)
  it's **cost-infeasible** — each reward eval is a billed `claude -p` session
  (~$0.3–1.5); a policy needs hundreds–thousands of episodes = $100s–$1000s on a
  solo ~10 hr/week project; (3) the reward set is **2 tasks (target 10), N=3
  untrusted** — RL would overfit those tasks, not generalize to arbitrary repos;
  (4) the actual wins being chased are about *what data is in the map* (reverse-refs,
  already added) not about needing a learned ranker. **The useful kernel already
  exists**: the self-improvement loop (docs/SELF_IMPROVEMENT.md) is a lightweight,
  human-gated optimization *over the design space* against that same reward — try a
  change, measure, keep-or-revert. That's the right granularity; formalizing its
  keep/revert as a tiny deterministic bandit over candidate ranking-weight configs
  (no model shipped, output stays byte-deterministic) is the only RL-adjacent idea
  worth a future look, and only after the suite is trustworthy (#1, #6).

- **Interactive TUI for browsing the map** — proposed 2026-06-17 to "increase
  accessibility." Parked: it optimizes the wrong consumer. atlas's output is built
  for an *LLM agent's context* (`atlas . > map.md` → fed to the agent), not for a
  human to sit and browse — so a full-screen interactive UI doesn't lower the
  barrier to the actual workflow, it adds a human-facing surface the workflow
  doesn't use. It also leans against the **"CLI and MCP only — no IDE plugins or
  GUI"** non-goal (PRD §3.2) in spirit, and a browse-the-symbols TUI drifts toward
  the **LSP/IDE code-intelligence** non-goal too. Cost is real: ratatui + crossterm
  (new deps, approval-gated), a whole interaction layer to maintain on a solo
  ~10 hr/week alpha, competing with the roadmap. **The in-scope way to "make atlas
  easy to use" is the MCP server (#7)** — that's how an agent pulls a fresh map as a
  tool call (zero human friction), plus CLI-native polish (TTY-aware colorized
  output, shell completions) that stays a pipe, not an app. Revisit only if real
  users ask to *inspect* maps by hand and the MCP/CLI path has shipped.

- **Cross-turn / session "delta map" injection** — researched 2026-06-20
  (deep-research wf_f77927e7) as a way to "improve the agent's memory + cut tokens"
  by emitting only the structural change since the agent last saw the map.
  **Verdict: low value as a context-injection feature — it does not beat prompt
  caching.** Confirmed evidence: (1) a budgeted map placed in the cache-stable
  prefix is re-read at ~10% of input cost (Anthropic prompt caching: cache reads =
  0.1× input price), and caching already cuts agentic cost 41–80% — so the per-turn
  re-read this would "save" is already cheap; (2) caching is prefix-based, so a delta
  that *changes every turn* would itself invalidate the cached prefix and be re-paid
  at full input cost each turn — plausibly worse than re-reading the cached full map;
  (3) the only regime where a delta wins is cross-session / after the 5-min cache TTL
  expires / after context-editing eviction — and that *cross-session structural delta
  is already shipped* as `atlas diff <old> <new>`. **In-scope alternative already
  chosen:** make the committed map cache-stable by default (`atlas --check`, shipped
  2026-06-20) and build progressive disclosure / lazy symbol expansion over MCP — the
  one lever that attacks context-window *occupancy*, which caching does not fix.
  Revisit only if a measured cross-session injection workflow shows the full-map
  re-send cost is material.

## Tried and reverted

The self-improvement loop (docs/SELF_IMPROVEMENT.md) appends reverted
approaches here with their numbers, so no future iteration retries them
blind. Format: `- **approach** — why it regressed, with stats (date)`

*(empty)*

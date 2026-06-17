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

## Tried and reverted

The self-improvement loop (docs/SELF_IMPROVEMENT.md) appends reverted
approaches here with their numbers, so no future iteration retries them
blind. Format: `- **approach** — why it regressed, with stats (date)`

*(empty)*

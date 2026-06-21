## What & why

<!-- What does this change, and why? Link any issue, e.g. "Closes #12". -->

## Checklist

- [ ] `cargo fmt --all` clean
- [ ] `cargo clippy --all-targets -- -D warnings` clean
- [ ] `cargo test` green
- [ ] Determinism preserved (sorted iteration; no `HashMap`-order reliance)
- [ ] No new `.unwrap()` / `.expect()` outside tests
- [ ] New dependency? Approved by a maintainer, and justified below against the [dependency policy](../CONTRIBUTING.md#dependencies)
- [ ] Ranking/budgeting change? Benchmark delta noted below

## Benchmark delta

<!-- Ranking/budgeting changes only: exploration-token / accuracy numbers vs
     baseline, per benchmark/README.md. Delete this section otherwise. -->

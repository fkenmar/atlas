#!/usr/bin/env python3
"""Deterministic answer-in-map coverage proxy for the comprehension benchmark.

For each comprehension question, check whether its *verified answer key* (the
`expect` substrings, scored by `match: any|all`) appears in the atlas map of the
pinned repo. Coverage = fraction of questions whose answer is present in the map.

Why this exists
---------------
The comprehension benchmark established that an answer **present in the map**
yields a one-turn correct response, while a missing answer forces the agent to
grep (more turns, more tokens). So "is the answer in the map?" tracks the same
signal the billed `claude -p` comprehension run measures — but it is FREE,
deterministic, and runs in milliseconds. Use it as the fast inner-loop metric
when tuning ranking/budgeting (e.g. per-symbol ranking): watch coverage move,
keep what raises it, then confirm the keep/revert at a milestone with a real
billed run. It does NOT replace the billed run — an answer in the map could
still be misread — it makes the iteration between runs cheap and objective.

Usage (paths default relative to benchmark/, mirroring comprehension.sh):
  python3 benchmark/coverage.py
  python3 benchmark/coverage.py comprehension/questions-smoke.yaml --budget 2048,3072
  python3 benchmark/coverage.py --atlas ../target/release/atlas --repo /path/to/clone

Exit status: 0 always (informational), unless --min-coverage is set and unmet.
Requires: python3 with PyYAML, git, and a built atlas binary.
"""
from __future__ import annotations

import argparse
import os
import subprocess
import sys
from pathlib import Path

try:
    import yaml
except ImportError:
    sys.exit("error: python3 needs PyYAML (pip install pyyaml)")

BENCH_DIR = Path(__file__).resolve().parent


def load_questions(qfile: Path) -> dict:
    if not qfile.is_file() or qfile.stat().st_size == 0:
        sys.exit(f"error: no such question file: {qfile}")
    return yaml.safe_load(qfile.read_text())


def ensure_repo(spec: dict, qfile: Path, override: str | None) -> Path:
    """Use --repo if given, else clone the pinned repo into .work/cache (the
    same shallow, rev-pinned clone comprehension.sh uses), caching across runs."""
    if override:
        repo = Path(override).expanduser().resolve()
        if not repo.is_dir():
            sys.exit(f"error: --repo is not a directory: {repo}")
        return repo
    url, rev = spec["repo"]["url"], str(spec["repo"]["rev"])
    cache = BENCH_DIR / ".work" / "cache" / f"coverage-{qfile.stem}"
    if not (cache / ".git").is_dir():
        print(f"cloning {url}@{rev} -> {cache}", file=sys.stderr)
        cache.parent.mkdir(parents=True, exist_ok=True)
        if cache.exists():
            subprocess.run(["rm", "-rf", str(cache)], check=True)
        subprocess.run(
            ["git", "clone", "--quiet", "--depth", "1", "--branch", rev, url, str(cache)],
            check=True,
        )
    return cache


def render_map(atlas: Path, repo: Path, budget: int) -> str:
    if not atlas.exists():
        sys.exit(f"error: atlas binary not found: {atlas} (cargo build --release?)")
    out = subprocess.run(
        [str(atlas), str(repo), "--budget", str(budget)],
        capture_output=True,
        text=True,
    )
    if out.returncode != 0:
        sys.exit(f"error: atlas failed (exit {out.returncode}):\n{out.stderr}")
    return out.stdout


def question_covered(q: dict, map_text: str) -> bool:
    """A question is 'covered' iff its answer key is satisfiable from the map
    alone, using the same any/all substring rule the agent's answer is scored by."""
    expect = [str(e) for e in q.get("expect", [])]
    if not expect:
        return False
    hits = [e for e in expect if e in map_text]
    return len(hits) == len(expect) if q.get("match", "any") == "all" else len(hits) > 0


def run_budget(spec: dict, repo: Path, atlas: Path, budget: int, verbose: bool) -> float:
    questions = spec["questions"]
    map_text = render_map(atlas, repo, budget)
    covered = 0
    for q in questions:
        ok = question_covered(q, map_text)
        covered += ok
        if verbose:
            print(f"  [{'HIT ' if ok else 'MISS'}] {q['id']}: {q.get('expect')}")
    pct = 100.0 * covered / len(questions)
    print(f"budget {budget:>5}: {covered}/{len(questions)} answers in map ({pct:.1f}%)")
    return pct


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("qfile", nargs="?", default="comprehension/questions-pytest-8.2.0.yaml",
                    help="question YAML (relative to benchmark/ by default)")
    ap.add_argument("--budget", default="2048",
                    help="token budget, or a comma list to sweep, e.g. 2048,3072")
    ap.add_argument("--atlas", default=os.environ.get("ATLAS_BIN", "../target/release/atlas"),
                    help="atlas binary (default: ../target/release/atlas, relative to benchmark/)")
    ap.add_argument("--repo", default=None, help="pre-checked-out repo dir (skip the pinned clone)")
    ap.add_argument("--verbose", "-v", action="store_true", help="per-question HIT/MISS")
    ap.add_argument("--min-coverage", type=float, default=None,
                    help="exit 1 if the lowest swept budget's coverage is below this %%")
    args = ap.parse_args()

    qfile = (BENCH_DIR / args.qfile).resolve() if not os.path.isabs(args.qfile) else Path(args.qfile)
    atlas = (BENCH_DIR / args.atlas).resolve() if not os.path.isabs(args.atlas) else Path(args.atlas)
    spec = load_questions(qfile)
    repo = ensure_repo(spec, qfile, args.repo)
    budgets = [int(b) for b in str(args.budget).split(",") if b.strip()]

    print(f"answer-in-map coverage · {qfile.name} · {len(spec['questions'])} questions · {repo.name}")
    lowest = 100.0
    for b in budgets:
        lowest = min(lowest, run_budget(spec, repo, atlas, b, args.verbose))

    if args.min_coverage is not None and lowest < args.min_coverage:
        print(f"FAIL: coverage {lowest:.1f}% < required {args.min_coverage:.1f}%", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

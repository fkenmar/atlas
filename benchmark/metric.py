#!/usr/bin/env python3
"""Compute the refined exploration-token metric from a claude -p stream.

Reads a `--output-format stream-json` JSONL session on stdin and emits one
JSON object on stdout. The headline number is `exploration_tokens`: the total
input-side tokens (fresh input + cache creation + cache reads) the agent
processed *up to and including the turn of its first file edit*. That isolates
the exploration phase the repo map is meant to shrink, and — unlike the old
whole-session proxy — it stops counting at the first edit, so verification and
retry turns (and a 30-turn cap blowout) no longer dominate the number.

Also emitted, for context and back-compat: `total_tokens` (same sum across the
whole session — the old metric), `turns_to_first_edit`, `num_turns`,
`cost_usd`, and `edited` (false if the agent never edited — then exploration
equals total).
"""
import json
import sys

# Tool names that count as "the agent started editing".
EDIT_TOOLS = {"Edit", "Write", "MultiEdit", "NotebookEdit", "create_file", "str_replace"}


def input_side(usage):
    if not isinstance(usage, dict):
        return 0
    return (
        (usage.get("input_tokens") or 0)
        + (usage.get("cache_creation_input_tokens") or 0)
        + (usage.get("cache_read_input_tokens") or 0)
    )


def main():
    exploration = 0
    total = 0
    turn = 0
    turns_to_first_edit = None
    first_edit_seen = False
    cost_usd = None
    num_turns = None

    for line in sys.stdin:
        line = line.strip()
        if not line:
            continue
        try:
            ev = json.loads(line)
        except json.JSONDecodeError:
            continue
        etype = ev.get("type")
        if etype == "assistant":
            msg = ev.get("message", {}) or {}
            tokens = input_side(msg.get("usage"))
            turn += 1
            total += tokens
            if not first_edit_seen:
                exploration += tokens
                for block in msg.get("content", []) or []:
                    if (
                        isinstance(block, dict)
                        and block.get("type") == "tool_use"
                        and block.get("name") in EDIT_TOOLS
                    ):
                        first_edit_seen = True
                        turns_to_first_edit = turn
                        break
        elif etype == "result":
            cost_usd = ev.get("total_cost_usd")
            num_turns = ev.get("num_turns")

    if not first_edit_seen:
        # The agent never edited — its whole session was exploration.
        turns_to_first_edit = turn

    print(
        json.dumps(
            {
                "exploration_tokens": exploration,
                "total_tokens": total,
                "turns_to_first_edit": turns_to_first_edit,
                "num_turns": num_turns if num_turns is not None else turn,
                "cost_usd": cost_usd,
                "edited": first_edit_seen,
            }
        )
    )


if __name__ == "__main__":
    main()

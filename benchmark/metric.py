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

Exits nonzero (printing nothing on stdout) when the stream is malformed or
empty — no `result` event or no observed usage — so a crashed `claude` or a
format change cannot masquerade as a perfect zero-token run; the caller then
records the run as null/failed rather than 0. NOTE: this counts the first
edit, not the first *correct* edit (PRD §8), so the caller must take medians
over passing runs only — a run that edits early but fails is not a valid
exploration sample.
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
    saw_result = False
    result_subtype = None

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
            saw_result = True
            result_subtype = ev.get("subtype")
            cost_usd = ev.get("total_cost_usd")
            num_turns = ev.get("num_turns")

    # Reject a malformed / failed / empty session rather than reporting a
    # misleading 0 — a crashed `claude`, a format change, or an empty stream
    # must NOT look like a perfect zero-token run. The caller records null.
    if not saw_result or total <= 0 or turn == 0:
        sys.stderr.write(
            "metric.py: no result event or no observed usage "
            f"(saw_result={saw_result}, turns={turn}, total_tokens={total}) "
            "— treating as a failed session\n"
        )
        sys.exit(1)

    if not first_edit_seen:
        # The agent never edited — its whole session was exploration.
        turns_to_first_edit = turn

    print(
        json.dumps(
            {
                "exploration_tokens": exploration,
                "total_tokens": total,
                # Counted in ASSISTANT MESSAGES, not claude `num_turns` — a turn
                # with extended thinking emits a separate thinking message, so
                # this scale exceeds num_turns. Don't compare the two.
                "assistant_msgs_to_first_edit": turns_to_first_edit,
                "assistant_msgs": turn,
                "num_turns": num_turns if num_turns is not None else turn,
                # error_max_turns = the session hit the turn cap (incomplete);
                # the caller drops these from medians — their token counts are
                # cap-contaminated, the main variance source.
                "capped": result_subtype == "error_max_turns",
                "result_subtype": result_subtype,
                "cost_usd": cost_usd,
                "edited": first_edit_seen,
            }
        )
    )


if __name__ == "__main__":
    main()

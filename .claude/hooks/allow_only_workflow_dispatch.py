#!/usr/bin/env python3
"""PreToolUse hook: let `actions_run_trigger` start a workflow and do nothing else (Decision 191).

The owner's token has Actions write so sessions can start `github-sync`, `bench` and the other
dispatched workflows. GitHub's one MCP tool for that also reruns and cancels runs and deletes run
logs, and a deny rule can only block the whole tool. This hook allows `method: run_workflow` and
blocks every other method. Exit 2 blocks the call and shows stderr to Claude; exit 0 allows it.
"""

import json
import sys

ALLOWED = "run_workflow"

GUIDE = (
    "actions_run_trigger may only start a workflow (method: run_workflow), Decision 191.\n"
    "Rerunning or cancelling a run, or deleting its logs, is the owner's call: ask them."
)


def main() -> int:
    try:
        event = json.load(sys.stdin)
        method = event["tool_input"].get("method")
    except (ValueError, KeyError, TypeError, AttributeError) as exc:
        # Fail closed: an unreadable event must not let a call through unchecked.
        print(f"allow_only_workflow_dispatch: could not read the event ({exc!r})", file=sys.stderr)
        return 2

    if method != ALLOWED:
        print(GUIDE, file=sys.stderr)
        return 2
    return 0


if __name__ == "__main__":
    sys.exit(main())

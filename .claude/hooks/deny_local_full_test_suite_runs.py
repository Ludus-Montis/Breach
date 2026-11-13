#!/usr/bin/env python3
"""PreToolUse hook: block a local run of the full Godot suite, which is CI's job (CLAUDE.md).

`ci/godot/scripts/run_tests.sh` runs the full suite when it gets no test path, so a call
whose first argument is not under `tests/` is blocked, and Claude is told the one-file
command instead. Exit 2 blocks the call and shows stderr to Claude; exit 0 allows it.

Known limit: a wrapper in front of the command (`env`, `time`, `timeout 600`) is not
recognised. The settings deny rules and CI are the other layers.
"""

import json
import re
import sys

DENY = re.compile(
    r"(?:^|[;&|(\n])\s*"                         # command position
    r"(?:\w+=\S*\s+)*"                           # optional VAR=value prefixes
    r"(?:(?:ba|z)?sh(?:\s+-\S+)*\s+)?"           # optional bash/sh/zsh (+ flags)
    r"(?:\S*/)?ci/godot/scripts/run_tests\.sh"   # the script, any path prefix
    r"(?!\s+(?:res://)?tests/)"                  # allowed: first arg is under tests/
)

GUIDE = (
    "The full suite runs in CI, not in a session (CLAUDE.md). Run one file instead:\n"
    "  bash ci/godot/scripts/run_tests.sh tests/test_<name>.gd"
)


def main() -> int:
    try:
        event = json.load(sys.stdin)
        command = event["tool_input"]["command"]
    except (ValueError, KeyError, TypeError) as exc:
        # Fail closed: an unreadable event must not let a command through unchecked.
        print(f"deny_local_full_test_suite_runs: could not read the event ({exc!r})", file=sys.stderr)
        return 2

    if DENY.search(command):
        print(GUIDE, file=sys.stderr)
        return 2
    return 0


if __name__ == "__main__":
    sys.exit(main())

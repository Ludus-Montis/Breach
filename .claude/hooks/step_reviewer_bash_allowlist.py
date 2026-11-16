#!/usr/bin/env python3
"""PreToolUse hook for the step-reviewer agent: its Bash may run read-only commands only.

The reviewer needs the step's PR, diff and issue, nothing more from the shell
(`.claude/agents/step-reviewer.md`). A command is allowed only when it is one of:

  gh pr diff <N> <args>        N a bare number, without `--web`
  gh pr view <N> <args>        N a bare number, without `--web`
  gh issue view <N> <args>     N a bare number, without `--web`
  git diff <args>              without `--output` (writes a file) or `--ext-diff`
                               (runs another program)

and has no chaining, pipes, redirection, substitution, comments or newlines.
Exit 2 blocks the call and shows stderr to the agent; exit 0 allows it.
"""

import json
import re
import shlex
import sys

SHELL_META = re.compile(r"[;&|<>`$\n\r()#]")
GIT_DIFF_BLOCKED = ("--output", "--ext-diff")
GH_WEB = ("--web", "-w")
GH_READS = (["gh", "issue", "view"], ["gh", "pr", "view"], ["gh", "pr", "diff"])

GUIDE = (
    "The step reviewer's shell runs these only, one at a time, no pipes or redirection:\n"
    "  gh pr diff <PR> [--name-only]   the step's change, as GitHub shows it\n"
    "  gh pr view <PR> [--comments]    the PR body, Documentation block and comments\n"
    "  gh issue view <N> [--comments]  the step's issue\n"
    "  git diff <base>...<head>        a local diff (--stat for the file list)\n"
    "Use Read, Grep and Glob for everything else."
)


def _git_diff(args: list[str]) -> bool:
    return not any(a.startswith(GIT_DIFF_BLOCKED) for a in args)


def _gh_read(args: list[str]) -> bool:
    if not args or not args[0].isdigit():
        return False
    return not any(a.startswith(GH_WEB) for a in args[1:])


def allowed(command: str) -> bool:
    if SHELL_META.search(command):
        return False
    try:
        words = shlex.split(command)
    except ValueError:
        return False
    if words[:2] == ["git", "diff"]:
        return _git_diff(words[2:])
    if words[:3] in GH_READS:
        return _gh_read(words[3:])
    return False


def main() -> int:
    try:
        event = json.load(sys.stdin)
        command = event["tool_input"]["command"]
    except (ValueError, KeyError, TypeError) as exc:
        # Fail closed: an unreadable event must not let a command through unchecked.
        print(f"step_reviewer_bash_allowlist: could not read the event ({exc!r})", file=sys.stderr)
        return 2

    if not isinstance(command, str) or not allowed(command):
        print(GUIDE, file=sys.stderr)
        return 2
    return 0


if __name__ == "__main__":
    sys.exit(main())

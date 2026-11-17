"""Tests for the deny_gh_api_writes hook, run as the harness runs it."""

import json
import subprocess
import sys
import unittest
from pathlib import Path

HOOK = Path(__file__).with_name("deny_gh_api_writes.py")
PR = "repos/Ludus-Montis/Breach/pulls/1043"
COMMENTS = "repos/Ludus-Montis/Breach/issues/1043/comments"


def run_hook(stdin: str) -> subprocess.CompletedProcess:
    return subprocess.run(
        [sys.executable, str(HOOK)], input=stdin, text=True, capture_output=True, check=False
    )


def run_command(command: str) -> subprocess.CompletedProcess:
    event = {"hook_event_name": "PreToolUse", "tool_name": "Bash", "tool_input": {"command": command}}
    return run_hook(json.dumps(event))


class DenyGhApiWritesTests(unittest.TestCase):
    def test_reads_are_allowed(self):
        for command in [
            "gh api -X GET search/code -f q='rubric-judge repo:Ludus-Montis/Breach' --jq .total_count",
            "gh api search/code -X GET -f q=x",
            "gh api search/code --method=get -F per_page=5",
            f"gh api {PR}",
            f"gh api {PR} --jq '.title' -q .body -H 'Accept: application/json'",
            f"gh api --paginate {PR}/files",
            f"gh pr view 1043",
            "git status && gh api user",
            "gh api repos/o/r/pulls/1",
            "gh api -X GET search/code -f q=foo",
            "echo done",
            "echo don't use gh pr merge",
        ]:
            with self.subTest(command=command):
                result = run_command(command)
                self.assertEqual(result.returncode, 0, result.stderr)

    def test_writes_are_blocked_however_they_are_spelled(self):
        for command in [
            f"gh api {COMMENTS} -f body=hi",  # fields with no method: gh sends POST
            f"gh api {COMMENTS} -fbody=hi",
            f"gh api {COMMENTS} -F body=@file.txt",
            f"gh api {COMMENTS} --field body=hi",
            f"gh api {COMMENTS} --raw-field=body=hi",
            f"gh api {COMMENTS} --input body.json",
            f"gh api -X PATCH {PR} -f title=x",
            f"gh api {PR} -XPATCH -f title=x",
            f"gh api {PR} -X patch -f title=x",
            f"gh api {PR} -X=PATCH",
            f"gh api {PR} --method=PATCH -f title=x",
            f"gh api {PR} --method delete",
            f"gh api {PR} -iXPATCH",  # combined short flags
            f"git status && gh api {COMMENTS} -f body=hi",
            f"git status\ngh api {COMMENTS} -f body=hi",
            f"GH_TOKEN=x gh api {COMMENTS} -f body=hi",
            f"env GH_DEBUG=1 gh api {COMMENTS} -f body=hi",
            f"/usr/bin/gh api {COMMENTS} -f body=hi",
            f"bash -c 'gh api {COMMENTS} -f body=hi'",
            f"echo $(gh api {COMMENTS} -f body=hi)",
            f"eval \"gh api {COMMENTS} -f body=hi\"",
            f"gh api {PR} -X GET -X PATCH",  # the last method given wins
        ]:
            with self.subTest(command=command):
                result = run_command(command)
                self.assertEqual(result.returncode, 2, command)
                self.assertIn("Decision 191", result.stderr)

    def test_a_gh_api_the_parser_never_finds_is_blocked(self):
        # The line names gh and api, but the parser finds fewer gh api calls than that:
        # a wrapper or indirection it doesn't follow. Fail closed without listing wrappers.
        for command in [
            "timeout 10 gh api repos/o/r/issues/1/comments -f body=x",
            "xargs -I{} gh api repos/o/r/issues/{}/comments -f body=x",
            "env -i gh api x -f a=b",
            "sudo -u me gh api x -XPOST",
            "nice -n 5 gh api x -f a=b",
            "G=gh; $G api x -f a=b",
            "timeout 10 gh api -X GET search/code -f q=foo",  # even a read, when it can't be seen
            "grep -n 'gh api x -f a=b' notes.md",  # text that mentions it: use the Grep tool
            "gh api repos/o/r/pulls/1; echo 'gh api x -f a=b'",
        ]:
            with self.subTest(command=command):
                result = run_command(command)
                self.assertEqual(result.returncode, 2, command)
                self.assertIn("Decision 191", result.stderr)

    def test_an_unparseable_gh_command_is_blocked(self):
        for command in [
            f"gh api {COMMENTS} -f body='unclosed",
            f"gh api {PR} -Z",  # a flag the hook doesn't know
        ]:
            with self.subTest(command=command):
                self.assertEqual(run_command(command).returncode, 2)

    def test_an_unreadable_event_is_blocked(self):
        for stdin in ["", "not json", "[]", json.dumps({"tool_name": "Bash"})]:
            with self.subTest(stdin=stdin):
                self.assertEqual(run_hook(stdin).returncode, 2)


if __name__ == "__main__":
    unittest.main()

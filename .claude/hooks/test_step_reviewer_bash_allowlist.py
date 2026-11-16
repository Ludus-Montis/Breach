"""Tests for the step_reviewer_bash_allowlist hook, run as the harness runs it."""

import json
import subprocess
import sys
import unittest
from pathlib import Path

HOOK = Path(__file__).with_name("step_reviewer_bash_allowlist.py")


def run_hook(stdin: str) -> subprocess.CompletedProcess:
    return subprocess.run(
        [sys.executable, str(HOOK)], input=stdin, text=True, capture_output=True, check=False
    )


def run_command(command: str) -> subprocess.CompletedProcess:
    event = {"hook_event_name": "PreToolUse", "tool_name": "Bash", "tool_input": {"command": command}}
    return run_hook(json.dumps(event))


class StepReviewerBashAllowlistTests(unittest.TestCase):
    def test_plain_git_diff_commands_are_allowed(self):
        for command in [
            "git diff",
            "git diff main...HEAD",
            "git diff --stat main...HEAD",
            "git diff main...HEAD -- sim/ tests/test_foo.gd",
            "  git diff --name-only origin/main...HEAD  ",
            "git diff -U5 main...HEAD -- 'content/a b.json'",
        ]:
            with self.subTest(command=command):
                result = run_command(command)
                self.assertEqual(result.returncode, 0, result.stderr)

    def test_reading_an_issue_is_allowed(self):
        for command in [
            "gh issue view 1042",
            "gh issue view 1042 --comments",
            "gh issue view 1042 --json title,body,labels",
        ]:
            with self.subTest(command=command):
                result = run_command(command)
                self.assertEqual(result.returncode, 0, result.stderr)

    def test_reading_a_pr_is_allowed(self):
        for command in [
            "gh pr view 1043",
            "gh pr view 1043 --comments",
            "gh pr view 1043 --json body,files",
            "gh pr diff 1043",
            "gh pr diff 1043 --name-only",
            "gh pr diff 1043 --patch",
        ]:
            with self.subTest(command=command):
                result = run_command(command)
                self.assertEqual(result.returncode, 0, result.stderr)

    def test_pr_reads_need_a_number_and_no_web(self):
        for command in [
            "gh pr view",
            "gh pr diff",
            "gh pr view feature/x",
            "gh pr view 1043 --web",
            "gh pr diff 1043 -w",
            "gh pr comment 1043 --body x",
            "gh pr review 1043 --approve",
            "gh pr checkout 1043",
            "gh pr ready 1043",
        ]:
            with self.subTest(command=command):
                self.assertEqual(run_command(command).returncode, 2)

    def test_other_gh_commands_are_blocked(self):
        for command in [
            "gh issue view",
            "gh issue view #1042",
            "gh issue view 1042 --web",
            "gh issue edit 1042 --body x",
            "gh issue comment 1042 --body x",
            "gh issue close 1042",
            "gh pr merge 1",
            "gh api repos/x/y/issues/1 -X PATCH",
        ]:
            with self.subTest(command=command):
                self.assertEqual(run_command(command).returncode, 2)

    def test_other_commands_are_blocked_with_the_guide(self):
        for command in [
            "ls",
            "git status",
            "git log",
            "git commit -m x",
            "gitdiff",
            "git difftool",
            "cat README.md",
        ]:
            with self.subTest(command=command):
                result = run_command(command)
                self.assertEqual(result.returncode, 2)
                self.assertIn("git diff", result.stderr)

    def test_chaining_substitution_and_redirection_are_blocked(self):
        for command in [
            "git diff; rm -rf x",
            "git diff && git push",
            "git diff || true",
            "git diff | tee out.txt",
            "git diff > out.patch",
            "git diff < in",
            "git diff $(rm x)",
            "git diff `rm x`",
            "git diff &",
            "git diff\nrm x",
        ]:
            with self.subTest(command=command):
                self.assertEqual(run_command(command).returncode, 2)

    def test_options_that_write_files_or_run_programs_are_blocked(self):
        for command in [
            "git diff --output=out.patch",
            "git diff --output out.patch",
            "git diff --ext-diff",
            "git -c diff.external=x diff",
        ]:
            with self.subTest(command=command):
                self.assertEqual(run_command(command).returncode, 2)

    def test_an_unreadable_event_fails_closed(self):
        for stdin in ["not json", "{}", json.dumps({"tool_input": {}})]:
            with self.subTest(stdin=stdin):
                self.assertEqual(run_hook(stdin).returncode, 2)


if __name__ == "__main__":
    unittest.main()

"""Tests for the deny_local_full_test_suite_runs hook, run as the harness runs it."""

import json
import subprocess
import sys
import unittest
from pathlib import Path

HOOK = Path(__file__).with_name("deny_local_full_test_suite_runs.py")
SCRIPT = "ci/godot/scripts/run_tests.sh"


def run_hook(stdin: str) -> subprocess.CompletedProcess:
    return subprocess.run(
        [sys.executable, str(HOOK)], input=stdin, text=True, capture_output=True, check=False
    )


def run_command(command: str) -> subprocess.CompletedProcess:
    event = {"hook_event_name": "PreToolUse", "tool_name": "Bash", "tool_input": {"command": command}}
    return run_hook(json.dumps(event))


class DenyLocalFullTestSuiteRunsTests(unittest.TestCase):
    def test_full_suite_runs_are_blocked_with_the_one_file_command(self):
        for command in [
            SCRIPT,
            f"bash {SCRIPT}",
            f"./{SCRIPT}",
            f"sh {SCRIPT}",
            f"bash {SCRIPT} && ls -a",
            f"bash {SCRIPT} 2>&1 | tail -20",
            f"bash {SCRIPT}; echo done",
            f"cd x && bash {SCRIPT}",
            # Stricter on purpose: -a is gdUnit's flag, not this script's, and runs nothing.
            f"{SCRIPT} -a",
            f"bash {SCRIPT} -a",
        ]:
            with self.subTest(command=command):
                result = run_command(command)
                self.assertEqual(result.returncode, 2)
                self.assertIn(f"bash {SCRIPT} tests/test_<name>.gd", result.stderr)

    def test_runs_naming_test_files_and_other_commands_are_allowed(self):
        for command in [
            f"{SCRIPT} tests/test_example.gd",
            f"bash {SCRIPT} tests/test_example.gd",
            f"{SCRIPT} tests/test_a.gd tests/test_b.gd",
            f"{SCRIPT} tests/",
            f"bash {SCRIPT} tests/test_example.gd && echo done",
            f"echo preparing && {SCRIPT} tests/test_example.gd",
            f"cat {SCRIPT}",
            f'echo "{SCRIPT}"',
            "git status",
        ]:
            with self.subTest(command=command):
                result = run_command(command)
                self.assertEqual(result.returncode, 0, result.stderr)

    def test_an_unreadable_event_is_blocked_not_allowed(self):
        for stdin in ["not json", "{}", json.dumps({"tool_input": {}}), "[]"]:
            with self.subTest(stdin=stdin):
                self.assertEqual(run_hook(stdin).returncode, 2)


if __name__ == "__main__":
    unittest.main()

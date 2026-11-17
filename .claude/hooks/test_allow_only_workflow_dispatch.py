"""Tests for the allow_only_workflow_dispatch hook, run as the harness runs it."""

import json
import subprocess
import sys
import unittest
from pathlib import Path

HOOK = Path(__file__).with_name("allow_only_workflow_dispatch.py")
TOOL = "mcp__github__actions_run_trigger"


def run_hook(stdin: str) -> subprocess.CompletedProcess:
    return subprocess.run(
        [sys.executable, str(HOOK)], input=stdin, text=True, capture_output=True, check=False
    )


def run_method(method) -> subprocess.CompletedProcess:
    tool_input = {"owner": "Ludus-Montis", "repo": "Breach", "workflow_id": "bench.yml", "ref": "main"}
    if method is not None:
        tool_input["method"] = method
    event = {"hook_event_name": "PreToolUse", "tool_name": TOOL, "tool_input": tool_input}
    return run_hook(json.dumps(event))


class AllowOnlyWorkflowDispatchTests(unittest.TestCase):
    def test_starting_a_workflow_is_allowed(self):
        result = run_method("run_workflow")
        self.assertEqual(result.returncode, 0, result.stderr)

    def test_every_other_method_is_blocked_with_the_reason(self):
        for method in [
            "rerun_workflow_run",
            "rerun_failed_jobs",
            "cancel_workflow_run",
            "delete_workflow_run_logs",
            "RUN_WORKFLOW",  # the server matches methods exactly; anything else is not a start
            "",
            None,
        ]:
            with self.subTest(method=method):
                result = run_method(method)
                self.assertEqual(result.returncode, 2)
                self.assertIn("run_workflow", result.stderr)
                self.assertIn("Decision 191", result.stderr)

    def test_an_unreadable_event_is_blocked(self):
        for stdin in ["", "not json", "[]", json.dumps({"tool_name": TOOL})]:
            with self.subTest(stdin=stdin):
                self.assertEqual(run_hook(stdin).returncode, 2)


if __name__ == "__main__":
    unittest.main()

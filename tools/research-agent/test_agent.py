"""Tests for the agent loop, driven by scripted responses. No API calls.

    python3 -m unittest discover -s tools/research-agent
"""

import json
import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from agent import AgentResult, build_request, check_citations, run_agent  # noqa: E402
from fake_client import FakeClient, response, text, tool_use  # noqa: E402

ANSWER = (
    "Decision 15 replaced Decision 14 [Decision 14, line 850]. "
    "Decision 14 had raised the limit from 4 to 5 [Decision 14, line 848]."
)


class LoopTest(unittest.TestCase):
    def test_runs_tools_until_end_turn(self):
        client = FakeClient([
            response("tool_use", tool_use("t1", "search_decisions_index", {"query": "touched files"})),
            response(
                "tool_use",
                tool_use("t2", "read_decision", {"number": 14}),
                tool_use("t3", "read_decision", {"number": 15}),
            ),
            response("end_turn", text(ANSWER)),
        ])
        result = run_agent(client, "Which Decision replaced Decision 14, and why?")

        self.assertTrue(result.complete)
        self.assertEqual((result.stopped, result.turns, result.answer), ("end_turn", 3, ANSWER))
        # Both parallel results went back in ONE user message, each answering its own id.
        last = client.calls[2]["messages"][-1]
        self.assertEqual(last["role"], "user")
        self.assertEqual([r["tool_use_id"] for r in last["content"]], ["t2", "t3"])
        self.assertEqual(check_citations(result), [])

    def test_text_that_looks_final_does_not_end_the_loop(self):
        """Anti-pattern 1: stopping because the text reads like an answer."""
        client = FakeClient([
            response(
                "tool_use",
                text("Final answer: Decision 15. Let me just confirm."),
                tool_use("t1", "read_decision", {"number": 15}),
            ),
            response("end_turn", text("Confirmed [Decision 15, line 900].")),
        ])
        result = run_agent(client, "q")
        self.assertEqual(result.turns, 2)
        self.assertEqual(result.answer, "Confirmed [Decision 15, line 900].")

    def test_the_turn_cap_means_unfinished_not_done(self):
        """Anti-pattern 2: treating an iteration cap as the normal way to stop."""
        loop = [response("tool_use", tool_use(f"t{i}", "read_decision", {"number": 1})) for i in range(3)]
        result = run_agent(FakeClient(loop), "q", max_turns=3)
        self.assertEqual((result.complete, result.stopped, result.answer), (False, "max_turns", None))

    def test_max_tokens_is_reported_as_unfinished(self):
        result = run_agent(FakeClient([response("max_tokens", text("Decision 15 repla"))]), "q")
        self.assertFalse(result.complete)
        self.assertEqual(result.stopped, "max_tokens")


class ErrorTest(unittest.TestCase):
    def test_a_failed_tool_goes_back_to_claude_as_a_structured_error(self):
        client = FakeClient([
            response("tool_use", tool_use("t1", "read_decision", {"number": 999})),
            response("end_turn", text("Decision 999 does not exist. Not found in the Decisions.")),
        ])
        result = run_agent(client, "What does Decision 999 say?")

        self.assertTrue(result.complete)
        sent = client.calls[1]["messages"][-1]["content"][0]
        self.assertEqual(sent["tool_use_id"], "t1")
        self.assertTrue(sent["is_error"])
        self.assertEqual(json.loads(sent["content"])["errorCategory"], "validation")


class RequestTest(unittest.TestCase):
    def test_tool_choice_is_left_at_auto(self):
        request = build_request("q")
        self.assertNotIn("tool_choice", request)
        self.assertEqual(
            [t["name"] for t in request["tools"]],
            ["search_decisions_index", "read_decision", "read_spec_lines"],
        )


class CitationTest(unittest.TestCase):
    def _result(self, answer):
        content = json.dumps({"number": 14, "first_line": 848, "last_line": 860})
        return AgentResult(answer, True, "end_turn", 2, [("read_decision", {"number": 14}, content, False)])

    def test_a_citation_outside_what_was_read_is_flagged(self):
        problems = check_citations(self._result("It was raised [Decision 14, line 9000]."))
        self.assertEqual(len(problems), 1)
        self.assertIn("line 9000", problems[0])

    def test_an_answer_with_no_sources_is_flagged(self):
        self.assertEqual(check_citations(self._result("Decision 15 replaced it.")), ["the answer cites no sources"])


if __name__ == "__main__":
    unittest.main()

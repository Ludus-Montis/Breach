"""Tests for Part B: a coordinator that hands each Decision to a reader subagent. No API calls.

    python3 -m unittest discover -s tools/research-agent
"""

import json
import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from agent import check_citations  # noqa: E402
from coordinator import route, run_coordinator  # noqa: E402
from fake_client import RoutedFakeClient, response, text, tool_use  # noqa: E402

CONTEXT = "Decision 14 is marked 'superseded by 15' in the index."


def ask(block_id, number, context=CONTEXT):
    return tool_use(block_id, "ask_reader", {"number": number, "question": "What changed and why?", "context": context})


def reader_reads(number, answer):
    return [
        response("tool_use", tool_use(f"r{number}", "read_decision", {"number": number})),
        response("end_turn", text(answer)),
    ]


class HubAndSpokeTest(unittest.TestCase):
    def setUp(self):
        self.client = RoutedFakeClient(route, {
            "coordinator": [
                response("tool_use", tool_use("c1", "search_decisions_index", {"query": "touched files"})),
                response("tool_use", ask("c2", 14), ask("c3", 15)),  # two readers in one turn
                response("end_turn", text(
                    "Decision 15 superseded Decision 14 [Decision 15, line 887]; "
                    "14 had raised the limit to 5 [Decision 14, line 848]."
                )),
            ],
            "reader:14": reader_reads(14, "It raised the limit from 4 to 5 [Decision 14, line 848]."),
            "reader:15": reader_reads(15, "It fixed an off-by-one; the limit stays 5 [Decision 15, line 885]."),
        })
        self.result = run_coordinator(self.client, "Which Decision replaced Decision 14, and why?")

    def test_the_run_finishes_with_sources_that_were_read(self):
        self.assertTrue(self.result.complete)
        self.assertEqual(check_citations(self.result), [])

    def test_a_reader_starts_with_only_what_the_coordinator_wrote(self):
        first = self.client.calls_for("reader:14")[0]
        self.assertEqual(len(first["messages"]), 1)  # no coordinator history
        prompt = first["messages"][0]["content"]
        self.assertIn("What changed and why?", prompt)
        self.assertIn(CONTEXT, prompt)

    def test_readers_get_only_the_reading_tools(self):
        tools = [t["name"] for t in self.client.calls_for("reader:15")[0]["tools"]]
        self.assertEqual(tools, ["read_decision", "read_spec_lines"])
        coordinator_tools = [t["name"] for t in self.client.calls_for("coordinator")[0]["tools"]]
        self.assertEqual(coordinator_tools, ["search_decisions_index", "ask_reader"])

    def test_both_findings_go_back_in_one_message_with_metadata_apart(self):
        last = self.client.calls_for("coordinator")[2]["messages"][-1]["content"]
        self.assertEqual([r["tool_use_id"] for r in last], ["c2", "c3"])
        finding = json.loads(last[0]["content"])
        self.assertEqual(finding["status"], "ok")
        self.assertIn("4 to 5", finding["finding"])
        self.assertEqual(finding["meta"]["stopped"], "end_turn")
        self.assertEqual(finding["meta"]["sources_read"], ["Decision 14, lines 848-883"])


class ErrorPropagationTest(unittest.TestCase):
    def run_one(self, reader_script):
        client = RoutedFakeClient(route, {
            "coordinator": [
                response("tool_use", ask("c1", 999)),
                response("end_turn", text("Not found in the Decisions.")),
            ],
            "reader:999": reader_script,
        })
        result = run_coordinator(client, "What does Decision 999 say?")
        sent = client.calls_for("coordinator")[1]["messages"][-1]["content"][0]
        return result, sent, json.loads(sent["content"])

    def test_a_reader_tool_error_reaches_the_coordinator_structured(self):
        result, sent, body = self.run_one(reader_reads(999, "Decision 999 does not exist."))
        self.assertTrue(result.complete)  # the whole run did not die
        self.assertTrue(sent["is_error"])  # nor was the failure hidden as an empty finding
        self.assertEqual(body["status"], "error")
        self.assertEqual(body["error"]["errorCategory"], "validation")
        self.assertFalse(body["error"]["isRetryable"])
        self.assertEqual(body["partial"], "Decision 999 does not exist.")

    def test_an_unfinished_reader_is_reported_retryable_with_its_partial_text(self):
        _, sent, body = self.run_one([response("max_tokens", text("Decision 999 is ab"))])
        self.assertTrue(sent["is_error"])
        self.assertEqual(body["error"]["errorCategory"], "transient")
        self.assertTrue(body["error"]["isRetryable"])
        self.assertEqual(body["partial"], "Decision 999 is ab")

    def test_a_finding_with_unread_sources_is_marked_unverified(self):
        client = RoutedFakeClient(route, {
            "coordinator": [response("tool_use", ask("c1", 14)), response("end_turn", text("x"))],
            "reader:14": reader_reads(14, "It was raised [Decision 14, line 9000]."),
        })
        run_coordinator(client, "q")
        body = json.loads(client.calls_for("coordinator")[1]["messages"][-1]["content"][0]["content"])
        self.assertEqual(body["status"], "unverified")
        self.assertIn("line 9000", body["source_problems"][0])


if __name__ == "__main__":
    unittest.main()

"""Retry-loop tests with a fake client returning canned responses (no API calls)."""

import copy
import sys
import unittest
from pathlib import Path
from types import SimpleNamespace as NS

sys.path.insert(0, str(Path(__file__).resolve().parent))
import retry  # noqa: E402

EXCERPT = "### Decision 5: Test\n\n**Date:** 2026-09-21\n\n> Superseded by Decision 9 on 2026-10-02.\n"
GOOD = {
    "decision_number": 5, "title": "Test", "date": "2026-09-21", "status": "superseded",
    "status_detail": None, "supersedes": [], "superseded_by": [9],
    "specifications_affected": [], "summary": "A test.",
}
BAD_DATE = dict(GOOD, date="2026-10-02")


def tool_response(record, stop_reason="tool_use"):
    thinking = NS(type="thinking", thinking="", signature="sig")
    return NS(stop_reason=stop_reason, content=[thinking, NS(type="tool_use", id="toolu_1", input=record)])


class FakeClient:
    def __init__(self, *responses):
        self.responses, self.requests = list(responses), []
        self.messages = self

    def create(self, **request):
        self.requests.append(copy.deepcopy(request))
        return self.responses.pop(0)


class RetryTest(unittest.TestCase):
    def setUp(self):
        self._saved = retry.load_excerpt, retry.known_decisions
        retry.load_excerpt = lambda n: EXCERPT
        retry.known_decisions = lambda: {5, 9}

    def tearDown(self):
        retry.load_excerpt, retry.known_decisions = self._saved

    def test_passes_first_time(self):
        client = FakeClient(tool_response(GOOD))
        self.assertEqual(retry.extract_with_retry(client, 5), (GOOD, []))
        self.assertEqual(len(client.requests), 1)

    def test_validation_error_is_fed_back_and_fixed(self):
        client = FakeClient(tool_response(BAD_DATE), tool_response(GOOD))
        record, errors = retry.extract_with_retry(client, 5)
        self.assertEqual((record, errors), (GOOD, []))

        second = client.requests[1]["messages"]
        self.assertEqual([m["role"] for m in second], ["user", "assistant", "user"])
        self.assertEqual(second[1]["content"][0].type, "thinking")  # whole turn kept
        result = second[2]["content"][0]
        self.assertEqual((result["type"], result["tool_use_id"], result["is_error"]), ("tool_result", "toolu_1", True))
        self.assertIn("'Superseded by' note", result["content"])

    def test_gives_up_after_max_retries_and_returns_errors(self):
        client = FakeClient(*(tool_response(BAD_DATE) for _ in range(3)))
        record, errors = retry.extract_with_retry(client, 5, max_retries=2)
        self.assertEqual(len(client.requests), 3)
        self.assertEqual(record, BAD_DATE)
        self.assertEqual(len(errors), 1)

    def test_max_tokens_reissues_with_more_room_without_feedback(self):
        client = FakeClient(tool_response({}, stop_reason="max_tokens"), tool_response(GOOD))
        self.assertEqual(retry.extract_with_retry(client, 5, max_retries=0), (GOOD, []))
        self.assertEqual(client.requests[1]["max_tokens"], 2 * client.requests[0]["max_tokens"])
        self.assertEqual(len(client.requests[1]["messages"]), 1)

    def test_missing_tool_call_gets_a_nudge(self):
        no_call = NS(stop_reason="end_turn", content=[NS(type="text", text="Here is a summary.")])
        client = FakeClient(no_call, tool_response(GOOD))
        self.assertEqual(retry.extract_with_retry(client, 5), (GOOD, []))
        self.assertEqual(client.requests[1]["messages"][2]["content"][0]["type"], "text")

    def test_refusal_is_not_retried(self):
        client = FakeClient(NS(stop_reason="refusal", content=[]))
        record, errors = retry.extract_with_retry(client, 5)
        self.assertIsNone(record)
        self.assertEqual(len(client.requests), 1)


if __name__ == "__main__":
    unittest.main()

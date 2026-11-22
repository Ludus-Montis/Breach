"""Step 5 tests: batch requests, result sorting and the follow-up batch (no API calls)."""

import copy
import sys
import unittest
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import batch  # noqa: E402
from prompt import FEW_SHOT_DECISIONS  # noqa: E402

CANNED = batch.load_jsonl(HERE / "fixtures" / "canned_results.jsonl")
REQUESTS = batch.batch_requests()
SENT = {r["custom_id"]: r for r in REQUESTS}

EXPECTED = {
    "decision-016": batch.RESUBMIT,     # no result line
    "decision-027": batch.FIX_REQUEST,  # invalid_request_error
    "decision-062": batch.RESUBMIT,     # overloaded_error
    "decision-089": batch.RESUBMIT,     # expired
    "decision-110": batch.RESUBMIT,     # canceled
    "decision-131": batch.MORE_ROOM,    # stop_reason max_tokens
    "decision-145": batch.FOLLOW_UP,    # fails validate()
    "decision-148": batch.FOLLOW_UP,    # no tool call
    "decision-155": batch.REFUSED,      # stop_reason refusal
}


def actions(outcomes):
    return {cid: o.action for cid, o in outcomes.items()}


class Requests(unittest.TestCase):
    def test_one_request_per_decision_with_unique_ids(self):
        self.assertEqual(len(REQUESTS), len(batch.BATCH_DECISIONS))
        self.assertEqual(len(SENT), len(REQUESTS))
        self.assertEqual(sorted(batch.parse_id(c)[0] for c in SENT), sorted(batch.BATCH_DECISIONS))

    def test_few_shot_decisions_are_not_in_the_batch(self):
        self.assertFalse(set(FEW_SHOT_DECISIONS) & set(batch.BATCH_DECISIONS))

    def test_params_are_a_plain_messages_body(self):
        for request in REQUESTS:
            params = request["params"]
            self.assertEqual(params["model"], "claude-opus-5-5")
            self.assertEqual(params["tool_choice"], {"type": "auto"})
            for unsupported in ("stream", "speed", "fallbacks"):
                self.assertNotIn(unsupported, params)

    def test_custom_id_round_trips(self):
        self.assertEqual(batch.parse_id(batch.custom_id(9)), (9, 0))
        self.assertEqual(batch.parse_id(batch.custom_id(145, 2)), (145, 2))
        with self.assertRaises(ValueError):
            batch.parse_id("request-1")

    @unittest.skipUnless(
        isinstance(REQUESTS[0]["params"]["system"], list), "cache_control TODO not enabled yet"
    )
    def test_cache_breakpoint_is_identical_in_every_request(self):
        systems = [r["params"]["system"] for r in REQUESTS]
        self.assertIn("cache_control", systems[0][-1])
        self.assertTrue(all(s == systems[0] for s in systems))


class SortResults(unittest.TestCase):
    def test_each_result_gets_its_action(self):
        got = actions(batch.sort_results(CANNED, SENT))
        for cid, action in EXPECTED.items():
            self.assertEqual(got[cid], action, cid)
        accepted = [c for c, a in got.items() if a == batch.ACCEPT]
        self.assertEqual(len(accepted), len(REQUESTS) - len(EXPECTED))

    def test_order_does_not_matter(self):
        forward = actions(batch.sort_results(CANNED, SENT))
        backward = actions(batch.sort_results(list(reversed(CANNED)), SENT))
        self.assertEqual(forward, backward)

    def test_accepted_record_matches_its_custom_id(self):
        for cid, outcome in batch.sort_results(CANNED, SENT).items():
            if outcome.action == batch.ACCEPT:
                self.assertEqual(outcome.record["decision_number"], batch.parse_id(cid)[0])

    def test_follow_up_gives_up_after_the_limit(self):
        line = copy.deepcopy(next(l for l in CANNED if l["custom_id"] == "decision-145"))
        line["custom_id"] = batch.custom_id(145, batch.MAX_FOLLOW_UPS)
        outcome = batch.sort_results([line], [])[line["custom_id"]]
        self.assertEqual(outcome.action, batch.GIVE_UP)


class NextBatch(unittest.TestCase):
    def setUp(self):
        self.outcomes = batch.sort_results(CANNED, SENT)
        self.follow = {r["custom_id"]: r for r in batch.next_batch(self.outcomes, SENT)}

    def test_only_fixable_results_are_resent(self):
        self.assertEqual(
            sorted(self.follow),
            ["decision-016", "decision-062", "decision-089", "decision-110",
             "decision-131", "decision-145-r1", "decision-148-r1"],
        )

    def test_resubmit_is_the_same_request(self):
        self.assertEqual(self.follow["decision-062"]["params"], SENT["decision-062"]["params"])

    def test_more_room_doubles_max_tokens_only(self):
        params = self.follow["decision-131"]["params"]
        self.assertEqual(params["max_tokens"], 2 * SENT["decision-131"]["params"]["max_tokens"])
        self.assertEqual(params["messages"], SENT["decision-131"]["params"]["messages"])

    def test_follow_up_sends_errors_back_as_a_tool_result(self):
        messages = self.follow["decision-145-r1"]["params"]["messages"]
        assistant, user = messages[-2], messages[-1]
        sent_content = self.outcomes["decision-145"].message["content"]
        self.assertEqual(assistant["content"], sent_content)  # thinking block kept, unchanged
        result = user["content"][0]
        self.assertEqual(result["type"], "tool_result")
        self.assertTrue(result["is_error"])
        self.assertEqual(result["tool_use_id"], "toolu_canned_145")
        self.assertIn("2026-01-01", result["content"])

    def test_missing_tool_call_gets_a_nudge(self):
        user = self.follow["decision-148-r1"]["params"]["messages"][-1]
        self.assertEqual(user["content"][0]["type"], "text")

    def test_sent_requests_are_not_changed(self):
        self.assertEqual(len(SENT["decision-145"]["params"]["messages"]), 1)


class Cost(unittest.TestCase):
    def test_batch_is_half_and_caching_is_cheaper_still(self):
        cost = batch.estimate_cost(REQUESTS)
        self.assertAlmostEqual(cost["batch"], cost["standard"] / 2)
        self.assertLess(cost["batch_cached"], cost["batch"])


if __name__ == "__main__":
    unittest.main()

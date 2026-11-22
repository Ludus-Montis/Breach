"""Step 6 tests: records checked against DECISIONS_INDEX.md (no API calls)."""

import copy
import sys
import unittest
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import batch  # noqa: E402
import calibrate  # noqa: E402

CANNED = batch.load_jsonl(HERE / "fixtures" / "canned_results.jsonl")
SENT = {r["custom_id"]: r for r in batch.batch_requests()}
KEY = calibrate.answer_key()


def accepted() -> dict[int, dict]:
    outcomes = batch.sort_results(CANNED, SENT)
    return {
        batch.parse_id(c)[0]: copy.deepcopy(o.record) for c, o in outcomes.items() if o.action == batch.ACCEPT
    }


class AnswerKeyTest(unittest.TestCase):
    def test_superseded_links_both_ways(self):
        self.assertEqual(KEY[62]["superseded_by"], [63])
        self.assertEqual(KEY[63]["supersedes"], [62])
        self.assertEqual(KEY[62]["status"], "superseded")

    def test_amended_is_not_superseded(self):
        self.assertEqual(KEY[20]["status"], "active")
        self.assertEqual(KEY[20]["superseded_by"], [])


class FillFromPeersTest(unittest.TestCase):
    def test_fills_a_link_only_the_later_record_states(self):
        records = accepted()
        changes = calibrate.fill_from_peers(records)
        self.assertEqual(changes, ["63: superseded_by += 66 (from Decision 66's supersedes)"])
        self.assertEqual(records[63]["status"], "superseded")

    def test_cannot_fill_from_outside_the_batch(self):
        records = accepted()
        calibrate.fill_from_peers(records)
        self.assertEqual(records[77]["superseded_by"], [])  # superseded by 79, not in the batch


class CompareTest(unittest.TestCase):
    def test_wrong_fields_after_fill(self):
        records = accepted()
        calibrate.fill_from_peers(records)
        wrong = {(r["decision"], r["field"]) for r in calibrate.compare(records, KEY) if not r["correct"]}
        self.assertEqual(
            wrong,
            {(24, "title"), (77, "status"), (77, "superseded_by"), (78, "status"), (78, "superseded_by")},
        )


if __name__ == "__main__":
    unittest.main()

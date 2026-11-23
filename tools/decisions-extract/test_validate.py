"""Semantic validation tests on hand-built records (no API calls)."""

import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from validate import known_decisions, validate  # noqa: E402

EXCERPT = """### Decision 5: Test heading

**Date:** 2026-09-21

> Superseded by Decision 9 on 2026-10-02.

Body text.
"""

KNOWN = {1, 2, 5, 9}

GOOD = {
    "decision_number": 5,
    "title": "Test heading",
    "date": "2026-09-21",
    "status": "superseded",
    "status_detail": None,
    "supersedes": [2],
    "superseded_by": [9],
    "specifications_affected": [],
    "summary": "A test.",
}


def bad(**changes):
    return dict(GOOD, **changes)


class ValidateTest(unittest.TestCase):
    def assertOneError(self, record, fragment, excerpt=EXCERPT):
        errors = validate(record, excerpt, KNOWN)
        self.assertEqual(len(errors), 1, errors)
        self.assertIn(fragment, errors[0])

    def test_good_record_passes(self):
        self.assertEqual(validate(GOOD, EXCERPT, KNOWN), [])

    def test_wrong_decision_number(self):
        self.assertOneError(bad(decision_number=2, supersedes=[]), "heading is Decision 5")

    def test_other_needs_detail(self):
        self.assertOneError(bad(status="other", superseded_by=[]), "status is 'other'")

    def test_superseded_needs_superseded_by(self):
        self.assertOneError(bad(superseded_by=[]), "superseded_by is empty")

    def test_in_part_only_with_superseded(self):
        self.assertOneError(
            bad(status="active", status_detail="in part: the cap"), "'in part:'"
        )

    def test_in_part_allowed_with_superseded(self):
        self.assertEqual(validate(bad(status_detail="in part: the cap"), EXCERPT, KNOWN), [])

    def test_date_from_supersession_note(self):
        self.assertOneError(bad(date="2026-10-02"), "'Superseded by' note")

    def test_date_not_in_excerpt(self):
        self.assertOneError(bad(date="2025-01-01"), "does not appear")

    def test_null_date_passes(self):
        self.assertEqual(validate(bad(date=None), EXCERPT, KNOWN), [])

    def test_self_link(self):
        errors = validate(bad(supersedes=[5]), EXCERPT, KNOWN)
        self.assertTrue(any("own number" in e for e in errors), errors)

    def test_unknown_link(self):
        self.assertOneError(bad(superseded_by=[999]), "[999]")

    def test_several_errors_reported_together(self):
        errors = validate(bad(date="2026-10-02", superseded_by=[]), EXCERPT, KNOWN)
        self.assertEqual(len(errors), 2, errors)

    def test_known_decisions_reads_index(self):
        known = known_decisions()
        self.assertIn(1, known)
        self.assertIn(157, known)
        self.assertGreaterEqual(len(known), 190)


if __name__ == "__main__":
    unittest.main()

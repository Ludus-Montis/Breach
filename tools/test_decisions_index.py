"""Tests for decisions_index.py (Decision 140)."""

from __future__ import annotations

import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from decisions_index import parse, render, status  # noqa: E402

LEDGER = """# Spec

## Decisions

### Decision 1 — First choice
> Superseded by Decision 3 on 2026-10-01.

**Rationale:** one.

### Decision 2 — A choice with | a pipe

### Decision 3 — Replaces the first
**Supersedes:** Decision 1

### Decision 4 — Refines two choices
**Amends:** Decision 2 (its order) and Decision 3
""".splitlines()


class DecisionsIndexTest(unittest.TestCase):
    def test_each_decision_is_listed_with_its_line(self):
        decisions = parse(LEDGER)
        self.assertEqual(sorted(decisions), [1, 2, 3, 4])
        self.assertEqual(decisions[1]["line"], 5)
        self.assertEqual(decisions[3]["title"], "Replaces the first")

    def test_superseded_by_comes_from_the_note_and_the_field(self):
        decisions = parse(LEDGER)
        self.assertEqual(decisions[1]["superseded_by"], {3})
        self.assertEqual(status(decisions[1]), "superseded by 3")

    def test_amends_marks_every_decision_it_names(self):
        decisions = parse(LEDGER)
        self.assertEqual(decisions[2]["amended_by"], {4})
        self.assertEqual(decisions[3]["amended_by"], {4})
        self.assertEqual(status(decisions[4]), "")

    def test_a_reference_to_a_missing_decision_adds_no_row(self):
        decisions = parse(["### Decision 5 — Late", "**Supersedes:** Decision 99"])
        self.assertEqual(sorted(decisions), [5])

    def test_rows_escape_pipes_and_run_in_number_order(self):
        rows = [r for r in render(parse(LEDGER)).splitlines() if r.startswith("| ") and r[2].isdigit()]
        self.assertEqual([r.split(" | ")[0] for r in rows], ["| 1", "| 2", "| 3", "| 4"])
        self.assertIn("A choice with \\| a pipe", rows[1])


if __name__ == "__main__":
    unittest.main()

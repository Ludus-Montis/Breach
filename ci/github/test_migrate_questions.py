"""Tests for migrate_questions.py (spec 36 step 1), on a copy of OPEN_QUESTIONS.md."""
import io
import sys
import unittest
from contextlib import redirect_stdout
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import migrate_questions  # noqa: E402
from fake_client import FakeClient  # noqa: E402

FIXTURE = (Path(__file__).resolve().parent / "fixtures" / "open_questions.md").read_text(
    encoding="utf-8"
)
SPEC_35 = "Spec 35: The unit store"


def entry(text, number=99):
    return migrate_questions.Entry(number, "A question", "today", text, "", "the body")


class ParseTest(unittest.TestCase):
    def setUp(self):
        self.entries = {each.number: each for each in migrate_questions.parse(FIXTURE)}

    def test_an_entry_parses_into_its_fields(self):
        q30 = self.entries[30]
        self.assertEqual(
            q30.title,
            "Track open questions and upcoming work as GitHub issues, milestones, labels"
            " and a project?",
        )
        self.assertTrue(q30.raised.startswith("2026-10-17, after spec 35 step 7b"))
        self.assertEqual(
            q30.raise_when,
            "The next design session, before more entries are added here or a new spec"
            " round starts",
        )
        self.assertTrue(q30.body.startswith("Today the open questions are this file"))
        self.assertNotIn("## Closed", q30.body)

    def test_possibly_settled_by_is_its_own_field(self):
        q1 = self.entries[1]
        self.assertTrue(q1.settled_by.startswith("Decision 4 chose abstracted lanes"))
        self.assertNotIn("Possibly settled by", q1.body)

    def test_closed_entries_stay(self):
        self.assertEqual(sorted(self.entries)[0], 1)
        self.assertEqual(max(self.entries), 30)
        self.assertNotIn(20, self.entries)


class LabelTest(unittest.TestCase):
    def test_raise_when_maps_to_one_label(self):
        milestones = [SPEC_35]
        cases = [
            ("The next design session", "raise-when:next-design", None),
            ("spec 35 done", "raise-when:spec-35", SPEC_35),
            ("any PR whose digests move", "raise-when:other", None),
        ]
        for text, label, milestone in cases:
            with self.subTest(text=text):
                issue = migrate_questions.issue_for(entry(text), milestones)
                self.assertEqual(issue["labels"], ["question", label])
                self.assertEqual(issue["milestone"], milestone)
                self.assertIn(text, issue["body"])

    def test_two_specs_named_is_other(self):
        text = "chooses between spec 34's list and spec 35's steps"
        issue = migrate_questions.issue_for(entry(text), [SPEC_35])
        self.assertEqual(issue["labels"], ["question", "raise-when:other"])
        self.assertIsNone(issue["milestone"])

    def test_the_issue_is_titled_by_number_and_title(self):
        issue = migrate_questions.issue_for(entry("x", number=7), [])
        self.assertEqual(issue["title"], "Q7. A question")


class MigrateTest(unittest.TestCase):
    def run_migration(self, client):
        out = io.StringIO()
        with redirect_stdout(out):
            migrate_questions.migrate(client, FIXTURE)
        return out.getvalue()

    def test_every_open_entry_becomes_an_issue_once(self):
        client = FakeClient(milestones=[SPEC_35])
        self.run_migration(client)
        titles = [issue["title"] for issue in client.created_issues]
        self.assertEqual(len(titles), len(set(titles)))
        self.assertEqual(len(titles), len(migrate_questions.parse(FIXTURE)))
        q23 = next(i for i in client.created_issues if i["title"].startswith("Q23."))
        self.assertEqual(q23["milestone"], 1)

    def test_a_migrated_entry_is_not_migrated_twice(self):
        existing = (412, "Q29. A side-by-side view of an outcome change, for the owner to sign it off")
        client = FakeClient(issues=[existing])
        log = self.run_migration(client)
        self.assertFalse([i for i in client.created_issues if i["title"].startswith("Q29.")])
        self.assertIn("Q29 -> #412", log.splitlines())
        self.assertIn("Q1 -> #1001", log.splitlines())


if __name__ == "__main__":
    unittest.main()

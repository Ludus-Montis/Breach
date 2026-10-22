"""Tests for project_backfill.py: the gate and apply (spec 40 step 6)."""
import io
import sys
import unittest
from contextlib import redirect_stdout
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import project_backfill  # noqa: E402

PROJECT = {"id": "P1", "field": "F1", "options": {"Inbox": "o0"}}


class FakeRepo:
    """Stands in for project_backfill.OpenItemsClient: the repo's open issues and PRs."""

    def __init__(self, items):
        self.items = items

    def open_items(self):
        return list(self.items)


class FakeProjectClient:
    """Stands in for graphql_client.ProjectClient, recording every call."""

    def __init__(self, contents):
        self.contents = set(contents)
        self.calls = []

    def project(self, owner, number):
        self.calls.append(("project", owner, number))
        return PROJECT

    def item_contents(self, project_id):
        self.calls.append(("item_contents", project_id))
        return set(self.contents)

    def add_item(self, project_id, content_id):
        self.calls.append(("add_item", content_id))
        self.contents.add(content_id)
        return f"item-{content_id}"


OPEN = [(10, "I10"), (11, "I11"), (12, "PR12")]


class ApplyTest(unittest.TestCase):
    def apply(self, writer):
        out = io.StringIO()
        with redirect_stdout(out):
            project_backfill.apply(FakeRepo(OPEN), writer, "Ludus-Montis", 3)
        return out.getvalue()

    def test_only_whats_missing_is_added(self):
        writer = FakeProjectClient(["I11"])
        printed = self.apply(writer)
        self.assertEqual([c for c in writer.calls if c[0] == "add_item"], [("add_item", "I10"), ("add_item", "PR12")])
        self.assertIn("2 added, 1 already there", printed)

    def test_the_items_are_listed_once(self):
        writer = FakeProjectClient(["I11"])
        self.apply(writer)
        self.assertEqual([c[0] for c in writer.calls].count("item_contents"), 1)

    def test_a_second_run_adds_nothing(self):
        writer = FakeProjectClient(["I11"])
        self.apply(writer)
        writer.calls.clear()
        printed = self.apply(writer)
        self.assertNotIn("add_item", [c[0] for c in writer.calls])
        self.assertIn("0 added, 3 already there", printed)


class GateTest(unittest.TestCase):
    def test_no_app_no_failure(self):
        ready, message = project_backfill.gate("", True)
        self.assertFalse(ready)
        self.assertIn("App", message)
        self.assertFalse(project_backfill.gate("123", False)[0])

    def test_the_app_set_up_acts(self):
        self.assertTrue(project_backfill.gate("123", True)[0])


if __name__ == "__main__":
    unittest.main()

"""Tests for project_status.py: the rule, the gate and apply (spec 40 step 3)."""
import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import project_status  # noqa: E402

PROJECT = {"id": "P1", "field": "F1", "options": {"Inbox": "o0", "Next": "o1", "In progress": "o2", "In review": "o3", "Done": "o4"}}


class FakeProjectClient:
    """Stands in for graphql_client.ProjectClient, recording every call."""

    def __init__(self, pr=None, project=None, items=None):
        self.pr = pr
        self.project_record = project or PROJECT
        self.items = dict(items or {})
        self.calls = []

    def pull_request(self, owner, repo, number):
        self.calls.append(("pull_request", number))
        return self.pr

    def project(self, owner, number):
        self.calls.append(("project", owner, number))
        return self.project_record

    def item_id(self, project_id, content_id):
        self.calls.append(("item_id", content_id))
        return self.items.get(content_id)

    def add_item(self, project_id, content_id):
        self.calls.append(("add_item", content_id))
        self.items[content_id] = f"item-{content_id}"
        return self.items[content_id]

    def set_status(self, project_id, item_id, field_id, option_id):
        self.calls.append(("set_status", item_id, option_id))


def pull(draft=False, labels=(), closing=()):
    return {"id": "PR", "draft": draft, "labels": list(labels), "closing": list(closing)}


class RuleTest(unittest.TestCase):
    def test_the_rule(self):
        self.assertEqual(project_status.status_for(True, ["handoff"]), "Next")
        self.assertEqual(project_status.status_for(True, ["sim"]), "In progress")
        self.assertEqual(project_status.status_for(False, []), "In review")
        self.assertEqual(project_status.status_for(False, ["handoff"]), "In review")


class GateTest(unittest.TestCase):
    def test_another_label_does_nothing(self):
        ready, message = project_status.gate("unlabeled", "sim", "123", True)
        self.assertFalse(ready)
        self.assertIn("sim", message)

    def test_the_handoff_label_acts(self):
        self.assertTrue(project_status.gate("labeled", "handoff", "123", True)[0])

    def test_other_events_act(self):
        self.assertTrue(project_status.gate("opened", "", "123", True)[0])

    def test_no_app_no_failure(self):
        ready, message = project_status.gate("opened", "", "", True)
        self.assertFalse(ready)
        self.assertIn("App", message)
        self.assertFalse(project_status.gate("opened", "", "123", False)[0])


class ApplyTest(unittest.TestCase):
    def apply(self, pr, items=None, project=None):
        reader = FakeProjectClient(pr=pr)
        writer = FakeProjectClient(project=project, items=items)
        project_status.apply(reader, writer, "Ludus-Montis/Breach", 7, "Ludus-Montis", 3)
        return reader, writer

    def test_the_closed_issues_follow_the_pr(self):
        pr = pull(closing=["I310", "I311"])
        _, writer = self.apply(pr, items={"PR": "iPR", "I310": "i310", "I311": "i311"})
        sets = [call for call in writer.calls if call[0] == "set_status"]
        self.assertEqual(sets, [("set_status", "iPR", "o3"), ("set_status", "i310", "o3"), ("set_status", "i311", "o3")])

    def test_an_item_not_yet_in_the_project_is_added(self):
        _, writer = self.apply(pull(draft=True))
        self.assertEqual(writer.calls[-2:], [("add_item", "PR"), ("set_status", "item-PR", "o2")])

    def test_the_pr_is_read_with_the_repo_token_only(self):
        reader, writer = self.apply(pull(), items={"PR": "iPR"})
        self.assertEqual(reader.calls, [("pull_request", 7)])
        self.assertNotIn("pull_request", [call[0] for call in writer.calls])

    def test_the_option_name_ignores_case(self):
        project = dict(PROJECT, options={"In Progress": "o2"})
        _, writer = self.apply(pull(draft=True), items={"PR": "iPR"}, project=project)
        self.assertEqual(writer.calls[-1], ("set_status", "iPR", "o2"))

    def test_an_unknown_status_option_fails_clearly(self):
        project = dict(PROJECT, options={"In review": "o3"})
        with self.assertRaisesRegex(SystemExit, "`Next` .it has: In review."):
            self.apply(pull(draft=True, labels=["handoff"]), project=project)


if __name__ == "__main__":
    unittest.main()

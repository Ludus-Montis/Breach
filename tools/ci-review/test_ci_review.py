"""CI re-review tests on canned `claude -p` output (no API calls).

The classes named Study* stay red until the STUDY lines in command.py, envelope.py and
rereview.py are filled in. Run: python3 -m unittest test_ci_review -v
"""

import json
import sys
import unittest
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
from command import build_command  # noqa: E402
from envelope import ReviewRunError, read_review  # noqa: E402
from prompt import build_prompt  # noqa: E402
from rereview import plan  # noqa: E402

FIX = HERE / "fixtures"
PRIOR = json.loads((FIX / "prior_findings.json").read_text())


def fixture(name: str) -> str:
    return (FIX / name).read_text()


class Schema(unittest.TestCase):
    def test_every_finding_field_is_required(self):
        # With strict output, an optional field is one the model may quietly leave out.
        item = json.loads((HERE / "schema.json").read_text())["properties"]["findings"]["items"]
        self.assertEqual(set(item["required"]), set(item["properties"]))

    def test_prior_id_is_nullable_so_a_new_finding_need_not_invent_one(self):
        item = json.loads((HERE / "schema.json").read_text())["properties"]["findings"]["items"]
        self.assertIn("null", item["properties"]["prior_id"]["type"])


class Prompt(unittest.TestCase):
    def test_first_review_has_no_prior_section(self):
        self.assertNotIn("Earlier findings", build_prompt("origin/main...HEAD", []))

    def test_rereview_carries_every_prior_id(self):
        text = build_prompt("origin/main...HEAD", PRIOR)
        for p in PRIOR:
            self.assertIn(p["id"], text)

    def test_criteria_name_what_to_skip(self):
        # Explicit categories, not "be conservative" (task statement 4.1).
        self.assertIn("Skip style", build_prompt("x", []))


class Command(unittest.TestCase):
    def test_print_mode_with_read_only_tools_and_a_turn_bound(self):
        cmd = build_command("p")
        self.assertEqual(cmd[:3], ["claude", "-p", "p"])
        tools = cmd[cmd.index("--allowedTools") + 1]
        for writer in ("Edit", "Write", "Bash(git push"):
            self.assertNotIn(writer, tools)
        self.assertIn("--max-turns", cmd)


class Envelope(unittest.TestCase):
    def test_error_run_is_a_structured_error_and_max_turns_is_retryable(self):
        with self.assertRaises(ReviewRunError) as ctx:
            read_review(fixture("error_max_turns.json"))
        self.assertEqual(ctx.exception.category, "error_max_turns")
        self.assertTrue(ctx.exception.retryable)


class StudyCommand(unittest.TestCase):
    def test_asks_for_one_json_object_validated_against_the_schema(self):
        cmd = build_command("p")
        self.assertEqual(cmd[cmd.index("--output-format") + 1], "json")
        schema = json.loads(cmd[cmd.index("--json-schema") + 1])
        self.assertIn("findings", schema["properties"])


class StudyEnvelope(unittest.TestCase):
    def test_reads_the_validated_review(self):
        review = read_review(fixture("rereview_ok.json"))
        self.assertEqual(len(review["findings"]), 3)

    def test_missing_validated_review_is_an_error_not_an_empty_review(self):
        # "No issues found" in free text is not a review that passed: it is no review.
        with self.assertRaises(ReviewRunError):
            read_review(fixture("no_structured_output.json"))


class StudyRereview(unittest.TestCase):
    def setUp(self):
        self.out = plan(PRIOR, json.loads(fixture("rereview_ok.json"))["structured_output"])

    def test_posts_only_new_findings(self):
        lines = sorted(f["line"] for f in self.out["post"])
        self.assertEqual(lines, [40, 95])  # the null one, and the one citing unknown R9

    def test_still_standing_finding_is_counted_not_reposted(self):
        self.assertEqual(self.out["still_open"], ["R2"])

    def test_unknown_resolved_id_is_ignored(self):
        self.assertEqual(self.out["resolved"], ["R1"])

    def test_prior_finding_nobody_mentioned_is_flagged(self):
        self.assertEqual(self.out["unmentioned"], ["R3"])


if __name__ == "__main__":
    unittest.main()

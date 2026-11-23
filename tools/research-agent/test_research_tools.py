"""Tests for the three tools, run against the real index and spec. No API calls.

    python3 -m unittest discover -s tools/research-agent
"""

import json
import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from research_tools import MAX_SPEC_LINES, TOOLS, run_tool  # noqa: E402


def call(name, **tool_input):
    content, is_error = run_tool(name, tool_input)
    return json.loads(content), is_error


class SearchTest(unittest.TestCase):
    def test_finds_decisions_by_title_words(self):
        body, is_error = call("search_decisions_index", query="max_touched_files_per_new_case")
        self.assertFalse(is_error)
        found = {row["number"]: row for row in body["results"]}
        self.assertEqual(found[14]["line"], 848)
        self.assertEqual(found[14]["status"], "superseded by 15")

    def test_no_match_is_a_valid_empty_result_not_an_error(self):
        body, is_error = call("search_decisions_index", query="zeppelin")
        self.assertFalse(is_error)
        self.assertEqual(body["results"], [])

    def test_a_query_with_no_words_is_a_validation_error(self):
        body, is_error = call("search_decisions_index", query="  ?! ")
        self.assertTrue(is_error)
        self.assertEqual(body["errorCategory"], "validation")


class ReadDecisionTest(unittest.TestCase):
    def test_reads_one_decision_with_line_numbers(self):
        body, is_error = call("read_decision", number=14)
        self.assertFalse(is_error)
        self.assertEqual(body["first_line"], 848)
        self.assertTrue(body["text"].startswith("848: ### Decision 14 — "))
        self.assertIn("Superseded by Decision 15", body["text"])
        self.assertNotIn("### Decision 15", body["text"])

    def test_an_unknown_number_is_a_structured_validation_error(self):
        body, is_error = call("read_decision", number=999)
        self.assertTrue(is_error)
        self.assertEqual(body["errorCategory"], "validation")
        self.assertFalse(body["isRetryable"])
        self.assertIn("999", body["message"])


class ReadSpecLinesTest(unittest.TestCase):
    def test_reads_a_numbered_slice(self):
        body, is_error = call("read_spec_lines", start=848, end=850)
        self.assertFalse(is_error)
        self.assertEqual(body["text"].splitlines()[0][:22], "848: ### Decision 14 —")

    def test_too_many_lines_is_a_validation_error(self):
        body, is_error = call("read_spec_lines", start=1, end=MAX_SPEC_LINES + 1)
        self.assertTrue(is_error)
        self.assertEqual(body["errorCategory"], "validation")


class DispatchTest(unittest.TestCase):
    def test_unknown_tool_and_bad_arguments_come_back_as_errors(self):
        for name, tool_input in [("delete_spec", {}), ("read_decision", {"num": 3})]:
            body, is_error = call(name, **tool_input)
            self.assertTrue(is_error, name)
            self.assertEqual(body["errorCategory"], "validation")


class DescriptionTest(unittest.TestCase):
    """2.1: each tool's description names the other tools, so Claude can tell them apart."""

    def test_each_description_points_to_the_other_tools(self):
        names = [tool["name"] for tool in TOOLS]
        for tool in TOOLS:
            others = [n for n in names if n != tool["name"]]
            mentioned = [n for n in others if n in tool["description"]]
            self.assertTrue(mentioned, f"{tool['name']} names no other tool")

    def test_schemas_are_strict(self):
        for tool in TOOLS:
            self.assertTrue(tool["strict"])
            self.assertFalse(tool["input_schema"]["additionalProperties"])


if __name__ == "__main__":
    unittest.main()

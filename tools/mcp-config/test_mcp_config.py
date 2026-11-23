"""Tests for the project-scope MCP config check (no network, no MCP server started).

StudyExampleConfig stays red until the STUDY 2.4 line in example.mcp.json is fixed.
Run: python3 -m unittest test_mcp_config -v
"""

import json
import sys
import unittest
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
from mcp_config import expand, literal_secrets, unset_variables  # noqa: E402

EXAMPLE = json.loads((HERE / "example.mcp.json").read_text())


class Expand(unittest.TestCase):
    def test_set_variable_is_substituted(self):
        self.assertEqual(expand("Bearer ${T}", {"T": "abc"}), ("Bearer abc", []))

    def test_default_used_only_when_unset(self):
        self.assertEqual(expand("${D:-./metrics}", {})[0], "./metrics")

    def test_empty_string_counts_as_set(self):
        self.assertEqual(expand("${D:-./metrics}", {"D": ""})[0], "")

    def test_unset_without_default_is_reported_and_left_as_written(self):
        self.assertEqual(expand("${T}", {}), ("${T}", ["T"]))


class Secrets(unittest.TestCase):
    def test_literal_token_is_caught(self):
        cfg = {"mcpServers": {"x": {"headers": {"Authorization": "Bearer ghp_" + "a" * 36}}}}
        self.assertEqual(literal_secrets(cfg), ["x.headers"])

    def test_variable_reference_is_not_a_secret(self):
        cfg = {"mcpServers": {"x": {"headers": {"Authorization": "Bearer ${TOKEN}"}}}}
        self.assertEqual(literal_secrets(cfg), [])


class StudyExampleConfig(unittest.TestCase):
    def test_example_has_no_literal_secret(self):
        self.assertEqual(literal_secrets(EXAMPLE), [])

    def test_example_needs_exactly_one_variable_from_each_developer(self):
        # The metrics folder has a default; the token must come from the developer's env.
        missing = unset_variables(EXAMPLE, env={})
        self.assertEqual(len(missing), 1, missing)


if __name__ == "__main__":
    unittest.main()

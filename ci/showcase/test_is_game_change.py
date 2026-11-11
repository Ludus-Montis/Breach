"""Tests for is_game_change (spec 44 step 3, Decision 167): a PR is captured when its diff
touches what the shipped game is built from, `.md` files excepted."""
import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import is_game_change  # noqa: E402


class IsGameChangeTest(unittest.TestCase):
    def test_docs_specs_tests_ci_tools_and_metrics_alone_are_not(self):
        for paths in [
            ["docs/index.md"],
            ["specs/44-project-site-and-showcase.md", "tests/sim/test_x.gd"],
            ["ci/showcase/manifest.py", ".github/workflows/showcase-capture.yml"],
            ["metrics/bench/2026-11-08-1.json", "tools/designer/index.html"],
            ["README.md", "CHANGELOG.md"],
            [],
        ]:
            self.assertFalse(is_game_change.is_game_change(paths), paths)

    def test_what_the_game_is_built_from_is(self):
        for paths in [
            ["presentation/skirmish/formation_2d/field_capture.gd"],
            ["native/rust/src/scenarios.rs", "docs/index.md"],
            ["content/scenarios/scenarios.json"],
            ["core/x.gd"],
            ["assets/units/grem.png"],
            ["main.tscn"],
            ["project.godot"],
        ]:
            self.assertTrue(is_game_change.is_game_change(paths), paths)

    def test_markdown_inside_the_game_paths_is_not(self):
        self.assertFalse(is_game_change.is_game_change(
            ["content/scenarios/README.md", "native/README.md"]))

    def test_a_prefix_must_be_a_whole_folder(self):
        self.assertFalse(is_game_change.is_game_change(["presentations/x.gd", "nativex/y.rs"]))

    def test_it_writes_its_answer_as_a_step_output(self):
        self.assertEqual(is_game_change.output_lines(True), ["game=true"])
        self.assertEqual(is_game_change.output_lines(False), ["game=false"])


if __name__ == "__main__":
    unittest.main()

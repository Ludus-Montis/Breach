"""Tests for needs_godot.py: which changes need the Godot suite and the sim checks."""
import unittest

from needs_godot import needs_godot


class NeedsGodotTest(unittest.TestCase):
    def test_docs_specs_skills_and_python_checks_do_not(self):
        for paths in [
            ["CLAUDE.md", "specs/35-unit-store.md", "OPEN_QUESTIONS.md"],
            ["AI_First_Development_Kit/principles/agent-sessions.md"],
            [".claude/skills/run-phase/SKILL.md"],
            ["ci/godot/scripts/check_ocp_shotgun_surgery.py", "ci/godot/scripts/test_x.py"],
            ["tools/designer/app.js", "tools/designer/test_export.py"],
        ]:
            self.assertFalse(needs_godot(paths), paths)

    def test_game_code_content_native_and_ci_wiring_do(self):
        for paths in [
            ["sim/skirmish/formation/unit_store.gd"],
            ["tests/sim/skirmish/formation/test_unit_store.gd"],
            ["content/units/grem.json"],
            ["presentation/skirmish/field.tscn"],
            ["native/rust/src/field.rs"],
            ["project.godot"],
            [".github/workflows/ci.yml"],
            [".pre-commit-config.yaml"],
            ["ci/godot/scripts/run_tests.sh"],
            ["README.md", "tools/formation_digest.gd"],
        ]:
            self.assertTrue(needs_godot(paths), paths)

    def test_nothing_changed_needs_nothing(self):
        self.assertFalse(needs_godot([]))


if __name__ == "__main__":
    unittest.main()

"""Checks the frontmatter of Breach's skills (task statement 3.2). No Claude call.

StudyFindDecision covers the lines marked STUDY 3.2 in .claude/skills/find-decision/SKILL.md.
Run from the repo root: python3 -m unittest discover -s tools/skill-check -v
"""

import re
import unittest
from pathlib import Path

import yaml

ROOT = Path(__file__).resolve().parents[2]
SKILLS = sorted((ROOT / ".claude" / "skills").glob("*/SKILL.md"))
WRITERS = {"Edit", "Write", "NotebookEdit"}


def parse(path: Path) -> tuple[dict, str]:
    _, front, body = path.read_text().split("---", 2)
    return yaml.safe_load(front), body


class AllSkills(unittest.TestCase):
    def test_each_has_a_name_matching_its_folder_and_a_description(self):
        for path in SKILLS:
            meta, _ = parse(path)
            self.assertEqual(meta["name"], path.parent.name, path)
            self.assertTrue(meta.get("description"), path)

    def test_a_skill_that_reads_arguments_says_what_they_are(self):
        for path in SKILLS:
            meta, body = parse(path)
            if re.search(r"\$ARGUMENTS|\$\d", body):
                self.assertIn("argument-hint", meta, path)

    def test_run_phase_hints_its_argument(self):
        meta, _ = parse(ROOT / ".claude/skills/run-phase/SKILL.md")
        self.assertIn("step", meta["argument-hint"])


class StudyFindDecision(unittest.TestCase):
    def setUp(self):
        self.meta, self.body = parse(ROOT / ".claude/skills/find-decision/SKILL.md")

    def test_runs_in_its_own_context(self):
        self.assertEqual(self.meta.get("context"), "fork")

    def test_runs_as_the_read_only_explorer(self):
        self.assertEqual(self.meta.get("agent"), "Explore")

    def test_pre_approves_no_tool_that_writes(self):
        tools = {t.strip() for t in self.meta.get("allowed-tools", "").split(",")}
        self.assertFalse(tools & WRITERS)

    def test_instructions_stand_alone(self):
        # A forked skill sees no conversation, so it must not lean on one.
        self.assertNotRegex(self.body, r"(?i)as discussed|conversation above|above in the conversation|from the conversation|the user said")


if __name__ == "__main__":
    unittest.main()

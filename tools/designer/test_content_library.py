"""content_library.py and GET /api/registry (Decision 128): trait targets follow the tags'
agreement narrowed by "only", and usage finds the content files using each id."""

from __future__ import annotations

import json
import sys
import tempfile
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from content_library import read_registry, read_units, targets  # noqa: E402

REPO = Path(__file__).resolve().parents[2]
TAGS = {
    "behaviour": {"id": "behaviour", "targets": ["unit", "leader"]},
    "combat": {"id": "combat", "targets": ["unit", "weapon"]},
    "physical": {"id": "physical", "targets": ["unit", "material"]},
}
UNIT_JSON = json.dumps({"tags": ["grem", "small"], "traits": {"cunning": 1, "hardened": 2}})


class TargetsTest(unittest.TestCase):
    def test_a_trait_sits_where_its_tags_agree(self):
        self.assertEqual(targets({"tags": ["behaviour"]}, TAGS), ["unit", "leader"])
        self.assertEqual(targets({"tags": ["combat", "physical"]}, TAGS), ["unit"])

    def test_only_narrows_the_targets(self):
        self.assertEqual(targets({"tags": ["combat"], "only": ["weapon"]}, TAGS), ["weapon"])

    def test_an_untagged_trait_sits_nowhere(self):
        self.assertEqual(targets({"tags": []}, TAGS), [])


class ReadRegistryTest(unittest.TestCase):
    def test_given_a_temp_repo_then_kinds_targets_and_usage_are_read(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            registry = root / "content" / "registry"
            registry.mkdir(parents=True)
            (registry / "tags.json").write_text(json.dumps({"tags": list(TAGS.values())}))
            traits = [
                {"id": "cunning", "tags": ["behaviour"]},
                {"id": "flows", "tags": []},
                {"id": "climber", "tags": ["physical"], "default": 0},
            ]
            (registry / "traits.json").write_text(json.dumps({"traits": traits}))
            (root / "content" / "units").mkdir()
            (root / "content" / "units" / "grem.json").write_text(UNIT_JSON)
            (root / "content" / "designer").mkdir()
            terrain = {"water": {"traits": {"flows": 1}}}
            (root / "content" / "designer" / "terrain.json").write_text(json.dumps(terrain))

            got = read_registry(root)

        self.assertEqual(got["statuses"], [])
        self.assertEqual(got["traits"][0]["targets"], ["unit", "leader"])
        self.assertEqual(got["usage"]["cunning"], ["content/units/grem.json"])
        self.assertEqual(got["usage"]["hardened"], ["content/units/grem.json"])
        self.assertEqual(got["usage"]["small"], ["content/units/grem.json"])
        self.assertEqual(got["usage"]["flows"], ["content/designer/terrain.json"])
        self.assertEqual(got["implicit"], {"climber": 0})

    def test_the_repo_registry_reads_with_known_targets_and_usage(self):
        got = read_registry(REPO)
        by_id = {t["id"]: t for t in got["traits"]}
        self.assertEqual(by_id["cunning"]["targets"], ["unit", "leader"])
        self.assertEqual(by_id["siege"]["targets"], ["weapon"])
        self.assertIn("content/weapons/brute_fists.json", got["usage"]["siege"])
        self.assertEqual(got["implicit"]["climber"], 0)
        self.assertEqual(got["implicit"]["swimmer"], 0)


class ReadUnitsTest(unittest.TestCase):
    def test_a_unit_shows_its_own_traits_and_the_implicit_ones_labelled(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            (root / "content" / "units").mkdir(parents=True)
            (root / "content" / "units" / "grem.json").write_text(UNIT_JSON)
            climbing = UNIT_JSON.replace('"hardened": 2', '"climber": 2')
            (root / "content" / "units" / "goat.json").write_text(climbing)
            got = read_units(root, {"climber": 0, "swimmer": 0})
        self.assertEqual([unit["id"] for unit in got], ["goat", "grem"])
        self.assertEqual(
            got[0]["traits"],
            [
                {"id": "climber", "level": 2, "implicit": False},
                {"id": "cunning", "level": 1, "implicit": False},
                {"id": "swimmer", "level": 0, "implicit": True},
            ],
        )
        self.assertEqual(got[1]["path"], "content/units/grem.json")
        self.assertIn({"id": "climber", "level": 0, "implicit": True}, got[1]["traits"])

    def test_the_registry_lists_the_repo_units_with_implicit_traits(self):
        got = read_registry(REPO)
        grem = next(unit for unit in got["units"] if unit["id"] == "grem")
        self.assertIn({"id": "swimmer", "level": 0, "implicit": True}, grem["traits"])


if __name__ == "__main__":
    unittest.main()

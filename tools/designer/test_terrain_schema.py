"""Tests for terrain_schema.py (spec 39 step 2b): the designer's terrain library to the crate's
schema (content/terrain/terrain_library.json) and back. Run:

    python -m unittest discover -s tools/designer -p "test_*.py"
"""

from __future__ import annotations

import json
import sys
import tempfile
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from terrain_schema import from_schema, in_use_problems, to_schema, used_ids  # noqa: E402

REPO_LIBRARY = Path(__file__).resolve().parents[2] / "content/terrain/terrain_library.json"

DESIGNER = {
    "terrains": [
        {
            "id": "DESERT",
            "label": "Desert",
            "glyph": ".",
            "color": "#d8c27a",
            "can_be_base": True,
            "needs_bridge": False,
            "move_cost": 1.5,
            "blocks_unit_classes": "cavalry, siege",
            "default_stability": 2,
            "default_max_height": 1,
            "default_max_width": 2,
            "default_max_depth": 0,
            "default_max_length": 2,
            "default_elevation": 3,
            "relief": {"amplitude": 2, "scale": 8},
            "bearing": 3,
            "foundation_max": 6,
            "dig_depth": 24,
            "strata": [{"material": "SAND", "min": 2, "max": 6}],
            "liquids": [
                {"material": "WATER", "min": 16, "max": 24, "chance": 0.2, "surface_chance": 0}
            ],
        }
    ],
    "materials": [
        {
            "id": "TIMBER",
            "label": "Timber",
            "color": "#7a5a33",
            "weight": 2,
            "span": 4,
            "strength": 6,
            "heat": 1,
            "traits": {"dig_difficulty": 1},
            "heat_transitions": [{"above": 3, "gains": "burning"}],
        }
    ],
    "features": [{"id": "ORE", "label": "Ore vein", "glyph": "o", "stub_effect": "metal"}],
    "road_move_multiplier": 0.5,
}


class ToSchemaTest(unittest.TestCase):
    def test_a_terrain_takes_gdscripts_names(self):
        desert = to_schema(DESIGNER)["terrains"][0]
        self.assertEqual(desert["display_name"], "Desert")
        self.assertEqual(desert["move_cost"], 1.5)
        self.assertEqual(desert["blocks_unit_classes"], ["cavalry", "siege"])
        self.assertEqual((desert["relief_amplitude"], desert["relief_scale"]), (2, 8))
        self.assertEqual(desert["default_elevation"], 3)
        self.assertEqual(desert["strata"], [{"material_id": "SAND", "min_cells": 2, "max_cells": 6}])
        water = desert["liquids"][0]
        self.assertEqual((water["material_id"], water["chance"]), ("WATER", 0.2))
        self.assertNotIn("label", desert)

    def test_a_material_and_its_heat_transitions(self):
        timber = to_schema(DESIGNER)["materials"][0]
        self.assertEqual((timber["display_name"], timber["strength"]), ("Timber", 6))
        self.assertEqual(
            timber["heat_transitions"],
            [{"threshold": 3, "rising": True, "becomes": "", "gains_trait": "burning"}],
        )

    def test_a_feature_keeps_its_notes(self):
        ore = to_schema(DESIGNER)["features"][0]
        self.assertEqual(ore, {"id": "ORE", "display_name": "Ore vein", "glyph": "o", "effect_notes": "metal"})

    def test_omitted_ground_fields_take_the_importers_defaults(self):
        terrain = to_schema({"terrains": [{"id": "OLD", "bearing": 4}]})["terrains"][0]
        self.assertEqual(terrain["display_name"], "OLD")
        self.assertEqual(terrain["foundation_max"], 4)
        self.assertEqual((terrain["liquids"], terrain["strata"]), ([], []))
        band = to_schema({"terrains": [{"id": "T", "strata": [{"material": "SOIL", "min": 3}]}]})
        self.assertEqual(band["terrains"][0]["strata"][0]["max_cells"], 3)

    def test_a_water_table_saved_before_decision_63_becomes_a_water_body(self):
        old = {"terrains": [{"id": "FIELDS", "water_table": {"min": 12, "max": 24}}]}
        body = to_schema(old)["terrains"][0]["liquids"][0]
        self.assertEqual((body["material_id"], body["min_cells"], body["max_cells"]), ("WATER", 12, 24))

    def test_liquids_and_temperatures_saved_before_decisions_64_and_66_convert(self):
        old = {"liquids": [{"id": "LAVA", "temperature": 1200}], "materials": [
            {"id": "ROCK", "loose": True, "dig_difficulty": 3,
             "heat_transitions": [{"above": 1100, "becomes": "LAVA"}]}]}
        materials = {entry["id"]: entry for entry in to_schema(old)["materials"]}
        self.assertEqual(materials["LAVA"]["heat"], 8)
        self.assertEqual(materials["LAVA"]["traits"], {"flows": 1})
        self.assertEqual(materials["ROCK"]["traits"], {"dig_difficulty": 3, "loose": 1})
        self.assertEqual(materials["ROCK"]["heat_transitions"][0]["threshold"], 8)

    def test_an_entry_without_an_id_is_refused(self):
        with self.assertRaisesRegex(ValueError, "terrain entry has no id"):
            to_schema({"terrains": [{"label": "Nameless"}]})


class RoundTripTest(unittest.TestCase):
    def test_the_designers_library_survives_the_schema_and_back(self):
        self.assertEqual(to_schema(from_schema(to_schema(DESIGNER))), to_schema(DESIGNER))

    def test_the_repos_library_has_a_ground_for_every_base_terrain(self):
        checked_in = json.loads(REPO_LIBRARY.read_text(encoding="utf-8"))
        for terrain in checked_in["terrains"]:
            if terrain["can_be_base"]:
                self.assertTrue(terrain["strata"], terrain["id"])
                self.assertGreater(terrain["dig_depth"], 0, terrain["id"])

    def test_the_repos_library_is_the_designers_written_back(self):
        checked_in = json.loads(REPO_LIBRARY.read_text(encoding="utf-8"))
        self.assertEqual(to_schema(from_schema(checked_in)), checked_in)


class InUseTest(unittest.TestCase):
    def setUp(self):
        self._tmp = tempfile.TemporaryDirectory()
        self.maps = Path(self._tmp.name)
        tiles = [{"terrain": "FOREST", "feature": "ORE"}, {"terrain": "FIELDS", "terrain_is_base": True}]
        demo = {"default_terrain": "FIELDS", "tiles": tiles}
        (self.maps / "demo_map.designer.json").write_text(json.dumps(demo), encoding="utf-8")

    def tearDown(self):
        self._tmp.cleanup()

    def test_ids_used_by_saved_maps_are_found(self):
        used = used_ids(self.maps)
        self.assertEqual(used["terrains"], {"FIELDS": ["demo_map"], "FOREST": ["demo_map"]})
        self.assertEqual(used["features"], {"ORE": ["demo_map"]})

    def test_dropping_a_terrain_a_saved_map_uses_is_refused(self):
        library = {"terrains": [{"id": "FIELDS"}], "features": [{"id": "ORE"}]}
        problems = in_use_problems(library, self.maps)
        self.assertEqual(len(problems), 1)
        self.assertIn("terrain 'FOREST' is still used by demo_map", problems[0])


if __name__ == "__main__":
    unittest.main()

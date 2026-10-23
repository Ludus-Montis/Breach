"""The Library page's filtering (library.js's LibraryFilter, Decision 128): search, tag
chips, target and used-only. Runs the JS with Node; skipped where Node isn't installed."""

from __future__ import annotations

import json
import shutil
import subprocess
import unittest
from pathlib import Path

HERE = Path(__file__).resolve().parent

REGISTRY = {
    "tags": [
        {"id": "behaviour", "tags": "trait", "targets": ["unit", "leader"]},
        {"id": "grem", "tags": "unit", "targets": []},
    ],
    "traits": [
        {"id": "cunning", "tags": ["behaviour"], "targets": ["unit", "leader"], "description": "Plays dead"},
        {"id": "siege", "tags": ["combat"], "targets": ["weapon"], "description": "Against structures"},
        {"id": "captor", "tags": ["behaviour"], "targets": ["unit", "leader"], "description": "Captures"},
        {"id": "climber", "tags": ["movement"], "targets": ["unit"], "description": "Climbs", "default": 0},
    ],
    "statuses": [],
}
USAGE = {"siege": ["content/weapons/brute_fists.json"]}
UNITS = [
    {
        "id": "goat",
        "traits": [
            {"id": "climber", "level": 2, "implicit": False},
            {"id": "swimmer", "level": 0, "implicit": True},
        ],
    },
    {"id": "stone", "traits": []},
]

RUNNER = """
const lib = require(process.argv[1]);
const input = JSON.parse(process.argv[2]);
const out = input.filters.map(f => lib.select(input.registry, f.kind, f, input.usage).map(e => e.id));
out.push(lib.tagChoices(input.registry, 'traits'), lib.tagChoices(input.registry, 'tags'));
out.push(input.registry.traits.map(lib.implicitOf));
out.push(input.units.map(lib.traitLine));
process.stdout.write(JSON.stringify(out));
"""


@unittest.skipIf(shutil.which("node") is None, "Node is not installed")
class LibraryFilterJsTest(unittest.TestCase):
    def run_filters(self, filters):
        payload = json.dumps(
            {"registry": REGISTRY, "usage": USAGE, "filters": filters, "units": UNITS}
        )
        result = subprocess.run(
            ["node", "-e", RUNNER, str(HERE / "library.js"), payload],
            capture_output=True,
            text=True,
            check=True,
        )
        return json.loads(result.stdout)

    def test_filters_select_sorted_entries_and_tag_choices(self):
        got = self.run_filters(
            [
                {"kind": "traits"},
                {"kind": "traits", "search": "PLAYS"},
                {"kind": "traits", "tags": ["behaviour"]},
                {"kind": "traits", "target": "weapon"},
                {"kind": "traits", "usedOnly": True},
                {"kind": "tags", "tags": ["unit"]},
            ]
        )
        self.assertEqual(got[0], ["captor", "climber", "cunning", "siege"])
        self.assertEqual(got[1], ["cunning"])
        self.assertEqual(got[2], ["captor", "cunning"])
        self.assertEqual(got[3], ["siege"])
        self.assertEqual(got[4], ["siege"])
        self.assertEqual(got[5], ["grem"])
        self.assertEqual(got[6], ["behaviour", "combat", "movement"])
        self.assertEqual(got[7], ["trait", "unit"])
        self.assertEqual(got[8], ["", "", "", "implicit 0"])

    def test_an_implicit_trait_is_labelled_with_its_default(self):
        got = self.run_filters([])
        self.assertEqual(got[-2][-1], "implicit 0")

    def test_a_units_traits_read_with_the_implicit_ones_labelled(self):
        got = self.run_filters([])
        self.assertEqual(got[-1], ["climber 2, swimmer 0 (implicit)", ""])


if __name__ == "__main__":
    unittest.main()

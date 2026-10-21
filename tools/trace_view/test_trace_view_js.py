"""The trace page's logic, trace_view.js (specs/37-outcome-trace.md, step 4): loading a base
and a head trace and the diff JSON, indexing them by tick, and which units to ring. Runs the
JS with Node; skipped where Node isn't installed.
"""

from __future__ import annotations

import json
import shutil
import subprocess
import tempfile
import unittest
from pathlib import Path

HERE = Path(__file__).resolve().parent

RUNNER = """
const view = require(process.argv[1]);
const fs = require('fs');
const read = (path) => fs.readFileSync(path, 'utf8');
const diff = process.argv[4] ? JSON.parse(read(process.argv[4])) : null;
const loaded = view.load(read(process.argv[2]), read(process.argv[3]), diff);
const asks = JSON.parse(process.argv[5]);
const out = { ticks: loaded.ticks, at: {}, ringed: {}, unit: {} };
asks.at.forEach((tick) => { out.at[tick] = loaded.at(tick); });
asks.ringed.forEach((tick) => { out.ringed[tick] = loaded.ringed(tick); });
asks.unit.forEach(([tick, id]) => { out.unit[tick + ':' + id] = loaded.unit(tick, id); });
process.stdout.write(JSON.stringify(out));
"""


def _line(tick, hp=18):
    units = [
        {"id": 57, "hp": hp, "position": [4.5, 2.0], "bearing": 90.0},
        {"id": 58, "hp": 20, "position": [4.5, 3.0], "bearing": 90.0},
    ]
    squads = [{"id": 3, "faction": "grem", "position": [4.5, 2.5], "units": units}]
    return {"run": "mirror_headon s1 rtrue", "tick": tick, "events": [], "squads": squads}


@unittest.skipIf(shutil.which("node") is None, "Node is not installed")
class TraceViewJsTest(unittest.TestCase):
    def setUp(self):
        self.dir = Path(tempfile.mkdtemp())

    def _write(self, name, text):
        path = self.dir / name
        path.write_text(text)
        return str(path)

    def _ask(self, base, head, diff=None, at=(), ringed=(), unit=()):
        files = [
            self._write("base.jsonl", "".join(json.dumps(line) + "\n" for line in base)),
            self._write("head.jsonl", "".join(json.dumps(line) + "\n" for line in head)),
            self._write("diff.json", json.dumps(diff)) if diff is not None else "",
        ]
        asks = json.dumps({"at": list(at), "ringed": list(ringed), "unit": list(unit)})
        result = subprocess.run(
            ["node", "-e", RUNNER, str(HERE / "trace_view.js"), *files, asks],
            capture_output=True,
            text=True,
            check=True,
        )
        return json.loads(result.stdout)

    def test_the_page_indexes_both_traces_by_tick(self):
        ticks = range(10, 16)
        out = self._ask([_line(t) for t in ticks], [_line(t) for t in ticks], at=ticks)
        self.assertEqual(out["ticks"], list(ticks))
        for tick in ticks:
            for side in ("base", "head"):
                seen = out["at"][str(tick)][side]
                self.assertEqual([s["id"] for s in seen["squads"]], [3])
                self.assertEqual([u["id"] for u in seen["units"]], [57, 58])
                self.assertEqual({u["squad"] for u in seen["units"]}, {3})
                self.assertEqual({u["faction"] for u in seen["units"]}, {"grem"})

    def test_the_differing_units_are_ringed(self):
        diff = {
            "first": {"tick": 12, "squad": 3, "unit": 57, "field": "hp"},
            "ticks": [
                {"tick": 12, "units": [{"id": 57, "squad": 3, "fields": ["hp"]}], "squads": []},
                {"tick": 13, "units": [{"id": 58, "squad": 3, "only": "base"}], "squads": []},
            ],
            "only": [],
        }
        ticks = range(10, 16)
        head = [_line(t, hp=15 if t >= 12 else 18) for t in ticks]
        out = self._ask([_line(t) for t in ticks], head, diff, ringed=[11, 12, 13])
        self.assertEqual(out["ringed"], {"11": [], "12": [57], "13": [58]})

    def test_a_tick_only_one_trace_has(self):
        out = self._ask(
            [_line(t) for t in range(10, 16)], [_line(t) for t in range(10, 15)], at=[15]
        )
        self.assertEqual(out["ticks"], list(range(10, 16)))
        self.assertEqual([u["id"] for u in out["at"]["15"]["base"]["units"]], [57, 58])
        self.assertEqual(out["at"]["15"]["head"], {"squads": [], "units": []})

    def test_the_selected_units_fields_on_both_sides(self):
        ticks = range(10, 16)
        head = [_line(t, hp=15) for t in ticks]
        out = self._ask([_line(t) for t in ticks], head, unit=[[12, 57], [12, 99]])
        self.assertEqual(out["unit"]["12:57"]["base"]["hp"], 18)
        self.assertEqual(out["unit"]["12:57"]["head"]["hp"], 15)
        self.assertEqual(out["unit"]["12:99"], {"base": None, "head": None})

    def test_infinity_and_nan_as_the_trace_writes_them(self):
        line = _line(10)
        text = json.dumps(line).replace('"bearing": 90.0', '"bearing": NaN', 1)
        text = text.replace('"hp": 20', '"hp": -Infinity', 1)
        base = self._write("odd.jsonl", text + "\n")
        result = subprocess.run(
            [
                "node",
                "-e",
                RUNNER,
                str(HERE / "trace_view.js"),
                base,
                base,
                "",
                json.dumps({"at": [], "ringed": [], "unit": [[10, 57], [10, 58]]}),
            ],
            capture_output=True,
            text=True,
            check=True,
        )
        out = json.loads(result.stdout)
        self.assertEqual(out["unit"]["10:57"]["base"]["bearing"], "NaN")
        self.assertEqual(out["unit"]["10:58"]["base"]["hp"], "-Infinity")


if __name__ == "__main__":
    unittest.main()

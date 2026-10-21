"""Tests for trace_diff.py (specs/37-outcome-trace.md, step 2): two outcome traces of a run,
the base's and the head's, matched by squad and unit id, and the first difference named."""
import contextlib
import io
import json
import tempfile
import unittest
from pathlib import Path

from trace_diff import main

RUN = "mirror_headon s1 rtrue"
DOWNED = [2, [['"type"', "downed"], ['"unit"', 58]]]


def _unit(unit_id, hp=18, position=(4.5, 2.0)):
    return {"id": unit_id, "hp": hp, "position": list(position), "target_id": 0}


def _tick(tick, units, events=None, squad_morale=60):
    squad = {"id": 3, "morale": squad_morale, "units": units}
    return {"run": RUN, "tick": tick, "events": events or [], "squads": [squad]}


def _base(tick):
    return _tick(tick, [_unit(57), _unit(60), _unit(61)])


def _head(tick):
    """The base until tick 411; from 412 unit 57 is hit (18 to 15) and unit 60 has moved,
    with a `downed` event only the head has at 412; from 420 unit 61 is gone."""
    if tick < 412:
        return _base(tick)
    units = [_unit(57, hp=15), _unit(60, position=(5.0, 2.0))]
    if tick < 420:
        units.append(_unit(61))
    return _tick(tick, units, [DOWNED] if tick == 412 else [])


class TraceDiffTest(unittest.TestCase):
    def setUp(self):
        self.dir = Path(tempfile.mkdtemp())

    def _write(self, name, lines):
        path = self.dir / name
        path.write_text("".join(json.dumps(line) + "\n" for line in lines))
        return str(path)

    def _run(self, base, head, *extra):
        out = io.StringIO()
        with contextlib.redirect_stdout(out):
            code = main([self._write("base.jsonl", base), self._write("head.jsonl", head), *extra])
        return code, out.getvalue().splitlines()

    def _changed(self, *extra):
        ticks = range(405, 425)
        return self._run([_base(t) for t in ticks], [_head(t) for t in ticks], *extra)

    def test_the_first_difference_is_named(self):
        code, lines = self._changed()
        self.assertEqual(code, 1)
        self.assertEqual(lines[0], "tick 412  squad 3  unit 57  hp: 18 -> 15")

    def test_one_sides_events_are_listed(self):
        _code, lines = self._changed()
        events = lines[lines.index("events at tick 412 only one side has:") + 1 :]
        self.assertEqual(events[0], "  head: " + json.dumps(DOWNED))
        self.assertNotIn("  base:", events[1])

    def test_each_tick_from_the_first_says_how_many_units_differ_and_in_what(self):
        _code, lines = self._changed()
        self.assertIn("tick 412  units 2  hp, position  events 1", lines)
        self.assertIn("tick 413  units 2  hp, position", lines)
        self.assertIn("tick 424  units 2  hp, position  base only 1", lines)
        self.assertFalse(any(line.startswith("tick 411  units") for line in lines))

    def test_units_are_matched_by_id_not_by_place(self):
        ticks = range(10, 16)
        head = [_base(t) for t in ticks]
        for line in head:
            line["squads"][0]["units"].reverse()
        code, lines = self._run([_base(t) for t in ticks], head)
        self.assertEqual(code, 0)
        self.assertEqual(lines, ["the traces are the same"])

    def test_a_unit_only_one_side_has(self):
        _code, lines = self._changed()
        self.assertIn("unit 61 (squad 3): base only, from tick 420", lines)

    def test_the_json_for_the_page(self):
        out = self.dir / "out.json"
        self._changed("--json", str(out))
        report = json.loads(out.read_text())
        ticks = {entry["tick"]: entry for entry in report["ticks"]}
        self.assertEqual(sorted(ticks), list(range(412, 425)))
        self.assertEqual(
            ticks[412]["units"],
            [
                {"id": 57, "squad": 3, "fields": ["hp"]},
                {"id": 60, "squad": 3, "fields": ["position"]},
            ],
        )
        self.assertIn({"id": 61, "squad": 3, "only": "base"}, ticks[420]["units"])
        self.assertEqual(report["first"]["unit"], 57)


if __name__ == "__main__":
    unittest.main()

"""Tests for bench_compare.py: tick_phases outputs from alternating rounds are read per
phase, and each phase's medians, change and spread are compared, marked where the two
sides' ranges don't overlap."""
import unittest

from bench_compare import compare, phases, report

RUN = """machine: AMD EPYC 7763 64-Core Processor, 4 threads, Linux
rust, 4096 a side, 32 wide: {tick} ms a tick
  deaths (bury)                       {deaths}
  scrum: seek: the core's work        {seek}
  scrum: seek: gather the rules' say     1.50
copies a tick (column + commit: calls, slots; value, put):
  units walk to places               73.0 +  16.0     729088         0        0
      position 25, rank 24
"""


def run(tick, deaths, seek):
    return RUN.format(tick=tick, deaths=deaths, seek=seek)


class BenchCompareTest(unittest.TestCase):
    def test_a_run_is_read_as_its_tick_and_each_phase(self):
        read = phases(run("314.3", "25.18", "31.65"))
        self.assertEqual(read, {
            "tick": 314.3,
            "deaths (bury)": 25.18,
            "scrum: seek: the core's work": 31.65,
            "scrum: seek: gather the rules' say": 1.5,
        })

    def test_each_phase_compares_medians_and_ranges(self):
        head = [phases(run(t, d, "30.00")) for t, d in [("300.0", "20.00"), ("310.0", "21.00"),
                                                        ("305.0", "22.00")]]
        base = [phases(run(t, d, "30.00")) for t, d in [("330.0", "26.00"), ("320.0", "25.00"),
                                                        ("325.0", "27.00")]]
        rows = {row["phase"]: row for row in compare(head, base)}
        deaths = rows["deaths (bury)"]
        self.assertEqual((deaths["head"], deaths["base"]), (21.0, 26.0))
        self.assertAlmostEqual(deaths["change"], -19.2, places=1)
        self.assertEqual((deaths["head_range"], deaths["base_range"]), ((20.0, 22.0), (25.0, 27.0)))
        self.assertTrue(deaths["apart"])
        self.assertFalse(rows["scrum: seek: the core's work"]["apart"])

    def test_overlapping_ranges_are_not_apart(self):
        head = [phases(run("300.0", d, "30.00")) for d in ("20.00", "26.00")]
        base = [phases(run("300.0", d, "30.00")) for d in ("25.00", "27.00")]
        rows = {row["phase"]: row for row in compare(head, base)}
        self.assertFalse(rows["deaths (bury)"]["apart"])

    def test_the_tick_comes_first_then_the_base_slowest_first(self):
        head = [phases(run("300.0", "20.00", "40.00"))]
        base = [phases(run("310.0", "25.00", "35.00"))]
        order = [row["phase"] for row in compare(head, base)]
        self.assertEqual(order[:3], ["tick", "scrum: seek: the core's work", "deaths (bury)"])

    def test_a_phase_one_side_lacks_is_left_out(self):
        head = [{"tick": 300.0, "new phase": 2.0}]
        base = [{"tick": 310.0, "old phase": 3.0}]
        self.assertEqual([row["phase"] for row in compare(head, base)], ["tick"])

    def test_the_report_marks_phases_apart(self):
        head = [phases(run("300.0", d, "30.00")) for d in ("20.00", "21.00")]
        base = [phases(run("330.0", d, "30.00")) for d in ("25.00", "26.00")]
        lines = report(compare(head, base), rounds=2)
        deaths = next(line for line in lines if line.lstrip().startswith("deaths (bury)"))
        seek = next(line for line in lines if "the core's work" in line)
        self.assertTrue(deaths.rstrip().endswith("*"), deaths)
        self.assertFalse(seek.rstrip().endswith("*"), seek)
        self.assertIn("2 rounds", lines[0])


if __name__ == "__main__":
    unittest.main()

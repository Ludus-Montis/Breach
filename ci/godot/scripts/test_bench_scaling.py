"""Tests for bench_scaling.py: a large battle's tick_phases runs set against a small one's, each
phase's median grown by a ratio, checked against the growth expected (spec 39 step 10a): the
tick within it says the small one can stand in, and the phases that grew faster are marked."""
import unittest

from bench_scaling import grown, report, stands_in

RUN = """machine: AMD EPYC 7763 64-Core Processor, 4 threads, Linux
{side} a side, 32 wide: {tick} ms a tick
  scrum: seek: the core's work        {seek}
  wounds: tend                        {tend}
series: 50 ticks from tick 212 (ms)       mean     95th     99th    worst  at tick
  tick                                 {tick}    9.00     9.00    11.00      230
"""


def run(side, tick, seek, tend):
    return RUN.format(side=side, tick=tick, seek=seek, tend=tend)


SMALL = [run(4096, "95.0", "38.00", "25.00"), run(4096, "97.0", "40.00", "27.00")]
LARGE = [run(24576, "566.0", "228.00", "200.00"), run(24576, "570.0", "240.00", "210.00")]


class GrownTest(unittest.TestCase):
    def test_each_phase_is_its_large_median_over_its_small_median(self):
        rows = {row[0]: row for row in grown(LARGE, SMALL)}
        self.assertAlmostEqual(rows["tick"][3], 568.0 / 96.0)
        self.assertAlmostEqual(rows["scrum: seek: the core's work"][3], 234.0 / 39.0)
        self.assertAlmostEqual(rows["wounds: tend"][3], 205.0 / 26.0)

    def test_the_tick_comes_first_then_the_phases_largest_first(self):
        names = [row[0] for row in grown(LARGE, SMALL)]
        self.assertEqual(names, ["tick", "scrum: seek: the core's work", "wounds: tend"])

    def test_a_phase_only_one_side_ran_is_left_out(self):
        large = [LARGE[0] + "  roster                               3.00\n"]
        names = [row[0] for row in grown(large, SMALL[:1])]
        self.assertNotIn("roster", names)


class StandsInTest(unittest.TestCase):
    def test_a_tick_grown_within_a_fifth_of_the_factor_stands_in(self):
        self.assertTrue(stands_in(grown(LARGE, SMALL), 6.0))

    def test_a_tick_grown_well_past_the_factor_doesnt(self):
        self.assertFalse(stands_in(grown(LARGE, SMALL), 4.0))

    def test_a_tick_grown_well_short_of_the_factor_doesnt(self):
        self.assertFalse(stands_in(grown(LARGE, SMALL), 8.0))


class ReportTest(unittest.TestCase):
    def test_a_phase_grown_past_the_band_is_marked_and_one_within_isnt(self):
        lines = report(grown(LARGE, SMALL), 6.0)
        tend = next(line for line in lines if "wounds: tend" in line)
        seek = next(line for line in lines if "seek" in line)
        self.assertTrue(tend.endswith("^"))
        self.assertFalse(seek.endswith("^"))

    def test_the_last_line_gives_the_verdict(self):
        self.assertIn("stands in", report(grown(LARGE, SMALL), 6.0)[-1])
        self.assertIn("doesn't stand in", report(grown(LARGE, SMALL), 4.0)[-1])


if __name__ == "__main__":
    unittest.main()

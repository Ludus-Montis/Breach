"""Tests for shard_tests.py: every test file lands in exactly one shard, the shards are
balanced by the recorded times, and the same inputs always give the same shards."""
import unittest

from shard_tests import shards, times_from_report


class ShardTestsTest(unittest.TestCase):
    FILES = [f"tests/test_{name}.gd" for name in "abcdefgh"]

    def test_every_file_lands_in_exactly_one_shard(self):
        split = shards(self.FILES, {}, 3)
        self.assertEqual(len(split), 3)
        self.assertEqual(sorted(f for shard in split for f in shard), sorted(self.FILES))

    def test_slow_files_are_spread_across_shards(self):
        times = {"tests/test_a.gd": 10.0, "tests/test_b.gd": 10.0, "tests/test_c.gd": 10.0}
        split = shards(self.FILES, times, 3)
        for shard in split:
            slow = [f for f in shard if times.get(f, 0.0) >= 10.0]
            self.assertEqual(len(slow), 1, split)

    def test_a_file_with_no_recorded_time_counts_as_the_median(self):
        times = {"tests/test_a.gd": 1.0, "tests/test_b.gd": 3.0, "tests/test_c.gd": 9.0}
        split = shards(["tests/test_a.gd", "tests/test_b.gd", "tests/test_c.gd",
                        "tests/test_new.gd"], times, 2)
        loads = sorted(sum(times.get(f, 3.0) for f in shard) for shard in split)
        self.assertEqual(loads, [7.0, 9.0])

    def test_the_same_inputs_give_the_same_shards(self):
        times = {"tests/test_a.gd": 2.0, "tests/test_b.gd": 2.0}
        self.assertEqual(shards(self.FILES, times, 4), shards(list(reversed(self.FILES)), times, 4))

    def test_times_are_read_per_file_from_a_gdunit_report(self):
        report = """<testsuites>
          <testsuite name="test_a" package="tests/sim">
            <testcase name="one" time="1.5"/><testcase name="two" time="0.5"/>
          </testsuite>
          <testsuite name="test_b" package="tests">
            <testcase name="one" time="3.0"/>
          </testsuite>
        </testsuites>"""
        self.assertEqual(
            times_from_report(report), {"tests/sim/test_a.gd": 2.0, "tests/test_b.gd": 3.0}
        )


if __name__ == "__main__":
    unittest.main()

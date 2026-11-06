"""Tests for check_hardware.py: one digest set taken on x86-64 and on arm64 must be equal tick
for tick (spec 42 step 1, Decision 161: the same result on any hardware). A difference fails
and names the set and the first run and tick that differ; a missing digest fails too."""
import contextlib
import io
import tempfile
import unittest
from pathlib import Path

import check_hardware
from check_hardware import verdict

X86 = [
    "mirror_headon s1 rtrue 1 aaaa",
    "mirror_headon s1 rtrue 2 bbbb",
    "field3 rtrue 1 cccc",
]


class VerdictTest(unittest.TestCase):
    def test_equal_digests_pass(self):
        passed, line = verdict("standard", X86, list(X86))
        self.assertTrue(passed)
        self.assertEqual(line, "hardware standard: equal on x86-64 and arm64, tick for tick.")

    def test_a_differing_tick_fails_naming_the_set_and_the_first_tick(self):
        arm = [X86[0], "mirror_headon s1 rtrue 2 dddd", "field3 rtrue 1 eeee"]
        passed, line = verdict("standard", X86, arm)
        self.assertFalse(passed)
        self.assertEqual(
            line,
            "hardware standard: DIFFER on arm64, first at mirror_headon s1 rtrue tick 2"
            " (2 runs differ).",
        )

    def test_a_run_cut_short_on_one_machine_differs(self):
        passed, line = verdict("standard", X86, X86[:1] + X86[2:])
        self.assertFalse(passed)
        self.assertIn("DIFFER on arm64, first at mirror_headon s1 rtrue tick 2", line)

    def test_a_missing_digest_fails(self):
        for x86, arm in ((X86, []), ([], X86)):
            passed, line = verdict("standard", x86, arm)
            self.assertFalse(passed)
            self.assertIn("hardware standard: MISSING", line)


class MainTest(unittest.TestCase):
    def _run(self, x86: list, arm: list):
        with tempfile.TemporaryDirectory() as folder:
            files = []
            for name, lines in (("x86", x86), ("arm", arm)):
                path = Path(folder) / f"{name}.txt"
                path.write_text("".join(f"{line}\n" for line in lines))
                files.append(str(path))
            out = io.StringIO()
            with contextlib.redirect_stdout(out):
                code = check_hardware.main(["check_hardware.py", "standard", *files])
        return code, out.getvalue()

    def test_equal_files_exit_zero(self):
        code, out = self._run(X86, X86)
        self.assertEqual(code, 0)
        self.assertIn("equal on x86-64 and arm64", out)

    def test_differing_files_exit_one(self):
        code, out = self._run(X86, X86[:2] + ["field3 rtrue 1 ffff"])
        self.assertEqual(code, 1)
        self.assertIn("first at field3 rtrue tick 1", out)

    def test_too_few_arguments_print_usage(self):
        out = io.StringIO()
        with contextlib.redirect_stdout(out):
            self.assertEqual(check_hardware.main(["check_hardware.py", "standard"]), 2)


if __name__ == "__main__":
    unittest.main()

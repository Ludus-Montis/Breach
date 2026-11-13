"""`bench-inspect` and `bench-series` read the store (spec 49 step 6, Decision 179): every side
they build (the head, the base, the references, each past commit of a backfill) comes from
`store.sh fetch`, so commits that share a `native/` tree build it once, the first to need it.
The workflows are read from their text and `bench_series.checkout` with its commands caught:
the runs themselves are CI's."""
import re
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path
from unittest import mock

ROOT = Path(__file__).resolve().parents[2]
INSPECT = ROOT / ".github/workflows/bench-inspect.yml"
SERIES = ROOT / ".github/workflows/bench-series.yml"
sys.path.insert(0, str(ROOT / "ci" / "metrics"))
import bench_series  # noqa: E402


def jobs(text):
    """Each job's name and its lines, from the workflow's text."""
    found, name, in_jobs = {}, None, False
    for line in text.splitlines():
        if line.startswith("jobs:"):
            in_jobs = True
            continue
        if not in_jobs:
            continue
        match = re.match(r"^  ([A-Za-z0-9_-]+):\s*$", line)
        if match:
            name = match.group(1)
            found[name] = []
        elif name:
            found[name].append(line)
    return {k: "\n".join(v) for k, v in found.items()}


def code(job_text):
    """The job's lines without comments, so a mention in a comment doesn't count."""
    return "\n".join(l for l in job_text.splitlines() if not l.lstrip().startswith("#"))


def may_use_the_store(test, name, text):
    test.assertIn("GH_TOKEN: ${{ github.token }}", text, name)
    test.assertIn("GH_REPO: ${{ github.repository }}", text, name)
    test.assertRegex(text, r"permissions:\s*\n\s+contents: write", name)


class BenchInspectReadsTheStoreTest(unittest.TestCase):
    def setUp(self):
        self.jobs = jobs(INSPECT.read_text(encoding="utf-8"))

    def test_no_job_builds_the_binaries_itself(self):
        for name, text in self.jobs.items():
            self.assertNotIn("cargo build", code(text), name)
            self.assertNotIn("native/build.sh", code(text), name)

    def test_the_head_the_base_and_the_reference_come_from_the_store(self):
        text = code(self.jobs["build"])
        self.assertRegex(text, r"ci/native/store\.sh fetch \S+ release ")
        self.assertRegex(text, r"for side in head base reference")

    def test_the_build_job_may_read_and_write_the_store(self):
        may_use_the_store(self, "build", self.jobs["build"])


class BenchSeriesReadsTheStoreTest(unittest.TestCase):
    def setUp(self):
        self.jobs = jobs(SERIES.read_text(encoding="utf-8"))

    def test_no_job_builds_the_binaries_itself(self):
        for name, text in self.jobs.items():
            self.assertNotIn("cargo build", code(text), name)
            self.assertNotIn("native/build.sh", code(text), name)

    def test_the_jobs_that_check_out_commits_may_read_and_write_the_store(self):
        for name in ("references", "bench"):
            may_use_the_store(self, name, self.jobs[name])


class CheckoutFetchesFromTheStoreTest(unittest.TestCase):
    """A backfill's commit gets its binaries from `store.sh fetch`; the store builds a missing
    key once and uploads it, so the next commit with the same `native/` tree downloads it."""

    def checkout(self, sha):
        calls = []

        def run(command, **kwargs):
            calls.append(command)
            if command[:3] == ["git", "worktree", "add"]:
                Path(command[-2]).mkdir(parents=True)
            if command[:2] == ["bash", "ci/native/store.sh"]:
                Path(command[-1]).mkdir(parents=True, exist_ok=True)
                (Path(command[-1]) / "breach_tick_phases").write_text("")
            return subprocess.CompletedProcess(command, 0, "", "")

        with mock.patch.object(bench_series.subprocess, "run", side_effect=run):
            folder = bench_series.checkout(sha, False, "")
            bench_series.checkout(sha, False, "")
        return folder, calls

    def setUp(self):
        self.cwd = Path.cwd()
        work = tempfile.mkdtemp()
        import os
        os.chdir(work)
        self.addCleanup(os.chdir, self.cwd)

    def test_a_commit_s_binaries_come_from_the_store_into_its_tree(self):
        sha = "ab" * 20
        folder, calls = self.checkout(sha)
        fetches = [c for c in calls if c[:2] == ["bash", "ci/native/store.sh"]]
        self.assertEqual(fetches, [["bash", "ci/native/store.sh", "fetch", sha, "release",
                                    "x86_64-unknown-linux-gnu", str(folder / "native" / "bin")]])

    def test_nothing_is_built_in_the_tree(self):
        _, calls = self.checkout("cd" * 20)
        for command in calls:
            self.assertNotIn("native/build.sh", command)


if __name__ == "__main__":
    unittest.main()

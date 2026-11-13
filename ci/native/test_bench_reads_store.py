"""`bench` reads the store (spec 49 step 5, Decision 179): its head, base and reference come
from `store.sh fetch` (`release`), and the sampling profile's binary from its `profiling` key,
instead of being built in worktrees. Read from the workflow's text: the run itself is CI's."""
import re
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
BENCH = ROOT / ".github/workflows/bench.yml"


def jobs(text):
    """Each job's name and its lines, from the workflow's text."""
    found, name = {}, None
    in_jobs = False
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


class BenchReadsTheStoreTest(unittest.TestCase):
    def setUp(self):
        self.jobs = jobs(BENCH.read_text(encoding="utf-8"))

    def test_no_job_builds_the_release_binaries_itself(self):
        for name, text in self.jobs.items():
            self.assertNotIn("native/build.sh", code(text), name)

    def test_every_job_fetches_its_sides_from_the_store(self):
        for name in ("bench", "record", "record-25k"):
            text = code(self.jobs[name])
            self.assertRegex(text, r"ci/native/store\.sh fetch \S+ release ", name)

    def test_the_reference_is_fetched_into_its_tree(self):
        for name in ("record", "record-25k"):
            self.assertRegex(code(self.jobs[name]), r'store\.sh fetch "\$REFERENCE" release .*REF_TREE', name)

    def test_the_profile_of_tick_phases_comes_from_its_profiling_key(self):
        self.assertRegex(code(self.jobs["bench"]), r"store\.sh fetch \S+ profiling ")

    def test_every_job_that_fetches_may_read_and_write_the_store(self):
        for name, text in self.jobs.items():
            if "store.sh fetch" not in code(text):
                continue
            self.assertIn("GH_TOKEN: ${{ github.token }}", text, name)
            self.assertIn("GH_REPO: ${{ github.repository }}", text, name)
            self.assertRegex(text, r"permissions:\s*\n\s+contents: write", name)


if __name__ == "__main__":
    unittest.main()

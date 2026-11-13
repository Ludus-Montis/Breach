"""sim-checks reads the store (spec 49 step 7, Decision 179).

Given a PR that doesn't touch native/, its merge commit's native/ tree is main's, so its key
is main's (store_key.sh): `build` and `base-build` download main's binaries and build
nothing. The workflow is checked here as text: each of the two jobs fetches its key with
`ci/native/store.sh fetch`, never builds the core itself, and may write to the store.
"""
import re
import unittest
from pathlib import Path

WORKFLOW = Path(__file__).resolve().parents[2] / ".github" / "workflows" / "sim-checks.yml"
LINUX = "x86_64-unknown-linux-gnu"


def job(name: str) -> str:
    """The text of one job under `jobs:`, up to the next job."""
    text = WORKFLOW.read_text(encoding="utf-8")
    jobs = text[text.index("\njobs:\n") :]
    match = re.search(rf"\n  {re.escape(name)}:\n(.*?)(?=\n  [A-Za-z0-9_-]+:\n|\Z)", jobs, re.S)
    if match is None:
        raise AssertionError(f"sim-checks.yml has no job {name}")
    return match.group(1)


class SimChecksReadsTheStoreTest(unittest.TestCase):
    def assert_fetches(self, name: str, commit: str):
        body = job(name)
        fetch = re.search(
            rf"store\.sh fetch\s+{re.escape(commit)}\s+release\s+{LINUX}\s+(\S+)", body
        )
        self.assertIsNotNone(fetch, f"{name} fetches {commit}'s release key")
        self.assertEqual(fetch.group(1), "native/bin", f"{name} fetches into native/bin")
        self.assertNotIn("native/build.sh", body, f"{name} builds only through the store")
        self.assertNotIn("native/rust/target", body, f"{name} keeps no cargo cache")
        self.assertIn("GH_TOKEN: ${{ github.token }}", body)
        self.assertRegex(body, r"permissions:\s*\n\s+contents: write")
        self.assertIn("fetch-depth: 0", body)

    def test_the_prs_own_build_fetches_its_key(self):
        self.assert_fetches("build", "HEAD")

    def test_the_bases_build_fetches_the_merge_bases_key(self):
        self.assert_fetches("base-build", '"$(git merge-base "$BASE_REF" HEAD)"')

    def test_the_base_is_keyed_with_the_heads_pin(self):
        # The key reads the pin from the working tree: the head's, as Decision 179 builds
        # every tree with the pin of the commit doing the building. No checkout of the base.
        self.assertNotIn("git checkout", job("base-build"))


if __name__ == "__main__":
    unittest.main()

"""Tests for check_build_inputs.py: no build input of the native core lives outside native/."""
import tempfile
import unittest
from pathlib import Path

from check_build_inputs import REPO_ROOT, find_build_inputs, tracked_files


def _write(root: Path, rel: str, text: str) -> str:
    path = root / rel
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(text, encoding="utf-8")
    return rel


class CheckBuildInputsTest(unittest.TestCase):
    def setUp(self):
        self._tmp = tempfile.TemporaryDirectory()
        self.root = Path(self._tmp.name)

    def tearDown(self):
        self._tmp.cleanup()

    def test_a_workflow_that_sets_rustflags_fails_and_is_named(self):
        rel = _write(
            self.root,
            ".github/workflows/bench.yml",
            "jobs:\n  b:\n    env:\n      RUSTFLAGS: -C target-cpu=native\n",
        )
        findings = find_build_inputs(self.root, [rel])
        self.assertEqual(len(findings), 1)
        self.assertEqual(findings[0].path, rel)
        self.assertEqual(findings[0].line, 4)
        self.assertIn("RUSTFLAGS", findings[0].text)

    def test_every_listed_input_is_caught(self):
        lines = [
            "export CARGO_PROFILE_RELEASE_LTO=fat",
            "export CARGO_BUILD_TARGET=x86_64-unknown-linux-gnu",
            "cargo build --features simd",
            "cargo rustc -- -C target-cpu=native",
            "RUSTUP_TOOLCHAIN=nightly cargo build",
        ]
        rel = _write(self.root, "ci/godot/scripts/build.sh", "\n".join(lines) + "\n")
        findings = find_build_inputs(self.root, [rel])
        self.assertEqual([f.line for f in findings], [1, 2, 3, 4, 5])

    def test_native_and_the_store_helper_are_allowed(self):
        paths = [
            _write(self.root, "native/build.sh", "RUSTFLAGS=x cargo build\n"),
            _write(self.root, "ci/native/store.sh", "RUSTUP_TOOLCHAIN=1.90 cargo build\n"),
        ]
        self.assertEqual(find_build_inputs(self.root, paths), [])

    def test_prose_is_not_a_build_input(self):
        paths = [
            _write(self.root, "ci/godot/README.md", "Never set RUSTFLAGS in a workflow.\n"),
            _write(self.root, "specs/49-x.md", "`--features` and `target-cpu`\n"),
        ]
        self.assertEqual(find_build_inputs(self.root, paths), [])

    def test_todays_workflows_and_scripts_pass(self):
        self.assertEqual(find_build_inputs(REPO_ROOT, tracked_files(REPO_ROOT)), [])


if __name__ == "__main__":
    unittest.main()

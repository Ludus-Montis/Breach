"""store_key.sh: the key a native binary is stored under (spec 49 step 1, Decision 179).

Each test builds a throwaway repo holding a copy of the script, so the script reads that
repo's pin, as it reads the head's in the real one.
"""

import shutil
import subprocess
import tempfile
import unittest
from pathlib import Path

SCRIPT = Path(__file__).resolve().parent / "store_key.sh"
PIN = '[toolchain]\nchannel = "{}"\ncomponents = ["rustfmt", "clippy"]\n'


def git(repo, *args):
    return subprocess.run(
        ["git", "-C", str(repo), *args], check=True, capture_output=True, text=True
    ).stdout.strip()


class StoreKeyTest(unittest.TestCase):
    def setUp(self):
        self.repo = Path(tempfile.mkdtemp())
        self.addCleanup(shutil.rmtree, self.repo)
        git(self.repo, "init", "-q")
        git(self.repo, "config", "user.email", "test@example.com")
        git(self.repo, "config", "user.name", "test")
        (self.repo / "ci/native").mkdir(parents=True)
        shutil.copy(SCRIPT, self.repo / "ci/native/store_key.sh")
        self.write("native/rust/rust-toolchain.toml", PIN.format("1.99.1"))
        self.write("native/rust/Cargo.toml", '[profile.release]\nlto = "thin"\n')
        self.write("native/rust/Cargo.lock", "version = 4\n")
        self.write("native/rust/src/lib.rs", "pub fn f() {}\n")
        self.write("native/build.sh", "cargo build\n")
        self.write("README.md", "Breach\n")
        self.base = self.commit("base")

    def write(self, path, text):
        target = self.repo / path
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_text(text)

    def commit(self, message):
        git(self.repo, "add", "-A")
        git(self.repo, "commit", "-q", "-m", message)
        return git(self.repo, "rev-parse", "HEAD")

    def key(self, commit, profile="release", target="x86_64-unknown-linux-gnu"):
        return subprocess.run(
            ["bash", str(self.repo / "ci/native/store_key.sh"), commit, profile, target],
            check=True, capture_output=True, text=True, cwd=self.repo,
        ).stdout.strip()

    def test_the_key_names_tree_toolchain_profile_and_target(self):
        tree = git(self.repo, "rev-parse", f"{self.base}:native")
        self.assertEqual(
            self.key(self.base), f"{tree}-1.99.1-release-x86_64-unknown-linux-gnu"
        )

    def test_a_change_outside_native_keeps_the_key(self):
        self.write("README.md", "Breach, changed\n")
        self.write("ci/other.yml", "on: push\n")
        self.assertEqual(self.key(self.base), self.key(self.commit("outside")))

    def test_a_change_to_a_build_input_changes_the_key(self):
        inputs = {
            "native/rust/src/lib.rs": "pub fn g() {}\n",
            "native/rust/Cargo.lock": "version = 4\n# bumped\n",
            "native/rust/Cargo.toml": '[profile.release]\nlto = "fat"\n',
            "native/build.sh": "cargo build -j 4\n",
            "native/rust/rust-toolchain.toml": PIN.format("1.99.2"),
        }
        before = self.key(self.base)
        for path, text in inputs.items():
            with self.subTest(path=path):
                self.write(path, text)
                after = self.key(self.commit(path))
                self.assertNotEqual(before, after)
                before = after

    def test_profiles_and_targets_key_apart(self):
        self.assertNotEqual(self.key(self.base, "release"), self.key(self.base, "profiling"))
        self.assertNotEqual(
            self.key(self.base, target="x86_64-unknown-linux-gnu"),
            self.key(self.base, target="x86_64-pc-windows-msvc"),
        )

    def test_the_toolchain_is_the_heads_not_the_keyed_commits(self):
        self.write("native/rust/rust-toolchain.toml", PIN.format("1.99.2"))
        self.commit("newer pin")
        self.assertIn("-1.99.2-release-", self.key(self.base))

    def test_a_commit_before_the_pin_is_keyed_with_the_heads(self):
        git(self.repo, "rm", "-q", "native/rust/rust-toolchain.toml")
        unpinned = self.commit("no pin")
        git(self.repo, "checkout", "-q", self.base)
        self.assertIn("-1.99.1-release-", self.key(unpinned))

    def test_missing_arguments_fail(self):
        result = subprocess.run(
            ["bash", str(self.repo / "ci/native/store_key.sh"), self.base],
            capture_output=True, text=True, cwd=self.repo,
        )
        self.assertNotEqual(result.returncode, 0)


if __name__ == "__main__":
    unittest.main()

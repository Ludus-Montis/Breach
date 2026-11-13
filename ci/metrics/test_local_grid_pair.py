"""local_grid.sh pair: each side's `release` binaries come from the `native-store`
pre-release over HTTPS, without credentials, and a side is built only when its key is
missing; then both are built, never one of each (spec 49 step 8, Q48 point 2).

Each test builds a throwaway repo holding copies of local_grid.sh and store_key.sh, a fake
native/build.sh and a fake local_pair.py, and puts a fake curl first on PATH. The fake curl
serves the store from a directory and logs every call.
"""

import os
import shutil
import stat
import subprocess
import tempfile
import unittest
import zipfile
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parent.parent
TARGET = "x86_64-unknown-linux-gnu"

FAKE_CURL = r"""#!/usr/bin/env bash
# A fake curl: `curl -fsSL -o <file> <url>`, served from $FAKE_STORE by the url's last part.
echo "curl $*" >> "$FAKE_LOG"
out= url=
while [ "$#" -gt 0 ]; do
	case "$1" in -o) out="$2"; shift ;; -*) ;; *) url="$1" ;; esac
	shift
done
[ -f "$FAKE_STORE/${url##*/}" ] || exit 22
cp "$FAKE_STORE/${url##*/}" "$out"
"""

# A fake breach_tick_phases: a settled series names tick 7; anything else prints a line.
FAKE_PHASES = """#!/usr/bin/env bash
case "$*" in
	--help) echo "usage: breach_tick_phases" ;;
	*series=1*) echo "inspect: 7" ;;
	*) echo "phases $*" ;;
esac
"""

FAKE_BUILD = """#!/usr/bin/env bash
# A fake native/build.sh: logs where it ran and fills native/bin/.
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
echo "build.sh $(cat "$here/rust/src/lib.rs")" >> "$FAKE_LOG"
mkdir -p "$here/bin"
cp "$here/phases.sh" "$here/bin/breach_tick_phases"
chmod +x "$here/bin/breach_tick_phases"
"""

FAKE_LOCAL_PAIR = """import sys
print("local_pair " + sys.argv[1])
"""


def git(repo, *args):
    return subprocess.run(
        ["git", "-C", str(repo), *args], check=True, capture_output=True, text=True
    ).stdout.strip()


def executable(path, text):
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(text)
    path.chmod(path.stat().st_mode | stat.S_IEXEC)


class LocalPairTest(unittest.TestCase):
    def setUp(self):
        self.tmp = Path(tempfile.mkdtemp())
        self.addCleanup(shutil.rmtree, self.tmp)
        self.repo = self.tmp / "repo"
        self.store = self.tmp / "store"
        self.store.mkdir()
        self.log = self.tmp / "log"
        self.log.write_text("")
        fakes = self.tmp / "fakes"
        executable(fakes / "curl", FAKE_CURL)

        self.repo.mkdir()
        git(self.repo, "init", "-q")
        git(self.repo, "config", "user.email", "test@example.com")
        git(self.repo, "config", "user.name", "test")
        for script in ("ci/metrics/local_grid.sh", "ci/native/store_key.sh"):
            target = self.repo / script
            target.parent.mkdir(parents=True, exist_ok=True)
            shutil.copy(ROOT / script, target)
        self.write("ci/metrics/local_pair.py", FAKE_LOCAL_PAIR)
        self.write("native/rust/rust-toolchain.toml", '[toolchain]\nchannel = "1.99.1"\n')
        self.write("native/rust/src/lib.rs", "base\n")
        executable(self.repo / "native/build.sh", FAKE_BUILD)
        executable(self.repo / "native/phases.sh", FAKE_PHASES)
        git(self.repo, "add", "-A")
        git(self.repo, "commit", "-q", "-m", "base")
        self.base = git(self.repo, "rev-parse", "HEAD")
        self.write("native/rust/src/lib.rs", "head\n")
        git(self.repo, "commit", "-q", "-am", "head")
        self.head = git(self.repo, "rev-parse", "HEAD")

        self.env = {
            "PATH": f"{fakes}{os.pathsep}{os.environ['PATH']}",
            "HOME": os.environ.get("HOME", str(self.tmp)),
            "FAKE_STORE": str(self.store),
            "FAKE_LOG": str(self.log),
            "PAIR_TARGET": TARGET,
            "PAIR_SIZES": "preset=4096",
        }

    def write(self, path, text):
        target = self.repo / path
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_text(text)

    def key(self, commit):
        return subprocess.run(
            ["bash", "ci/native/store_key.sh", commit, "release", TARGET],
            check=True, capture_output=True, text=True, cwd=self.repo,
        ).stdout.strip()

    def stock(self, commit):
        """Puts the commit's `release` zip in the store: native/bin/ by name alone."""
        with zipfile.ZipFile(self.store / f"{self.key(commit)}.zip", "w") as z:
            z.write(self.repo / "native/phases.sh", "breach_tick_phases")

    def pair(self):
        result = subprocess.run(
            ["bash", "ci/metrics/local_grid.sh", "pair", self.head, self.base, "1"],
            capture_output=True, text=True, cwd=self.repo, env=self.env,
        )
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        return result.stdout + result.stderr

    def calls(self, word):
        return [line for line in self.log.read_text().splitlines() if line.startswith(word)]

    def test_both_keys_stored_builds_nothing(self):
        self.stock(self.head)
        self.stock(self.base)
        out = self.pair()
        self.assertEqual(self.calls("build.sh"), [])
        curls = self.calls("curl")
        self.assertEqual(len(curls), 2)
        for commit in (self.head, self.base):
            url = f"releases/download/native-store/{self.key(commit)}.zip"
            self.assertTrue(any(url in call for call in curls), curls)
        self.assertFalse(any("Authorization" in call for call in curls), curls)
        self.assertIn("from the store", out)
        self.assertIn("local_pair", out)
        self.assertEqual(git(self.repo, "worktree", "list").count("\n"), 0)

    def test_one_key_missing_builds_both_and_says_why(self):
        self.stock(self.head)
        out = self.pair()
        self.assertEqual(sorted(self.calls("build.sh")), ["build.sh base", "build.sh head"])
        self.assertIn(f"the base's key ({self.key(self.base)}) isn't in the store", out)
        self.assertIn("building both", out)
        self.assertIn("local_pair", out)

    def test_a_host_with_no_store_target_builds_both(self):
        self.stock(self.head)
        self.stock(self.base)
        self.env["PAIR_TARGET"] = ""
        self.env["PAIR_UNAME"] = "Darwin arm64"
        out = self.pair()
        self.assertEqual(sorted(self.calls("build.sh")), ["build.sh base", "build.sh head"])
        self.assertEqual(self.calls("curl"), [])
        self.assertIn("no store target", out)


if __name__ == "__main__":
    unittest.main()

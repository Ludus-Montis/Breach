"""store.sh: fetch a native binary from the `native-store` pre-release, or build and
upload it (spec 49 step 3, Decision 179).

Each test builds a throwaway repo holding copies of store.sh and store_key.sh, with a fake
native/build.sh, and puts fakes of gh, rustc, rustup and cargo first on PATH. The fake gh
serves the store from a directory and logs every call.
"""

import json
import os
import shutil
import stat
import subprocess
import tempfile
import unittest
import zipfile
from pathlib import Path

HERE = Path(__file__).resolve().parent
TARGET = "x86_64-unknown-linux-gnu"
REPO = "Ludus-Montis/Breach"

FAKE_GH = r"""#!/usr/bin/env bash
# A fake gh: the store is $FAKE_STORE (a directory per release), every call is logged.
echo "gh $*" >> "$FAKE_LOG"
[ "$1" = release ] || exit 3
cmd="$2" tag="$3"; shift 3
rel="$FAKE_STORE/$tag"
case "$cmd" in
	view) [ -d "$rel" ] || exit 1; ls "$rel" ;;
	create) mkdir -p "$rel" ;;
	download)
		[ -z "${FAKE_DOWNLOAD_FAILS:-}" ] || exit 1
		pat= dir=.
		while [ "$#" -gt 0 ]; do
			case "$1" in -p) pat="$2"; shift ;; -D) dir="$2"; shift ;; esac
			shift
		done
		[ -f "$rel/$pat" ] || exit 1
		mkdir -p "$dir" && cp "$rel/$pat" "$dir/" ;;
	upload)
		[ -d "$rel" ] || exit 1
		[ ! -e "$rel/$(basename "$1")" ] || exit 1
		cp "$1" "$rel/" ;;
	*) exit 3 ;;
esac
"""

FAKE_BUILD = """#!/usr/bin/env bash
# A fake native/build.sh: logs the toolchain it ran under and fills native/bin/.
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
echo "build.sh $1 $RUSTUP_TOOLCHAIN" >> "$FAKE_LOG"
mkdir -p "$here/bin"
cat "$here/rust/src/lib.rs" > "$here/bin/libbreach_rust.linux.x86_64.so"
echo digest > "$here/bin/breach_digest"
echo phases > "$here/bin/breach_tick_phases"
"""

FAKE_CARGO = """#!/usr/bin/env bash
echo "cargo $* $RUSTUP_TOOLCHAIN" >> "$FAKE_LOG"
mkdir -p target/profiling
echo profiling-phases > target/profiling/tick_phases
"""

FAKE_RUSTC = f"""#!/usr/bin/env bash
printf 'rustc 1.99.1\\nhost: {TARGET}\\n'
"""

FAKE_RUSTUP = """#!/usr/bin/env bash
echo "rustup $*" >> "$FAKE_LOG"
"""


def git(repo, *args):
    return subprocess.run(
        ["git", "-C", str(repo), *args], check=True, capture_output=True, text=True
    ).stdout.strip()


def executable(path, text):
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(text)
    path.chmod(path.stat().st_mode | stat.S_IEXEC)


class StoreTest(unittest.TestCase):
    def setUp(self):
        self.tmp = Path(tempfile.mkdtemp())
        self.addCleanup(shutil.rmtree, self.tmp)
        self.repo = self.tmp / "repo"
        self.store = self.tmp / "store"
        self.log = self.tmp / "log"
        self.log.write_text("")
        self.out = self.tmp / "out"
        fakes = self.tmp / "fakes"
        executable(fakes / "gh", FAKE_GH)
        executable(fakes / "cargo", FAKE_CARGO)
        executable(fakes / "rustc", FAKE_RUSTC)
        executable(fakes / "rustup", FAKE_RUSTUP)

        self.repo.mkdir()
        git(self.repo, "init", "-q")
        git(self.repo, "config", "user.email", "test@example.com")
        git(self.repo, "config", "user.name", "test")
        for script in ("store.sh", "store_key.sh"):
            target = self.repo / "ci/native" / script
            target.parent.mkdir(parents=True, exist_ok=True)
            shutil.copy(HERE / script, target)
        self.write("native/rust/rust-toolchain.toml", '[toolchain]\nchannel = "1.99.1"\n')
        self.write("native/rust/src/lib.rs", "the core\n")
        executable(self.repo / "native/build.sh", FAKE_BUILD)
        git(self.repo, "add", "-A")
        git(self.repo, "commit", "-q", "-m", "base")
        self.commit = git(self.repo, "rev-parse", "HEAD")

        self.env = {
            "PATH": f"{fakes}{os.pathsep}{os.environ['PATH']}",
            "HOME": os.environ.get("HOME", str(self.tmp)),
            "FAKE_STORE": str(self.store),
            "FAKE_LOG": str(self.log),
            "GITHUB_REPOSITORY": REPO,
        }

    def write(self, path, text):
        target = self.repo / path
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_text(text)

    def key(self, profile="release"):
        return subprocess.run(
            ["bash", "ci/native/store_key.sh", self.commit, profile, TARGET],
            check=True, capture_output=True, text=True, cwd=self.repo,
        ).stdout.strip()

    def stock(self, profile, files):
        """Puts a zip of `files` (name -> text) in the store under the commit's key."""
        release = self.store / "native-store"
        release.mkdir(parents=True, exist_ok=True)
        with zipfile.ZipFile(release / f"{self.key(profile)}.zip", "w") as archive:
            for name, text in files.items():
                archive.writestr(name, text)

    def on_push_to_main(self):
        self.env.update(GITHUB_EVENT_NAME="push", GITHUB_REF="refs/heads/main")

    def on_pull_request(self, head_repo):
        event = self.tmp / "event.json"
        event.write_text(
            json.dumps({"pull_request": {"head": {"repo": {"full_name": head_repo}}}})
        )
        self.env.update(GITHUB_EVENT_NAME="pull_request", GITHUB_EVENT_PATH=str(event))

    def fetch(self, profile="release"):
        return subprocess.run(
            ["bash", "ci/native/store.sh", "fetch", self.commit, profile, TARGET,
             str(self.out)],
            capture_output=True, text=True, cwd=self.repo, env=self.env,
        )

    def calls(self, prefix):
        return [line for line in self.log.read_text().splitlines()
                if line.startswith(prefix)]

    def stored(self, profile="release"):
        return self.store / "native-store" / f"{self.key(profile)}.zip"

    def test_a_stored_key_is_downloaded_and_nothing_is_built(self):
        self.on_push_to_main()
        self.stock("release", {"libbreach_rust.linux.x86_64.so": "stored core\n"})
        result = self.fetch()
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(
            (self.out / "libbreach_rust.linux.x86_64.so").read_text(), "stored core\n"
        )
        self.assertEqual(self.calls("build.sh"), [])
        self.assertEqual(self.calls("cargo"), [])
        self.assertEqual(self.calls("gh release upload"), [])

    def test_a_missing_key_is_built_with_the_pin_and_uploaded_on_main(self):
        self.on_push_to_main()
        result = self.fetch()
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(self.calls("build.sh"), ["build.sh release 1.99.1"])
        self.assertEqual((self.out / "breach_digest").read_text(), "digest\n")
        self.assertEqual(
            (self.out / "libbreach_rust.linux.x86_64.so").read_text(), "the core\n"
        )
        with zipfile.ZipFile(self.stored()) as archive:
            self.assertEqual(
                sorted(archive.namelist()),
                ["breach_digest", "breach_tick_phases", "libbreach_rust.linux.x86_64.so"],
            )

    def test_the_store_is_made_when_missing(self):
        self.on_push_to_main()
        result = self.fetch()
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(len(self.calls("gh release create native-store")), 1)
        self.assertTrue(self.stored().exists())

    def test_a_same_repo_pull_request_uploads(self):
        self.on_pull_request(REPO)
        result = self.fetch()
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertTrue(self.stored().exists())

    def test_a_fork_pull_request_builds_and_does_not_upload(self):
        self.on_pull_request("someone/Breach")
        result = self.fetch()
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(self.calls("build.sh"), ["build.sh release 1.99.1"])
        self.assertTrue((self.out / "breach_digest").exists())
        self.assertEqual(self.calls("gh release upload"), [])
        self.assertEqual(self.calls("gh release create"), [])

    def test_outside_github_nothing_is_uploaded(self):
        result = self.fetch()
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertTrue((self.out / "breach_digest").exists())
        self.assertEqual(self.calls("gh release upload"), [])

    def test_an_upload_that_finds_the_asset_skips(self):
        self.on_push_to_main()
        self.stock("release", {"libbreach_rust.linux.x86_64.so": "stored core\n"})
        self.env["FAKE_DOWNLOAD_FAILS"] = "1"
        result = self.fetch()
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(self.calls("gh release upload"), [])
        with zipfile.ZipFile(self.stored()) as archive:
            self.assertEqual(archive.namelist(), ["libbreach_rust.linux.x86_64.so"])

    def test_profiling_stores_tick_phases_alone(self):
        self.on_push_to_main()
        result = self.fetch("profiling")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(
            self.calls("cargo"),
            ["cargo build --profile profiling --bin tick_phases 1.99.1"],
        )
        self.assertEqual(self.calls("build.sh"), [])
        self.assertEqual(
            sorted(p.name for p in self.out.iterdir()), ["breach_tick_phases"]
        )
        with zipfile.ZipFile(self.stored("profiling")) as archive:
            self.assertEqual(archive.namelist(), ["breach_tick_phases"])

    def test_the_key_builds_from_its_own_commit_not_the_working_tree(self):
        self.on_push_to_main()
        self.write("native/rust/src/lib.rs", "uncommitted\n")
        result = self.fetch()
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(
            (self.out / "libbreach_rust.linux.x86_64.so").read_text(), "the core\n"
        )

    def test_a_target_other_than_the_host_is_refused(self):
        result = subprocess.run(
            ["bash", "ci/native/store.sh", "fetch", self.commit, "release",
             "x86_64-pc-windows-msvc", str(self.out)],
            capture_output=True, text=True, cwd=self.repo, env=self.env,
        )
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("host", result.stderr)
        self.assertEqual(self.calls("build.sh"), [])


if __name__ == "__main__":
    unittest.main()

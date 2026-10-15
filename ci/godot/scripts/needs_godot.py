#!/usr/bin/env python3
"""Whether a change needs Godot: the gdUnit4 suite, the scaling check and the sim checks
(digests, trials, the suite under Rust). Docs, specs, skills, the kit and Python-only
tools don't; game code, tests, content, scenes, the Rust core and the CI wiring itself
do. Decided by the files changed, not by the commit type: a `chore` can touch any of
them.

Usage: needs_godot.py [base-ref]  - prints `godot=true|false`, and appends it to
$GITHUB_OUTPUT when set. With no base ref (a push to main) it prints true.
"""
import os
import subprocess
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

GODOT_SUFFIXES = (".gd", ".tres", ".tscn", ".gdextension", ".gdshader", ".import", ".uid")
GODOT_DIRS = ("native/", "content/", "addons/", "presentation/", "sim/", "tests/", ".github/")
GODOT_FILES = {"project.godot", ".pre-commit-config.yaml", "ci/godot/scripts/run_tests.sh"}


def needs_godot(paths: list) -> bool:
    for path in paths:
        if path in GODOT_FILES or path.endswith(GODOT_SUFFIXES):
            return True
        if path.startswith(GODOT_DIRS) and not path.endswith(".md"):
            return True
    return False


def changed(base_ref: str) -> list:
    merge_base = subprocess.run(
        ["git", "merge-base", base_ref, "HEAD"], capture_output=True, text=True, check=True
    ).stdout.strip()
    return subprocess.run(
        ["git", "diff", "--name-only", merge_base, "HEAD"],
        capture_output=True, text=True, check=True,
    ).stdout.split()


def main(arguments: list) -> int:
    base_ref = arguments[0] if arguments else ""
    answer = True if not base_ref else needs_godot(changed(base_ref))
    line = f"godot={'true' if answer else 'false'}"
    print(line)
    if os.environ.get("GITHUB_OUTPUT"):
        with open(os.environ["GITHUB_OUTPUT"], "a", encoding="utf-8") as out:
            out.write(line + "\n")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))

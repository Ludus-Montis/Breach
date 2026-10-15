#!/usr/bin/env python3
"""ocp-shotgun-surgery, per AI_First_Development_Kit/principles/solid-mechanical.md (O)
and config/thresholds.yaml (solid_mechanical.ocp.max_touched_files_per_new_case).

Ported from https://github.com/SSidey/Sweepminer's dev_kit/ci/godot/scripts/
check_ocp_shotgun_surgery.py (same kit, same gap - originally marked "no known static
tool" in this repo's own ci/godot/README.md; Sweepminer's diff-scoped approach is a
real, simple mechanical implementation of the rubric's literal definition).

Counts pre-existing .gd files modified (not added) in this branch vs. a base ref.

Honest limit: this counts *modified* pre-existing files - it cannot distinguish "one
new case forced N files open" from any other reason N files changed together (e.g. a
deliberate, justified refactor). A genuine exception should be recorded as a Decision
(AI_First_Development_Kit/principles/decision-ledger.md), not silently ignored. A
change such a Decision covers says so with a `Waived-by: Decision N` trailer on any of
the branch's commits: the check then lists the files as usual and passes.

Only meaningful with a real base ref to diff against (a PR's merge-base) - not run as
part of the working-tree-only pre-commit gate, only in CI where that ref exists.
"""
import re
import subprocess
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import base_ref as base_ref_module  # noqa: E402

REPO_ROOT = Path(
    subprocess.check_output(["git", "rev-parse", "--show-toplevel"], text=True).strip()
)
THRESHOLDS_FILE = REPO_ROOT / "AI_First_Development_Kit" / "config" / "thresholds.yaml"
EXCLUDED_DIRS = {"addons", "AI_First_Development_Kit"}
WAIVER = re.compile(r"^Waived-by:\s*(Decision \d+)\s*$", re.MULTILINE)


def read_threshold(key: str, default: int) -> int:
    text = THRESHOLDS_FILE.read_text(encoding="utf-8")
    match = re.search(rf"^\s*{re.escape(key)}:\s*(\d+)\s*$", text, re.MULTILINE)
    return int(match.group(1)) if match else default


def modified_preexisting_files(base_ref: str):
    merge_base = subprocess.run(
        ["git", "merge-base", base_ref, "HEAD"],
        cwd=REPO_ROOT,
        capture_output=True,
        text=True,
        check=True,
    ).stdout.strip()

    diff = subprocess.run(
        ["git", "diff", "--name-only", "--diff-filter=M", merge_base, "HEAD", "--", "*.gd"],
        cwd=REPO_ROOT,
        capture_output=True,
        text=True,
        check=True,
    ).stdout.splitlines()

    return [
        path
        for path in diff
        if Path(path).parts and Path(path).parts[0] not in EXCLUDED_DIRS
    ]


def waiver(messages: list):
    """The Decision a `Waived-by: Decision N` trailer in any of `messages` names, or None."""
    for message in messages:
        found = WAIVER.search(message)
        if found:
            return found.group(1)
    return None


def branch_messages(base_ref: str) -> list:
    merge_base = subprocess.run(
        ["git", "merge-base", base_ref, "HEAD"],
        cwd=REPO_ROOT, capture_output=True, text=True, check=True,
    ).stdout.strip()
    log = subprocess.run(
        ["git", "log", "--format=%B%x00", f"{merge_base}..HEAD"],
        cwd=REPO_ROOT, capture_output=True, text=True, check=True,
    ).stdout
    return [message for message in log.split("\0") if message.strip()]


def main() -> int:
    base_ref = base_ref_module.resolve(sys.argv)  # Decision 50: a stacked PR's own base
    max_touched = read_threshold("max_touched_files_per_new_case", 3)

    try:
        touched = modified_preexisting_files(base_ref)
    except subprocess.CalledProcessError as error:
        print(f"check_ocp_shotgun_surgery: could not diff against '{base_ref}': {error}")
        return 0

    if len(touched) > max_touched:
        waived_by = waiver(branch_messages(base_ref))
        if waived_by is not None:
            print(
                f"check_ocp_shotgun_surgery: {len(touched)} pre-existing files modified "
                f"(threshold: {max_touched}), waived by {waived_by}."
            )
            return 0
        print(
            f"check_ocp_shotgun_surgery: {len(touched)} pre-existing files modified "
            f"(threshold: {max_touched}). If this is one new case/behaviour, it should "
            f"plug into an existing extension point instead. If it's a justified broad "
            f"change, record why in a Decision:"
        )
        for path in touched:
            print(f"  {path}")
        return 1

    print(f"check_ocp_shotgun_surgery: {len(touched)} pre-existing files modified, below threshold.")
    return 0


if __name__ == "__main__":
    sys.exit(main())

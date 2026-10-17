#!/usr/bin/env python3
"""check_outcomes: a PR's battle digests (tools/formation_digest.gd) against its base's.

The digest job compares a branch's two engines with each other, so an outcome change both
engines share would pass it. This compares the branch's GDScript digest of a set with the
base's, tick by tick. A branch that means to change outcomes - a rule a Decision changes, a
fix, a digest that hashes something new - says why in an `Outcomes-changed: <why>` trailer
on any of its commits: the check then names the runs that changed and passes. Without one
it fails. A set the base can't digest (it didn't exist there) passes with a note.

    check_outcomes.py <set> <base digest file> <head digest file> [base ref]

The base ref (base_ref.py) gives the branch's commits whose trailers are read.
"""
import re
import subprocess
import sys
from pathlib import Path

import base_ref as base_ref_module

REPO_ROOT = Path(__file__).resolve().parents[3]
TRAILER = re.compile(r"^Outcomes-changed:[ \t]*(\S.*?)\s*$", re.MULTILINE)
SHOWN = 10  # runs named in the report


def _runs(lines: list) -> dict:
    """Each run's ticks in order: {run: [(tick, hash), ...]}, a line being `<run> <tick> <hash>`."""
    out = {}
    for line in lines:
        parts = line.split()
        if len(parts) < 3:
            continue
        out.setdefault(" ".join(parts[:-2]), []).append((parts[-2], parts[-1]))
    return out


def changed_runs(base: list, head: list) -> list:
    """[(run, its first differing tick), ...] in the head's order, then runs only the base has."""
    before = _runs(base)
    after = _runs(head)
    out = []
    for run in list(after) + [run for run in before if run not in after]:
        old = before.get(run, [])
        new = after.get(run, [])
        for index in range(max(len(old), len(new))):
            if index >= len(old) or index >= len(new) or old[index] != new[index]:
                out.append((run, (new if index < len(new) else old)[index][0]))
                break
    return out


def reason(messages: list):
    """The reason an `Outcomes-changed:` trailer in any of `messages` gives, or None."""
    for message in messages:
        found = TRAILER.search(message)
        if found:
            return found.group(1)
    return None


def verdict(name: str, base: list, head: list, messages: list):
    """(passed, report lines) for digest set `name`."""
    if not base:
        return True, [f"check_outcomes {name}: no base digest (a new set?): not compared."]
    changed = changed_runs(base, head)
    if not changed:
        return True, [f"check_outcomes {name}: same as the base, tick for tick."]
    why = reason(messages)
    count = f"{len(changed)} run{'s' if len(changed) != 1 else ''}"
    listed = [f"  {run} from tick {tick}" for run, tick in changed[:SHOWN]]
    if len(changed) > SHOWN:
        listed.append(f"  ... and {len(changed) - SHOWN} more")
    if why is not None:
        return True, [f"check_outcomes {name}: {count} changed, as the branch says: {why}"] + listed
    return False, (
        [f"check_outcomes {name}: {count} changed against the base, and no commit says why:"]
        + listed
        + [
            "  A change meant to keep every outcome must leave the digest the same. If the",
            "  branch means to change outcomes, add a trailer to one of its commits:",
            "  `Outcomes-changed: <why>` (a Decision, a fix, a digest that hashes more).",
        ]
    )


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


def _lines(path: str) -> list:
    file = Path(path)
    return file.read_text(encoding="utf-8").splitlines() if file.is_file() else []


def main(argv: list) -> int:
    if len(argv) < 4:
        print(__doc__)
        return 2
    name, base_file, head_file = argv[1:4]
    base_ref = base_ref_module.resolve(["", argv[4]] if len(argv) > 4 else [""])
    try:
        messages = branch_messages(base_ref)
    except subprocess.CalledProcessError as error:
        print(f"check_outcomes {name}: could not read the branch's commits against '{base_ref}': {error}")
        messages = []
    passed, lines = verdict(name, _lines(base_file), _lines(head_file), messages)
    print("\n".join(lines))
    return 0 if passed else 1


if __name__ == "__main__":
    sys.exit(main(sys.argv))

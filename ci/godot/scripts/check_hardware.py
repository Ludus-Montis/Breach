#!/usr/bin/env python3
"""check_hardware: one digest set taken on x86-64 and on arm64, required equal tick for tick.

Decision 161 lets a step change outcomes, so matching the base's digests no longer guards
them; what it keeps is the same result for a version on any hardware (spec 42 step 1). The
`digests-arm64` job in sim-checks takes the set on GitHub's arm64 runner, and this compares it
with the `digests` job's x86-64 digest of the same set (lines `<run> <tick> <hash>`, as
native/bin/breach_digest writes them). A difference fails, naming the set and the first run
and tick that differ; a digest missing on either side fails too.

    check_hardware.py <set> <x86-64 digest file> <arm64 digest file>
"""
import sys
from pathlib import Path

from check_outcomes import changed_runs


def verdict(name: str, x86: list, arm: list):
    """(passed, the report line) for digest set `name`."""
    if not x86 or not arm:
        missing = " and ".join(
            machine for machine, lines in (("x86-64", x86), ("arm64", arm)) if not lines
        )
        return False, f"hardware {name}: MISSING the {missing} digest."
    changed = changed_runs(x86, arm)
    if not changed:
        return True, f"hardware {name}: equal on x86-64 and arm64, tick for tick."
    run, tick = changed[0]
    count = f"{len(changed)} run{'s' if len(changed) != 1 else ''} differ"
    return False, f"hardware {name}: DIFFER on arm64, first at {run} tick {tick} ({count})."


def _lines(path: str) -> list:
    file = Path(path)
    return file.read_text(encoding="utf-8").splitlines() if file.is_file() else []


def main(argv: list) -> int:
    if len(argv) != 4:
        print(__doc__)
        return 2
    name, x86_file, arm_file = argv[1:4]
    passed, line = verdict(name, _lines(x86_file), _lines(arm_file))
    print(line)
    return 0 if passed else 1


if __name__ == "__main__":
    sys.exit(main(sys.argv))

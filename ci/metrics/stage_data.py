#!/usr/bin/env python3
"""Stage a measuring workflow's new files for its data PR (spec 43 step 3, Decision 164).

The folder holds the new files at their paths in the repo (`metrics/bench/<date>-<run id>.json`,
`metrics/raw/bench/<run id>.log`). Every path must be data (Decision 163: under `metrics/`,
not under `metrics/schema/`) and a plain file, not a link. If any isn't, the whole folder is
refused and nothing is copied, so no PR is opened. Otherwise each file is copied into the
checkout at the same path.

Usage: stage_data.py <folder> [checkout]  - prints each copied path, one a line.
"""
import shutil
import sys
from pathlib import Path

from data_only import is_data_only


class Refused(Exception):
    pass


def new_files(folder: Path) -> list:
    return sorted(p for p in folder.rglob("*") if p.is_symlink() or not p.is_dir())


def refusals(folder: Path, files: list) -> list:
    problems = []
    for path in files:
        relative = path.relative_to(folder).as_posix()
        if path.is_symlink():
            problems.append(f"{relative}: a link, not a file")
        elif not is_data_only([relative]):
            problems.append(f"{relative}: not under metrics/, or under metrics/schema/")
    return problems


def stage(folder, checkout) -> list:
    folder, checkout = Path(folder), Path(checkout)
    files = new_files(folder)
    problems = refusals(folder, files)
    if problems:
        raise Refused("refused, nothing copied:\n" + "\n".join(problems))
    copied = []
    for path in files:
        relative = path.relative_to(folder).as_posix()
        target = checkout / relative
        target.parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(path, target)
        copied.append(relative)
    return copied


def main(argv: list) -> int:
    checkout = argv[2] if len(argv) > 2 else "."
    try:
        copied = stage(argv[1], checkout)
    except Refused as refused:
        print(refused, file=sys.stderr)
        return 1
    print("\n".join(copied))
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))

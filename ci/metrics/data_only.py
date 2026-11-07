#!/usr/bin/env python3
"""Whether a PR is data-only (spec 43 step 2, Decision 163): every path it changes is under
`metrics/`, and none under `metrics/schema/` (a schema change is code, and goes through the
normal checks). A data-only PR's `sim-checks`, `docs-gate` and `rust` jobs pass at once;
`metrics.yml` checks its records.

Usage: data_only.py [base-ref]  - prints `data_only=true|false`, and appends it to
$GITHUB_OUTPUT when set. With no base ref (a push or a manual run) it prints false.
"""
import os
import sys
from pathlib import Path

DATA = "metrics/"
NOT_DATA = "metrics/schema/"


def is_data_only(paths: list) -> bool:
    return bool(paths) and all(
        path.startswith(DATA) and not path.startswith(NOT_DATA) for path in paths
    )


def output_lines(answer: bool) -> list:
    return [f"data_only={'true' if answer else 'false'}"]


def main(arguments: list) -> int:
    base_ref = arguments[0] if arguments else ""
    answer = False
    if base_ref:
        sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "godot" / "scripts"))
        from needs_godot import changed

        answer = is_data_only(changed(base_ref))
    lines = output_lines(answer)
    print("\n".join(lines))
    if os.environ.get("GITHUB_OUTPUT"):
        with open(os.environ["GITHUB_OUTPUT"], "a", encoding="utf-8") as out:
            out.write("\n".join(lines) + "\n")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))

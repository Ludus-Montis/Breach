#!/usr/bin/env python3
"""Which gdUnit4 suites the pre-commit hook runs: the staged ones, and nothing else.

CI is the test runner (CLAUDE.md): a commit runs only the suite being written red, then
green. Every other suite, and the full run, is left to CI's sim-checks. A suite is a
`test_*.gd` under `tests/`; other `.gd` files there are helpers.

Usage: staged_tests.py <path>...  - prints one `res://` path a line.
"""
import sys


def staged_tests(paths: list) -> list:
    suites = []
    for path in paths:
        path = path.replace("\\", "/")
        name = path.rsplit("/", 1)[-1]
        if path.startswith("tests/") and name.startswith("test_") and name.endswith(".gd"):
            suite = "res://" + path
            if suite not in suites:
                suites.append(suite)
    return suites


if __name__ == "__main__":
    print("\n".join(staged_tests(sys.argv[1:])))

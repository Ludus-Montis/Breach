#!/usr/bin/env python3
"""Splits the gdUnit4 suite into N shards that CI runs side by side (sim-checks.yml,
ci.yml). Each test file goes whole into one shard: a file is self-contained, so where it
runs can't change its result. Shards are balanced by the per-file times recorded in
ci/godot/test_times.json (a file with no recorded time counts as the median), greedily,
slowest file first, onto the least loaded shard; ties go by path, so the same inputs
always give the same shards. A stale times file only makes the shards less even.

Usage:
  shard_tests.py <count> <index>        the `-a res://...` arguments for shard <index>
  shard_tests.py --update <results.xml>...  rewrite test_times.json from gdUnit4 reports
                                            (one per shard)
"""
import json
import statistics
import subprocess
import sys
import xml.etree.ElementTree as ET
from pathlib import Path

REPO_ROOT = Path(
    subprocess.check_output(["git", "rev-parse", "--show-toplevel"], text=True).strip()
)
TIMES = REPO_ROOT / "ci" / "godot" / "test_times.json"


def test_files() -> list:
    """Every gdUnit4 test file under tests/, as repo-relative paths."""
    return sorted(str(p.relative_to(REPO_ROOT)) for p in (REPO_ROOT / "tests").rglob("test_*.gd"))


def shards(files: list, times: dict, count: int) -> list:
    """`files` split into `count` shards balanced by `times` (path -> seconds)."""
    default = statistics.median(times.values()) if times else 1.0
    weighted = sorted(((times.get(f, default), f) for f in files), key=lambda w: (-w[0], w[1]))
    split = [[] for _ in range(count)]
    loads = [0.0] * count
    for seconds, path in weighted:
        least = min(range(count), key=lambda i: (loads[i], i))
        split[least].append(path)
        loads[least] += seconds
    return [sorted(shard) for shard in split]


def times_from_report(xml_text: str) -> dict:
    """Seconds per test file (path -> the sum of its cases' times) in a gdUnit4 report."""
    out = {}
    for suite in ET.fromstring(xml_text).iter("testsuite"):
        path = f"{suite.get('package')}/{suite.get('name')}.gd"
        out[path] = round(sum(float(c.get("time") or 0) for c in suite.iter("testcase")), 3)
    return out


def main(arguments: list) -> int:
    if len(arguments) >= 2 and arguments[0] == "--update":
        times = {}
        for report in arguments[1:]:
            times.update(times_from_report(Path(report).read_text(encoding="utf-8")))
        TIMES.write_text(json.dumps(times, indent=1, sort_keys=True) + "\n", encoding="utf-8")
        print(f"shard_tests: {len(times)} files' times written to {TIMES.relative_to(REPO_ROOT)}")
        return 0
    if len(arguments) != 2:
        print(__doc__)
        return 2
    count, index = int(arguments[0]), int(arguments[1])
    times = json.loads(TIMES.read_text(encoding="utf-8")) if TIMES.exists() else {}
    print(" ".join(f"-a res://{path}" for path in shards(test_files(), times, count)[index]))
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))

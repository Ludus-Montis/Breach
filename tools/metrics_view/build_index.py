#!/usr/bin/env python3
"""Lists every record under metrics/ (not metrics/raw/, not metrics/schema/) into one bundle,
`[{"path": "metrics/bench/<file>", "record": {...}}, ...]`, which the metrics page loads
(spec 43 step 7). It is built, never committed: the `metrics-pages` workflow builds it when it
publishes the page, and you build it to open the page from disk. Nothing in the data is
shared between records, so data PRs never conflict over it.

    build_index.py [metrics] [tools/metrics_view/records.json]
"""
import json
import sys
from pathlib import Path

SKIP = {"raw", "schema"}


def bundle(metrics) -> list:
    metrics = Path(metrics)
    found = []
    for path in sorted(metrics.rglob("*.json")):
        relative = path.relative_to(metrics)
        if relative.parts[0] in SKIP:
            continue
        found.append({"path": f"metrics/{relative.as_posix()}",
                      "record": json.loads(path.read_text(encoding="utf-8"))})
    return found


def main(argv: list) -> int:
    metrics = argv[0] if argv else "metrics"
    out = Path(argv[1] if len(argv) > 1 else Path(__file__).resolve().parent / "records.json")
    items = bundle(metrics)
    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_text(json.dumps(items, separators=(",", ":")), encoding="utf-8")
    print(f"build_index: {len(items)} record(s) into {out}")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))

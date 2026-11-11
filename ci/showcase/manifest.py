#!/usr/bin/env python3
"""The showcase's records (spec 44 step 3, Decision 167). Each PR has its own record,
`pr-<N>.json` on the pre-release `showcase-index`, written only by that PR's runs (one
concurrency group a PR), so two PRs never write the same file. `manifest.json`, which the
site reads, is built from every record: every capture, newest first, each with its PR's
number, title, state and link.

Retention: an open PR keeps its last two captures; once it closes, merged or not, only its
last. A capture of a commit already captured replaces it (its assets keep their names).

Usage (each prints the assets to delete, "<release> <name>" a line):
  manifest.py add <record.json> <capture.json> <pr> <title> <url>
  manifest.py close <record.json> <state> <title> <closed-at> <pr> <url>
  manifest.py merge <manifest.json> <record.json>...
"""
import json
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from entry import assets  # noqa: E402

KEPT_OPEN = 2
KEPT_CLOSED = 1
CARRIED = ("title", "state", "url", "closed_at")


def new_record(pr: int, title: str, url: str) -> dict:
    return {"pr": pr, "title": title, "state": "open", "url": url, "captures": []}


def _removed(captures: list) -> list:
    return [
        (capture["release"], name)
        for capture in captures
        for made in capture["scenarios"]
        for name in assets(made)
    ]


def _kept(record: dict, keep: int) -> tuple:
    captures = record["captures"]
    cut = max(len(captures) - keep, 0)
    return dict(record, captures=captures[cut:]), _removed(captures[:cut])


def add_capture(record: dict, capture: dict) -> tuple:
    captures = [c for c in record["captures"] if c["commit"] != capture["commit"]]
    return _kept(dict(record, captures=[*captures, capture]), KEPT_OPEN)


def close(record: dict, state: str, title: str, closed_at: str) -> tuple:
    return _kept(dict(record, state=state, title=title, closed_at=closed_at), KEPT_CLOSED)


def merge(records: list) -> dict:
    captures = [
        {"pr": record["pr"], **{k: record[k] for k in CARRIED if k in record}, **capture}
        for record in records
        for capture in record["captures"]
    ]
    captures.sort(key=lambda c: c["date"], reverse=True)
    return {"version": 1, "captures": captures}


def _load(path: str, pr: str, title: str, url: str) -> dict:
    if os.path.exists(path):
        return json.load(open(path, encoding="utf-8"))
    return new_record(int(pr), title, url)


def _write(path: str, data: dict) -> None:
    with open(path, "w", encoding="utf-8") as out:
        json.dump(data, out, indent=1)


def main(arguments: list) -> int:
    command, rest = (arguments[0], arguments[1:]) if arguments else ("", [])
    if command == "add" and len(rest) == 5:
        path, capture_path, pr, title, url = rest
        record = dict(_load(path, pr, title, url), title=title)
        record, removed = add_capture(record, json.load(open(capture_path, encoding="utf-8")))
    elif command == "close" and len(rest) == 6:
        path, state, title, closed_at, pr, url = rest
        record, removed = close(_load(path, pr, title, url), state, title, closed_at)
    elif command == "merge" and rest:
        _write(rest[0], merge([json.load(open(p, encoding="utf-8")) for p in rest[1:]]))
        return 0
    else:
        print(__doc__.strip(), file=sys.stderr)
        return 2
    _write(path, record)
    for release, name in removed:
        print(release, name)
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))

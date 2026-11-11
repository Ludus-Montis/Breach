#!/usr/bin/env python3
"""Import the owner's PC's runs as local records (spec 48 step 1, Decision 178).

A folder of `breach_tick_phases` outputs, each named `s<side>_q<squads>_r<round>.txt`,
becomes one record per battle (cell), its rounds gathered from that cell's files, written to
`metrics/local/<date>-<n>.json`, `n` counting on from the records already there that day.
The records are a series of their own (`series` `"local"`, `provenance.kind` `"local"`):
CI's charts read `series` and don't draw them. They compare in raw ms, so they have no
reference (`not-measured`). The machine is `ci/metrics/local_machine.json`, the owner's PC.

Each file is checked against its name and the machine before anything is written: the side
it prints (`<side> a side`) and its thread count (`machine: ..., <n> threads, ...`). A file
that differs is refused, by name, and nothing is written. The binary doesn't print its squad
count, so the squads come from the name alone.

    local_record.py <folder> --commit <sha> --date <YYYY-MM-DD> [--out metrics]
        [--machine ci/metrics/local_machine.json] [--fight 10]
"""
import json
import re
import subprocess
import sys
from pathlib import Path

import bench_record

FILE = re.compile(r"^s(\d+)_q(\d+)_r(\d+)\.txt$")
SIDE = re.compile(r"^(\d+) a side, ", re.M)
THREADS = re.compile(r"^machine: .*?, (\d+) threads,", re.M)
HERE = Path(__file__).resolve().parent
NOT_APPLICABLE = {"na": "not-applicable"}
NOT_RECORDED = {"na": "not-recorded"}


class Refused(ValueError):
    """A file whose output differs from its name or the machine."""


def load_machine(path) -> dict:
    return json.loads(Path(path).read_text(encoding="utf-8"))


def _check(path: Path, text: str, side: int, machine: dict):
    printed = SIDE.search(text)
    if not printed or int(printed.group(1)) != side:
        found = printed.group(1) if printed else "no side"
        raise Refused(f"{path.name}: prints side {found}, its name says {side}")
    threads = THREADS.search(text)
    if not threads or int(threads.group(1)) != machine["threads"]:
        found = threads.group(1) if threads else "no"
        raise Refused(f"{path.name}: prints {found} threads, the machine has "
                      f"{machine['threads']}")


def cells(folder, machine: dict) -> dict:
    """{(side, squads): [each round's output, in round order]}, cells in (side, squads) order.
    Raises Refused, naming the file, on the first one that differs from its name."""
    found = {}
    for path in sorted(Path(folder).iterdir()):
        named = FILE.match(path.name)
        if not named:
            continue
        side, squads, round_ = (int(group) for group in named.groups())
        text = path.read_text(encoding="utf-8")
        _check(path, text, side, machine)
        found.setdefault((side, squads), []).append((round_, text))
    return {cell: [text for _, text in sorted(rounds)] for cell, rounds in sorted(found.items())}


def build_record(side: int, squads: int, texts: list, machine: dict, context: dict,
                 fight: int = 10) -> dict:
    return {
        "schema": "record-v1",
        "series": "local",
        "commit": context["commit"],
        "source": {**context["source"], "args": [f"side={side}", f"squads={squads}",
                                                 f"fight={fight}"], "rounds": len(texts)},
        "machine": machine["machine"],
        "results": bench_record.measures(texts),
        "reference": {"na": "not-measured"},
        "provenance": {"run_id": dict(NOT_APPLICABLE), "url": dict(NOT_APPLICABLE),
                       "date": context["date"], "kind": "local"},
    }


def write_records(records: list, metrics, date: str) -> list:
    """`<metrics>/local/<date>-<n>.json`, `n` from 1 after the day's records already there."""
    folder = Path(metrics) / "local"
    folder.mkdir(parents=True, exist_ok=True)
    taken = [int(p.stem[len(date) + 1:]) for p in folder.glob(f"{date}-*.json")
             if p.stem[len(date) + 1:].isdigit()]
    after = max(taken, default=0)
    paths = []
    for at, record in enumerate(records, start=after + 1):
        path = folder / f"{date}-{at}.json"
        path.write_text(json.dumps(record, indent=2, sort_keys=True) + "\n", encoding="utf-8")
        paths.append(path)
    return paths


def _git(*args) -> str:
    try:
        return bench_record.git(".", *args)
    except (OSError, subprocess.CalledProcessError):
        return ""


def commit(sha: str) -> dict:
    """The commit as the bench records name it; its first-parent position only from a full
    clone (a shallow one counts wrong), and gaps for what git can't resolve."""
    full = _git("rev-parse", sha)
    if not full:
        return {"sha": sha, "position": dict(NOT_RECORDED), "pr": dict(NOT_RECORDED),
                "title": dict(NOT_RECORDED), "outcomes_changed": dict(NOT_RECORDED)}
    message = _git("log", "-1", "--format=%B", full)
    subject = message.splitlines()[0] if message else ""
    shallow = _git("rev-parse", "--is-shallow-repository") == "true"
    return {"sha": full,
            "position": dict(NOT_RECORDED) if shallow else bench_record.position(full),
            "pr": bench_record.pr_number(subject), "title": bench_record.title(subject),
            "outcomes_changed": bench_record.outcomes_changed(message)}


def main(argv: list) -> int:
    option = bench_record.option
    folder, sha, date = argv[0], option(argv, "--commit"), option(argv, "--date")
    if not sha or not date:
        print(__doc__)
        return 2
    machine = load_machine(option(argv, "--machine", HERE / "local_machine.json"))
    try:
        found = cells(folder, machine)
    except Refused as why:
        print(f"local_record: refused {why}")
        return 1
    blob = _git("rev-parse", f"{sha}:{bench_record.SOURCE_FILE}")
    context = {"commit": commit(sha), "date": date,
               "source": {"harness": bench_record.HARNESS, "blob": blob or dict(NOT_RECORDED),
                          "profile": "release"}}
    fight = int(option(argv, "--fight", 10))
    records = [build_record(side, squads, texts, machine, context, fight)
               for (side, squads), texts in found.items()]
    for path in write_records(records, option(argv, "--out", "metrics"), date):
        print(f"local_record: wrote {path}")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))

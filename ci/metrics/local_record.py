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
count, so the squads come from the name alone. A machine line that names the CPU (spec 48
step 3) is preferred over the machine file; `unknown` keeps the file's.

Spec 48 step 5a: a cell run with `series=1` also keeps its p95 and p99 over the whole series
(`tick-p95`, `tick-p99`) and the `inspect:` ticks each round named (`source.inspect`). Files
named `s<side>_q<squads>_r<round>_repeat.txt` hold the repeat mode's output (`at=<t>,...
repeat=<n>`): one record per (cell, tick), harness `tick_phases-repeat`, each round's median
of the `n` repeats a value, with that round's q1 and q3 as its `quartiles` pair (spec 48
step 5d). The output prints each phase's median and quartiles, not each lap.

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
REPEAT_FILE = re.compile(r"^s(\d+)_q(\d+)_r(\d+)_repeat\.txt$")
REPEAT = re.compile(r"^repeat: tick (\d+), (\d+) repeats$", re.M)
INSPECT = re.compile(r"^inspect: ([\d,]+)$", re.M)
QUARTILE_HEAD = re.compile(r"^  \(ms\)\s+median\s+q1\s+q3\b")
QUARTILE_ROW = re.compile(r"^  (\S.*?)\s+[\d.]+\s+([\d.]+)\s+([\d.]+)\s+\[")
PRINTED = re.compile(r"^machine: (.+?)(?:, (\d+) cores)?, \d+ threads(?:, (\d+) GB)?, (.+)$",
                     re.M)
REPEAT_HARNESS = "tick_phases-repeat"
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


def repeats(folder, machine: dict) -> dict:
    """{(side, squads, tick): (n, [each round's block for that tick, in round order])}, in
    (side, squads, tick) order, from the `_repeat.txt` files. Raises Refused, naming the file,
    on the first one that differs from its name or the machine."""
    found = {}
    for path in sorted(Path(folder).iterdir()):
        named = REPEAT_FILE.match(path.name)
        if not named:
            continue
        side, squads, round_ = (int(group) for group in named.groups())
        text = path.read_text(encoding="utf-8")
        _check(path, text, side, machine)
        heads = list(REPEAT.finditer(text))
        if not heads:
            raise Refused(f"{path.name}: no repeat: line, so not the repeat mode's output")
        machine_line = text.splitlines()[0] + "\n"
        for at, head in enumerate(heads):
            end = heads[at + 1].start() if at + 1 < len(heads) else len(text)
            tick, n = int(head.group(1)), int(head.group(2))
            block = machine_line + text[head.start():end]
            found.setdefault((side, squads, tick), (n, []))[1].append((round_, block))
    return {key: (n, [block for _, block in sorted(blocks)])
            for key, (n, blocks) in sorted(found.items())}


def inspect_ticks(text: str) -> list:
    """The ticks a settled series' `inspect:` line names; [] without one."""
    found = INSPECT.search(text)
    return [int(tick) for tick in found.group(1).split(",")] if found else []


def machine_of(texts: list, machine: dict) -> dict:
    """The printed machine when the first output names its CPU, else the machine file's.
    Cores and RAM are taken when printed, else kept from the file."""
    printed = PRINTED.search(texts[0]) if texts else None
    if not printed or printed.group(1) == "unknown":
        return machine["machine"]
    cpu, cores, ram_gb, system = printed.groups()
    out = {**machine["machine"], "cpu": cpu, "os": system}
    if cores:
        out["cores"] = int(cores)
    if ram_gb:
        out["ram_mb"] = int(ram_gb) * 1024
    return out


def _record(args: list, texts: list, results: dict, machine: dict, context: dict,
            **source) -> dict:
    return {
        "schema": "record-v1",
        "series": "local",
        "commit": context["commit"],
        "source": {**context["source"], **source, "args": args, "rounds": len(texts)},
        "machine": machine_of(texts, machine),
        "results": results,
        "reference": {"na": "not-measured"},
        "provenance": {"run_id": dict(NOT_APPLICABLE), "url": dict(NOT_APPLICABLE),
                       "date": context["date"], "kind": "local"},
    }


def build_record(side: int, squads: int, texts: list, machine: dict, context: dict,
                 fight: int = 10) -> dict:
    """A cell's record: `fight` ticks, or the settled series when every round printed one,
    with its p95 and p99 and each round's `inspect:` ticks."""
    cell = [f"side={side}", f"squads={squads}"]
    spread = bench_record.percentiles(texts)
    if not spread:
        return _record(cell + [f"fight={fight}"], texts, bench_record.measures(texts),
                       machine, context)
    return _record(cell + ["series=1", "series=settled"], texts,
                   {**bench_record.measures(texts), **spread}, machine, context,
                   inspect=[inspect_ticks(text) for text in texts])


def build_repeat_record(side: int, squads: int, tick: int, n: int, texts: list,
                        machine: dict, context: dict) -> dict:
    """One tick of a cell replayed `n` times a round: each round's median a value."""
    args = [f"side={side}", f"squads={squads}", f"at={tick}", f"repeat={n}"]
    return _record(args, texts, with_quartiles(bench_record.measures(texts), texts), machine,
                   context, harness=REPEAT_HARNESS)


def quartiles(text: str) -> dict:
    """{path: [q1, q3]} from a repeat block's table of middles (spec 48 step 5d); {} for output
    from before it, which printed the IQR as one width."""
    found, reading = {}, False
    for line in text.splitlines():
        if QUARTILE_HEAD.match(line):
            reading = True
        elif reading and (row := QUARTILE_ROW.match(line)):
            found[bench_record.phase_path(row.group(1))] = [float(row.group(2)),
                                                            float(row.group(3))]
    return found


def with_quartiles(results: dict, texts: list) -> dict:
    """Each measured result with each round's [q1, q3] beside its value, where every round
    printed them; a gap keeps none."""
    rounds = [quartiles(text) for text in texts]
    for path, measure in results.items():
        pairs = [found.get(path) for found in rounds]
        if isinstance(measure.get("values"), list) and None not in pairs:
            measure["quartiles"] = pairs
    return results


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
        repeated = repeats(folder, machine)
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
    records += [build_repeat_record(side, squads, tick, n, texts, machine, context)
                for (side, squads, tick), (n, texts) in repeated.items()]
    for path in write_records(records, option(argv, "--out", "metrics"), date):
        print(f"local_record: wrote {path}")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))

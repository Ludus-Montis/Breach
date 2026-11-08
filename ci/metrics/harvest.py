#!/usr/bin/env python3
"""The harvest importer (spec 43 step 6, Decision 162): reads the archived logs under
`metrics/raw/` and writes records marked `harvested`, the numbers as they were seen then.

- **Each bench run's `SUMMARY`** becomes `metrics/bench/<date>-<run id>.json`: its head's
  phases (the medians of a rounds comparison, whose rounds the summary doesn't keep, or a
  single round's values), its tool and args, and its head. The head's branch and the base it
  was compared with are kept in `commit`, so the drawing can place a branch's numbers on
  `main`'s line. The machine and the reference are `{"na": "not-recorded"}`: the old runs
  printed neither in a comparable form.
- **Each `sim-checks` run's `scaling_check` sections** become `metrics/scaling/<date>-<run
  id>.json`: each stage and size's ms a tick, `scaling/<stage>/<size>`.

A run that already has a fresh record (one of step 4's, archived later) is left alone. A run
whose log has nothing readable (expired, failed, or another tool) writes nothing. The
output depends only on the archive, and a file is written only when its content changes, so
a re-run over the same archive adds nothing. Its records go through an ordinary PR (Decision
164 lets only bench, metrics-archive and bench-series merge their own).

    harvest.py [metrics/raw] [metrics]
"""
import json
import re
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import bench_record  # noqa: E402

STAMP = re.compile(r"^\d{4}-\d\d-\d\dT[\d:.]+Z ")
HEAD = re.compile(r"^== (.+?) \(([0-9a-f]{7,40})\): (\S+)\s*(.*)$")
BASE = re.compile(r"^== base (.+?) \(([0-9a-f]{7,40})\)")
ROW = re.compile(r"^  (\S.*?)\s+(-?\d+\.\d+)\s+(-?\d+\.\d+)\s+[+-]?\d+\.\d%")
SCALE = re.compile(r"^\s+([a-z]+)\s+(\d+) a side\s+(\d+(?:\.\d+)?) ms")
NOT_RECORDED = {"na": "not-recorded"}
NOT_MEASURED = {"na": "not-measured"}
NOT_APPLICABLE = {"na": "not-applicable"}


def plain(text: str) -> list:
    return [STAMP.sub("", line) for line in text.splitlines()]


def summary(lines: list) -> list:
    """The last SUMMARY's lines, between its markers; [] when there is none."""
    begins = [i for i, line in enumerate(lines) if line == "SUMMARY BEGIN"]
    for start in reversed(begins):
        for end in range(start + 1, len(lines)):
            if lines[end] == "SUMMARY END":
                return lines[start + 1:end]
    return []


def measure(value: float) -> dict:
    return {"unit": "ms", "values": dict(NOT_RECORDED),
            "median": value if value > 0 else dict(NOT_MEASURED)}


def summary_results(lines: list) -> dict:
    """The head's results: a rounds comparison's head medians, else round 1's output."""
    if any(line.startswith("medians over") for line in lines):
        results = {}
        for line in lines[lines.index(next(l for l in lines if l.startswith("medians over"))):]:
            if line.startswith("=="):
                break
            row = ROW.match(line)
            if row and row.group(1) != "phase":
                results[bench_record.phase_path(row.group(1))] = measure(float(row.group(3)))
        return results
    head = []
    for line in lines:
        if line.startswith("== round 1, base") or (head and BASE.match(line)):
            break
        head.append(line)
    return bench_record.measures(["\n".join(head)])


def harness(tool: str) -> str:
    return tool if tool.startswith("breach_") else f"{tool}.gd"


def provenance(run: dict) -> dict:
    return {"run_id": run["id"], "url": run.get("html_url", NOT_RECORDED),
            "date": run["created_at"][:10], "kind": "harvested"}


def unknown_commit(sha: str, branch, base) -> dict:
    return {"sha": sha, "branch": branch, "base": base, "position": dict(NOT_RECORDED),
            "pr": dict(NOT_RECORDED), "title": dict(NOT_RECORDED),
            "summary": dict(NOT_RECORDED), "step": dict(NOT_RECORDED),
            "outcomes_changed": dict(NOT_RECORDED)}


def bench_record_from(run_json: dict, text: str):
    lines = summary(plain(text))
    head = next((HEAD.match(line) for line in lines
                 if HEAD.match(line) and not line.startswith(("== base", "== round"))), None)
    if not head:
        return None
    results = summary_results(lines)
    if not results:
        return None
    base = next((BASE.match(line).group(2) for line in lines if BASE.match(line)),
                dict(NOT_APPLICABLE))
    rounds = (run_json.get("inputs_from_log") or {}).get("ROUNDS", "")
    return {
        "schema": "record-v1",
        "series": "bench",
        "commit": unknown_commit(head.group(2), head.group(1), base),
        "source": {"harness": harness(head.group(3)), "args": head.group(4).split(),
                   "rounds": int(rounds) if str(rounds).isdigit() else dict(NOT_RECORDED)},
        "machine": dict(NOT_RECORDED),
        "results": results,
        "reference": dict(NOT_RECORDED),
        "provenance": provenance(run_json["run"]),
    }


def scaling_record_from(run_json: dict, text: str):
    results = {}
    for line in plain(text):
        if "\x1b" in line:
            continue
        found = SCALE.match(line)
        if found:
            path = f"scaling/{found.group(1)}/{found.group(2)}"
            value = float(found.group(3))
            results.setdefault(path, {"unit": "ms", "values": [value], "median": value}
                               if value > 0 else {"unit": "ms", "values": dict(NOT_MEASURED),
                                                  "median": dict(NOT_MEASURED)})
    if not results:
        return None
    run = run_json["run"]
    return {
        "schema": "record-v1",
        "series": "scaling",
        "commit": unknown_commit(run["head_sha"], run.get("head_branch", NOT_RECORDED),
                                 dict(NOT_RECORDED)),
        "source": {"harness": "scaling_check", "args": dict(NOT_RECORDED),
                   "rounds": dict(NOT_RECORDED)},
        "machine": dict(NOT_RECORDED),
        "results": results,
        "reference": dict(NOT_RECORDED),
        "provenance": provenance(run),
    }


def has_fresh_record(folder: Path, run_id) -> bool:
    """Whether the run already has a record of its own (a run of step 4's record job that
    metrics-archive later archived): its fresh record stands, and nothing is harvested."""
    for path in list(folder.glob(f"*-{run_id}.json")) + list(folder.glob(f"*-{run_id}-*.json")):
        try:
            kind = json.loads(path.read_text(encoding="utf-8")).get("provenance", {}).get("kind")
        except (OSError, ValueError):
            continue
        if kind != "harvested":
            return True
    return False


READERS = {"bench": ("bench", bench_record_from), "sim-checks": ("scaling", scaling_record_from)}


def harvest(raw, out) -> list:
    """Write every record the archive gives; returns the files written or changed."""
    raw, out = Path(raw), Path(out)
    written = []
    for folder, (series, read) in READERS.items():
        for json_path in sorted((raw / folder).glob("*.json"), key=lambda p: p.stem):
            log_path = json_path.with_suffix(".log")
            if not log_path.exists():
                continue
            run_json = json.loads(json_path.read_text(encoding="utf-8"))
            if has_fresh_record(out / series, run_json["run"]["id"]):
                continue
            record = read(run_json, log_path.read_text(encoding="utf-8", errors="replace"))
            if record is None:
                continue
            target = out / series / f"{record['provenance']['date']}-{run_json['run']['id']}.json"
            content = json.dumps(record, indent=2, sort_keys=True) + "\n"
            if target.exists() and target.read_text(encoding="utf-8") == content:
                continue
            target.parent.mkdir(parents=True, exist_ok=True)
            target.write_text(content, encoding="utf-8")
            written.append(target)
    return written


def main(argv: list) -> int:
    raw = argv[0] if argv else "metrics/raw"
    out = argv[1] if len(argv) > 1 else "metrics"
    written = harvest(raw, out)
    print(f"harvest: {len(written)} record(s) written or changed under {out}")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))

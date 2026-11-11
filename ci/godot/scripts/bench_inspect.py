#!/usr/bin/env python3
"""The inspection matrix's plan and summary (`bench-inspect.yml`, spec 47 step 4, Decision 177).

`plan` reads each size's inspection ticks from the base's series (its `inspect:` line,
`breach_tick_phases preset=<size>`) and prints the matrix: one job per (size, tick), as JSON.

`summary` joins the jobs. Each job timed head and base in turn on its one runner, so its own
comparison is like for like (`bench_compare.py`: medians, change, `*` where the ranges are
apart). Jobs may run on different CPUs, so across jobs the ticks are combined only as ratios
to the reference each job timed beside them (Decision 172): a tick's median over the median
of its reference's tick. The summary names which CPU ran which ticks, and gives the two
series' 95th and 99th percentiles from the plan.

    bench_inspect.py plan 4096=base_series_4096.txt 25k=base_series_25k.txt
    bench_inspect.py summary <folder> 4096 25k

The summary's folder holds `series/<size>_{head,base}.txt` and, per job,
`inspect-<size>-<tick>/` with `head_<n>.txt`, `base_<n>.txt`, `reference_<n>.txt` and `cpu.txt`.
"""
import glob
import json
import os
import re
import statistics
import sys

from bench_compare import compare, compare_spreads, phases, report, report_spreads, spreads

INSPECT = re.compile(r"^inspect: ([\d,]+)$", re.M)


def ticks(text):
    """The inspection ticks named by a series' `inspect:` line; [] when it has none."""
    found = INSPECT.search(text)
    return [int(t) for t in found.group(1).split(",")] if found else []


def matrix(series_by_size):
    """One job per (size, tick), each size taking its ticks from its own series."""
    return [{"size": size, "tick": t} for size, text in series_by_size.items() for t in ticks(text)]


def _read(path):
    with open(path, encoding="utf-8") as source:
        return source.read()


def _runs(folder, side):
    paths = sorted(glob.glob(os.path.join(folder, f"{side}_*.txt")),
                   key=lambda p: int(re.search(r"_(\d+)\.txt$", p).group(1)))
    return [phases(_read(path)) for path in paths]


def _job(folder):
    """One job's rounds, or None when it wrote none (it failed or timed out)."""
    heads, bases, refs = (_runs(folder, side) for side in ("head", "base", "reference"))
    if not heads or not bases or not refs or not all(heads + bases + refs):
        return None
    cpu_path = os.path.join(folder, "cpu.txt")
    cpu = _read(cpu_path).strip() if os.path.exists(cpu_path) else "unknown"
    reference = statistics.median(run["tick"] for run in refs)
    return {"heads": heads, "bases": bases, "cpu": cpu, "reference": reference}


def _jobs(root, size):
    """[(tick, job or None)] for each job folder of this size, earliest tick first."""
    found = []
    for folder in glob.glob(os.path.join(root, f"inspect-{size}-*")):
        tick = int(folder.rsplit("-", 1)[1])
        found.append((tick, _job(folder)))
    return sorted(found, key=lambda pair: pair[0])


def combined(root, size):
    """One row per tick whose job wrote its runs: base and head as × the job's reference."""
    rows = []
    for tick, job in _jobs(root, size):
        if job is None:
            continue
        row = next(r for r in compare(job["heads"], job["bases"]) if r["phase"] == "tick")
        rows.append({
            "tick": tick,
            "cpu": job["cpu"],
            "reference": job["reference"],
            "base": row["base"] / job["reference"],
            "head": row["head"] / job["reference"],
            "change": row["change"],
            "apart": row["apart"],
        })
    return rows


def _report_combined(rows):
    lines = [
        "the ticks combined, × the reference each job timed beside them (* = ranges apart):",
        f"  {'tick':>6} {'base':>8} {'head':>8} {'change':>8}   {'reference':>9}   cpu",
    ]
    for row in rows:
        lines.append(
            f"  {row['tick']:>6} {row['base']:8.3f} {row['head']:8.3f} {row['change']:+7.1f}%"
            f"   {row['reference']:6.2f} ms   {row['cpu']}" + ("  *" if row["apart"] else "")
        )
    by_cpu = {}
    for row in rows:
        by_cpu.setdefault(row["cpu"], []).append(str(row["tick"]))
    lines.append("  CPUs: " + "; ".join(f"{cpu}: {','.join(t)}" for cpu, t in by_cpu.items()))
    return lines


def _report_series(root, size):
    paths = [os.path.join(root, "series", f"{size}_{side}.txt") for side in ("head", "base")]
    if not all(os.path.exists(path) for path in paths):
        return [f"the series at {size}: not run"]
    head, base = (spreads(_read(path)) for path in paths)
    if not head or not base:
        return [f"the series at {size}: no percentiles printed"]
    return [f"the series at {size}, one run each on the plan's runner:"] + report_spreads(
        compare_spreads([head], [base]))


def report_size(root, size):
    """One size's summary: each tick's job head against base, the series' percentiles, then
    the ticks combined as ratios to their references."""
    lines = [f"=== {size}"]
    for tick, job in _jobs(root, size):
        if job is None:
            lines.append(f"tick {tick}: no runs (the job failed or timed out; read its log)")
            continue
        lines.append(f"tick {tick} on {job['cpu']}, reference {job['reference']:.2f} ms:")
        lines.extend(report(compare(job["heads"], job["bases"]), len(job["heads"])))
    lines.extend(_report_series(root, size))
    lines.extend(_report_combined(combined(root, size)))
    return lines


def main(argv):
    if argv[:1] == ["plan"] and len(argv) > 1:
        pairs = dict(arg.split("=", 1) for arg in argv[1:])
        print(json.dumps(matrix({size: _read(path) for size, path in pairs.items()})))
        return 0
    if argv[:1] == ["summary"] and len(argv) > 2:
        for size in argv[2:]:
            print("\n".join(report_size(argv[1], size)))
        return 0
    print(__doc__)
    return 2


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))

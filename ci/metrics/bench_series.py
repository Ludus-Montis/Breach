#!/usr/bin/env python3
"""The bench-series backfill (spec 43 step 5, Decision 162): times each commit of `main`'s
first-parent line with its own harness, against its era's reference, in turn for 5 rounds,
and writes a record per harness under `metrics/bench/`.

- **Harness per commit:** the crate's `breach_tick_phases` where the commit's tree has it;
  otherwise the GDScript tools, `tools/tick_phases.gd` (whole tick and phases) and
  `tools/formation_bench.gd` (whole tick of its clash, a record of its own). The bridge, the
  last commit with both, is timed with all three, so the history states the factor between
  the eras.
- **References:** the bridge for the crate's binary, `eee3e81` for the GDScript tools.
- A harness that prints nothing it can read (a tool that doesn't parse at that commit) has
  each of the reference's paths as `{"na": "not-measured"}`.

- **`--set 25k`** (step 8b): the 25k set on the Rust-era commits instead (series_25k.py).

    bench_series.py plan [--commits "<sha> ..."] [--per-job 3] [--set 4096|25k]   # the jobs
    bench_series.py build [--commits "<sha> ..."] [--set 4096|25k]   # default: the references
    bench_series.py run --commits "<sha> ..." --offset <n> --out <folder> [--godot <bin>]
        [--set 4096|25k]
"""
import json
import os
import subprocess
import sys
import time
from datetime import datetime, timezone
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import bench_record  # noqa: E402
import series_25k  # noqa: E402

BRIDGE = series_25k.BRIDGE
GDSCRIPT_REFERENCE = "eee3e81fc92b026f869fdfced2f16a5306884d0b"
SINCE = "2026-10-14"
ROUNDS = 5
GAP = {"na": "not-measured"}
NOT_APPLICABLE = {"na": "not-applicable"}
FIGHT_SIZE = 1024
NOISE = ("Godot Engine", "Initialize godot-rust", "WARNING")
# The host's triple: a commit's binaries come from the native store (spec 49 step 6).
STORE_TARGET = "x86_64-unknown-linux-gnu"
REFERENCES = {"4096": [GDSCRIPT_REFERENCE, BRIDGE], "25k": [BRIDGE]}


def whole_tick(text: str) -> dict:
    """formation_bench.gd's mid-fight average at FIGHT_SIZE a side, as `tick`."""
    for line in text.splitlines():
        cells = line.replace("/", " ").split()
        if len(cells) == 6 and cells[0] == str(FIGHT_SIZE):
            try:
                return {"tick": float(cells[3])}
            except ValueError:
                return {}
    return {}


HARNESSES = {
    "breach_tick_phases": {
        "file": "native/rust/src/bin/tick_phases.rs", "reference": BRIDGE,
        "args": ["side=4096", "squads=4", "fight=3"], "parse": bench_record.phases,
        "godot": False,
    },
    "tick_phases.gd": {
        "file": "tools/tick_phases.gd", "reference": GDSCRIPT_REFERENCE,
        "args": ["engine=rust", "side=4096", "squads=4", "fight=3"],
        "parse": bench_record.phases, "godot": True,
    },
    "formation_bench.gd": {
        "file": "tools/formation_bench.gd", "reference": GDSCRIPT_REFERENCE,
        "args": [f"sizes={FIGHT_SIZE}"], "parse": whole_tick, "godot": True,
    },
}
CARGO, GDSCRIPT = ["breach_tick_phases"], ["tick_phases.gd", "formation_bench.gd"]


def harnesses(tree: set, sha: str) -> list:
    """The harnesses a commit is timed with, from the paths its tree holds."""
    cargo = [h for h in CARGO if HARNESSES[h]["file"] in tree]
    gdscript = [h for h in GDSCRIPT if HARNESSES[h]["file"] in tree]
    if cargo and sha != BRIDGE:
        return cargo
    return cargo + gdscript


def harness_record(harness: str, head_texts: list, reference_texts: list, context: dict) -> dict:
    spec = HARNESSES[harness]
    results = bench_record.measures(head_texts, spec["parse"])
    reference = bench_record.measures(reference_texts, spec["parse"])
    if not results:
        results = {path: {"unit": "ms", "values": dict(GAP), "median": dict(GAP)}
                   for path in reference}
    machine = dict(context["machine"])
    if not spec["godot"]:
        machine["godot"] = dict(NOT_APPLICABLE)
    return {
        "schema": "record-v1",
        "series": "bench",
        "commit": context["commit"],
        "source": {"harness": harness, "blob": context["blob"], "args": list(spec["args"]),
                   "rounds": len(head_texts), "profile": "release"},
        "machine": machine,
        "results": results,
        "reference": {"commit": spec["reference"], "results": reference,
                      "ratio": bench_record.ratios(results, reference)},
        "provenance": context["provenance"],
    }


def plan(commits: list, per_job: int) -> list:
    return [{"index": i, "offset": start, "commits": " ".join(commits[start:start + per_job])}
            for i, start in enumerate(range(0, len(commits), per_job))]


def record_number(place: int, harness_index: int, attempt) -> int:
    """Unique across the run's jobs (by the commit's place in the whole list) and attempts."""
    return 1000 * (int(attempt) - 1) + place * len(HARNESSES) + harness_index + 1


def default_commits(main_ref: str = "origin/main") -> list:
    out = bench_record.git(".", "rev-list", "--first-parent", "--reverse", f"--since={SINCE}",
                           main_ref, "--", "sim", "native")
    return out.split()


def set_commits(commits: list, set_name: str, tree_of) -> list:
    """The commits a set times: every one for 4096, the Rust era's for 25k."""
    if set_name == "4096":
        return list(commits)
    if set_name == "25k":
        return series_25k.commits(commits, tree_of)
    raise ValueError(f"unknown set {set_name}: 4096 or 25k")


def tree(sha: str) -> set:
    files = [HARNESSES[h]["file"] for h in HARNESSES]
    return set(bench_record.git(".", "ls-tree", "--name-only", sha, "--", *files).split())


def needs_build(folder: Path) -> bool:
    """False when the worktree's binaries are there (the references, fetched once a run)."""
    built = folder / "native" / "bin"
    return not (built.is_dir() and any(built.iterdir()))


def checkout(sha: str, needs_godot: bool, godot: str) -> Path:
    """A worktree of `sha` under trees/ with its binaries from the native store (and imported
    for Godot), once. `store.sh fetch` builds a key that isn't stored and uploads it, so the
    commits that share a `native/` tree build it once between them (spec 49 step 6)."""
    folder = Path("trees") / sha[:12]
    if not folder.exists():
        subprocess.run(["git", "worktree", "add", "-q", "--detach", str(folder), sha], check=True)
    if needs_build(folder):
        started = time.monotonic()
        built = subprocess.run(["bash", "ci/native/store.sh", "fetch", sha, "release", STORE_TARGET,
                                str(folder / "native" / "bin")], capture_output=True, text=True)
        outcome = "fetched" if built.returncode == 0 else f"fetch failed:\n{built.stderr[-2000:]}"
        print(f"{sha[:7]} {outcome} in {time.monotonic() - started:.0f} s")
    marker = folder / ".imported"
    if needs_godot and not marker.exists():
        subprocess.run([godot, "--headless", "--path", ".", "--import"], cwd=folder,
                       capture_output=True, timeout=1800)
        marker.touch()
    return folder


def time_once(harness: str, folder: Path, godot: str) -> str:
    """One run's output, or "" when it failed (its stderr is printed)."""
    spec = HARNESSES[harness]
    if spec["godot"]:
        command = [godot, "--headless", "--path", ".", "--script", f"res://{spec['file']}",
                   "--", *spec["args"]]
    else:
        command = [f"native/bin/{harness}", *spec["args"]]
    try:
        done = subprocess.run(command, cwd=folder, capture_output=True, text=True, timeout=3600)
    except (OSError, subprocess.TimeoutExpired) as error:
        print(f"{harness} in {folder}: {error}")
        return ""
    if done.returncode != 0:
        print(f"{harness} in {folder} exited {done.returncode}:\n{done.stderr[-2000:]}")
    return "\n".join(line for line in done.stdout.splitlines() if not line.startswith(NOISE))


def time_rounds(harness: str, head: Path, reference: Path, godot: str):
    """ROUNDS runs of each, in turn: the commit first in odd rounds, the reference in even."""
    heads, references = [], []
    for round_number in range(1, ROUNDS + 1):
        if round_number % 2:
            heads.append(time_once(harness, head, godot))
            references.append(time_once(harness, reference, godot))
        else:
            references.append(time_once(harness, reference, godot))
            heads.append(time_once(harness, head, godot))
    return heads, references


def godot_version(godot: str) -> object:
    try:
        return subprocess.run([godot, "--version"], capture_output=True, text=True,
                              timeout=60).stdout.strip() or dict(bench_record.NOT_RECORDED)
    except OSError:
        return dict(bench_record.NOT_RECORDED)


def run_where(out: str) -> dict:
    run_id = os.environ.get("GITHUB_RUN_ID", "0")
    return {"out": out, "run_id": run_id, "attempt": os.environ.get("GITHUB_RUN_ATTEMPT", "1"),
            "date": datetime.now(timezone.utc).date().isoformat()}


def provenance(where: dict) -> dict:
    run_id = where["run_id"]
    return {"run_id": int(run_id),
            "url": f"{os.environ.get('GITHUB_SERVER_URL', 'https://github.com')}/"
                   f"{os.environ.get('GITHUB_REPOSITORY', '')}/actions/runs/{run_id}",
            "date": where["date"], "kind": "fresh"}


def run_25k(commits: list, offset: int, out: str) -> list:
    """The 25k set on each commit, the checks beside the bridge (series_25k.py)."""
    where = run_where(out)
    machine = {**bench_record.machine(), "godot": dict(NOT_APPLICABLE)}
    bridge = checkout(BRIDGE, False, "")
    written = []
    for place, sha in enumerate(commits, start=offset):
        sha = bench_record.git(".", "rev-parse", sha)
        folders = {"head": checkout(sha, False, ""), "reference": bridge}
        context = {"commit": bench_record.commit_info(sha), "machine": machine,
                   "provenance": provenance(where),
                   "blob": bench_record.git(".", "rev-parse", f"{sha}:{series_25k.SOURCE_FILE}")}
        written += series_25k.time_commit(
            lambda shape, side: series_25k.run_once(folders[side], shape), context, place, where)
    return written


def run(commits: list, offset: int, out: str, godot: str) -> list:
    where = run_where(out)
    run_id, attempt, date = where["run_id"], where["attempt"], where["date"]
    machine = {**bench_record.machine(), "godot": godot_version(godot)}
    written = []
    for place, sha in enumerate(commits, start=offset):
        sha = bench_record.git(".", "rev-parse", sha)
        commit = bench_record.commit_info(sha)
        for index, harness in enumerate(harnesses(tree(sha), sha)):
            spec = HARNESSES[harness]
            head = checkout(sha, spec["godot"], godot)
            reference = checkout(spec["reference"], spec["godot"], godot)
            heads, references = time_rounds(harness, head, reference, godot)
            context = {"commit": commit, "machine": machine, "provenance": provenance(where),
                       "blob": bench_record.git(".", "rev-parse", f"{sha}:{spec['file']}")}
            record = harness_record(harness, heads, references, context)
            path = bench_record.write_record(record, out, date, run_id, attempt,
                                             record_number(place, index, attempt))
            print(f"{sha[:7]} {harness}: {path.name}")
            written.append(path)
    return written


def main(argv: list) -> int:
    command = argv[0] if argv else ""
    commits = bench_record.option(argv, "--commits", "").split()
    set_name = bench_record.option(argv, "--set", "4096")
    if command == "plan":
        per_job = int(bench_record.option(argv, "--per-job", "3"))
        chosen = commits or set_commits(default_commits(), set_name, tree)
        print(json.dumps(plan(chosen, per_job)))
        return 0
    if command == "build":
        for sha in commits or REFERENCES[set_name]:
            print(checkout(bench_record.git(".", "rev-parse", sha), False, ""))
        return 0
    if command == "run" and set_name == "25k":
        run_25k(commits, int(bench_record.option(argv, "--offset", "0")),
                bench_record.option(argv, "--out"))
        return 0
    if command == "run":
        run(commits, int(bench_record.option(argv, "--offset", "0")),
            bench_record.option(argv, "--out"), bench_record.option(argv, "--godot", "godot"))
        return 0
    print(__doc__)
    return 2


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))

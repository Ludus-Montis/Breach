"""The 25k backfill (spec 43 step 8b, Decision 165): `bench-series` with `set=25k` times step
8a's set (bench_set.shapes) on each Rust-era commit, `breach_tick_phases <shape> series=1`, 5
rounds, the two 24,576 checks in turn with the bridge, and writes one record a shape, as
`record-25k` does on each merge.

- **Which commits:** those the 4,096 backfill times with the crate's binary (their tree has
  `tick_phases.rs`). The GDScript era is left out: hours at 25k for little.
- **A shape the commit's harness can't run** (an argument it rejects, `series` missing: any
  round without its series) writes no record. The page shows a gap, and the job goes on.
"""
import subprocess
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import bench_record  # noqa: E402
import bench_set  # noqa: E402

HARNESS = "breach_tick_phases"
SOURCE_FILE = bench_record.SOURCE_FILE
BRIDGE = "8ca422726e68909554eddad7eeee2199e0fe656d"
ROUNDS = 5
# The set's records in a bench-series run are `-5nn` (+1000 a re-run attempt), apart from
# record-25k's `-1nn` and the 4,096 backfill's numbers.
FIRST_NUMBER = 500


def commits(candidates: list, tree_of) -> list:
    """The candidates, in order, whose tree (`tree_of(sha)`, a set of paths) has the binary."""
    return [sha for sha in candidates if SOURCE_FILE in (tree_of(sha) or set())]


def shapes() -> list:
    return bench_set.shapes()


def shape_record(shape: str, head_texts: list, reference_texts: list, context: dict):
    """The record of one shape, or None when any round lacks its series (the harness rejected
    the shape or has no `series`)."""
    if not bench_record.percentiles(head_texts) or not bench_record.measures(head_texts):
        return None
    machine = dict(context["machine"])
    machine["godot"] = dict(bench_record.NOT_APPLICABLE)
    full = {
        "commit": context["commit"],
        "source": {"harness": HARNESS, "blob": context["blob"],
                   "args": shape.split() + [bench_set.SERIES_ARG],
                   "rounds": len(head_texts), "profile": "release"},
        "machine": machine,
        "provenance": context["provenance"],
        "reference_commit": BRIDGE,
    }
    references = reference_texts if bench_record.percentiles(reference_texts) else []
    return bench_record.build_record(head_texts, references, full)


def record_number(place: int, shape_index: int, attempt) -> int:
    return 1000 * (int(attempt) - 1) + FIRST_NUMBER + place * len(shapes()) + shape_index + 1


def time_rounds(timer, shape: str, bridge: bool):
    """ROUNDS runs of the shape; beside the bridge, in turn: the commit first in odd rounds."""
    heads, references = [], []
    for round_number in range(1, ROUNDS + 1):
        if not bridge:
            heads.append(timer(shape, "head"))
        elif round_number % 2:
            heads.append(timer(shape, "head"))
            references.append(timer(shape, "reference"))
        else:
            references.append(timer(shape, "reference"))
            heads.append(timer(shape, "head"))
    return heads, references


def time_commit(timer, context: dict, place: int, where: dict) -> list:
    """Every shape of one commit, timed with `timer(shape, "head" | "reference")`, which
    returns a run's output ("" when it failed); the paths written."""
    written = []
    for index, (shape, bridge) in enumerate(shapes()):
        heads, references = time_rounds(timer, shape, bridge)
        record = shape_record(shape, heads, references, context)
        sha = str(context["commit"].get("sha", ""))[:7]
        if record is None:
            print(f"{sha} {shape} series=1: no series in every round, no record")
            continue
        path = bench_record.write_record(record, where["out"], where["date"], where["run_id"],
                                         where["attempt"],
                                         record_number(place, index, where["attempt"]))
        print(f"{sha} {shape} series=1: {path.name}")
        written.append(path)
    return written


def run_once(folder: Path, shape: str) -> str:
    """One `series=1` run of the shape in a built worktree; "" when it failed."""
    command = [f"native/bin/{HARNESS}", *shape.split(), bench_set.SERIES_ARG]
    try:
        done = subprocess.run(command, cwd=folder, capture_output=True, text=True,
                              timeout=3600, stdin=subprocess.DEVNULL)
    except (OSError, subprocess.TimeoutExpired) as error:
        print(f"{shape} in {folder}: {error}")
        return ""
    if done.returncode != 0:
        print(f"{shape} series=1 in {folder} exited {done.returncode}:\n{done.stderr[-600:]}")
        return ""
    return done.stdout

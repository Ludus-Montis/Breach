#!/usr/bin/env python3
"""Write a bench record (spec 43 step 4, Decision 162): one `breach_tick_phases` run of a
commit on main's first-parent line, timed against the reference in turn, becomes
`metrics/bench/<date>-<run id>[-<attempt>].json`, holding each round's time and the median
for the whole tick (`tick`) and every phase under it (`tick/scrum/seek`), the reference's same
measurement and the ratio of each median to the reference's. A run of a commit off that line
(a branch) writes none.

A phase printed `0.00` in any round took under the printed resolution (0.005 ms): it is
`{"na": "not-measured"}`, never 0 (the schema's gap rule), and has no ratio.

    bench_record.py --head head_1.txt ... --reference reference_1.txt ... --sha <sha>
        --reference-sha <sha> --args "<args>" --out <folder> [--main origin/main]

Run in the bench job: it reads the machine (lscpu, nproc, /proc/meminfo, /etc/os-release,
rustc), the runner image (ImageOS, ImageVersion), the commit (git, and its PR through the API
when GITHUB_TOKEN is set) and the run (GITHUB_RUN_ID, GITHUB_RUN_ATTEMPT).
"""
import json
import os
import re
import statistics
import subprocess
import sys
from datetime import datetime, timezone
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "godot" / "scripts"))
from bench_compare import phases  # noqa: E402

HARNESS = "breach_tick_phases"
SOURCE_FILE = "native/rust/src/bin/tick_phases.rs"
NOT_RECORDED = {"na": "not-recorded"}
NOT_MEASURED = {"na": "not-measured"}
NOT_APPLICABLE = {"na": "not-applicable"}
PR = re.compile(r"\s*\(#(\d+)\)\s*$")


def phase_path(name: str) -> str:
    """A tick_phases phase name as a results path: `scrum: seek` is `tick/scrum/seek`."""
    if name == "tick":
        return "tick"
    parts = []
    for part in name.split(":"):
        slug = re.sub(r"[^a-z0-9]+", "-", part.lower().replace("'", "")).strip("-")
        if slug:
            parts.append(slug)
    return "/".join(["tick"] + parts)


def measures(texts: list, parse=phases) -> dict:
    """{path: measure} over the rounds' outputs, for every phase each round printed."""
    runs = [parse(text) for text in texts]
    shared = set.intersection(*(set(run) for run in runs)) if runs else set()
    out = {}
    for name in sorted(shared):
        values = [run[name] for run in runs]
        if any(value <= 0 for value in values):
            out[phase_path(name)] = {"unit": "ms", "values": dict(NOT_MEASURED),
                                     "median": dict(NOT_MEASURED)}
        else:
            out[phase_path(name)] = {"unit": "ms", "values": values,
                                     "median": statistics.median(values)}
    return out


def ratios(results: dict, reference: dict) -> dict:
    out = {}
    for path, measure in results.items():
        theirs = reference.get(path, {}).get("median")
        ours = measure.get("median")
        if isinstance(ours, (int, float)) and isinstance(theirs, (int, float)) and theirs:
            out[path] = round(ours / theirs, 6)
    return out


def build_record(head_texts: list, reference_texts: list, context: dict) -> dict:
    results = measures(head_texts)
    reference = measures(reference_texts)
    return {
        "schema": "record-v1",
        "series": "bench",
        "commit": context["commit"],
        "source": context["source"],
        "machine": context["machine"],
        "results": results,
        "reference": {"commit": context["reference_commit"], "results": reference,
                      "ratio": ratios(results, reference)},
        "provenance": context["provenance"],
    }


def write_record(record: dict, out, date: str, run_id, attempt, number=None) -> Path:
    """`metrics/bench/<date>-<run id>[-<n>].json`: `n` is `number` when given (a run that
    writes several records), else the attempt of a re-run."""
    if number is not None:
        suffix = f"-{number}"
    else:
        suffix = "" if str(attempt) in ("", "1") else f"-{attempt}"
    path = Path(out) / "metrics" / "bench" / f"{date}-{run_id}{suffix}.json"
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(record, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    return path


def pr_number(subject: str):
    found = PR.search(subject)
    return int(found.group(1)) if found else dict(NOT_RECORDED)


def title(subject: str) -> str:
    return PR.sub("", subject)


def outcomes_changed(message: str) -> bool:
    return any(line.startswith("Outcomes-changed:") for line in message.splitlines())


def git(cwd, *args) -> str:
    return subprocess.run(["git", "-C", str(cwd), *args], check=True, capture_output=True,
                          text=True).stdout.strip()


def on_first_parent(sha: str, main_ref: str, cwd=".") -> bool:
    full = git(cwd, "rev-parse", sha)
    return full in git(cwd, "rev-list", "--first-parent", main_ref).split()


def position(sha: str, cwd=".") -> int:
    return int(git(cwd, "rev-list", "--first-parent", "--count", sha))


def _command(*args):
    try:
        return subprocess.run(list(args), check=True, capture_output=True, text=True).stdout
    except (OSError, subprocess.CalledProcessError):
        return ""


def machine() -> dict:
    cpu = re.search(r"^Model name:\s*(.+)$", _command("lscpu"), re.M)
    cores = _command("nproc").strip()
    ram = re.search(r"^MemTotal:\s*(\d+) kB", Path("/proc/meminfo").read_text(), re.M) \
        if Path("/proc/meminfo").exists() else None
    os_name = re.search(r'^PRETTY_NAME="?([^"\n]+)', Path("/etc/os-release").read_text(), re.M) \
        if Path("/etc/os-release").exists() else None
    rustc = _command("rustc", "--version").strip()
    return {
        "runner_image": os.environ.get("ImageOS") or dict(NOT_RECORDED),
        "runner_version": os.environ.get("ImageVersion") or dict(NOT_RECORDED),
        "cpu": cpu.group(1).strip() if cpu else dict(NOT_RECORDED),
        "cores": int(cores) if cores.isdigit() else dict(NOT_RECORDED),
        "ram_mb": int(ram.group(1)) // 1024 if ram else dict(NOT_RECORDED),
        "os": os_name.group(1) if os_name else dict(NOT_RECORDED),
        "godot": dict(NOT_APPLICABLE),
        "rustc": rustc or dict(NOT_RECORDED),
    }


def pull_request(number) -> dict:
    """The PR's step label and its Summary's first paragraph, from the API; gaps without it."""
    token, repo = os.environ.get("GITHUB_TOKEN"), os.environ.get("GITHUB_REPOSITORY")
    if not isinstance(number, int) or not token or not repo:
        return {"step": dict(NOT_RECORDED), "summary": dict(NOT_RECORDED)}
    import urllib.request

    request = urllib.request.Request(
        f"https://api.github.com/repos/{repo}/pulls/{number}",
        headers={"Authorization": f"Bearer {token}", "Accept": "application/vnd.github+json"})
    try:
        with urllib.request.urlopen(request, timeout=30) as response:
            pr = json.load(response)
    except OSError:
        return {"step": dict(NOT_RECORDED), "summary": dict(NOT_RECORDED)}
    steps = [label["name"] for label in pr.get("labels", []) if label["name"].startswith("step:")]
    summary = re.search(r"^## Summary\s*\n+(.+?)(?:\n\s*\n|\n#|\Z)", pr.get("body") or "", re.S | re.M)
    return {"step": steps[0] if steps else dict(NOT_APPLICABLE),
            "summary": " ".join(summary.group(1).split()) if summary else dict(NOT_RECORDED)}


def commit_info(sha: str) -> dict:
    full = git(".", "rev-parse", sha)
    message = git(".", "log", "-1", "--format=%B", full)
    subject = message.splitlines()[0] if message else ""
    number = pr_number(subject)
    return {"sha": full, "position": position(full), "pr": number, "title": title(subject),
            **pull_request(number), "outcomes_changed": outcomes_changed(message)}


def files_after(argv: list, flag: str) -> list:
    at = argv.index(flag) + 1
    found = []
    while at < len(argv) and not argv[at].startswith("--"):
        found.append(argv[at])
        at += 1
    return found


def option(argv: list, flag: str, default=None):
    return argv[argv.index(flag) + 1] if flag in argv else default


def main(argv: list) -> int:
    sha, main_ref = option(argv, "--sha"), option(argv, "--main", "origin/main")
    if not on_first_parent(sha, main_ref):
        print(f"bench_record: {sha} isn't on {main_ref}'s first-parent line; no record")
        return 0
    head = [Path(p).read_text(encoding="utf-8") for p in files_after(argv, "--head")]
    reference = [Path(p).read_text(encoding="utf-8") for p in files_after(argv, "--reference")]
    run_id = os.environ.get("GITHUB_RUN_ID", "0")
    date = datetime.now(timezone.utc).date().isoformat()
    context = {
        "commit": commit_info(sha),
        "source": {"harness": HARNESS, "blob": git(".", "rev-parse", f"{sha}:{SOURCE_FILE}"),
                   "args": option(argv, "--args", "").split(), "rounds": len(head),
                   "profile": "release"},
        "machine": machine(),
        "provenance": {"run_id": int(run_id),
                       "url": f"{os.environ.get('GITHUB_SERVER_URL', 'https://github.com')}/"
                              f"{os.environ.get('GITHUB_REPOSITORY', '')}/actions/runs/{run_id}",
                       "date": date, "kind": "fresh"},
        "reference_commit": git(".", "rev-parse", option(argv, "--reference-sha")),
    }
    record = build_record(head, reference, context)
    if not record["results"]:
        print("bench_record: a round printed no phases; no record")
        return 1
    path = write_record(record, option(argv, "--out"), date, run_id,
                        os.environ.get("GITHUB_RUN_ATTEMPT", "1"))
    print(f"bench_record: wrote {path}")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))

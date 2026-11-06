"""Spec 43 step 1: keeps what the Actions logs hold before they expire (90 days), under
metrics/raw/<series>/. For each completed run, its record from the Actions API (the run, its
jobs and their steps) as <run id>.json and its log as <run id>.log: the whole log for
`bench`, the digest and scaling jobs only for `sim-checks` (not the gdUnit output). A run
already archived is skipped, so a re-run only adds what's new; a log already expired is
recorded as {"na": "not-recorded"} (Decision 162's gap).

Oldest runs first, since they expire first. Each run costs two requests (its jobs and its
log archive); the archiver stops while RESERVE requests are left on the token's hourly
limit, and a later run carries on where it stopped.

    python3 ci/metrics/archive_runs.py [metrics/raw]

reads GITHUB_REPOSITORY and GITHUB_TOKEN, and GITHUB_RUN_ID for the records' `archived`.
"""
import io
import json
import os
import re
import sys
import urllib.error
import urllib.request
import zipfile
from dataclasses import dataclass, field
from datetime import date
from pathlib import Path

API = "https://api.github.com"
RESERVE = 20
GAP = {"na": "not-recorded"}

# series: (workflow file, which jobs' logs to keep; None keeps them all).
SERIES = {
    "bench": ("bench.yml", None),
    "sim-checks": ("sim-checks.yml", re.compile(r"digest|scaling", re.IGNORECASE)),
}

RUN_FIELDS = ("id", "name", "run_number", "run_attempt", "event", "status", "conclusion",
              "head_branch", "head_sha", "display_title", "created_at", "run_started_at",
              "updated_at", "html_url", "path")
JOB_FIELDS = ("id", "name", "status", "conclusion", "started_at", "completed_at",
              "runner_name", "labels", "runner_group_name")
STEP_FIELDS = ("number", "name", "status", "conclusion", "started_at", "completed_at")

TIMESTAMP = re.compile(r"^\d{4}-\d\d-\d\dT[\d:.]+Z ?")
ENV_LINE = re.compile(r"^\s+([A-Z_][A-Z0-9_]*):\s?(.*)$")
TOP_LEVEL_JOB = re.compile(r"^\d+_(.+)\.txt$")


@dataclass
class Archived:
    files: list = field(default_factory=list)
    stopped: bool = False


def job_logs(archive_bytes):
    """Each job's log text from a run's log archive: its `<n>_<job>.txt`, or where the
    archive has none for a job, its `<job>/` step files in order."""
    whole, steps = {}, {}
    with zipfile.ZipFile(io.BytesIO(archive_bytes)) as archive:
        for name in sorted(archive.namelist()):
            text = archive.read(name).decode("utf-8", errors="replace")
            if "/" in name:
                steps.setdefault(name.split("/", 1)[0], []).append(text)
            elif TOP_LEVEL_JOB.match(name):
                whole[TOP_LEVEL_JOB.match(name).group(1)] = text
    for job_name, texts in steps.items():
        whole.setdefault(job_name, "".join(texts))
    return whole


def plain_lines(text):
    return [TIMESTAMP.sub("", line) for line in text.splitlines()]


def runner_image(text):
    found = {}
    for line in plain_lines(text):
        for key in ("Image", "Version"):
            if line.startswith(f"{key}: ") and key.lower() not in found:
                found[key.lower()] = line[len(key) + 2:].strip()
    return found or dict(GAP)


def log_env(texts):
    """The `env:` blocks the log prints for each step: bench's inputs, as the run saw them
    (the Actions API doesn't keep a dispatch's inputs)."""
    env, inside = {}, False
    for text in texts:
        for line in plain_lines(text):
            if line.strip() == "env:":
                inside = True
                continue
            match = ENV_LINE.match(line) if inside else None
            if match:
                env.setdefault(match.group(1), match.group(2).strip())
            else:
                inside = False
    return env


def run_record(run):
    record = {key: run.get(key) for key in RUN_FIELDS}
    record["head_commit_message"] = (run.get("head_commit") or {}).get("message")
    record["actor"] = (run.get("actor") or {}).get("login")
    return record


def job_record(job, logs):
    record = {key: job.get(key) for key in JOB_FIELDS}
    record["steps"] = [{key: step.get(key) for key in STEP_FIELDS} for step in job.get("steps") or []]
    record["runner_image"] = runner_image(logs[job["name"]]) if job.get("name") in logs else dict(GAP)
    return record


def kept_jobs(logs, keep):
    return [name for name in logs if keep is None or keep.search(name)]


def archive_run(client, run, series, folder, archived):
    workflow, keep = SERIES[series]
    jobs = client.list_jobs(run["id"])
    archive_bytes = client.run_logs(run["id"])
    logs = job_logs(archive_bytes) if archive_bytes else {}
    kept = kept_jobs(logs, keep)
    record = {
        "workflow": workflow,
        "run": run_record(run),
        "jobs": [job_record(job, logs) for job in jobs],
        "log": {"file": f"{run['id']}.log", "jobs": kept} if archive_bytes else dict(GAP),
        "inputs_from_log": log_env(logs.values()) if archive_bytes else dict(GAP),
        "archived": archived,
    }
    written = []
    if archive_bytes:
        log_path = folder / f"{run['id']}.log"
        log_path.write_text("".join(f"===== job: {name}\n{logs[name]}\n" for name in kept), encoding="utf-8")
        written.append(log_path)
    json_path = folder / f"{run['id']}.json"
    json_path.write_text(json.dumps(record, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    return written + [json_path]


def pending(client, root):
    """Every completed run not yet archived, oldest first, with its series."""
    found = []
    for series, (workflow, _) in SERIES.items():
        for run in client.list_runs(workflow):
            if run.get("status") == "completed" and not (Path(root) / series / f"{run['id']}.json").exists():
                found.append((run.get("created_at") or "", run["id"], series, run))
    return [(series, run) for _, _, series, run in sorted(found, key=lambda item: item[:2])]


def archive(client, root, archived_by, today):
    result, archived = Archived(), {"date": today, "by_run": archived_by}
    for series, run in pending(client, root):
        if client.remaining() < RESERVE + 2:
            result.stopped = True
            break
        folder = Path(root) / series
        folder.mkdir(parents=True, exist_ok=True)
        result.files.extend(archive_run(client, run, series, folder, archived))
    return result


class ActionsClient:
    def __init__(self, repo, token):
        self.base, self.token, self.left = f"{API}/repos/{repo}", token, None

    def _open(self, url):
        request = urllib.request.Request(url)
        request.add_header("Authorization", f"Bearer {self.token}")
        request.add_header("Accept", "application/vnd.github+json")
        request.add_header("X-GitHub-Api-Version", "2022-11-28")
        response = urllib.request.urlopen(request)
        left = response.headers.get("X-RateLimit-Remaining")
        self.left = int(left) if left is not None else self.left
        return response

    def _list(self, path, key):
        items, page = [], 1
        while True:
            with self._open(f"{self.base}/{path}?per_page=100&page={page}") as response:
                batch = json.load(response)[key]
            items.extend(batch)
            if len(batch) < 100:
                return items
            page += 1

    def list_runs(self, workflow):
        return self._list(f"actions/workflows/{workflow}/runs", "workflow_runs")

    def list_jobs(self, run_id):
        return self._list(f"actions/runs/{run_id}/jobs", "jobs")

    def run_logs(self, run_id):
        try:
            with self._open(f"{self.base}/actions/runs/{run_id}/logs") as response:
                return response.read()
        except urllib.error.HTTPError as error:
            if error.code in (404, 410):
                return None
            raise

    def remaining(self):
        return self.left if self.left is not None else RESERVE + 2


def main(argv):
    root = Path(argv[1]) if len(argv) > 1 else Path("metrics/raw")
    client = ActionsClient(os.environ["GITHUB_REPOSITORY"], os.environ["GITHUB_TOKEN"])
    result = archive(client, root, os.environ.get("GITHUB_RUN_ID", "local"), date.today().isoformat())
    print(f"archived {len(result.files)} files under {root}")
    if result.stopped:
        print(f"stopped with {client.remaining()} requests left this hour: run it again to carry on")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))

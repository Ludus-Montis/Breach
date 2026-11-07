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
import http.client
import io
import json
import os
import re
import sys
import time
import urllib.error
import urllib.request
import zipfile
from dataclasses import dataclass, field
from datetime import date
from pathlib import Path

API = "https://api.github.com"
RESERVE = 20
GAP = {"na": "not-recorded"}
NOTHING = {"na": "not-applicable"}
SKIPPED = "skipped.txt"
# What a download can fail with that a later try may not: a dropped or cut-off connection.
TRANSIENT = (OSError, http.client.HTTPException, zipfile.BadZipFile)

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
SECTION_START = re.compile(r"##\[group\]Run |Post job cleanup\.")


@dataclass
class Archived:
    files: list = field(default_factory=list)
    stopped: bool = False
    failed: list = field(default_factory=list)


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
    if job.get("conclusion") == "skipped":
        record["runner_image"] = dict(NOTHING)
    else:
        record["runner_image"] = runner_image(logs[job["name"]]) if job.get("name") in logs else dict(GAP)
    return record


def sections(text):
    """A job's log cut where each step's output starts (`##[group]Run ...`, `Post job
    cleanup.`); the first section is `Set up job`'s."""
    found = []
    for line in text.splitlines(keepends=True):
        if not found or SECTION_START.search(line):
            found.append([])
        found[-1].append(line)
    return ["".join(lines) for lines in found]


def step_sections(job, text, keep):
    """The sections of a job's log that its kept steps printed. The first sim-checks runs did
    everything in one job, `sim-checks`. Every step that ran prints one section, in order,
    but `Complete job` (and a skipped step) prints none, so the sections are matched to those
    steps in turn. Where the counts differ, the whole log is kept rather than guessed at."""
    ran = [step for step in job.get("steps") or []
           if step.get("conclusion") != "skipped" and step.get("name") != "Complete job"]
    if not any(keep.search(step.get("name") or "") for step in ran):
        return []
    cut = sections(text)
    if len(cut) != len(ran):
        return [(f"{job['name']} (whole log: its steps could not be matched)", text)]
    return [(f"{job['name']} / {step['name']}", chunk)
            for step, chunk in zip(ran, cut) if keep.search(step.get("name") or "")]


def kept_logs(jobs, logs, keep):
    """(label, text) for each log kept: a whole job's where its name is kept (or every job's,
    with nothing to match), else its kept steps' sections."""
    records = {job.get("name"): job for job in jobs}
    kept = []
    for name, text in logs.items():
        if keep is None or keep.search(name):
            kept.append((name, text))
        elif name in records:
            kept.extend(step_sections(records[name], text, keep))
    return kept


def archive_run(client, run, series, folder, archived):
    workflow, keep = SERIES[series]
    jobs = client.list_jobs(run["id"])
    archive_bytes = client.run_logs(run["id"])
    logs = job_logs(archive_bytes) if archive_bytes else {}
    kept = kept_logs(jobs, logs, keep)
    if archive_bytes and not kept:
        # Nothing to keep (its digest and scaling jobs were all skipped): only its id is
        # kept, in skipped.txt, so a later run doesn't read it again.
        skipped = folder / SKIPPED
        with skipped.open("a", encoding="utf-8") as out:
            out.write(f"{run['id']}\n")
        return [skipped]
    if not archive_bytes:
        log = dict(GAP)
    else:
        log = {"file": f"{run['id']}.log", "jobs": [label for label, _ in kept]}
    record = {
        "workflow": workflow,
        "run": run_record(run),
        "jobs": [job_record(job, logs) for job in jobs],
        "log": log,
        "inputs_from_log": log_env(logs.values()) if archive_bytes else dict(GAP),
        "archived": archived,
    }
    written = []
    if kept:
        log_path = folder / f"{run['id']}.log"
        log_path.write_text("".join(f"===== job: {label}\n{text}\n" for label, text in kept), encoding="utf-8")
        written.append(log_path)
    json_path = folder / f"{run['id']}.json"
    json_path.write_text(json.dumps(record, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    return written + [json_path]


def skipped_ids(folder):
    path = Path(folder) / SKIPPED
    return set(path.read_text(encoding="utf-8").split()) if path.exists() else set()


def pending(client, root):
    """Every completed run not yet archived or skipped, oldest first, with its series."""
    found = []
    for series, (workflow, _) in SERIES.items():
        skipped = skipped_ids(Path(root) / series)
        for run in client.list_runs(workflow):
            done = str(run["id"]) in skipped or (Path(root) / series / f"{run['id']}.json").exists()
            if run.get("status") == "completed" and not done:
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
        try:
            result.files.extend(archive_run(client, run, series, folder, archived))
        except TRANSIENT as error:
            print(f"run {run['id']}: {type(error).__name__}: {error}; left for a later run")
            result.failed.append(run["id"])
    return result


def with_retries(read, attempts=4, pause=2.0):
    """`read()`, tried again after a dropped or cut-off connection, waiting 2, 4, 8 s."""
    for attempt in range(attempts):
        try:
            return read()
        except TRANSIENT as error:
            refused = isinstance(error, urllib.error.HTTPError) and error.code < 500
            if refused or attempt == attempts - 1:
                raise
            time.sleep(pause * 2 ** attempt)


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

    def _read(self, url, read):
        with self._open(url) as response:
            return read(response)

    def _list(self, path, key):
        items, page = [], 1
        while True:
            url = f"{self.base}/{path}?per_page=100&page={page}"
            batch = with_retries(lambda: self._read(url, json.load))[key]
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
            return with_retries(lambda: self._read(f"{self.base}/actions/runs/{run_id}/logs", lambda r: r.read()))
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
    if result.failed:
        print(f"::warning::{len(result.failed)} runs not archived (their download failed): "
              f"{', '.join(map(str, result.failed))}. Run it again to take them.")
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))

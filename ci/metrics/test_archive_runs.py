"""Tests for archive_runs (spec 43 step 1): each run's log and record under metrics/raw/,
skipped by run id, a gap where the log has expired."""
import io
import json
import sys
import tempfile
import unittest
import zipfile
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import archive_runs  # noqa: E402

FIXTURES = HERE / "fixtures"


def fixture(name):
    return (FIXTURES / name).read_text(encoding="utf-8")


def log_zip(jobs, step_files=False):
    """A run's log archive as the Actions API serves it: `<n>_<job>.txt` for each job's whole
    log, and (older runs) a `<job>/` folder of its steps' logs."""
    buffer = io.BytesIO()
    with zipfile.ZipFile(buffer, "w") as archive:
        for number, (job, text) in enumerate(jobs.items()):
            if step_files:
                archive.writestr(f"{job}/1_Set up job.txt", text)
            else:
                archive.writestr(f"{number}_{job}.txt", text)
    return buffer.getvalue()


def run(run_id, status="completed", sha="abc123"):
    return {
        "id": run_id,
        "name": "Bench",
        "run_number": run_id % 100,
        "run_attempt": 1,
        "event": "workflow_dispatch",
        "status": status,
        "conclusion": "success" if status == "completed" else None,
        "head_branch": "main",
        "head_sha": sha,
        "head_commit": {"message": "perf: a faster seek"},
        "display_title": "Bench",
        "created_at": "2026-10-14T10:00:00Z",
        "run_started_at": "2026-10-14T10:00:00Z",
        "updated_at": "2026-10-14T10:06:00Z",
        "html_url": f"https://github.com/o/r/actions/runs/{run_id}",
        "actor": {"login": "someone"},
        "path": ".github/workflows/bench.yml",
        "repository": {"full_name": "o/r"},
    }


def job(job_id, name):
    return {
        "id": job_id,
        "name": name,
        "status": "completed",
        "conclusion": "success",
        "started_at": "2026-10-14T10:00:00Z",
        "completed_at": "2026-10-14T10:05:00Z",
        "runner_name": "GitHub Actions 7",
        "labels": ["ubuntu-latest"],
        "runner_group_name": "GitHub Actions",
        "steps": [
            {"number": 1, "name": "Set up job", "status": "completed", "conclusion": "success",
             "started_at": "2026-10-14T10:00:00Z", "completed_at": "2026-10-14T10:00:01Z"},
        ],
    }


class FakeActions:
    """The three reads the archiver makes, and the rate limit it watches."""

    def __init__(self, runs, jobs, logs, remaining=5000):
        self.runs, self.jobs, self.logs = runs, jobs, logs
        self.left = remaining
        self.log_reads = []

    def list_runs(self, workflow):
        return list(self.runs.get(workflow, []))

    def list_jobs(self, run_id):
        self.left -= 1
        return list(self.jobs.get(run_id, []))

    def run_logs(self, run_id):
        self.left -= 1
        self.log_reads.append(run_id)
        return self.logs.get(run_id)

    def remaining(self):
        return self.left


def actions(**overrides):
    sim_jobs = {
        "suite (1)": fixture("sim_checks_suite_job.txt"),
        "digests (core)": fixture("sim_checks_digest_job.txt"),
        "scaling (small)": fixture("sim_checks_scaling_job.txt"),
    }
    settings = {
        "runs": {"bench.yml": [run(101), run(102, status="in_progress")],
                 "sim-checks.yml": [run(201), run(202)]},
        "jobs": {101: [job(1, "bench")],
                 201: [job(2, "suite (1)"), job(3, "digests (core)"), job(4, "scaling (small)")],
                 202: [job(5, "digests (core)")]},
        "logs": {101: log_zip({"bench": fixture("bench_job.txt")}),
                 201: log_zip(sim_jobs),
                 202: None},
    }
    settings.update(overrides)
    return FakeActions(**settings)


def files(root):
    return sorted(str(path.relative_to(root)) for path in Path(root).rglob("*") if path.is_file())


class ArchiveTest(unittest.TestCase):
    def setUp(self):
        self.root = Path(tempfile.mkdtemp())

    def archive(self, client):
        return archive_runs.archive(client, self.root, archived_by="999", today="2026-11-06")

    def record(self, series, run_id):
        return json.loads((self.root / series / f"{run_id}.json").read_text(encoding="utf-8"))

    def test_each_completed_run_has_its_log_and_record(self):
        self.archive(actions())
        self.assertEqual(files(self.root), [
            "bench/101.json", "bench/101.log",
            "sim-checks/201.json", "sim-checks/201.log", "sim-checks/202.json",
        ])

    def test_a_second_run_adds_no_file_and_reads_no_log(self):
        self.archive(actions())
        before = {name: (self.root / name).read_bytes() for name in files(self.root)}
        client = actions()
        written = self.archive(client)
        self.assertEqual(written.files, [])
        self.assertEqual(client.log_reads, [])
        self.assertEqual({name: (self.root / name).read_bytes() for name in files(self.root)}, before)

    def test_an_expired_log_is_recorded_as_a_gap(self):
        self.archive(actions())
        record = self.record("sim-checks", 202)
        self.assertEqual(record["log"], {"na": "not-recorded"})
        self.assertEqual(record["inputs_from_log"], {"na": "not-recorded"})
        self.assertEqual(record["jobs"][0]["runner_image"], {"na": "not-recorded"})
        self.assertEqual(record["jobs"][0]["name"], "digests (core)")

    def test_bench_keeps_the_whole_log(self):
        self.archive(actions())
        log = (self.root / "bench" / "101.log").read_text(encoding="utf-8")
        self.assertIn("SUMMARY BEGIN", log)
        self.assertIn("Version: 20261005.1.0", log)

    def test_sim_checks_keeps_the_digest_and_scaling_jobs_only(self):
        self.archive(actions())
        log = (self.root / "sim-checks" / "201.log").read_text(encoding="utf-8")
        self.assertIn("core: outcomes unchanged", log)
        self.assertIn("tick_ms=0.8", log)
        self.assertNotIn("gdUnit4", log)
        self.assertEqual(self.record("sim-checks", 201)["log"],
                         {"file": "201.log", "jobs": ["digests (core)", "scaling (small)"]})

    def test_the_record_keeps_the_run_jobs_and_steps(self):
        self.archive(actions())
        record = self.record("bench", 101)
        self.assertEqual(record["workflow"], "bench.yml")
        self.assertEqual(record["run"]["head_sha"], "abc123")
        self.assertEqual(record["run"]["head_commit_message"], "perf: a faster seek")
        self.assertEqual(record["run"]["actor"], "someone")
        self.assertEqual(record["jobs"][0]["steps"][0]["name"], "Set up job")
        self.assertEqual(record["jobs"][0]["runner_image"],
                         {"image": "ubuntu-24.04", "version": "20261005.1.0"})
        self.assertEqual(record["archived"], {"date": "2026-11-06", "by_run": "999"})

    def test_bench_inputs_are_read_from_the_logs_env(self):
        self.archive(actions())
        self.assertEqual(self.record("bench", 101)["inputs_from_log"], {
            "TOOL": "breach_tick_phases", "ARGS": "side=4096 squads=4 fight=3", "BASE": "",
        })

    def test_older_archives_of_step_logs_are_read_by_job(self):
        client = actions(logs={101: log_zip({"bench": fixture("bench_job.txt")}, step_files=True),
                               201: None, 202: None})
        self.archive(client)
        self.assertIn("SUMMARY END", (self.root / "bench" / "101.log").read_text(encoding="utf-8"))

    def test_it_stops_before_the_rate_limit_and_a_rerun_carries_on(self):
        client = actions(remaining=archive_runs.RESERVE + 2)
        first = self.archive(client)
        self.assertTrue(first.stopped)
        self.assertEqual(files(self.root), ["bench/101.json", "bench/101.log"])
        second = self.archive(actions())
        self.assertFalse(second.stopped)
        self.assertEqual(len(files(self.root)), 5)


if __name__ == "__main__":
    unittest.main()

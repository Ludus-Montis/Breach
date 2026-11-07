# The metrics history's tools

Spec 43 builds a history of the tick's performance under `metrics/` (Decision 162). This
folder holds the tools that write it and check it.

## Archive the old logs before they expire

`bench` and `sim-checks` logs expire 90 days after their run (bench run #1's, of
2026-10-14, about 2027-01-12). The `metrics-archive` workflow keeps what they hold:

1. Start **metrics-archive** from the Actions tab (it takes no inputs; it runs on `main`).
2. It writes, for each completed run not yet on `main`, under `metrics/raw/<series>/`:
   - `<run id>.json`: the run's record from the Actions API (the run, its commit and
     message, its jobs and their steps with times and conclusions, each job's runner image
     as its log printed it) and bench's inputs as its log's `env:` blocks show them (the API
     doesn't keep a dispatch's inputs);
   - `<run id>.log`: the whole log for `bench`; only the digest and scaling jobs' logs for
     `sim-checks`, not the gdUnit output. The first `sim-checks` runs did everything in one
     job: their digest and scaling steps' parts of its log are kept (matched to the steps in
     turn; where they can't be, the whole job's log is kept).
   A log already expired is `{"na": "not-recorded"}` in the `.json`, and has no `.log`; a
   run whose digest and scaling jobs were all skipped is `{"na": "not-applicable"}`.
3. It sends them through [the data-PR route](#the-data-pr-route): a PR on
   `chore/metrics-archive-<run id>` that merges itself once its checks pass.
4. Each run costs two API requests; with hundreds of runs the archiver may reach the
   token's hourly limit. It then stops, says so in the summary, and keeps what it has.
   Start the workflow again an hour later, once its PR has merged: runs already on `main` are
   skipped. A download that still fails after three retries leaves its run out, names it,
   and turns the run red; what was archived goes into its data PR all the same, and the next run takes
   the runs left out.

Run it once more just before spec 43 step 4 lands, so the archive reaches bench's last
run before records start.

Locally (`GITHUB_REPOSITORY` and `GITHUB_TOKEN` set):

```bash
python3 ci/metrics/archive_runs.py metrics/raw
```

## The records and their check

A record is one measurement run of one commit, one file under `metrics/<series>/` named
`<date>-<run id>[-<n>].json` (Decision 162). Its shape is
[`metrics/schema/record-v1.schema.json`](../../metrics/schema/record-v1.schema.json):

```json
{
  "schema": "record-v1",
  "series": "bench",
  "commit": {"sha": "…", "position": 812, "pr": 777, "title": "…", "summary": "…",
             "step": "step:43-1", "outcomes_changed": false},
  "source": {"harness": "breach_tick_phases", "blob": "…", "args": ["…"], "rounds": 5,
             "profile": "release"},
  "machine": {"runner_image": "…", "runner_version": "…", "cpu": "…", "cores": 4,
              "ram_mb": 15990, "os": "…", "godot": {"na": "not-applicable"}, "rustc": "…"},
  "results": {"tick": {"unit": "ms", "values": [12.1, 12.3], "median": 12.2},
              "tick/scrum/seek": {"na": "not-measured"}},
  "reference": {"commit": "8ca4227", "results": {"tick": {"…": "…"}}, "ratio": {"tick": 1.21}},
  "provenance": {"run_id": 19203344, "url": "…", "date": "2026-11-07", "kind": "fresh"}
}
```

- Every field but `schema` is optional, and any value may be a gap in its place:
  `{"na": "not-recorded" | "not-measured" | "not-applicable"}`. A gap is never `0`.
- `results` (and `reference.results`) map a path, lowercase words joined by `/`
  (`^[a-z0-9_-]+(/[a-z0-9_-]+)*$`), to `{"unit", "values", "median"}`. A finer record adds
  children under a path (`tick/scrum/seek` under `tick/scrum`).
- The record, and each measure, take no fields beyond those listed; `commit`, `source`,
  `machine`, `reference` and `provenance` may carry more. A new top-level field, or a change
  to a listed one, is a new version: `metrics/schema/record-v2.schema.json`, and records
  naming it in `schema`. Readers accept every version.

`ci/metrics/check_records.py` checks every record (not `metrics/raw/`, not
`metrics/schema/`): its file name, its schema version (an unknown one fails), the schema
itself, and that no measure's `values` or `median` is `0`. Each problem names the file and
the field. It needs `jsonschema`:

```bash
pip install "jsonschema==4.*"
```

```bash
python3 ci/metrics/check_records.py metrics metrics/schema
```

## Data-only PRs

A PR is data-only when every path it changes is under `metrics/` and none is under
`metrics/schema/` (Decision 163; `ci/metrics/data_only.py`). The `metrics` workflow checks
the records on any PR that touches `metrics/` and says in its summary whether the PR is
data-only. On a data-only PR, `sim-checks`, `docs-gate` and `rust` start, see it and pass at
once, so the required checks still report; no Documentation block or changelog entry is
needed. A schema change is code: the normal checks apply.

Its tests: `python3 -m unittest discover -s ci/metrics -p "test_*.py"` (the check's tests are
skipped where `jsonschema` isn't installed; CI installs it).

## The data-PR route

`bench`, `metrics-archive` and `bench-series` deliver their files through one composite
action, [`.github/actions/metrics-pr`](../../.github/actions/metrics-pr/action.yml) (spec 43
step 3, Decision 164):

```yaml
- uses: ./.github/actions/metrics-pr
  with:
    folder: ${{ runner.temp }}/metrics-new   # files at their repo paths: metrics/bench/…
    series: bench
    app-id: ${{ vars.PROJECT_APP_ID }}
    private-key: ${{ secrets.PROJECT_APP_PRIVATE_KEY }}
```

1. `ci/metrics/stage_data.py` copies the folder's files into the checkout at the same paths.
   If any path is not data (outside `metrics/`, or under `metrics/schema/`) or is a link, it
   refuses the whole folder, copies nothing, and the action stops before any token or PR.
   An empty folder opens no PR.
2. It mints the repo's GitHub App token, pushes `chore/metrics-<series>-<run id>` with it (a
   re-run adds `-<attempt>`), and opens `chore(metrics): <series> <run id>`. A PR opened with
   `GITHUB_TOKEN` would start no checks.
3. It turns on auto-merge (squash): the PR merges once its required checks pass and it has
   no conflict, with no one's approval.

**The owner's set-up, once:**
- The App (`PROJECT_APP_ID`) has **Contents: read and write** and **Pull requests: read and
  write** on this repo.
- Settings → General → **Allow auto-merge** is on.
- The `main` branch rule requires the `metrics` workflow's **records** check. If it also
  requires an approving review, the App must be allowed to bypass it, or the data PRs wait.

Anything else that writes under `metrics/` (the code and CI series' importers) opens a PR
that waits for the owner.

# The metrics history's tools

Spec 43 builds a history of the tick's performance under `metrics/` (Decision 162). This
folder holds the tools that write it.

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
     `sim-checks`, not the gdUnit output.
   A log already expired is `{"na": "not-recorded"}` in the `.json`, and has no `.log`.
3. It pushes them on a branch `chore/metrics-archive-<date>` and links a PR from it in the
   run's summary. Open that PR and merge it (until spec 43 step 3, which lets the workflow
   merge its own data PRs, Decision 164).
4. Each run costs two API requests; with hundreds of runs the archiver may reach the
   token's hourly limit. It then stops, says so in the summary, and keeps what it has.
   Merge that PR and start the workflow again an hour later: runs already on `main` are
   skipped.

Run it once more just before spec 43 step 4 lands, so the archive reaches bench's last
run before records start.

Locally (`GITHUB_REPOSITORY` and `GITHUB_TOKEN` set):

```bash
python3 ci/metrics/archive_runs.py metrics/raw
```

Its tests: `python3 -m unittest discover -s ci/metrics -p "test_*.py"`.

---
spec_type: code
status: active
parent_spec: ../Breach — Reverse Tower Defense Design Spec.md
---

# Spec 43: The metrics history, its bench series first

## Purpose

The owner asked for a history of the tick's performance, kept in the repo, per commit that
changed it, from the first bench run on, with the full breakdown, the machine, the commit
and its aim, and the harness and inputs. It is drawn by a separate tool (Q37, #603). Nothing
records it today:
- `bench` prints a text `SUMMARY` with no machine details, and its logs expire after 90 days.
  Bench run #1's (2026-10-14) expires about **2027-01-12**.
- `sim-checks`' `scaling_check` timings are a second, denser series, with the same deadline.
- The GDScript harnesses (`formation_bench.gd`, `tick_phases.gd`) retired in spec 39 step
  9d-iii. The logs are the only record of what they measured.

**Settled by Decisions 162** (where the history lives, the record, gaps, references, what is
benched), **163** (a data-only PR's checks) and **164** (the measuring workflows merge their
own data PRs). Nothing here changes a simulation outcome.

## Status

Agreed in the design session of 2026-11-06 on Q37 (#603). Seven steps, each its own PR,
built with `/run-phase` as "spec 43 step N".
- **Step 1 is time-sensitive:** its archive must be merged before 2027-01-12, and the
  archive workflow re-run just before step 4 lands.
- Steps 2 and 3 come before any record is written. Steps 4 to 7 follow in order. Step 6 may
  come any time after step 2.
- The other series (code, CI, Gate 1, and the measurements not captured yet) are a later
  round: Q38 (#747).

## Steps

### Step 1, the raw archive

A hand-started `metrics-archive` workflow (`.github/workflows/metrics-archive.yml`) that keeps
what the logs hold before they expire.
- **For each `bench` run:** its whole log, as `metrics/raw/bench/<run id>.log`, and the run's
  record from the Actions API (inputs, head sha, dates, conclusion) as `<run id>.json`.
- **For each `sim-checks` run:** the `scaling_check` and digest sections of its log, as
  `metrics/raw/sim-checks/<run id>.log`, and the run's job and step records (names, start and
  end times, conclusions, runner image) as `<run id>.json`. Not the gdUnit output in full.
- **Idempotent by run id:** a run already archived is skipped, so a re-run only adds what's
  new. A log already expired is recorded in its `.json` as `{"na": "not-recorded"}`.
- **Delivery:** the workflow pushes a branch `chore/metrics-archive-<date>`, and the owner
  opens and merges an ordinary PR (Decision 164's interim route). Step 3 moves it onto the
  data-PR route.
- **Check:** whether the Actions API keeps run, job and step records after the logs expire.
  Record the answer here, under "Measured". If it doesn't, the CI series in Q38 has the same
  deadline, and the archive keeps the job records for it now.

### Step 2, the record schema and the data-only checks

- **`metrics/schema/record-v1.schema.json`:** Decision 162's record. Every field is optional,
  each value may be `{"na": "not-recorded" | "not-measured" | "not-applicable"}`, and
  `results` is a map from a path (`^[a-z0-9_-]+(/[a-z0-9_-]+)*$`) to
  `{"unit", "values", "median"}`. The schema states its version in `schema`.
- **`ci/metrics/check_records.py`:** validates every `metrics/**/*.json` except under
  `metrics/raw/` against the schema its `schema` field names. It fails on an unknown version,
  on a measure whose `values` or `median` is `0` where the source gave nothing (the importers
  write a gap instead), and on a file name not of the form `<date>-<run id>[-<n>].json`.
- **`.github/workflows/metrics.yml`:** runs the check on any PR that touches `metrics/`, and
  reports whether the PR is data-only (Decision 163: every path under `metrics/`, none under
  `metrics/schema/`).
- **The other workflows:** `sim-checks`, `docs-gate` and `rust` gain a first step that
  detects a data-only diff and ends the job with success. Their required checks still report.

### Step 3, the data-PR route

A composite action (`.github/actions/metrics-pr/`) that every measuring workflow uses.
- It takes a folder of new files and copies them under `metrics/`, refusing any path
  outside it or under `metrics/schema/`.
- It mints the App token (`actions/create-github-app-token`, `PROJECT_APP_ID`), pushes a
  branch `chore/metrics-<series>-<run id>`, and opens a PR titled
  `chore(metrics): <series> <run id>`.
- It turns on auto-merge (squash). The PR merges when `metrics.yml`'s checks pass and there
  is no conflict (Decision 164).
- **Owner set-up, named in the step's PR:** the App gets `contents: write` and
  `pull_requests: write` on this repo; the repo allows auto-merge; `metrics.yml`'s check is
  required.
- `metrics-archive` moves onto the route.

### Step 4, `bench` writes records

- **Machine:** a step gathers the runner image and version, CPU model (`lscpu`), cores
  (`nproc`), RAM (`/proc/meminfo`), OS, `rustc` and (where used) Godot versions.
- **Records:** `ci/godot/scripts/bench_compare.py` (or a sibling) writes one record per run
  of a commit on `main`'s first-parent line, with each round's values for every phase path
  from `breach_tick_phases` and the whole tick from `breach_formation_bench`, their medians,
  the reference's same measurement, and the ratio. A run on a branch writes none.
- **The reference:** the cargo era's (Decision 162: the bridge commit), measured in the same
  job, interleaved, 5 rounds.
- **The summary:** a Mermaid `xychart` of the last records' ratio to the reference, read from
  `metrics/bench/` and the new record.
- **On merge:** a `push` trigger on `main` for paths `native/**` and `sim/**` starts `bench`
  with the reference and 5 rounds, and the records go through step 3's route.

### Step 5, the `bench-series` backfill

A hand-started workflow that takes a list of commits (by default each first-parent commit on
`main` that touched `sim/` or `native/` since 2026-10-14, 69 at this spec's writing) and
times each with its own harness.
- **Harness per commit:** the cargo binaries where the commit has them; otherwise
  `tools/formation_bench.gd` for the whole tick, and `tools/tick_phases.gd` for phases where it
  parses. A phase that doesn't parse is `{"na": "not-measured"}`. A commit before 2026-10-17
  whose `formation_bench` takes other args uses its own, recorded in `source.args`.
- **References:** `eee3e81` for GDScript-era commits, and the bridge commit for cargo-era ones.
  The bridge is also timed with the GDScript harnesses against `eee3e81`, so the history
  states the factor between the eras.
- **First, confirm the bridge:** that `8ca4227` has both the GDScript tools and the
  `breach_*` binaries. If not, use the last commit that does, and record it here.
- **Spread:** a matrix of jobs, each a few commits plus its reference, 5 rounds, interleaved
  as `bench.yml` does. Each job's records go through step 3's route, so the backfill lands as
  several data PRs.

### Step 6, the harvest importer

`ci/metrics/harvest.py` reads `metrics/raw/` and writes records marked `harvested`.
- **From each bench `SUMMARY`:** its phases and medians, its tool and args, and its head.
  The machine is `{"na": "not-recorded"}`, and so is the reference.
- **From each `scaling_check` section:** its timings, as a `scaling` series.
- The importer runs again over the whole of `metrics/raw/` with the same output, so a re-run
  adds nothing. It opens an ordinary PR, not a self-merging one (Decision 164 names only the
  three workflows).

### Step 7, the drawing

`tools/metrics_view/` beside spec 37's `tools/trace_view/`: static HTML and JS, no build, and
reading `metrics/` only.
- Tick ms against the reference, per commit along `main`, stacked by phase. Two commits are
  compared at the deepest phase level both have, with a gap shown below that.
- Each point links to its PR and its record.
- The line breaks where the harness changes (GDScript, then cargo), with the bridge's factor
  stated there, never joined across.
- Harvested records are hollow points, apart from fresh ones.
- A small Node or Python script lists `metrics/` into the index the page loads. CI regenerates
  it on data PRs, as part of the data.

## Not in this spec

- **The other series** (Q38, #747): code from git history, CI times, Gate 1's `PROGRESS_LOG.md`,
  PR and issue timings, and measurements not captured yet (per-test durations, peak memory,
  build times, CI failures by kind). They use Decision 162's shape.
- **Benching branch commits into the history:** a comparison stays in its run's summary.

## Measured

(Filled in by steps 1 and 5.)

## Scenarios

### Step 1
- Given bench and `sim-checks` runs with logs, when `metrics-archive` runs, then each run has
  its `.log` and `.json` under `metrics/raw/`, and a second run adds no file.
- Given a run whose log has expired, when it is archived, then its `.json` records the log as
  `{"na": "not-recorded"}`.

### Step 2
- Given a record with a gap written `0`, or a phase path with a capital, when the check runs,
  then it fails and names the file and the field.
- Given a record of an unknown `schema` version, when the check runs, then it fails.
- Given a PR touching only `metrics/bench/`, when `sim-checks` runs, then it passes without
  running the suite. Given one that also touches `metrics/schema/` or any other path, then the
  suite runs.

### Step 3
- Given a folder with a file whose path leaves `metrics/`, when the action runs, then it
  fails and opens no PR.
- Given a data PR the action opened, when its checks pass, then it merges with no one's
  approval.

### Step 4
- Given a `bench` run on `main`, when it finishes, then one record per commit validates and
  holds the machine, every phase's rounds and median, and the ratio to the reference.
- Given a `bench` run on a branch, when it finishes, then it writes no record.

### Step 5
- Given a GDScript-era commit whose `tick_phases` fails to parse, when it is benched, then
  its whole tick is recorded and its phases are `{"na": "not-measured"}`.
- Given the bridge commit, when the backfill runs, then it has a record from each harness.

### Step 6
- Given the archive, when the importer runs twice, then the second run writes nothing new.

### Step 7
- Given records from both eras, when the page draws them, then the line breaks between the
  eras and the bridge's factor is shown.
- Given a commit with only `tick/scrum` and another with `tick/scrum/seek`, then they are
  compared at `tick/scrum`.

## Test-first order

1. Step 1: a Python test red on the archiver's skip-by-run-id and its log extraction (a
   fixture log), then the workflow, checked by a run.
2. Step 2: red on the check rejecting the bad fixtures above, then the schema and check; then
   the data-only detection, red on a fixture diff.
3. Step 3: red on the action's path guard, then the action; auto-merge is checked by a run.
4. Step 4: red on the record writer's output validating, from a fixture `SUMMARY`, then the
   writer.
5. Step 5: red on the harness choice per commit (fixture trees), then the workflow.
6. Step 6: red on the importer's re-run writing nothing, then the importer.
7. Step 7: red on the comparison at the deepest common level (JS logic, as `trace_view.js`
   is tested), then the page.

## Rubric answers

- **single-noun-phrase:** the raw archive; the record schema and its check; the data-PR
  route; bench's records; the backfill; the harvest importer; the drawing.
- **ocp-extension-point:** a new series is a folder under `metrics/` and new paths; a new
  harness is a `source` value; a new schema version is a file under `metrics/schema/`.
- **lsp-contract-scope:** not applicable; no subtypes.
- **isp-fit:** the drawing reads only records; the importer only `metrics/raw/`.
- **dip-direction:** the workflows depend on the data-PR action, never on the token or the
  merge details.
- **general-over-special:** general: one record shape and gap rule for every series. The one
  special case is Decision 164's self-merge, with the owner's reason.
- **order-independent:** not applicable; no simulation outcome changes.
- **bounded-work:** not applicable to the tick; one bench job per qualifying merge.

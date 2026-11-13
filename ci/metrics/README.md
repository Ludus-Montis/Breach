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
   A log already expired is `{"na": "not-recorded"}` in the `.json`, and has no `.log`. A
   run with nothing to keep (its digest and scaling jobs all skipped, as on a docs-only or
   data-only PR) keeps no files: only its id, in `<series>/skipped.txt`, so it isn't read
   again.
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
itself, and that no measure's `values` or `median` is `0`. A measure may give `quartiles`,
one `[q1, q3]` pair per entry of `values` (spec 48 step 5d); where it does, it must have as
many pairs as values, each holding its value (`q1 <= value <= q3`). Each problem names the
file and the field. It needs `jsonschema`:

```bash
pip install "jsonschema==4.*"
```

```bash
python3 ci/metrics/check_records.py metrics metrics/schema
```

## Bench records

Each merge to `main` that touches `native/` or `sim/` starts `bench`'s `record` job (spec 43
step 4). It times the merged commit with `breach_tick_phases side=4096 squads=4 fight=3`
against the cargo era's reference, `8ca4227`, in turn for 5 rounds, and writes one record,
`metrics/bench/<date>-<run id>.json` (`ci/metrics/bench_record.py`):

- `results`: the whole tick as `tick` and each phase under it (`scrum: seek` is
  `tick/scrum/seek`), each with its 5 rounds and median, in ms. A phase printed `0.00` (under
  0.005 ms) is `{"na": "not-measured"}`.
- `reference`: the reference commit, its same measurement, and each median's `ratio` to the
  reference's (below 1 is faster).
- `machine`, `commit` (with its PR, step label and `Outcomes-changed:` trailer), `source` and
  `provenance`, as the schema has them.

The record goes through [the data-PR route](#the-data-pr-route) and merges itself. The run's
summary compares the commit with the reference and draws the last 20 records' tick ratio
(`ci/metrics/bench_chart.py`). To record `main`'s head by hand, start **Bench** on `main`
with **record** ticked; on any other branch it writes no record.

Every `bench` job takes its binaries from the native store (spec 49 step 5, Decision 179;
`native/README.md`): `ci/native/store.sh fetch` puts the head's, the base's and the
reference's `release` key in each tree's `native/bin/`, and a `profile` of
`breach_tick_phases` runs its `profiling` key. A key that isn't stored is built once and
uploaded, so a run builds the head at most, and nothing when the head's `native/` tree is
already stored. Profiling any other tool still builds it in place.

### The 25k set

`bench`'s `record-25k` job (spec 43 step 8a, Decision 165) follows `record` on the same merges.
It times nine shapes with `breach_tick_phases <shape> series=1`, 5 rounds in turn
(`ci/metrics/bench_set.py shapes` lists them):

- the grid, which fits `t(x) = b·x + c·x²` (`x` units a side, `b` and `c` at least 0): lanes of
  1,024 at `side=1024 squads=1`, `side=4096 squads=4` and `side=8192 squads=8`; one squad at
  `side=2048`, `4096`, `8192` and `12288`;
- the checks, never fitted: `side=24576 squads=24` and `side=24576 squads=1`, each with the
  bridge `8ca4227` timed beside it, so their records have a `reference`. The grid's have none.

Each shape is one record, `metrics/bench/<date>-<run id>-<1nn>.json`, with `source.args`
naming the shape. From spec 47 step 2a the series runs until the field settles, not until one
side breaks, so its records also carry `series=settled` in `source.args` (step 2b). The mark
isn't part of the shape (`bench_set.py` and the page drop it with `series=1`), and the 25k
chart breaks its lines where it starts. A set recorded on `main` between 2a and 2b has no mark. `tick` is the series' mean tick, `tick-p95` and `tick-p99` its percentiles,
each with its 5 rounds and median; the phases are their series means. All nine go in one data
PR (`chore/metrics-bench-25k-<run id>`), whose body holds each shape's miss at 24,576 a side,
(predicted − measured) ÷ measured, flagged beyond 10%, and "the form no longer fits" when the
merge before on `main` was flagged on that shape too (`bench_set.py summary`). Raise a
`question` issue then (Decision 165). The fit is never stored; the page computes it from the
records (step 8c), tested against the same fixtures (`fixtures/bench_fit.json`). Until then,
the tick history page and `bench_chart.py` leave out fresh `series=1` records, so the 4,096
line keeps its points.

A run that fails or times out (90 minutes) writes nothing. The job's concurrency group never
cancels a running set, but GitHub keeps one pending run a group: a third merge during a run
replaces the second's pending one, and that merge has no set.

## Busy threads

Since spec 50 step 3, `breach_tick_phases` ends a series and a repeated tick with a `busy
threads` block: each phase's CPU time over its wall time (`native/README.md`, Tick phases).
`bench_record.py` writes it beside the ms as `busy/<key>`, with the unit `threads`:
`busy/tick` for the whole tick, `busy/scrum/seek` for `tick/scrum/seek`, each with its rounds
and their median. Only the head's rounds carry them; the reference's don't, so they have no
ratio. The schema doesn't change (`unit` is free text), and the metrics page leaves them out
of the phases it charts and compares (`metrics_view.js`'s `inMs`). A binary from before step 3,
or a run on Windows (`local_record.py`, the owner's records), prints none, and its record has
no `busy/` keys. Rayon's idle workers spin a little before they sleep, so a short phase's
reading is an upper bound on its useful parallel work.

## Repeat records

`bench-inspect` (spec 47, Decision 177) replays chosen ticks live. Each merge to `main` that
touches `native/` or `sim/` runs it, with the merge as head and the `main` before it as base
(step 5a). Each inspect job, one per (size, inspection tick), then writes one record,
`metrics/inspect/<date>-<run id>-<n>.json` (`n` the job's place in the matrix, from 1):

- `series` is `inspect`, `source.harness` is `tick_phases-repeat`, and `source.args` are the
  replay's, `preset=<size> at=<tick> repeat=<n>`. `check_records.py` fails a repeat record
  whose args don't name `at=` and `repeat=`.
- `results`: each round's median of the repeats, for the tick and each phase, and their
  median across the rounds, as a bench record's.
- `reference`: the bridge, `8ca4227`, timed with `record`'s battle (`side=4096 squads=4
  fight=3`) in the same job, so records from different runners combine only as ratios
  (Decision 172).

The summary job gathers the jobs' records into one data PR (`chore/metrics-inspect-<run
id>`), which merges itself. A run whose head is off `main`'s first-parent line, a job that
failed, and anything before 5a write none: there is no backfill. The repeats guide and the
whole tick judges (Q47): phases whose cost builds up across ticks (`strays`, `groups`,
`bodies: pushes`, `stamina`) read smaller repeated. The bench and 25k pages read only
`series` `bench`, so these records don't touch their lines; plotting them is step 5b.

## The backfill

`bench-series` (spec 43 step 5) fills the history from before step 4: start it from the
Actions tab. With no inputs it takes each first-parent commit on `main` that touched `sim/`
or `native/` since 2026-10-14 (67), three to a job (`ci/metrics/bench_series.py`).

| A commit's tree holds | It is timed with | Against |
|---|---|---|
| `native/rust/src/bin/tick_phases.rs` | `breach_tick_phases side=4096 squads=4 fight=3` | the bridge, `8ca4227` |
| only the GDScript tools | `tools/tick_phases.gd engine=rust side=4096 squads=4 fight=3`, and `tools/formation_bench.gd sizes=1024` (its mid-fight average as `tick`), a record each | `eee3e81` |
| both, and it is the bridge | all three | as above |

Each commit and its reference run in turn for 5 rounds. A harness that prints nothing
readable at a commit has the reference's paths as `{"na": "not-measured"}`. Each job's records
go through [the data-PR route](#the-data-pr-route), so the backfill lands as one data PR a job.
`commits` takes a list instead (a re-run of the commits a job missed, say).

### The 25k backfill

With `set` at `25k` (spec 43 step 8b, Decision 165), `bench-series` times [the 25k
set](#the-25k-set) instead, on each commit the 4,096 backfill times with
`breach_tick_phases` (23 on 2026-11-11, from `827b8ea`; the GDScript era is left out:
`ci/metrics/series_25k.py`). Each shape runs `series=1` for 5 rounds, the two 24,576 checks
in turn with the bridge, a record a shape, numbered `-5nn` (+1000 a re-run attempt). A
shape a commit's harness can't run (an argument it rejects, or a round with no series) writes
no record, and the job goes on: the page shows a gap. Its data PRs are
`chore/metrics-bench-series-<run id>-25k-<job>`. Start it once, on `main`, with `commits`
empty; the 14 harvested `preset=25k` runs and the 2 `settle=10 fight=5` ones stay as they are.

To keep builds short and disk free: a `references` job builds `eee3e81` and `8ca4227` once
and hands their binaries to every timing job; within a job every commit builds into one
shared folder, so the dependencies are compiled once and each commit recompiles only what
changed; that folder is restored from one cache entry the `references` job writes once
(`bench-series-target-v1`; bump it to rebuild); and each job first deletes runner tools it
doesn't use. Each job's summary gives every build's time, the job's time and its free disk.

## Local runs, the owner's PC

Spec 48 step 1, Decision 178. Runs on the owner's PC are a series of their own, under
`metrics/local/`, drawn apart from CI's: `"series": "local"`, `"kind": "local"`, no reference
(they compare in raw ms) and the machine from `local_machine.json`. CI's charts read `series`
and leave them out; `check_records.py` checks them as it checks any record.

- `bash ci/metrics/local_grid.sh grid [rounds]` (after `bash native/build.sh`; the bare
  `local_grid.sh [rounds]` is the same) runs the grid of squad size against squad count:
  sides 3,072 to 24,576, squads 1, 2, 4, 8 and 24, each cell's settled series (`series=1`),
  the rounds taken in turn, into `bench-grid/` (git-ignored), then imports it.
- `bash ci/metrics/local_grid.sh inspect [rounds]` (spec 48 step 5b, after `grid`) replays
  each cell's `inspect:` ticks (round 1's) with `at=<ticks> repeat=50`, the rounds in turn,
  into `bench-grid/inspect/`, and imports only those: one record per (cell, tick).
  `inspect` and `pair` pass `laps=1` (step 5f) where the binary takes it (a pair's base from
  before 5f doesn't), so the raw output keeps every lap after each tick's table.
- `bash ci/metrics/local_grid.sh pair <head> <base> [rounds]` (5b, default 5 rounds) builds
  both commits in worktrees under `bench-pair/` (git-ignored), runs each side's settled
  series once per size, then times both in turn (head first in odd rounds) at the base's
  `inspect:` ticks, one tick a run, as CI's `bench-inspect` does on one runner. Its sizes are
  CI's, `PAIR_SIZES="preset=4096 preset=25k"`. `ci/metrics/local_pair.py` then prints, per
  size and tick, each phase's medians, the head's change and `*` where the ranges are apart
  (`bench_compare.py`'s table), and the two series' 95th and 99th. It records nothing.
  `BREACH_TICK_PHASES` names another binary for `grid` and `inspect`.
- `python3 ci/metrics/local_record.py <folder> --commit <sha> --date <YYYY-MM-DD>` imports
  outputs named `s<side>_q<squads>_r<round>.txt`: one record per cell,
  `metrics/local/<date>-<n>.json`, `n` counting on from that day's records. It refuses a
  file whose printed side or thread count differs from its name or the machine, by name,
  and writes nothing. `breach_tick_phases` doesn't print its squad count, so that comes from
  the name alone. A machine line that names the CPU (spec 48 step 3) is preferred over
  `local_machine.json`; `unknown` keeps the file's.
- A cell run with `series=1` (spec 48 step 5a) also keeps `tick-p95` and `tick-p99` over the
  whole settled series, its args end `series=1 series=settled`, and `source.inspect` holds
  the `inspect:` ticks each round named.
- Files named `s<side>_q<squads>_r<round>_repeat.txt` hold spec 47's repeat mode
  (`at=<t>,... repeat=<n>`): one record per (cell, tick), `source.harness`
  `tick_phases-repeat`, args `side= squads= at=<t> repeat=<n>`. Each round's median of the
  `n` repeats is a value, and that round's `q1` and `q3` are its `quartiles` pair (spec 48
  step 5d). The laps themselves (`laps=1`'s `laps:` block, step 5f) stay in the raw output:
  their lines end in `]`, so the importer skips them, as it does the interval's.
  Output from before step 5d printed the IQR as one width, so its records keep no quartiles. The repeats are a guide, not the judge: they under-report phases whose cost builds up
  across ticks (`strays`, `groups`, `bodies: pushes`, `stamina`; Q47, #909).

**What a local pair is good for** (spec 48 step 4, the owner's decision of 2026-11-12): a
direction guide, not a judge. On four pairs with known CI results, the PC moved the same way
as CI for an ordinary change, and left alone what CI left alone, but its sizes were a quarter
to five times CI's. So a local pair may show whether a change speeds up the phase it aims at
and leaves the rest alone; its percentages are never quoted as a change's size, and `bench`
on CI judges before a merge. A change to how memory is allocated (the allocator, kept
buffers, pooling) can move phases the opposite way on the PC (#783's `strays` and `bodies:
sums and applying`), so it goes straight to CI. The numbers are in spec 48, `## Measured`.

Keep a run's outputs under `metrics/raw/local/<date>-<short sha>/`. The first is the owner's
2026-11-11 grid on `a5f574d` (20 records, 3 rounds each).

## Harvest the archived logs

`ci/metrics/harvest.py` (spec 43 step 6) turns what `metrics-archive` kept under
`metrics/raw/` into records marked `"kind": "harvested"`, the numbers as they were seen then:

- each bench run's `SUMMARY`: its head's phases (a rounds comparison's medians, or one round's
  values), tool and args, into `metrics/bench/<date>-<run id>.json`. `commit` keeps the head's
  `sha`, `branch` and the `base` it was compared with; the machine and reference are
  `not-recorded`.
- each `sim-checks` run's `scaling_check` lines, into `metrics/scaling/<date>-<run id>.json`,
  as `scaling/<stage>/<size>`.

It writes a file only when its content changes, so a re-run adds nothing, and it never
overwrites a fresh record. Run it after `metrics-archive` has added runs, and open an
ordinary PR with what it wrote (its records aren't one of the self-merging series):

```bash
python3 ci/metrics/harvest.py metrics/raw metrics
```

## Data-only PRs

A PR is data-only when every path it changes is under `metrics/` and none is under
`metrics/schema/` (Decision 163; `ci/metrics/data_only.py`). The `metrics` workflow checks
the records on every PR and says in its summary whether the PR is
data-only. On a data-only PR, `sim-checks`, `docs-gate` and `rust` start, see it and pass at
once, so the required checks still report; no Documentation block or changelog entry is
needed. A schema change is code: the normal checks apply.

Its tests: `python3 -m unittest discover -s ci/metrics -p "test_*.py"` (the check's tests are
skipped where `jsonschema` isn't installed; CI installs it).

## The data-PR route

`bench`, `metrics-archive`, `bench-series` and `bench-inspect` (spec 47 step 5a) deliver their files through one composite
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

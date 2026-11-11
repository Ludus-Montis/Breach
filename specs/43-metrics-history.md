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
own data PRs). Step 8, the 25k series and its scaling model, is settled by **Decision 165**.
Nothing here changes a simulation outcome.

## Status

Agreed in the design session of 2026-11-06 on Q37 (#603). Seven steps, each its own PR,
built with `/run-phase` as "spec 43 step N".
- **Step 8** (8a, 8b, 8c) was added in the design session of 2026-11-08 on Q40 (#820), after
  steps 1 to 7 merged. 8a comes first; 8b and 8c may follow in either order.
  8c was split at build (2026-11-11) into 8c (the fit and the 25k chart's lines) and 8d
  (its toggles, the 4,096 chart's "on track" lines and the detail panel), which needs 8c.
- **Step 1 is time-sensitive:** its archive must be merged before 2027-01-12, and the
  archive workflow re-run just before step 4 lands.
- Steps 2 and 3 come before any record is written. Steps 4 to 7 follow in order. Step 6 may
  come any time after step 2.
- The other series (code, CI, Gate 1, and the measurements not captured yet) are a later
  round: Q38 (#747).
- **Step 9** was added in the design session of 2026-11-11 on Q43 (#855), settled by
  **Decision 172**. It needs only step 7, so it may come before or after step 8. It touches the same page as
  8c, so the two don't run side by side (Decision 171); whichever comes second wires the 25k
  chart to the CPU toggles (step 9 came first, so 8d does).

## Pre-mortem (round 3, Q43, asked at the start of the round)

- **Regret in a month:** the ratio is the default, and a real regression of 8% is put down
  to the runner; or the ratio hides that one CPU's results relative to the reference drift
  apart over time.
- **Riskiest assumption:** that dividing by the reference timed beside a record removes the
  runner, i.e. that a change speeds every CPU up by the same factor. A change bound by the
  cache or memory need not.
- **Cheapest test:** the records already hold one commit measured on two CPUs (`8ca4227`,
  three benches): 13–33% apart in ms, 3–11% as a ratio (see Measured). The same-runner A/B
  Q43 started (`bench`, `preset=4096`, `5af0ed5` against `bfce23c`, 5 rounds) says whether
  the 60% rise was the runner. Its result goes under Measured before this round's PR merges.
- **Guarded against:** the CPU toggles let a reader keep to one runner when a few percent
  matter, and Decision 172's Consequences say to confirm such a gap with a same-runner `bench`.

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
  of a commit on `main`'s first-parent line, with each round's values for the whole tick and
  every phase path from `breach_tick_phases`, their medians, the reference's same
  measurement, and the ratio. A run on a branch writes none. (Amended 2026-11-07 with the
  owner: this read "the whole tick from `breach_formation_bench`", a binary that never
  existed; `formation_bench.gd` retired in spec 39 step 9d-iii with no cargo port, and
  `breach_tick_phases` prints the whole tick of the same battle above its phases.)
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
  (Settled 2026-11-07 with the owner: a GDScript-era commit has **two records**, each from one
  battle. `tick_phases.gd` with `engine=rust side=4096 squads=4 fight=3` (its own tick and
  phases, the cargo records' battle) and, as `-n`, `formation_bench.gd` with `sizes=1024` (its
  mid-fight average as `tick`).)
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
  it on data PRs, as part of the data. (Amended 2026-11-08 with the owner, who asked to avoid
  conflicts and to see the page on the web: one index edited by every data PR makes data PRs
  that run together conflict, so they can't merge themselves. The index is built, never
  committed: the `metrics-pages` workflow builds it and publishes the page on GitHub Pages on
  each push to `main` that changes `metrics/`, and `build_index.py` builds it locally.)

### Step 8a, `bench` records the 25k set

Settled by **Decision 165** (Q40, #820). A `record-25k` job in `bench.yml`, beside `record`.
- **When:** the same merges as `record` (a push to `main` touching `native/` or `sim/`, or by
  hand with `record`). `needs: record`, so the 4,096 record always lands first and stays the
  quick guide. `concurrency` queues and never cancels (`cancel-in-progress: false`), so every
  merge gets its set. `timeout-minutes: 90`. A run that fails or times out writes nothing,
  and the page shows the gap.
- **First, confirm `record` has run on GitHub.** At this step's writing it never had: its
  trigger came with #786, after the last merge touching the crate (#782). Bench #97
  (<https://github.com/Ludus-Montis/Breach/actions/runs/42492666350>) was started by hand
  with `record` on 2026-11-08 to prove it. If its data PR didn't land, fix `record` first.
- **The set:** `breach_tick_phases ... series=1` (from contact to rout, `LIMIT` ticks at
  most), 5 rounds, interleaved as `record` does:
  - the grid: lanes of 1,024 at `side=1024 squads=1`, `side=4096 squads=4` and
    `side=8192 squads=8`; one squad at `side=2048`, `4096`, `8192` and `12288` (`squads=1`);
  - the check: `side=24576 squads=24` and `side=24576 squads=1`;
  - the bridge (`8ca4227`) beside the two checks only, so the 25k chart has its ratio to the
    reference. The grid needs none: the fit and its check share a job and a runner.
- **The records:** one per shape, series `bench`, under `metrics/bench/` through the data-PR
  route (Decision 164), all in one data PR. `source.args` names the shape. `results`:
  - `tick` is the series' mean tick, `tick-p95` and `tick-p99` its percentiles, each with
    its 5 rounds' values and their median; phases are their series means, as now;
  - the keys fit `record-v1`'s pattern, so the schema doesn't change. If the writer needs it
    to, that is a `record-v2` file under `metrics/schema/` (Decision 162) and the PR waits for
    the owner.
- **No fit in CI.** The model is computed by the page (step 8c) from these records, never
  stored, so a change of form refits the whole history. The data PR's summary does print the
  two misses and flags a miss beyond 10%, and a second flagged merge in a row on one shape,
  from the same code the page uses (a Python port is fine if it is tested against the same
  fixtures).
- **Cost:** timing is seconds a run at these sizes; the job is mostly its build, about 15
  to 25 minutes a merge.

### Step 8b, the 25k backfill

`bench-series` gains a `set` input: `4096` (today's, the default) or `25k` (step 8a's set,
with the bridge beside the two checks).
- **Which commits:** with `set=25k`, by default every commit the 4,096 backfill timed with
  the crate's `breach_tick_phases` (the Rust era: 18 at this step's writing, from before the
  bridge to `92909e1`). The GDScript era (57 commits) is left out: hours at 25k for little.
- **A commit whose harness can't run a shape** (an argument it rejects, `series` missing)
  writes no record for that shape. The page shows a gap.
- **Spread and route** as step 5: a few commits a job, the shared build cache, a data PR per
  job.
- **Last, start it.** Once this step's PR is merged, its session dispatches it once:
  `bench-series` on `main` with `set=25k` and `commits` empty. The owner monitors the runs.
- **The harvested 25k runs** (14 `preset=25k`, and 2 `side=24576 squads=24 settle=10
  fight=5`) are already records. They stay harvested, drawn only behind the toggle (step 8c).

### Step 8d, the 25k chart: its toggles, the 4,096 chart's lines and the detail panel

**Split at build:** from step 8c, PR #872, 2026-11-11 (see step 8c, under Built).
**Needs:** 8c
**Touches:** `tools/metrics_view/`

On the 25k chart step 8c draws (`tools/metrics_view/metrics_25k.js` holds the fit and the
set's lines; `metrics_25k_page.js` draws them):
- **"× the bridge":** a toggle that draws the ratio to the bridge instead of ms. The goal
  lines hide in that view.
- **The 4,096 chart** gains dotted "on track for Decision 159" lines, target and interim:
  for each commit, the 4-lane series tick at which the lanes fit predicts 50 ms (and 100 ms)
  at 24 lanes, drawn against the 25k set's 4-lane point. Its `fight=3` series stays as it is.
  These lines break at a commit with no 25k set.
- **A point's detail panel:** that commit's fitted curve over size, with its grid points and
  the measured 24,576. No separate grid chart.
- **Each point's shape is its CPU** (amended 2026-11-11, Decision 172): one shape per CPU,
  shared with the legend of the page's CPU toggles, which show or hide its points. The fit
  and the miss use every record of the commit, whether or not its CPU is shown.
- **"Show harvested points"** adds the harvested 25k runs, hollow and unfitted.

### Step 9, the chart by the reference

**Needs:** 7
**Touches:** `tools/metrics_view/`

Settled by **Decision 172** (Q43, #855). On the tick history page (`tools/metrics_view/`).
- **The unit pickers.** The whole-tick chart: "× the reference" (default) or "ms". The phase
  chart: "× the reference" (default), "ms" or "% of tick". "× the reference" is a record's
  value ÷ the tick of the reference in the same record (`reference.results`, the tick's
  median); a phase in that unit is the phase's ms ÷ that reference tick, so the stack's top is
  the tick's ratio. The whole-tick chart stays on a log scale in both units, and its axis
  names the unit.
- **The reference mark.** In ms, the phase chart keeps each record's reference dash. In
  × the reference, one line at 1 replaces them. In % of tick there is none.
- **% of tick** is each phase's ms ÷ the record's whole tick, so the bar's full height is
  100%. Hiding a phase leaves its share out and changes no other share.
- **The phase toggles.** Each entry of the phase legend, "other" included, shows or hides
  that phase in every unit. Hidden phases are left out of the stack, which closes up.
- **The CPU toggles.** A row under the page's header, one checkbox per CPU found in the
  records (`machine.cpu`), named as the record names it, with "unknown" for records without
  one; all on by default. A hidden CPU's records leave every chart. The whole-tick line joins
  the points left (still breaking where the harness changes).
- **Tooltips** give every point's CPU, its ms, and its ratio where it has a reference.
- **The compare** (a point's detail panel and table) shows the phase chart's unit, and names
  both records' CPUs in its title.
- **Records with no reference** are left out in × the reference. A note under each chart
  says how many, and that "ms" shows them. "Show harvested points" still adds the harvested
  points in ms.
- **The note on the bridge** (the line break between harnesses) is unchanged: the two eras'
  references are different commits, so their ratios are not joined.

## Not in this spec

- **The other series** (Q38, #747): code from git history, CI times, Gate 1's `PROGRESS_LOG.md`,
  PR and issue timings, and measurements not captured yet (per-test durations, peak memory,
  build times, CI failures by kind). They use Decision 162's shape.
- **Benching branch commits into the history:** a comparison stays in its run's summary.
- **The Pages site as a project site** (a landing page, a tab per measure, gameplay
  examples), raised with Q40 (#820): settled by Decisions 166 and 167, built in spec 44.
- **`breach_scaling`'s CI limit** (×3 a doubling) against Decision 159's ×2.2, and one squad's
  pair work at 25k: a later session (Decision 165, Bounded).

## Measured

(Filled in by steps 1, 2, 4 and 5.)

**Step 1, 2026-11-06:**
- **The archive costs two requests a run** (its jobs, and its log archive, which holds every
  job's log), oldest first. With 88 bench and 373 `sim-checks` runs that is about 920, near
  `GITHUB_TOKEN`'s 1000 an hour, so the archiver stops 20 short and a re-run carries on.
- **A dispatch's inputs are not in the Actions API's run record.** Bench's inputs are read
  from its log's `env:` blocks into the `.json`'s `inputs_from_log`; once the log expires,
  they are lost with it.
- **Whether the API keeps run, job and step records after the logs expire** can't be seen
  yet: no run is 90 days old before 2027-01-12. The archive keeps every run's job and step
  records now, so the CI series (Q38) doesn't depend on the answer. The first archive run
  after 2027-01-12 answers it: a `.json` with `"log": {"na": "not-recorded"}` and its `jobs`
  filled means yes.
- **Until `f587f70` (2026-10-22) the scaling check ran in the `CI` workflow** (`ci.yml`'s `scaling`
  job), not `sim-checks`, so its runs from 2026-10-14 to then are not archived by this step.
  Adding `ci.yml` is one line in `archive_runs.py`'s `SERIES`.
- **The first archive run (#1, 2026-11-06)** wrote 53 bench and 175 `sim-checks` runs in
  ten minutes, then a log download was cut off (`IncompleteRead`) and the archiver stopped;
  `| tee` hid the failure, so the run showed green. It also kept nothing for the first 37
  `sim-checks` runs (2026-10-14 and 15), which did everything in one job, `sim-checks`, and
  wrote empty `.log` files for runs whose digest and scaling jobs were skipped. Fixed: each
  download is retried and a failed one leaves its run for later, the step fails visibly,
  one-job runs keep their digest and scaling steps' sections, and a run with nothing to keep
  is `{"na": "not-applicable"}`. That run's branch is not to be merged.
- **Later (2026-11-07), at the owner's word:** a run with nothing to keep keeps no files,
  only its id in `metrics/raw/<series>/skipped.txt`; the 50 `not-applicable` `.json` files
  archived until then moved there. Every data PR starts a `sim-checks` run whose jobs skip,
  so they would otherwise add one empty record each.

**Step 2, 2026-11-07:**
- **The record's version is the one required field.** The check can't choose a schema
  without it; every other field is optional, as Decision 162 says.
- **"The suite runs" for a PR that also touches `metrics/schema/` or another path** is read
  as "the normal checks apply" (Decision 163: a schema change is code). `needs_godot.py`
  still decides whether Godot is needed: a schema-and-check change, being JSON and Python,
  runs no suite, while one that also touches `sim/` does.
- **`0` is refused in any measure,** whatever its unit, as the step says. A later series
  whose measure can truly be `0` (a count) needs a new schema version that says so.
- **`rust` never starts on a data-only PR** (its `paths` hold nothing under `metrics/`), so
  its data-only step is a guard only. `sim-checks`' existing `scope` job takes the rule
  (`needs_godot.py` calls `data_only.py`), so its jobs skip, which counts as passing.

**Step 4, 2026-11-07:**
- **The bridge holds:** `8ca4227` has `tools/formation_bench.gd`, `tools/tick_phases.gd` and
  `breach_tick_phases` (built by its `native/build.sh`), so it is the cargo era's reference.
- **The args are `side=4096 squads=4 fight=3`,** `bench`'s default, not `preset=4096`
  (Decision 159's daily bench): the reference's `breach_tick_phases` has no `preset`.
- **Phase names become paths** by splitting on `:`, lowercasing, dropping apostrophes and
  joining other runs of characters with `-`, under `tick`: `scrum: engage, prepare, stance` is
  `tick/scrum/engage-prepare-stance`, `rout: regroup's search` is `tick/rout/regroups-search`.
- **`breach_tick_phases` prints two decimals,** so a phase under 0.005 ms prints `0.00`. Such
  a phase is `{"na": "not-measured"}` in that record, with no ratio, never `0`.
- **Each record's machine** comes from the runner's `ImageOS` and `ImageVersion`, `lscpu`,
  `nproc`, `/proc/meminfo`, `/etc/os-release` and `rustc --version`; Godot is
  `not-applicable`. Its step label and summary come from its PR through the API, and a merge
  with no step label has `step` `not-applicable`.

**Step 5, 2026-11-07:**
- **67 commits** touched `sim/` or `native/` on `main`'s first-parent line since 2026-10-14:
  49 have only the GDScript tools, 8 both (the bridge, `8ca4227`, is the last of them) and 10
  only `breach_tick_phases`. The earliest with the binary, `827b8ea`, builds it and takes
  `squads`.
- **`formation_bench.gd`'s args never changed** in the window (`sizes`, `width`, `march`,
  `settle`, `fight`, `seed`), so no commit needs its own; `tick_phases.gd` took `engine`,
  `side` and `squads` from the start.
- **A harness that prints nothing readable** (it doesn't parse at that commit, or fails) has
  each of the reference's paths as `{"na": "not-measured"}`.
- **Record names** are `<date>-<run id>-<n>`, `n` counting harnesses across the whole list of
  commits (and 1000 more for each re-run), so the matrix's jobs never collide.

**Step 6, 2026-11-08:**
- **The archive gives 434 records:** 87 of the 96 bench runs (35 `breach_tick_phases`, 52
  `tick_phases.gd`; the other 9 timed other tools or failed) and 347 of the 348 `sim-checks`
  runs, each with `scaling/{march,fight,stacks}/{128,256,512}`-style paths. A second run writes
  nothing.
- **Every `sim-checks` run was on a PR branch,** and most bench heads were branches, so a
  harvested record's commit is the head as it was run: its `sha` (short, for bench), its
  `branch`, and for bench the `base` it was compared with, which the drawing uses to place it
  on `main`'s line. `position`, `pr` and the rest are `not-recorded`.
- **A rounds comparison's summary keeps only medians and ranges,** so its `values` are
  `not-recorded`; a single round's are its one value.
- **A run that already has a fresh record** (step 4's record runs, once `metrics-archive`
  keeps them) is left alone, so a harvest never overwrites one.

**Step 7, 2026-11-08:**
- **Placing a harvested record:** at its head's commit when that is on `main`'s line, else at
  the base it was compared with; one with neither (every `scaling` record, whose runs were all
  on PR branches) isn't drawn on the line.
- **A commit benched more than once** (the trial and the backfill) draws its latest run.
- **A line breaks** at a commit with a fresh record elsewhere but no number on it, so the
  eight `tick_phases.gd` gaps of Oct 24 and 25, and the commits timed only with the binary
  before the bridge, are never bridged.
- **Harvested points are hidden by default** (the owner's choice, 2026-11-08), behind a "Show
  harvested points" toggle: the backfill gave every merge a fresh record, and the harvested
  bench records are mostly branch comparisons with other args and no shared reference.
- **The bridge** reads 137.5 ms (`tick_phases.gd`) against 71.4 ms (`breach_tick_phases`) on the
  backfill's run, ×1.93.

**Design session, 2026-11-08 (Q40, #820), from the archive:**
- **Lanes grow linearly in squad count.** 14 harvested pairs, `preset=4096` and `preset=25k`
  on one commit and runner: 25k ÷ 4,096 from ×4.8 to ×7.4, median about ×6 (exponent in squad
  count 0.88 to 1.11). The belief that they grow quadratically does not hold.
- **One squad bends upward.** `breach_scaling` on `1156225`, one run a size: march ×2.05,
  ×2.06, ×2.19 a doubling from 3,072 to 24,576 a side; fight 33, 78, 201 and 587 ms, ×2.33,
  ×2.58, ×2.92. From 512 to 4,096 the fight grows ×2.0 to ×2.2.
- **The form:** fitted to 3,072, 6,144 and 12,288, `b·x + c·x²` predicts 584 ms at 24,576
  (−0.5%), 62% of it quadratic; `a·x^k` predicts 486 ms (−17%).
- **The runner varies:** the bridge's `side=4096 squads=4 fight=3` tick is 64 ms on an EPYC
  9V45 and 99 ms on a Xeon 8573C, so every record times its reference beside it.

**Q43, 2026-11-11 (the runner, in the records on `main` at `902fe4d`):**
- **One commit on two CPUs:** `8ca4227` (the reference itself) was timed on an EPYC 7763 and
  a Xeon 8573C in three benches. `tick_phases.gd` `engine=rust side=4096`: 156.4 and 137.5 ms,
  ratio 0.576 and 0.617. `breach_tick_phases` `side=4096 squads=4 fight=3`: 94.4 and 71.4 ms,
  ratio 1.004 and 0.971. `formation_bench.gd` `sizes=1024`: 21.1 and 15.9 ms, ratio 0.031 and
  0.034. In ms they differ by 13–33%, as ratios by 3–11%.
- **The 60% rise:** `bfce23c` 26.8 ms over a 66.0 ms reference (EPYC 9V45), ratio 0.406;
  `5af0ed5` 43.0 ms over 99.9 ms (Xeon 8573C), ratio 0.430: 6% apart as ratios.
- **The same-runner A/B** (`bench` runs 102 and 103, started 2026-11-11 00:50): pending; its
  `SUMMARY` goes here.

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

### Step 8a
- Given a merge to `main` touching `native/`, when `bench` runs, then `record-25k` starts
  after `record` and opens one data PR with nine records, the two checks each with the
  bridge's.
- Given a check whose prediction misses by 12%, then the data PR's summary flags it; given
  the merge before also flagged on that shape, then it says the form no longer fits.

### Step 8b
- Given a Rust-era commit whose harness rejects `series=1`, when the 25k backfill runs, then
  it writes no record for that shape and the job still succeeds.

### Step 8c
- Given grid records that lie exactly on `b·x + c·x²`, then the fit returns `b` and `c` and
  the predicted 24,576 within rounding.
- Given grid records that would need a negative `c`, then the fit returns `c = 0`.
- Given a commit with no 25k set, then the 25k chart breaks its lines there.

### Step 8d
- Given lanes fit `b` and `c`, then the "on track" 4-lane tick predicts 50 ms at 24 lanes.
- Given a commit with no 25k set, then the 4,096 chart's "on track" lines break there.

### Step 9
- Given a record with tick 43.0 ms and a reference tick of 99.9 ms, then its value in
  × the reference is 0.430, and a phase of 10 ms in it is 0.100.
- Given a record whose phases are 10, 20 and 70 ms of a 100 ms tick, when the 20 ms phase is
  hidden in % of tick, then the others stay 10% and 70%.
- Given records from two CPUs, when one CPU is hidden, then its records leave both charts and
  the compare's choices, and the whole-tick line joins the others.
- Given a record with no reference, then it is absent in × the reference and counted in the
  note, and present in ms.

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
8. Step 8a: red on the record writer's output for a fixture `series=1` output (`tick`,
   `tick-p95`, `tick-p99` and their rounds), and on the miss and flag from fixture records,
   then the job, checked by a run.
9. Step 8b: red on the commit list and shape skipping for `set=25k` (fixture trees), then the
   workflow; then the dispatch.
10. Step 8c: red on the fit, the non-negative clamp and the 25k lines' breaks (JS logic,
   tested as step 7's is), then the chart. Step 8d: red on the derived 4-lane line and its
   breaks, then the toggles, the lines and the panel.
11. Step 9: red on the unit conversions (× the reference, % of tick with phases hidden) and
   the CPU filter (JS logic in `metrics_view.js`, tested as step 7's is), then the page.

## Rubric answers

- **single-noun-phrase:** the raw archive; the record schema and its check; the data-PR
  route; bench's records; the backfill; the harvest importer; the drawing; the 25k set; its
  backfill; the 25k chart; the chart by the reference.
- **ocp-extension-point:** a new series is a folder under `metrics/` and new paths; a new
  harness is a `source` value; a new schema version is a file under `metrics/schema/`.
- **lsp-contract-scope:** not applicable; no subtypes.
- **isp-fit:** the drawing reads only records; the importer only `metrics/raw/`.
- **dip-direction:** the workflows depend on the data-PR action, never on the token or the
  merge details.
- **general-over-special:** general: one record shape and gap rule for every series; one
  model form and tolerance for every shape (Decision 165). The one
  special case is Decision 164's self-merge, with the owner's reason.
- **order-independent:** not applicable; no simulation outcome changes.
- **bounded-work:** not applicable to the tick; two bench jobs per qualifying merge (`record`
  and `record-25k`).

## Built

### Step 8c, the 25k chart: the fit and the measured lines

**Split at build:** from step 8c, PR #872, 2026-11-11: step 8c held about eight features
across two charts, over one PR's size (Decision 171); 8c builds the fit and the 25k chart's
core, 8d the toggles, the 4,096 chart's lines and the detail panel.
**Needs:** 8a, 8b
**Touches:** `tools/metrics_view/`

On the tick history page (`tools/metrics_view/`), below the 4,096 chart. Sizes on the page
are a side, mirrored: one note says so ("4,096 is 4,096 against 4,096"), and labels after it
give the size alone unless it could be misread.
- **The 25k chart:** 24,576 (49,152 units), absolute ms on a log scale, along `main`'s
  first-parent commits:
  - two measured series, 24 lanes of 1,024 and one squad of 24,576: p95 as the line, p99 as a
    thin band above it;
  - each shape's projection, dotted, in its colour, labelled "estimate: b·x + c·x² from the
    grid";
  - a merge whose miss is beyond 10% is ringed;
  - Decision 159's lines: 50 ms "target (p95, p99)" and 100 ms "interim (p95, p99)";
  - a point's tooltip: the runner's CPU, the miss, and the quadratic share `c·x²/t`;
  - a line breaks at a commit on `main`'s line with no 25k set.
- **The fit:** per commit and shape, non-negative least squares of `b·x + c·x²` on the grid's
  `tick` (mean) medians; the miss against the measured 24,576 `tick`. The form is named on
  the chart with "Decision 165". The fit and the miss use every record of the commit.

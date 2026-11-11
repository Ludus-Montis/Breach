---
spec_type: code
status: active
parent_spec: ../Breach — Reverse Tower Defense Design Spec.md
---

# Spec 48: The owner's PC as a local bench series, drawn apart from CI's

## Purpose

The owner's grid of squad size against squad count (`native/README.md`, Results, 2026-11-11)
found that three phases cost the square of a squad's size: `units walk to places`,
`scrum: engage, prepare, stance` and `scrum: seek`. CI's benches can't show it, because the 25k
preset splits its units into 24 squads of 1,024. The owner asked for such runs on the metrics
page, with the CPU's details, a curve for each grouping, Decision 159's lines and per-phase
detail.

**Settled by Decision 178:**
- local runs are a series of their own, under `metrics/local/`, drawn apart from CI's;
- they come from one machine, the owner's PC, and compare in raw ms;
- every round is kept;
- they guide day to day only once they track CI's progression.

**It leans on spec 47** (Decision 177) for how a local run is timed, and builds nothing of its
own for that. Spec 47's repeat mode (`at=`, `repeat=`) replays chosen ticks live many times,
and its series runs until the field settles and names its inspection ticks (`inspect:`).
Locally, both give tighter numbers per run than `fight=10`, on ticks that CI's
`bench-inspect` times too. So a local run and a CI run can be set side by side tick for tick.

## Status

Agreed in the design session of 2026-11-11. Five steps, each its own PR, built with
`/run-phase` as "spec 48 step N".
- Steps 1 and 3 can be built side by side.
- Step 2 needs step 1's records.
- Step 4 needs step 1, and the owner's PC to run it.
- Step 5 needs step 1, and spec 47 steps 1 and 2 merged (the repeat mode and `inspect:`). Its
  `Needs` line can name only this spec's steps, so the spec 47 dependency is stated here and in
  the step.

## Pre-mortem

- **Regret in a month:** local runs pile up from different commits and settings, and get read
  as if they were CI's, or the PC drifts (an update, a background job) and a run looks like a
  regression that isn't one.
- **Riskiest assumption:** that the PC's progression tracks CI's: a change that speeds CI's
  tick by some percent speeds the PC's by about the same. The owner hopes local runs "might
  speed up the bench flow"; that holds only if they do.
- **Cheapest test:** step 4. Run commit pairs whose CI same-runner result is known on the PC,
  and compare the changes. Its spread is known already: the grid's 3 rounds are mostly within
  ±5%, and up to ±15% in the noisiest cell.

## The machine

12th Gen Intel Core i7-12700K @ 3.60 GHz, 12 cores (8 performance, 4 efficiency) and 20
threads, 32 GB, Windows. As the owner gave it on 2026-11-11: `breach_tick_phases` prints
`machine: unknown, 20 threads, Windows` there.

## Steps

### Step 1, the local record, its importer, and the first grid

**Needs:** none
**Touches:** `metrics/schema/record-v1.schema.json`, `ci/metrics/local_record.py` (new),
`ci/metrics/test_local_record.py` (new), `ci/metrics/fixtures/local/`,
`ci/metrics/local_machine.json` (new), `ci/metrics/local_grid.sh` (new),
`ci/metrics/README.md`, `metrics/local/` (the first records), `metrics/raw/local/` (the
first outputs)

- **The record.** Decision 162's shape, with:
  - `series` `"local"`;
  - `provenance.kind` `"local"` (the schema's enum gains it), `date`, and `run_id` as a gap;
  - `machine` from `ci/metrics/local_machine.json` (the owner's PC, as above);
  - `commit` the sha the run built;
  - `source.args` the battle's arguments;
  - `reference` a gap (`not-measured`);
  - `results` every phase path as `bench_record.py` names them (`phase_path`), each with its
    rounds in `values` and their median.
- **One record per battle (cell)**, its rounds gathered from the files for that cell.
- **The importer:** `python3 ci/metrics/local_record.py <folder> --commit <sha> --date <date>`.
  It reads `breach_tick_phases` outputs named `s<side>_q<squads>_r<round>.txt`. It reuses
  `bench_record.py`'s parsing, and checks each file's `side`, `squads` and thread count
  against its name and the machine file.
- **The grid script:** `bash ci/metrics/local_grid.sh [rounds]` runs the grid the owner ran
  (sides 3,072 to 24,576, squads 1, 2, 4, 8 and 24, `fight=10`, rounds in turn) into
  `bench-grid/` (git-ignored), then the importer.
- **The first data:**
  - the owner's 2026-11-11 run on `a5f574d`, its 60 outputs, kept under
    `metrics/raw/local/2026-11-11-a5f574d/`;
  - 20 records under `metrics/local/`.

  `check_records.py` checks them as it checks any record.

Scenarios:
- Given the fixture outputs for one cell (3 rounds), when imported, then the record holds
  each phase's 3 values and their median, and passes `check_records.py`.
- Given a file whose printed `side` or `squads` differs from its name, then the importer
  refuses it and names the file.
- Given a local record, then CI's charts don't draw it (their filters read `series`).

### Step 2, the page's local section

**Needs:** 1
**Touches:** `tools/metrics_view/metrics_local.js` (new), `tools/metrics_view/metrics_local_page.js`
(new), `tools/metrics_view/test/metrics_local.test.js` (new), `tools/metrics_view/index.html`,
`tools/metrics_view/metrics_view.css`, `tools/metrics_view/README.md`

A section below CI's, headed by the machine (CPU, cores and threads, RAM, OS) and the run's
commit and date:
- **Curves, two groupings:** the tick against units a side, one curve per squad count; or
  against squad size, one curve per total. Both are on log scales, each point the median, with
  a bar for its rounds' range.
- **A phase picker:** the tick, or any phase the records hold. The default shows the tick with
  the three costliest phases at the largest cell as thin curves.
- **Decision 159's lines:** 50 ms "target" and 100 ms "interim" at 24,576 a side, labelled "on
  CI's 4-thread runner; a guide here".
- **Runs over time:** a run picker (by date and commit) when there are several. The default is
  the newest, with the one before it dotted.
- **CI's charts are unchanged,** and never draw a local record.

Scenarios:
- Given the 20 records of 2026-11-11, when drawn by squad count, then there are 5 curves of 4
  points, and the 1-squad curve's tick at 24,576 is 429.7 ms.
- Given the phase picker set to `units walk to places`, when drawn by squad size, then the 4
  totals' curves lie on one line of slope about 2 (per squad).
- Given CI's records and local ones, then the 4,096 and 25k charts draw the same points as
  without the local ones.

### Step 3, `breach_tick_phases` names the CPU on Windows

**Needs:** none
**Touches:** the crate's machine line (`native/rust/src/phases.rs` or where `machine:` is
printed) and its tests, `native/README.md` (Tick phases)

- **On Windows,** the `machine:` line names the CPU from the OS (the registry's
  `ProcessorNameString`, or the CIM `Win32_Processor` name), its cores and threads, and RAM,
  as it does on Linux. If the OS doesn't answer, it says `unknown` as now.
- **The importer** prefers the printed machine over `local_machine.json` when it's known.

Scenarios:
- Given a run on Windows, then `machine:` names the CPU, not `unknown`.
- Given a CPU name the OS can't give, then the line says `unknown` and the run goes on.

### Step 4, does the PC track CI?

**Needs:** 1
**Touches:** this spec's `## Measured` section, `metrics/local/` and `metrics/raw/local/` (the
runs), `ci/metrics/local_grid.sh` (a pair mode, if needed)

On the owner's PC, by hand, rounds in turn (5 or more):
- **How each pair is timed:** with spec 47's repeat mode at the base's `inspect:` ticks once
  it has merged (step 5's grid script), and `fight=10` until then.
- **The pairs**, each with a CI same-runner result (spec 47 step 3 times #780, #783 and #773
  on CI the same way, so one set of CI numbers serves both checks):
  - `bfce23c` against `main` at `5ea4c29` (CI: 25k tick −8%, `bodies: pair search` −36%,
    `strays` +63%);
  - `5af0ed5` against `bfce23c` at 4,096 (CI: +1.5%, no change);
  - `482f664` against `2aedfac` at 25k (spec 41 step 4 alone, on CI).
- **Passing:** the tick and every phase CI moved beyond noise move the same way on the PC,
  with changes within the two machines' ranges. Phases CI left unchanged stay unchanged.
- **The verdict:** it goes in `## Measured` and on the PR. If it passes, `/run-phase` and the
  benches' docs say a local grid may guide day-to-day work, with CI's `bench` confirming
  before a merge. If it fails, the owner decides what the local series is for.

### Step 5, local runs in spec 47's repeat mode

**Needs:** 1
**Touches:** `ci/metrics/local_record.py` and its tests, `ci/metrics/fixtures/local/`,
`ci/metrics/local_grid.sh`, `ci/metrics/README.md`

Starts once spec 47 steps 1 and 2 have merged (the repeat mode and `inspect:`).

- **The importer reads the repeat mode's output:** a `repeat:` line naming the tick and `n`.
  The record's `source.harness` is `tick_phases-repeat`, as spec 47 step 5 names CI's, and
  its args carry `at` and `repeat`. Each repeat's lap is a value, so its spread is kept. One
  record per (battle, tick).
- **The settled series too:** the record holds its p95 and p99 over the whole series, and the
  `inspect:` ticks it named.
- **The grid script's modes:**
  - `bash ci/metrics/local_grid.sh grid [rounds]` runs the grid as now, with each cell's
    settled series;
  - `... inspect [rounds]` replays each cell's `inspect:` ticks with `repeat=50`;
  - `... pair <head> <base> [rounds]` builds both commits and times them in turn at the base's
    `inspect:` ticks, as CI's `bench-inspect` does on one runner. Step 4 uses this.
- **The page (step 2)** shows a repeat record's ticks as points at their tick, beside the
  cell's series.

Scenarios:
- Given fixture output from the repeat mode at 2 ticks with `n` 50, when imported, then there
  are 2 records, each with 50 values per phase and the median, and both pass
  `check_records.py`.
- Given a pair run, then the script prints, per tick, the head's change against the base for
  the tick and each phase, flagged where the ranges are apart, as `bench`'s summary does.

## Measured

Filled in by step 4.

## Not in this spec

- **Other machines** (Decision 178, Alternatives), unless diagnosing a user's.
- **The work the grid points at** (one squad's quadratic work in `march::sync`, engage and the
  seek): the next design round.

## Test-first order

1. Step 1: red on the importer's record from fixture outputs, and on the refusal of a
   mismatched file (`test_local_record.py`, alone). Then the importer, then the first data,
   checked by `check_records.py`.
2. Step 2: red on the curves' points from fixture records (`metrics_local.test.js`, alone).
   Then the page.
3. Step 3: red on the machine line's parsing of a Windows answer (a cargo test with a canned
   answer). Then the lookup.
4. Step 4: no code; a measured verdict.
5. Step 5: red on importing the repeat mode's fixture output (`test_local_record.py`, alone).
   Then the importer's repeat records, then the script's modes.

## Rubric answers

- **single-noun-phrase:** the local record and its importer; the local section; the machine
  line on Windows; the PC-against-CI check; local runs in the repeat mode.
- **ocp-extension-point:** a new grid shape is a list in `local_grid.sh`; a new chart grouping
  is a function in `metrics_local.js`.
- **lsp-contract-scope:** not applicable; no subtypes.
- **isp-fit:** the importer reads only `breach_tick_phases`' printed lines; the page reads only
  records whose `series` is `local`.
- **dip-direction:** the page depends on the record's shape, never on the binary's output.
- **general-over-special:** general: every local run is the same kind of record, drawn by one
  rule, apart from CI's (Decision 178).
- **pre-mortem:** the riskiest assumption is tested in step 4, before local runs guide
  anything.
- **order-independent:** not applicable; no simulation outcome changes.
- **bounded-work:** not applicable to the tick.

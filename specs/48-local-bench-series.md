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
- Step 4 needs step 1, and the owner's PC to run it. Done 2026-11-12: a local pair is a
  direction guide, not a judge (`## Measured`).
- Steps 5d to 5f (added 2026-11-12) keep the repeats' spread: 5d the record's quartiles, then 5e
  (the page) and 5f (every lap in the raw output) side by side.
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
  **Found at build (2026-11-11):** `breach_tick_phases` prints `<side> a side, <width> wide`
  and no squad count, so the importer checks the side and the thread count, and the squads
  come from the name alone. Printing the squads is a change to the binary, left out here.
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

**Found at build (2026-11-11):**
- The scenario's "slope about 2 (per squad)" is a toggle, **per squad**, which divides each
  value by the cell's squad count. Without it, `units walk to places` by squad size has a
  slope of about 1.2 and the totals sit apart; with it, each total's slope is 1.8 to 1.9 and
  they lie on one line.
- The records' `machine` (from `ci/metrics/local_machine.json`) has no thread count: the
  file's `threads` sits beside it, for the importer's check. So the heading names cores, not
  threads. Putting the threads in the record is a schema change, left out here.
- "The largest cell" is the most units a side, and of those the costliest tick (24,576 in one
  squad). Its three costliest phases are the three quadratic ones: `scrum: seek`, `units
  walk to places` and `scrum: engage, prepare, stance`.

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

### Step 5b, the grid script's modes

**Split at build:** from step 5, PR #936, 2026-11-11: three behaviours with their own tests (Python; shell; JS), and the page is step 2's (PR #933), outside step 5's `Touches`. Q47 (#909) was answered (a): the repeats stay a guide, with their caveat.
**Needs:** 5a
**Touches:** `ci/metrics/local_grid.sh`, `ci/metrics/README.md`, and a test of the pair
summary (`ci/metrics/test_local_pair.py`, new, or `bench_compare.py`'s if it is reused)

- **The grid script's modes:**
  - `bash ci/metrics/local_grid.sh grid [rounds]` runs the grid as now, with each cell's
    settled series;
  - `... inspect [rounds]` replays each cell's `inspect:` ticks with `repeat=50`;
  - `... pair <head> <base> [rounds]` builds both commits and times them in turn at the base's
    `inspect:` ticks, as CI's `bench-inspect` does on one runner. Step 4 uses this.

Scenarios:
- Given a pair run, then the script prints, per tick, the head's change against the base for
  the tick and each phase, flagged where the ranges are apart, as `bench`'s summary does.

**Found at build (2026-11-11):** the step didn't say which battle `pair` times, nor which
round's `inspect:` ticks `inspect` replays. Built as: `pair` times CI's sizes (`preset=4096`
and `preset=25k`, `PAIR_SIZES` overrides), one tick a run since the repeat mode's table reads
the last tick of a run, and prints through `ci/metrics/local_pair.py` (reusing
`bench_compare.py`'s table), with each side's settled series run once for its p95 and p99.
`inspect` replays round 1's ticks and imports from `bench-grid/inspect/` alone, so the grid's
records aren't written twice. `pair` records nothing. Outside `Touches`: `.gitignore`
(`bench-pair/`) and `ci/metrics/local_pair.py` (new).

### Step 5d, a repeat record keeps each round's quartiles

**Added:** 2026-11-12, the owner's choice on #936's follow-up: keep the laps' spread without
keeping the laps in the record (each lap a value would make a repeat record about 50 times
larger, a few MB a grid under `metrics/local/`).
**Needs:** 5a
**Touches:** `metrics/schema/record-v1.schema.json`, `ci/metrics/check_records.py` and its
tests, `ci/metrics/local_record.py` and its tests, `ci/metrics/fixtures/local/`,
`ci/metrics/README.md`

The repeat mode already prints each phase's median, IQR and 95% interval of the median for
each round, so this needs no change to `native/`.
- **The schema:** a measure gains an optional `quartiles`, one `[q1, q3]` pair per entry of
  `values`, in the same order. Optional, so every record already written stays valid.
- **`check_records.py`:** where `quartiles` is given, it has as many pairs as `values`, and each
  pair holds its value: `q1 <= value <= q3`.
- **The importer:** a repeat record's measure keeps each round's IQR as that round's pair,
  beside the round's median in `values`. Settled-series and grid records carry none.

Scenarios:
- Given fixture output from the repeat mode, 2 rounds, when imported, then each phase's
  measure has 2 values and 2 quartile pairs, each pair holding its value, and the record
  passes `check_records.py`.
- Given a record whose `quartiles` has fewer pairs than `values`, or a pair that doesn't hold
  its value, then `check_records.py` fails it and names the measure.
- Given a record without `quartiles`, then `check_records.py` passes it as before.

**Found at build (2026-11-12):** the repeat mode prints each phase's IQR as a width,
`q3 - q1` (`phases/repeat.rs`'s `lines`), not `q1` and `q3`, so the importer can't read a
round's pair from the output as this step assumed. The schema and `check_records.py` are built
(PR on `feature/repeat-record-quartiles`); the importer waits on the owner's choice, raised as a
`question` issue: print `q1` and `q3` from `native/`, or something else.

### Step 5e, the page draws a repeat's quartiles

**Needs:** 5d
**Touches:** `tools/metrics_view/metrics_local.js` and its test,
`tools/metrics_view/metrics_local_page.js`, `tools/metrics_view/README.md`

- **A repeat point's bar** (step 5c) spans its rounds' quartiles, the lowest `q1` to the
  highest `q3`, where the record has them, and the range of its rounds' medians as now where it
  doesn't. The hover text says which.

Scenarios:
- Given a repeat record with quartiles, then its bar runs from the lowest `q1` to the highest
  `q3` of its rounds.
- Given a repeat record without them, then its bar is its rounds' range, as before.

### Step 5f, the raw output keeps every lap

**Needs:** 5d
**Touches:** `native/rust/src/phases/repeat.rs` and its tests, `native/rust/src/bin/tick_phases.rs`,
`native/README.md` (Tick phases), `ci/metrics/local_grid.sh`, `ci/metrics/README.md`

- **`breach_tick_phases ... at=<t> repeat=<n> laps=1`** prints, after a repeated tick's table,
  a `laps:` line and then one line per phase (the tick first) with its `n` laps in ms, in the
  order they ran. The printing comes after the timing, so the times don't change. Without
  `laps=1` the output is as now, so nothing that reads it changes.
- **`local_grid.sh inspect` and `pair`** pass `laps=1`, so the raw files kept under
  `metrics/raw/local/` hold every lap. The importer skips the `laps:` block: records keep the
  medians and quartiles (step 5d), not the laps.

Scenarios:
- Given `repeat=5 laps=1` at one tick, then the output has a `laps:` block with 5 values for
  the tick and for each phase, and the table above it is as without `laps=1`.
- Given output with a `laps:` block, when imported, then the record is the same as without it.

## Measured

### Step 4, does the PC track CI? (2026-11-12)

**How it ran.** Every pair's commits predate spec 47's repeat mode, so `local_grid.sh pair`
couldn't time them (the base prints no `inspect:` ticks). As the step allows, each pair was
timed with the whole tick, `breach_tick_phases preset=<size> fight=10`, head and base in turn
for 5 rounds, each commit built in its own worktree, on the owner's PC (i7-12700K, 20 threads,
Windows). The summaries are `bench_compare.py`'s; the outputs are under
`metrics/raw/local/2026-11-12-pairs/`. A fourth pair, #780 alone, was added once the first
three pointed at #783's allocator. CI's numbers are spec 47 step 3's (same runner, 5 rounds),
except the `bfce23c` to `5ea4c29` line, which is from this spec's step list.

Change from base to head, medians over 5 rounds; `*` the ranges apart:

| Pair | Size | Phase | CI | PC |
|---|---|---|---|---|
| `5af0ed5` vs `bfce23c` (#779, no change) | 4,096 | tick | +1.5%, no change | +0.0% |
| | | every phase | none beyond noise | none beyond noise |
| `2aedfac` vs `214ca57` (#780, the pair search) | 25k | tick | −9.2%* | −2.4%* |
| | | `bodies: pair search` | −40.8%* | −22.8%* |
| | | the rest | not reported | none beyond noise, but `bodies: sums and applying` +5.3%* |
| `482f664` vs `2aedfac` (#783, mimalloc, foldhash, kept buffers) | 25k | tick | −3.9%* | −21.5%* |
| | | `wounds (tend)` | −21.6%* | −42.4%* |
| | | `store settles` | −44.0%* | −53.2%* |
| | | `scrum: engage, prepare, stance` | −13.3%* | −17.8%* |
| | | `shuffle (re-form)` | −15.1%* | −29.7%* |
| | | `bodies: pushes` | −4.7%* | −52.0%* |
| | | `strays` | +24.1%* | −25.6%* (opposite) |
| | | `bodies: sums and applying` | +15.9%* | −11.1%* (opposite) |
| | | `stamina` | −15.6% | +9.5%* |
| | | `roster`, `bodies: pair search`, `fight`, `rout: rallies`, `bodies: sync` | not reported | −19 to −28%* |
| `bfce23c` to `5ea4c29` (#780, #783 and others) | 25k | tick | −8% | −24.1%* |
| | | `bodies: pair search` | −36% | −44.5%* |
| | | `strays` | +63% | −30.4%* (opposite) |
| | | `stamina` | +28% | +13.1%* |
| | | `bodies: sums and applying` | +19% | −10.2%* (opposite) |

**Verdict: fails on size, so the owner decided (2026-11-12): a local pair is a direction
guide, not a judge,** on the same terms as Q47's answer for the repeat mode.
- **Direction tracks for an ordinary change.** The no-change pair stays flat, and #780 moves
  only the phase it aims at, the way CI does.
- **Size doesn't track, either way:** about 5 times CI's change across #783, about a quarter
  of it on #780's tick. The machines differ (20 threads on Windows against CI's 4-thread Xeon
  on Linux), so a local percentage is never CI's.
- **A change to how memory is allocated diverges even in direction.** Across #783, `strays`
  and `bodies: sums and applying` move against CI's: mimalloc replaced Windows' allocator,
  a far larger gain there than over glibc's on CI, and it outweighs the kept buffers' cost.
  Every commit since #783 uses mimalloc on both, but a change to allocation (the allocator,
  kept buffers, pooling) still goes straight to CI.

So: a local pair may show whether a change speeds up the phase it aims at and leaves the
rest alone. Its percentages are never quoted as a change's size, and `bench` on CI judges
before a merge (`ci/metrics/README.md`, Local runs).

**Found at build:** the step list's first pair reads "`bfce23c` against `main` at `5ea4c29`",
but CI's numbers are for the change from `bfce23c` to `5ea4c29`: `bfce23c` is the older, 60
commits back. Its folder keeps the order it ran in (`bfce23c` as head); the table above reads
it the right way round.

## Not in this spec

- **Other machines** (Decision 178, Alternatives), unless diagnosing a user's.
- **Quartiles in CI's repeat records** (`bench-inspect`, spec 47): step 5d's schema field is
  optional, so CI's records could take it later; this spec changes only the local importer.
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
5. Step 5a: red on importing the repeat mode's fixture output (`test_local_record.py`, alone).
   Step 5b: red on the pair summary. Step 5c: red on the repeats' points
   (`metrics_local.test.js`, alone).
   Then the importer's repeat records, then the script's modes.
6. Step 5d: red on a fixture's quartile pairs (`test_local_record.py`) and on a record whose
   pairs don't match its values (`test_check_records.py`). Then the schema and the importer.
   Step 5e: red on a repeat bar from quartiles (`metrics_local.test.js`, alone). Step 5f: red
   on the `laps:` block (a cargo test in `phases/repeat`), then on importing output with it.

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

## Built

### Step 5a, the importer reads the repeat mode and the settled series

**Split at build:** from step 5, PR #936, 2026-11-11: three behaviours with their own tests (Python; shell; JS), and the page is step 2's (PR #933), outside step 5's `Touches`. Q47 (#909) was answered (a): the repeats stay a guide, with their caveat.
**Needs:** 1
**Touches:** `ci/metrics/local_record.py` and its tests, `ci/metrics/fixtures/local/`,
`ci/metrics/README.md`

- **The repeat mode's output:** a `repeat:` line naming the tick and `n`. The record's
  `source.harness` is `tick_phases-repeat`, as spec 47 step 5 names CI's, and its args carry
  `at` and `repeat`. One record per (battle, tick).
- **The settled series too:** the record holds its p95 and p99 over the whole series, and the
  `inspect:` ticks it named.
- **Step 3's rest:** the importer prefers the printed machine over `local_machine.json` when
  it names the CPU.

**Found at build (2026-11-11):** the repeat mode prints each phase's median, IQR and 95%
interval of the median, not each lap, so a lap can't be a value. A record's values are its
rounds' medians (rounds of `repeat=<n>`, run in turn), and the laps' spread stays in the
output. Keeping every lap would need the binary to print them, a change to `native/` left out
here. Built as: repeat files are named `s<side>_q<squads>_r<round>_repeat.txt`; a settled
cell's args end `series=1 series=settled`, its p95 and p99 are `tick-p95` and `tick-p99`,
and `source.inspect` holds each round's `inspect:` ticks. The fixtures were made on a cloud
runner, their machine line set to the PC's.

Scenarios:
- Given fixture output from the repeat mode at 2 ticks with `n` 50, 2 rounds, when imported,
  then there are 2 records, each with a value per round per phase and the median, and both
  pass `check_records.py`.
- Given a cell run with `series=1`, then its record holds `tick-p95`, `tick-p99` and each
  round's `inspect:` ticks.
- Given a machine line that names the CPU, then the record's machine is the printed one.

### Step 5c, the page shows the repeats

**Split at build:** from step 5, PR #936, 2026-11-11: three behaviours with their own tests (Python; shell; JS), and the page is step 2's (PR #933), outside step 5's `Touches`. Q47 (#909) was answered (a): the repeats stay a guide, with their caveat.
**Needs:** 2, 5a
**Touches:** `tools/metrics_view/metrics_local.js` and its test, `tools/metrics_view/README.md`

- **The page (step 2)** shows a repeat record's ticks as points at their tick, beside the
  cell's series, with Q47's caveat beside them: the repeats under-report phases whose cost
  builds up across ticks; the whole tick is the judge.

Scenarios:
- Given a cell's settled record and its repeat records, then the repeats are points at their
  ticks beside the series.

**Found at build (2026-11-12):** "at their ticks" can't be a place on the page's axes, which
are units a side or squad size. So a repeat is a hollow point at its cell's x, set just left of
the cell's point, one step per tick, with its rounds' bar; the tick and the repeat count are in
its hover text. Repeat records are kept apart from the run's cells (`run.repeats`), so the
series keeps one point per cell. Q47's caveat is in the legend whenever repeats are drawn.

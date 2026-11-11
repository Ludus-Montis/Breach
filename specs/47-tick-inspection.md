---
spec_type: code
status: active
parent_spec: ../Breach — Reverse Tower Defense Design Spec.md
---

# Spec 47: Inspecting a tick by replaying it live, beside a series that runs until the field settles

## Purpose

The whole-tick `bench` times 16 ticks a round, from contact until one side breaks. Its 95th
and 99th percentiles are both just its worst tick, and a change aimed at one phase waits for
whole runs to separate it from noise. The owner asked (Q44, #866) for a faster guide that
still represents a phase under a full tick's pressure, with the whole tick staying the judge.

**Settled by Decision 177**, which amends Decision 159 (the series its percentiles are taken
over):
- `breach_tick_phases` replays chosen ticks live, many times;
- the series runs until the field settles;
- each inspection tick runs as its own job, head and base on one runner;
- nothing is recorded or plotted until a ranking check against past merges passes.

## Status

Agreed in the design session of 2026-11-11 (the owner: "Go for a, a self hosted runner could
become an extension question"). Five steps, each its own PR, built with `/run-phase` as
"spec 47 step N".
- Steps 1 and 2 can be built side by side.
- Step 3 is the riskiest assumption's test. Step 5 waits for it.
- Step 4 needs 1 and 2.
- A fixed machine for every job is Q45 (#877), outside this spec.

## Pre-mortem

- **Regret in a month:** we build the repeat mode, iterate on the seek with it, and its
  gains shrink or vanish in the whole tick. We'd have spent a spec on a guide we can't trust.
- **Riskiest assumption:** that a tick replayed from a saved state times its phases as the
  same tick does inside a running battle, for the changes we actually make. Cache, allocator
  and kept-buffer warmth are what could differ. On 2026-11-11, `strays` (+63%) and `stamina`
  (+28%) moved in the whole tick between `bfce23c` and `main`, with their own code unchanged.
- **Cheapest test:** step 3. Replay the merges of three PRs whose whole-tick effect is known,
  and check the mode ranks them as the whole-tick bench did. It runs before anything is
  recorded or plotted (step 5). If it fails, the owner decides whether the mode stays a
  guide with a stated caveat, or is dropped.

## The numbers this starts from

From 2026-11-11 (`bench` runs #102 and #103; detail on #866):

| | 4,096 | 25k |
|---|---|---|
| Tick, mean | ~33 ms | 174.4 ms (`main`, Xeon 6973P-C) |
| Tick, p95 = p99 = worst of 16 | ~42 ms | 193.9 ms |
| `scrum: seek` | ~11 ms | ~53 ms |
| `bodies: pair search` | ~5 ms | ~21 ms |
| Series | ticks 91–106, one side breaks at 107 | the same shape |

Spikes are in the rout: `rout: flight` and `rout: rallies` average 0.8 ms at 4,096 but reach
4.5–4.7 ms at tick 105.

## Steps

### Step 1, the repeat mode

**Needs:** none
**Touches:** `native/rust/src/phases/repeat.rs` (new) and its tests,
`native/rust/src/phases.rs`, `native/rust/src/bin/tick_phases.rs`,
`native/rust/src/field.rs` (the put-back), `native/README.md` (Tick phases)

`breach_tick_phases ... at=<t>[,<t>...] repeat=<n>`, with any preset or battle arguments:
- **Play to each tick once.** The battle plays to the state just before tick `t`. A copy of
  everything a tick reads is kept in memory: the field, the squads, the play's own state and
  the field's caches that aren't state (`reforms`).
- **Step tick `t` `n` times.** Before each repeat, the copy is put back into the *running*
  field and play. The field's kept `buffers` aren't replaced, so they and the allocator stay
  warm, as in a real battle (the owner: "as live as can be"). The put-back is untimed.
  - One untimed tick is stepped first, so the first timed repeat isn't the cold one.
- **Every lap is timed, every repeat.** For the tick and each phase, it prints the median,
  the interquartile range and a 95% interval of the median, on lines `bench_compare.py`
  reads as it reads a run's. A `repeat:` line names the tick and `n`.
- **Ticks in the order given.** It plays forward once, keeping each copy as it passes. An
  `at` past the battle's end is refused, naming the last tick.
- **`n` defaults to 50.** Tune it from the first real runs: the target is a 95% interval of
  the tick's median within ±1%.

Scenarios:
- Given a battle and a tick `t`, when tick `t` is repeated `n` times, then every repeat
  leaves the same state (the cuts' print, `cuts::print`, checked after the first and the
  last), and it equals the state the battle reaches by playing through `t` once.
- Given the put-back, when it runs, then the field's kept buffers keep their capacity (as
  `Buffers::warm` says), and the result is the same as a field read back from a snapshot.
- Given `at=` past the series' end, then the run is refused with the last tick named.

### Step 2, the series runs until the field settles, and names its inspection ticks

**Needs:** none
**Touches:** `native/rust/src/phases/series.rs` and its tests,
`native/rust/src/phases/presets.rs` and its tests, `ci/godot/scripts/bench_scaling.py` (if its
expectations read the series' length), `native/README.md` (Tick phases)

- **The end.** A series ends when neither side has a squad fighting or routing (the field
  has settled), or at `LIMIT` (600) ticks. Before, it ended when either side had no squad
  that wasn't routing.
- **`inspect:`.** The series prints a line naming its inspection ticks (Decision 177):
  - its 3 slowest ticks, a tie going to the earlier tick;
  - 4 ticks evenly through it: its first tick, the ticks a third and two thirds of the way,
    and its last.

  A tick picked twice is listed once, and the next slowest takes its place, so there are
  always 7 if the series has 7 ticks.
- **Percentiles.** Its p95 and p99 are taken over the whole series, as now, which is longer.
  Decision 159's target is read from them.
- **The tick history.** Records before and after this step measure different series. The
  record's `source.args` say so (`series=settled` after this step; none before), and the tick history's
  25k chart breaks its line there, as it does at a harness change (spec 43).

Scenarios:
- Given `preset=4096`, when the series runs, then it continues past the tick where one side
  breaks (107) until no squad fights or routes, and its ticks include the rout's spikes.
- Given a series of known tick times (a fixture), when `inspect:` is chosen, then the
  ticks are the 3 slowest (the earlier on a tie) and the 4 spread. A duplicate is replaced by
  the next slowest.
- Given the digests, then they are unchanged: the series only times.

### Step 3, the ranking check (the riskiest assumption)

**Needs:** 1
**Touches:** this spec's `## Measured` section (the result), `native/README.md` (how to read
the mode)

Run by hand through `bench` (`args` with `at=` and `repeat=`, 5 rounds, head and base on one
runner), at 4,096 and 25k. The ticks are those step 2's `inspect:` names on the base, or, if
step 2 isn't merged, contact (91), the rout's spike (105) and the last of the old series (106).
- **The cases:**
  - #780: `bodies: pair search` fell 36% in the whole tick;
  - #783: `strays` +63%, `stamina` +28% and `bodies: sums and applying` +19% in the whole
    tick between `bfce23c` and `main`, which may be the case where the two differ;
  - #773: the seek's claims settled in rounds. It changed outcomes, so its states differ:
    compare the spread, not tick by tick.
- **Passing:** for each case, the phases its whole-tick bench moved beyond noise move the
  same way in the repeats. The sizes agree within the two runs' ranges, and the ranking of
  the three by tick change is the same.
- **Recorded:** the numbers and the verdict go in `## Measured` below, and a comment on
  #866.
- **If it fails,** the owner decides whether the mode stays a guide with a caveat (which
  phases it can't see), or is dropped. Step 5 doesn't start until then.

### Step 4, the inspection matrix

**Needs:** 1, 2
**Touches:** `.github/workflows/bench-inspect.yml` (new), `ci/godot/scripts/bench_compare.py`
(if the summary needs a join across jobs), `ci/godot/README.md` (the workflow)

A new workflow, started by hand with `head`, `base`, `sizes` (default both), `repeat` and
`rounds` (default 5):
- **build:** builds head and base once and hands the binaries on (as `bench-series`'s
  `references` job does).
- **plan:** runs the base's series at each size, takes its `inspect:` ticks, and runs the
  head's series too, so the summary has both series' p95 and p99 on one runner.
- **inspect:** a matrix, one job per (size, tick). Each times head and base in turn for
  `rounds` rounds of `repeat` repeats on its one runner. It records its CPU, and times the
  reference beside it (Decision 172), so numbers combine across jobs as ratios.
- **summary:** one table per size:
  - per tick, the head's change against the base for the tick and each phase, flagged where
    the ranges are apart, with the job's CPU;
  - the series' p95 and p99 from **plan**.

Scenarios:
- Given a head and a base, when the workflow runs, then each inspection tick's job reports
  head against base from one runner, and names its CPU and its reference's time.
- Given jobs on different CPUs, then the summary combines them only as ratios to their
  references, and says which CPUs ran which ticks.

### Step 5, recording and plotting the repeats

**Needs:** 3, 4
**Touches:** `ci/metrics/` (the record and its checks), `tools/metrics_view/`, the
`bench-inspect` workflow's data PR

Only once step 3 has passed:
- **Records.** A run of `bench-inspect` on `main` writes a record per (size, tick) in
  Decision 162's shape. `source.harness` is `tick_phases-repeat`, and the args carry `at` and
  `repeat`. Its data PR merges itself, as `bench`'s do (Decisions 163 and 164). There is no
  backfill: from merges forward only.
- **Plotting.** The tick history gains small multiples per phase, × the reference by default
  (Decision 172). Each shows the repeats beside the same phase's lap from the whole tick, so
  the gap between them shows how far to trust the fast number.

## Measured

Filled in by step 3.

## Not in this spec

- **A phase timed alone,** from the state just before it (Decision 177, Alternatives): later,
  only for a phase whose in-tick repeats still swing too much.
- **A fixed machine for every job:** Q45 (#877).
- **The performance work itself** (the neighbour-list build, the seek, the roster's copies,
  `floor` as a library call): the round that follows, guided by this spec's mode.

## Test-first order

1. Step 1: red on a repeated tick leaving the same state as one played through, and on the
   put-back keeping warm buffers (cargo tests, as `snapshot.rs`'s are). Then the mode. Then
   red on its printed lines (`phases/tests.rs`).
2. Step 2: red on the series ending when the field settles (a small battle that routes), and
   on `inspect:` chosen from a fixture of tick times. Then the series.
3. Step 3: no code; a measured verdict.
4. Step 4: no unit test. Checked by one run of the workflow on `main` against `main`, whose
   ticks must show no change beyond noise.
5. Step 5: red on the record's checks (`ci/metrics/check_records.py`'s tests) with a repeat
   record. Then the writer, then the page.

## Rubric answers

- **single-noun-phrase:** the repeat mode; the settled series and its inspection ticks; the
  ranking check; the inspection matrix; the repeat records.
- **ocp-extension-point:** a new inspection rule is a function the series' `inspect:` calls; a
  new size is a preset.
- **lsp-contract-scope:** not applicable; no subtypes.
- **isp-fit:** the mode reads only what a tick reads and the laps; the workflow reads only the
  binaries' printed lines.
- **dip-direction:** the workflow depends on `breach_tick_phases`' printed lines, never on the
  crate's types.
- **general-over-special:** general: one rule picks every commit's inspection ticks, and every
  phase is timed where it runs in the tick (Decision 177).
- **pre-mortem:** the riskiest assumption is tested in step 3, before step 5 records or plots
  anything.
- **order-independent:** not applicable; no simulation outcome changes. Ties between slow ticks
  go to the earlier tick.
- **bounded-work:** not applicable to the tick; the harness costs repeats × ticks, outside it.

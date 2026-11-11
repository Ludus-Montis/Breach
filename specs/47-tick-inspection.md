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
- Steps 1 and 2a can be built side by side; 2b follows 2a (step 2 split at build).
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

### Step 2b, the record marks the settled series, and the tick history breaks there

**Split at build:** from step 2, PR #904, 2026-11-11: two behaviours, the crate's series
(2a) and the records and page (2b), each with its own tests (cargo; Python and JS).
**Needs:** 2a
**Touches:** `.github/workflows/bench.yml` (the 25k set's `--args`), `ci/metrics/bench_set.py`
and its tests, `tools/metrics_view/metrics_25k.js`, `tools/metrics_view/metrics_view.js`
(`segments`) and their tests, `ci/metrics/README.md`, `tools/metrics_view/README.md`

- **The tick history.** Records before and after step 2a measure different series. The
  record's `source.args` say so (`series=settled` after it; none before), and the tick
  history's 25k chart breaks its line there, as it does at a harness change (spec 43).
  `series=settled` isn't part of a record's shape: the shape is the args without `series=1`
  and `series=settled`, so the families keep their records.
- **Merge it with 2a,** or before the next 25k set runs on `main`: a set recorded between the
  two is the settled series without the mark.

Scenarios:
- Given a 25k set's records with and without `series=settled`, when the shapes are read
  (`bench_set.py`, `metrics_25k.js`), then both have the same shape.
- Given a family's points across the change, when its line is drawn, then it breaks between
  the last record without the mark and the first with it.

### Step 3, the ranking check (the riskiest assumption)

**Needs:** 1
**Touches:** this spec's `## Measured` section (the result), `native/README.md` (how to read
the mode)

Run by hand through `bench` (`args` with `at=` and `repeat=`, 5 rounds, head and base on one
runner), at 4,096 and 25k. The ticks are those step 2a's `inspect:` names on the base, or, if
step 2a isn't merged, contact (91), the rout's spike (105) and the last of the old series (106).
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

**Needs:** 1, 2a
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

### Step 3, the ranking check (2026-11-11)

**How it ran.** The repeat mode is step 1's (#906), not on any of the cases' commits, so it was
ported onto each merge and its parent (branches `chore/spec47-step3-<PR>-head` and `-base`,
not for merging: step 1's commit, with `Field::put_back` a plain copy before #783's kept buffers
and `Series::push` added where it was missing). Each case then ran through `bench` (from
`chore/spec47-step3-bench`, which prints only the comparison), head and base on one runner,
5 rounds, `repeat=50`; beside each, the whole tick (`preset=4096`, `preset=25k`) on the same
two commits, for a same-runner yardstick (runs #107–#136, AMD EPYC 9V74 / 9V45, 4 threads).
- **Ticks:** at 4,096 contact (91), the rout's spike (105) and the last of the old series (106),
  as the spec says. At 25k one side breaks at **101** (104 before #773), not 107 (the repeat mode then stopped at the break), so 105 and 106
  don't exist there: 91, 100 and 101 instead. Step 2a's `inspect:` will name them on its own.

**The numbers** (change from base to head, medians over 5 rounds; `*` ranges apart):

| | whole tick, 4,096 | repeats, 4,096 (t91 / t105 / t106) | whole tick, 25k | repeats, 25k (t91 / t100 / t101) |
|---|---|---|---|---|
| **#780** tick | −5.2%* | −2.1 / −3.9* / −4.2%* | −9.2%* | −7.5* / −6.9* / −6.2%* |
| `bodies: pair search` | −28.6%* | −24.7* / −27* / −28.1%* | −40.8%* | −38.3* / −40.4* / −40.6%* |
| **#783** tick | −6.6%* | −3.0 / −6.6* / −4.8%* | −3.9%* | −2.0* / −4.9* / −6.4%* |
| `wounds (tend)` | −18.2%* | −19.8* / −20.1* / −20.9%* | −21.6%* | −19.2* / −19.3* / −19.6%* |
| `scrum: engage, prepare, stance` | −11.3%* | −12.5* / −17.0* / −2.6% | −13.3%* | −17.2* / −12.9* / −13.7%* |
| `bodies: sums and applying` | +31.6%* | +31.1* / +29.9* / +19.7%* | +15.9%* | +22.8* / +16.2* / +16.9%* |
| `store settles` | −40.5%* | −40* / −45* / −46%* | −44.0%* | −40.1* / −47.5* / −51.0%* |
| `shuffle (re-form)` | −16.1%* | −18.0* / −18.0* / −16.4%* | −15.1%* | −18.0* / −19.2* / −21.9%* |
| `stamina` | −29.7%* | −13.8 / −15.4* / −4.2% | −15.6% | −15.1* / −15.9* / −18.2%* |
| `strays` | +6.6% | +13.7 / +5.7 / +14.0%* | +24.1%* | +10.4* / +7.7* / +6.7%* |
| `groups (form up, split)` | −72.4%* | 0 / −14* / −10.4%* | −0.7% | +1.3 / −8.4* / −8.5%* |
| `bodies: pushes` | −16.2%* | −4.8* / −4.8* / 0% | −4.7%* | −3.1 / −0.8 / 0% |
| `scrum: seek` | −4.9%* | −3.3 / −3.9* / −3.2%* | −3.7%* | −2.3* / −4.1* / −4.9%* |
| **#773** tick | −15.9%* | −23.6* / −15.3* / −20.9%* | −15.5%* | −22.3* / −14.0* / −15.0%* |
| `scrum: seek` | −39.4%* | −41.3* / −46.7* / −49.7%* | −39.3%* | −47.4* / −49.2* / −51.7%* |
| the rout (`flight`, `rallies`) | +58%*, +46%* | ±: +33/+46%* at t105, −22% rallies at t106 | −42%*, −8%* | +32/+21%* at t100, +56/+36%* at t101 |

**The verdict.**
- **Direction:** passes. Every phase a case's whole-tick bench moved beyond noise moves the same
  way in the repeats, at both sizes, except where the whole tick itself swings (below).
- **Ranking:** passes. By tick change the repeats rank the three as the whole tick does at each
  size and every tick: 4,096 #773 > #783 > #780, 25k #773 > #780 > #783.
- **Size:** passes for the phases each change aimed at (#780's pair search within 1–4 points of
  the whole tick's, #773's seek within 2–12, #783's tend, engage, sums, store settles, shuffle
  within a few). **Not for every phase #783 moved:**
  - `strays` at 25k: +7 to +10% in the repeats against +24% in the whole tick. The repeats see
    it rise, at under half the size. This is the phase the pre-mortem named (+63% on `main`).
  - `groups` and `bodies: pushes` at 4,096: −72% and −16% in the whole tick, −10 to −14% and
    0 to −5% in the repeats. `groups` swings in the whole tick with its code untouched (#780's
    base, 25k: 1.49–4.42 ms over 5 rounds), so the whole tick is the doubtful side there.
  - `stamina` at 4,096: −30% whole, −4 to −15% repeats.
- #773 changes outcomes, so its rout runs at other ticks on the two sides. Tick by tick the rout's
  phases go either way, as expected. Its spread (seek far the largest, then the tick) is the
  whole tick's.

**By the letter of "Passing", the check fails on size for #783.** Its direction and ranking hold.
The owner decides whether the mode stays a guide with that caveat, or is dropped (Q47, #909).
Step 5 waits for that decision.


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
2. Step 2a: red on the series ending when the field settles (a small battle that routes), and
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

## Built

### Step 2a, the series runs until the field settles, and names its inspection ticks

**Split at build:** from step 2, PR #904, 2026-11-11: the crate's series here; the records'
mark and the tick history's break are step 2b.
**Needs:** none
**Touches:** `native/rust/src/phases/series.rs` and its tests,
`native/rust/src/phases/presets.rs`, `native/rust/src/bin/tick_phases.rs` (its doc),
`native/README.md` (Tick phases)

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

Built as: `series::stirs` (a listed squad with a living unit fighting or routing) ends the
series; `series::inspect` names the ticks, earliest first, as `inspect: 91,...` after the
series' rows, comma-separated as `at=` takes them. "A third of the way" is index
`⌊k·(n−1)/3⌋` of the series' `n` ticks. `bench_compare.py`, `bench_scaling.py`,
`bench_record.py` and the showcase's `timings.py` stop at the line, so none changed.

Scenarios:
- Given a battle that routes (the small clash of `series/tests.rs`), when the series runs,
  then it continues past the tick where one side breaks until no squad fights or routes.
  At `preset=4096` the series' length is read from the first `bench` run on it.
- Given a series of known tick times (a fixture), when `inspect:` is chosen, then the
  ticks are the 3 slowest (the earlier on a tie) and the 4 spread. A duplicate is replaced by
  the next slowest.
- Given the digests, then they are unchanged: the series only times.

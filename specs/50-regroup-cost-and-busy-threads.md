---
spec_type: code
status: active
parent_spec: ../Breach — Reverse Tower Defense Design Spec.md
---

# Spec 50: Where regroup's time goes, and how many threads each phase keeps busy

## Purpose

After the clash, at 25k, `scrum: regroup and chasers` is the costliest phase of the tick, and
it runs on one thread. CI's 25k series (24 squads of 1,024 a side, `bench` run 43285905277,
`main` at 7106ad9) gives it 57 ms of a 149 ms mean tick (38%). The session that found it
(#964's) proposed weighing threads against a cadence (Q32, #291). This round settles neither
yet. It first finds **why** the phase costs what it does, then fixes that without changing
outcomes, as Decision 159's order puts it: work that follows change comes before threads, and
options that change outcomes come last.

The owner also asked how many of the machine's threads the tick actually uses. The records say
"20 threads" (the owner's PC) or "4" (CI), but not how many are busy. This round records it for
each phase.

**Settled in the design session of 2026-11-12 (the owner):**
- **Regroup:** option 1. Sub-laps first. Then the fix that the sub-laps point to, with
  outcomes identical. Threads come only if a large serial remainder is left, and a cadence
  stays with Q32 and Decision 160.
- **The gauge:** `bench-inspect`'s repeat records at 25k's tail ticks (`## The numbers this
  starts from`).
- **Busy threads:** in this spec, as its own step, beside the regroup work rather than
  before it.

No new Decision. No step changes an outcome or a standing rule.

## Status

Agreed in the design session of 2026-11-12. Three steps, each its own PR, built with
`/run-phase` as "spec 50 step N".
- Steps 1 and 3 can be built side by side.
- Step 2 needs step 1 **merged and measured**. Step 1's merge runs `bench-inspect`, which
  times the new sub-laps at the tail ticks. Step 2 starts by reading that run.

## Pre-mortem

- **Regret in a month:** regroup gets fast, the mean tick drops from about 149 ms towards
  95 ms, and p95/p99 don't move, because the clash sets them and regroup costs 0.01 ms there.
  This spec accepts that cost and says it up front: it is a mean and tail-tick win. **The p95/p99
  lever is the clash's seek, pair search and walk** (`## Not in this spec`). A second regret:
  a cadence or threads get built for a cost whose cause was a quadratic scan.
- **Riskiest assumption:** that we know why regroup costs 60 ms. The evidence is mixed:
  - The code points to a per-squad quadratic. Per-unit linear scans run over squad-sized
    lists: `living.iter().find`, `loose.contains` and `homes.iter().find` in `regroup.rs` and
    `pursuit.rs`, and `chasers.retain` for each unit back.
  - The numbers point to something growing with the whole field. One squad of 24,576 a side
    costs 5.3 ms in the phase. The cost per squad of 1,024 grows as squads are added: 1.2 ms
    at 4 squads, 1.5 at 8 and 2.4 at 24. Each tick also has two `Crowd::of` builds over every
    body (one in regroup and one in the chase), and `trade_in` runs for every squad with
    loose units.
- **Cheapest test:** step 1. Split the lap into sub-laps, change nothing else, and read the
  run that its merge records.

## The numbers this starts from

CI, `main` at 7106ad9, 4 threads. The series comes from `bench` run 43285905277. The ticks
come from `bench-inspect` run 43291722824 (50 repeats, 5 rounds, medians).

| 25k, 24 squads of 1,024 a side | Tick | regroup and chasers | Largest phases |
|---|---|---|---|
| Series: mean / p95 / p99 | 149 / 209 / 245 ms | 57 ms (mean) | |
| Ticks 91, 98, 99, 100 (the clash; the slowest) | 199–292 ms | 0.01 ms | seek 55–86, pair search 21–30, walk 13–20 |
| Ticks 166, 242, 318 (after the clash) | 128–140 ms | 58–65 ms | regroup, pair search 11–13, rallies 5–25 |

| Squads of 1,024 a side (`bench` series means) | 4 | 8 | 24 |
|---|---|---|---|
| regroup and chasers | 4.86 ms | 11.88 ms | 56.66 ms |
| per squad | 1.22 ms | 1.49 ms | 2.36 ms |

**Why `bench-inspect` can be the gauge.** Spec 47's ranking check (Q47, #909) found four
phases whose cost builds up across ticks, which the repeat mode under-reports: `strays`,
`groups`, `bodies: pushes` and `stamina`. Regroup is not on that list. But the check's PRs
didn't touch regroup, so the list can't clear it either. The records clear it instead:
- The series' mean for regroup (57 ms) matches the repeats at the tail ticks (58–65 ms).
- Regroup is about half of every tail tick.
- If the repeats under-reported regroup, they would read lower than the series.

Regroup's cost comes from the tick's own state (who is loose, who is chasing, where the bodies
stand), and the repeat mode puts that state back before each lap. Step 1 checks the
match again with its sub-laps.

## Steps

### Step 1, regroup's sub-laps

**Needs:** none
**Touches:** `native/rust/src/scrum/regroup.rs`, `native/rust/src/regroup.rs`,
`native/rust/src/pursuit.rs`, `native/rust/src/scrum/step.rs`, `native/rust/src/phases/tests.rs`,
`native/README.md` (Tick phases), `tools/metrics_view/README.md` (if it names the phase)

- **The parts.** The phase `scrum: regroup and chasers` is split into parts with `laps::split`.
  The last part ends with `lap`, as spec 41 step 1's parts do. In order:
  - `scrum: regroup: regroupers` (the `regrouper` list, `returns` included);
  - `scrum: regroup: faced` (the state lock and `foes_faced`);
  - `scrum: regroup: crowd` (its `Crowd::of`);
  - `scrum: regroup: walk-back`;
  - `scrum: regroup: trade-in`;
  - `scrum: chase: chasing` (the `Chasing` list and its `returns`);
  - `scrum: chase: lookups` (the loop over chasers, up to the walks);
  - `scrum: chase: crowd` (its `Crowd::of`);
  - `scrum: chase: steer` (the walks; it ends the phase).

  Walk-back and trade-in run in turn for each regrouper, so each one's time adds up over the
  squads. The records' keys become `tick/scrum/regroup/<part>` and `tick/scrum/chase/<part>`
  (`bench_record.py` slugs each `:`-separated part).
- **The old name retires.** `scrum: regroup and chasers` is no longer lapped. The history's line
  for it ends at this merge, and the parts' sum takes its place. A phase that was never split
  keeps its name.
- **No outcome changes.** A split times nothing the simulation reads (`laps.rs`). The digests
  stay identical.

Scenarios:
- Given a timed 4,096 battle run past its clash, when its laps are read, then each regroup and
  chase part is there in the order above, and `scrum: regroup and chasers` is not.
- Given the same battle timed and untimed, when their states are compared, then they are equal
  (`phases::tests`' existing check, which now covers the splits).

**After merge (the measured part of the step, recorded under `## Measured`):** read
`bench-inspect`'s run for the merge at 25k's tail ticks, and `bench`'s 4-, 8- and 24-squad
rows. Write down:
- each part's median at the tail ticks;
- each part's cost per squad at 4, 8 and 24 squads (flat, or growing);
- whether the parts' sum still matches the series' mean.

The part that dominates names step 2's fix.

### Step 2, the fix the sub-laps name

**Needs:** 1
**Touches:** what step 1 names, within `native/rust/src/regroup.rs`, `native/rust/src/pursuit.rs`,
`native/rust/src/scrum/regroup.rs`, `native/rust/src/trade.rs`, `native/rust/src/roster.rs` and
their tests; `native/README.md` (Results)

This step is written before its data, so it settles the rules and lists the likely fixes. The
session that builds it reads step 1's `## Measured` first. If the data names something this step
doesn't cover, it stops and asks.

- **The rule.** The fix keeps outcomes identical, so the digests don't change, and it needs no
  Decision (Decision 159's "work that follows change"). After it, a squad's part of the phase
  stays bounded by its own units, not by the field's (`bounded-work.md`).
- **The likely fixes, by the part that dominates:**
  - **A lookup part** (`walk-back` or `chase: lookups`): the unit, home and loose lookups by
    id become one map per squad per tick (id → index), built once and read for each unit.
    `chasers.retain` for each unit back becomes one `retain` per squad after the loop. Ids are
    unique, so no tie changes.
  - **A crowd part:** build only what the steer reads. For example, take the bodies near the
    walkers through the field's grid instead of every body. If the crowd's content stays the
    same, its order must be kept too.
  - **`trade-in`:** run it only when something can trade (a loose unit came home, or a place
    freed), as a work list fed by those changes, in the squads' listed order.
  - **`regroupers` or `chase: chasing`** (`returns`, the terrain pour): only on rough ground.
    The 25k preset is open ground, so it should read near zero there.
- **Ruled out: one `Crowd` shared by regroup and the chase.** The chase's crowd sees the
  positions that regroup's walks just moved, so sharing one would change outcomes.
- **Threads.** If, after the fix, a serial part still costs more than about 10 ms at the tail
  ticks, say so in the PR and name the part. Threading it is a later round.
- **Measure.** Start `bench` with `rounds` of 5 against `main`. At 25k the phase falls, and
  no other phase is slower beyond noise. `breach_scaling` passes. The cost per squad (4, 8
  and 24 squads) is about flat.

Scenarios:
- Given the 4,096 and 25k digests, when the fix is built, then they are identical
  (`sim-checks`).
- Given a regroup with many loose units in one squad (a cargo test), when it steps, then each
  unit's walk and the chasers left are as before the fix. Write the test against the old code
  first: it pins the behaviour before the change.

### Step 3, busy threads per phase

**Needs:** none
**Touches:** `native/rust/Cargo.toml`, `native/rust/Cargo.lock`, `native/rust/src/laps.rs`,
`native/rust/src/phases/series.rs`, `native/rust/src/phases/repeat.rs`,
`native/rust/src/phases/tests.rs`, `ci/metrics/bench_record.py`, `ci/metrics/test_bench_record.py`,
`ci/metrics/README.md`, `native/README.md` (Tick phases)

- **What it measures.** At each `lap` and `split`, the process's CPU time
  (`CLOCK_PROCESS_CPUTIME_ID`, through `libc`) is read beside the wall clock. A phase's
  **busy threads** is its CPU time divided by its wall time, summed over the series or the
  repeats like its ms. One thread working reads about 1.0. Four threads fully busy read
  about 4.0.
- **Where it's recorded.** `breach_tick_phases` prints it beside each phase in the series
  and in the repeat mode. `bench_record.py` writes it as `busy/<phase key>` (for example
  `busy/scrum/seek`) with the unit `threads`, in `bench`'s and `bench-inspect`'s records.
  The record schema doesn't change, because `unit` is free text. The separate prefix keeps
  busy threads out of the phase chart's ms sums.
- **Windows.** No busy column on Windows: `GetProcessTimes` updates only every ~15.6 ms, which
  is too coarse for most phases. The local importer (`local_record.py`) is unchanged, and the
  owner's records carry no `busy/` keys.
- **The spin test (the step's riskiest assumption).** Rayon's idle workers spin briefly before
  they sleep, so CPU time can count waiting as work. Look at a phase that runs on one thread
  right after a threaded one (for example `units walk to places` after `bodies`). If it reads
  well above 1.0 at 25k, write a line in `ci/metrics/README.md` and `native/README.md` saying
  busy threads is an **upper bound** on useful parallel work, and record the reading under
  `## Measured`. Don't try to correct for it in this step.
- **No outcome changes.** Reading a clock changes nothing the simulation reads.

Scenarios:
- Given a 4,096 battle timed with `threaded=false`, when its busy threads are read, then every
  phase that took more than 1 ms reads between 0.8 and 1.2.
- Given `breach_tick_phases` output with busy columns (a fixture), when `bench_record.py`
  writes the record, then each phase has a `busy/<key>` with the unit `threads`, and
  `check_records.py` passes it.
- Given output without busy columns (an older binary, or Windows), when the record is written,
  then there are no `busy/` keys and the record passes.

## Measured

(Step 1's sub-laps, and step 3's spin reading, are recorded here as they land.)

**Step 3's spin reading, first look** (PR for step 3, 2026-11-12): this session's container, 4
threads, the `fast` profile, `preset=4096`'s battle (`side=4096 squads=4 series=1`), the whole
series. `units walk to places`, which runs on one thread right after the threaded `bodies`
parts, reads **1.00**: no spin shows there. A few short phases read above 1 with nothing of
theirs threaded (`wounds (tend)` 1.37, `rout: re-forms` 1.33), and the threaded ones read
`bodies: pushes` 2.98, `scrum: seek` 2.71, `bodies: pair search` 2.48, `scrum: facing` 1.96,
`scrum: walk` 1.67; the tick 1.49. So busy threads is written up as an upper bound for short
phases only. The 25k reading comes from the merge's `bench-inspect` run (`busy/` keys in
`metrics/inspect/`) and the next `bench-25k` record.

## Not in this spec

- **The clash's cost, which sets p95/p99:** seek (55–86 ms at the slowest ticks), pair search
  and walk, all already on threads (spec 47's "the round that follows"; spec 48's grid also
  found seek costing the square of a squad's size). It is the next performance design round.
  Step 3's busy threads tell it how well those phases use CI's 4 threads.
- **Threading regroup:** only if step 2 leaves a large serial part.
- **A cadence for squad rules** (Q32, #291; Decision 160): it stays open, and comes last in
  Decision 159's order.
- **Busy threads on the metrics page:** read from the records for now. A chart comes in a later
  step if the figures earn one.
- **Busy threads on Windows** (above).

## Test-first order

1. Step 1: red on the laps' order and names (a cargo test in `phases`, alone: the parts are
   missing). Then the splits. The timed-equals-untimed check stays green.
2. Step 2: first the pinning cargo test, green on the old code. Then the fix, keeping it green.
   Then `bench`.
3. Step 3: red on busy threads with `threaded=false` (a cargo test in `phases`, alone), then
   on `bench_record.py`'s `busy/` keys (`test_bench_record.py`, alone). Then the clock and the
   printing.

## Rubric answers

- **single-noun-phrase:** regroup's sub-laps; the fix they name; busy threads per phase.
- **ocp-extension-point:** a new part is one more `split`; a new measure for each lap is one more
  reading in `laps::counted`.
- **lsp-contract-scope:** not applicable; no subtypes.
- **isp-fit:** `bench_record.py` reads only the printed lines; nothing in the simulation reads a
  lap.
- **dip-direction:** the records depend on the binary's printed output, never on its types.
- **general-over-special:** general. Every phase gets busy threads. Sub-laps use the one
  `split` mechanism every phase uses. Step 2's fix is bounded-work's general rule (no scan of a
  squad inside a loop over its units, no pass over the field for each squad), not a regroup
  special case.
- **pre-mortem:** the riskiest assumption (why regroup costs what it does) is tested by step 1,
  which step 2 needs, merged and measured. Step 3's own (Rayon's spin) is tested inside it.
- **order-independent:** no outcome changes. Step 2's digests must stay identical. An id map
  doesn't rank by id, and ids are unique, so no tie moves.
- **bounded-work:** step 2 is this rule. A squad's regroup and chase work stays bounded by its
  own units; checked by the cost per squad at 4, 8 and 24 squads, and by `breach_scaling`.

## Built

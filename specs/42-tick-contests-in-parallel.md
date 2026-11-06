---
spec_type: code
status: active
parent_spec: ../Breach — Reverse Tower Defense Design Spec.md
---

# Spec 42: The tick's contests settled in parallel, behaviour judged by playback

## Purpose

Spec 41 keeps outcomes identical, so the serial contests stay in place:
- seek's claims (`scrum::plan`): each seeker takes a slot against the claims made by those
  ahead of it in contest order;
- the rout's landings (counting, re-forms, the caught count and joins, `src/rout/`);
- tend's takings (`src/wounds.rs`).

In run #77 at 25k, `scrum: seek, walk and facing` took 92 ms (95th 114), `rout` 12 (95th 36)
and `wounds (tend)` 9, against Decision 159's 50 ms. This spec settles those contests in
parallel. Each unit proposes what it wants, reading the state as it was when the pass began.
Then a rule that doesn't depend on order settles who gets what. **This changes behaviour**,
under **Decision 161**: every step's digests differ from its base's, and every step lands
only after the owner's feel test.

The numbers on `main` at 49aa335, before this spec (`bench` runs #85 and #86, A/A, 3 rounds)
are under "Measured".

## Status

Agreed in the design session of 2026-11-06 on #743 (the owner: "Sounds reasonable, I can get 41
underway and trigger this once step 1 is done"). Three steps, each its own PR, built with
`/run-phase` as "spec 42 step N".
- **Starts after spec 41 step 1.** Its laps and profile split seek's 92 ms, and step 2 here is
  judged against them.
- **Replaces spec 41 step 2** (seek's search off the serial loop, with claims kept serial).
  Step 2 here takes the search and the claims off the serial loop together, so that work isn't
  done twice. Spec 41 steps 3 and 4 are unchanged and may land in any order with this spec's.
- Step 1 here may be built at any time.

## Rules for every step

- **Kept, always** (Decision 161):
  - **Deterministic for a version:** the same result run after run, and on x86-64 and arm64
    alike (step 1's check).
  - **Thread-count independent:** CI's both-ways check (threads on and off) finds the digests
    and trials equal.
  - **Nothing by id or list order** (Decision 97): the mirror trials, both ways round, with
    at least 300 seeds per order, show no bias beyond about 2 standard deviations. The
    reversed-lists tests gain a case for each contest the step changes.
  - **Bounded:** a fixed number of settling rounds, each a local check within a radius
    through a grid.
- **The gate** (Decision 161). Each step's commit carries
  `Outcomes-changed: Decision 161, <what changes on playback>`. Its PR gives:
  - the outcome trace's first differences (`traces/` in `sim-digests-<set>`, spec 37);
  - the trials' numbers;
  - **what to watch:** the battles, and what should look the same or different.
  The owner signs off playback at Gate 2 before merging. The step's digests become the
  baseline.
- **Benched** against the step's base, with `rounds` of 5 or more, at `preset=4096` and
  `preset=25k`, numbers in the PR. Compare ticks only within one run.
- **Parallel work** follows Decision 159's pattern: map on the pool into buffers, then apply
  in squad and member order. Any reduction runs in a fixed order, and a pass asks
  `threaded.on(units)`.

## The settling rule

Every contest in this spec is settled the same way, in a fixed number of rounds `R`:

1. **Propose** (on the pool). Each contender that isn't yet placed picks its best choice by
   its own rules. It reads the state as the pass began, plus what earlier rounds settled.
2. **Settle** (on the pool). A proposal wins if no other proposal that conflicts with it ranks
   higher. Conflicts are found through a grid, within the contest's radius: for seek, two
   claims closer than the claim radius. Proposals rank by the contest's own key, which is
   about what things are and where they stand (seek's `contest::order`: arrival, roll plus
   initiative, initiative, speed, then the seeded draw). They never rank by id or by the
   order in which the proposals were made. The draw makes the key total, so every winner
   beats all its conflicts.
3. **Apply** the winners in squad and member order. Losers propose again in the next round,
   against what is now settled.

After `R` rounds, a contender not yet placed falls back as the contest's rules say for no
choice (for seek, it presses toward the nearest slot within its leash). `R` is a constant
chosen from the bench and the trace, recorded in this spec. Raising it brings the result
closer to today's serial one.

## Steps

### Step 1, the same result on any hardware

Decision 161 drops matching the old digests, so the guarantee it keeps needs a check of its
own. Today every digest is taken on `ubuntu-latest` (x86-64).
- `sim-checks` gains a job on GitHub's arm64 runner (`ubuntu-24.04-arm`). It takes one digest
  set on the branch and requires it equal to the x86-64 job's digest of the same set, tick for
  tick. A difference fails with the first tick and the set.
- The set is the cheapest that exercises seek, rout and tend (the step names it). The full
  sets stay x86-64 only.
- `ci/godot/README.md` lists the job.
- No outcome changes in this step.

### Step 2, seek's claims settled in rounds

`scrum::plan` (`native/rust/src/scrum.rs`) sorts seekers by contest order and takes them one
at a time:
- `kept` (the slot it made for, if still open);
- then `pick` (the nearest open slot by `consider`'s key);
- then `aim` (claim it) or `press`.

Each one reads the `Claims` made before it. Under the settling rule:
- **Round 1:**
  - every seeker's `kept` and `pick` run on the pool against no claims (only the ring's
    status, `within` and the crowd's bodies, as at the pass's start);
  - a proposal wins if no proposal of a higher contest key lies within the claim radius
    (`Claims::near`'s test);
  - winners `aim` and claim.
- **Later rounds:**
  - losers pick again on the pool, against the claims settled so far;
  - a slot found claimed is marked dead for everyone, as today.
- **After `R` rounds**, a seeker still unplaced takes the crowded fallback
  (`pick(seeker, true)`, then `press`) on the pool. Press claims nothing, so no settling is
  needed.
- **A kept slot** competes in round 1 like any other proposal. A seeker keeps its slot unless
  a higher-ranked seeker proposes within the claim radius of it.
- **`within` becomes callable from the pool** (`Fn + Sync`), or what it reads is precomputed,
  as spec 41 step 2 planned. `Claims` becomes a flat grid of claimed points (`src/grid.rs`'s
  cells), and `pick` no longer clones each seeker's owned slots.
- **Expected on playback:**
  - in a dense scrum, a few more seekers press rather than aim in the tick a crowd forms, and
    find a slot a tick later;
  - slot choices differ where two seekers wanted the same one;
  - no change where seekers don't contend.

### Step 3, the rout's and tend's landings by the same rule

Measured first: spec 41 step 1's laps name the rout's parts, and this step builds the ones
over 1 ms at 25k. For each serial landing (the rout's counting, re-forms, caught count and
joins; tend's takings), the step:
- writes down what each unit reads that an earlier unit wrote within the pass. Those reads
  become reads of the state as the pass began;
- states the contest's key (it has one today, from Decision 97: for joins, nearest first,
  then the seeded draw; for takings, the units' seeded draws' order) and its conflict (two
  routers joining one squad past its room, two takers on one body);
- settles it by the settling rule, with `R` rounds, and applies in squad and member order.

Where a landing has no conflict (a count summed from the pass's start), it becomes a
reduction in a fixed order with no rounds. **Expected on playback:** routers join and re-form
a tick later at most when several reach the same squad at once. Takings differ only where
two takers reach one body in the same tick.

## Not in this spec

- **Reading the last tick's state across phases** (double-buffered columns), so that phases
  could overlap: Decision 161 allows it, but no measurement shows a need. Raise it if, after
  this spec, the tick is waiting on one phase's end before the next can start.
- **Still declined** (Decision 151, kept by 161): level of detail, a different tick rate,
  weighted-sum steering, thinning decisions beyond each unit's beat. Decision 160's cadence
  for squad rules stays benched only.
- **`f32` scalar columns** (#739, item 7): only if a profile shows the tick waiting on memory.

## Scenarios

### Step 1
- Given the branch's digest set taken on x86-64 and on arm64, then they are equal tick for
  tick, and the job passes.
- Given a digest that differs between the two (a test fixture), then the job fails and names
  the set and the first tick that differs.

### Step 2
- Given two seekers proposing slots within the claim radius of each other, when settled, then
  the one with the higher contest key claims, and the other proposes again next round.
- Given the same scrum with its seekers listed in reverse order (same ids, same seed), then
  the plan is identical.
- Given the same scrum on one thread and on the pool, then the plan is identical.
- Given seekers with no contended slots, then each aims at the slot the serial plan gives it.
- Given a seeker unplaced after `R` rounds, then it presses toward the nearest slot within its
  leash, or idles if there is none.
- Given the mirror trials both ways round, 300 seeds per order, then the first-spawned side's
  wins differ by no more than about 2 standard deviations.

### Step 3
- Given two routers reaching one squad with room for one, when settled, then the nearer
  joins (the seeded draw on a tie), whatever the lists' order.
- Given two takers on one downed body, then the one first in the seeded draws' order takes
  it, on one thread and on the pool alike.
- Given a count read from the pass's start, then it is the same whatever the order the units
  are visited.

## Test-first order

1. Step 1: a script test red on the comparison failing for a differing fixture, then the
   comparison; then the arm64 job (checked by a run).
2. Step 2: a cargo test red on two conflicting proposals settling by contest key, then the
   settling; then the reversed-lists and threads-on-and-off scenarios; then the fallback
   after `R` rounds; then `Claims` as a flat grid.
3. Step 3: per landing, red on its conflict scenario, then its settling; then the reversed
   lists.

## Rubric answers

- **single-noun-phrase:** the cross-hardware digest check; the seek claims' settling rounds;
  the landings' settling.
- **ocp-extension-point:** a new contest gives a key, a conflict radius and a fallback to the
  one settling rule.
- **lsp-contract-scope:** not applicable; no subtypes.
- **isp-fit:** the settling sees only each proposal's key, point and radius.
- **dip-direction:** each contest depends on the settling rule and the grid, not on another
  contest's code.
- **general-over-special:** general. One settling rule for every contest, and one gate for
  every change in behaviour (Decision 161).
- **order-independent:** proposals rank by the contest's key and the seeded draw. A winner is
  decided by a local comparison, not by when it was proposed. Results apply in squad and
  member order and don't depend on the number of threads (Decisions 97 and 159).
- **bounded-work:** `R` rounds, each a grid query within the contest's radius per proposal.

## Measured

`main` at 49aa335 (spec 41's design, before any of its steps), CI's 4-thread runner, 3 rounds:

_Filled in from `bench` runs #85 (25k) and #86 (4096) before this spec's PR is opened._

---
spec_type: code
status: active
parent_spec: ../Breach — Reverse Tower Defense Design Spec.md
---

# Spec 38: Tick performance after the unit store

## Purpose

Spec 35 moved every unit's per-tick state into the store and made the tick 21.7% faster
(428.0 → 335.3 ms, `tick_phases engine=rust side=4096 squads=4 fight=10`, against `1cc4412`,
5 alternating rounds; spec 34, "Measured"). Real time needs under 100 ms. This round orders
what is left by measured gain per tick:

1. **Groups and regroup bounded** (step 1): later in the battle they cost about 310 ms of a
   385–466 ms tick, because each reads whole-battle columns once per squad or per pair of
   squads (Decision 150).
2. **The bench covers the whole battle** (step 2): the bench read so far times 10 ticks just
   after contact, which hid step 1's cost. It also has no terrain, so nothing that pays the
   ground is measured.
3. **The ground and the mover in Rust, then the rout's per-router work** (step 3).
4. **Stamina, the downed's passes and the walk to places** ported (step 4).
5. **Threading, then slot packing** (step 5).

**Settled by Decisions 150** (steps 1 and 2) **and 151** (ports retire their GDScript twin,
step 2b; a performance change may change outcomes but never behaviour). Decisions 97, 129
(with 131 and 151), 130, 138, 139, 144 and 146 still hold. Answers issues #278 (Q23) and #277
(Q22); carries #279 (Q24) and #281 (Q26).

## Status

Agreed in the design session of 2026-10-21. Steps 1a, 1b, 1c, 2 and 2b are settled and
built with `/run-phase` as "spec 38 step N", one PR each, in that order. Steps 3, 4 and 5 are ordered here but each
needs a design session before it is built; their subjects are under each step.

## Evidence (bench runs on the 7c branch, `e0231cd`)

| Run | Arguments | Tick | Largest phases (ms) |
|---|---|---|---|
| 39043389452 (against `1cc4412`, 5 rounds, AMD EPYC 7763) | `fight=10` | 335.3 | rout 72.3, wounds 26.6, bodies 26.1, recovery 22.4, stamina 22.2, blows 22.1, walk to places 21.6 |
| 39043391384 (AMD EPYC 9V45) | `settle=40 fight=10` | 466.5 | **regroup 177.3, groups 131.4**, rout 35.7, bodies 20.7, walk to places 19.6 |
| 39043393472 (AMD EPYC 9V45) | `settle=100 fight=10` | 385.5 | **regroup 186.1, groups 131.2**, walk to places 11.5, bodies 9.8; the rout is over (0.09) |

All runs have `engine=rust side=4096 squads=4`. Only Rust kernels run natively today: seek,
the scrum walk, facing, parting and blows' picks, about 50 ms. The rest of the tick is
GDScript loops.

## Settled

- **Groups (`FormationGroups._form_up`)** loops over every pair of groups at ease. For each
  pair, `_back_row` reads the whole battle's RANK column for both squads (`UnitView.read`), and
  `_gap` copies the whole POSITION column (`column()`). Under Rust each is a copy of every
  slot in the battle, so the pass costs pairs × battle size.
- **Regroup (`ScrumRegroup.step`)**, for each regrouping squad, builds a `ScrumNear` index of
  every hostile unit in the battle (`_foes_faced`), holds the battle's columns
  (`UnitView.hold([squad], HELD)`) and reads two more (`UnitView.read`) in `_walk_back`. It
  costs regrouping squads × battle size.
- **Both decide from a snapshot already,** so reading once changes no outcome. Regroup works
  out every squad's foes before any squad walks. Groups gathers every meeting, sorts them by
  key (gap snapped, then both squads' seeded draws) and only then applies them. Neither writes
  RANK or POSITION inside its loop.
- **The bench is the instrument.** No step here is judged on a single run: each step runs
  `bench` against its base with `rounds=5` and records the result in spec 34's "Measured"
  section. From step 2 on, also with the whole-battle series.

## Steps

### Step 1a, groups bounded

- `_form_up` reads RANK and POSITION once a pass, before the pair loop, through one read view
  (`UnitView.read`, Decision 146) per store; `_back_row` and `_gap` take it.
- Each group's back row is worked out once a pass, not once per pair.
- Candidate pairs come from a grid of the groups' positions, with a cell at least the largest
  reach `_gap` tests (`one.width + other.width + both back rows + group_join`), not from every
  pair. The pair's own test (`_together`, the reach check) is unchanged.
- **Order:** gathering pairs from a grid changes the order meetings are appended, and
  `sort_custom` isn't stable. The meeting key must therefore order any two different
  meetings. Today it is [gap, draw, draw]; if two meetings can tie on all three (the same
  two draws can't belong to two pairs unless a squad draws equal to another), append the
  pair's positions in the battle's frame (Decision 97: where they stand, never ids). Record
  what was found in "Built".
- **Outcomes unchanged:** `sim-checks`' digests on both engines, and
  `test_formation_list_order.gd` with a case of three groups meeting at once.

### Step 1b, regroup bounded

- Each faction's hostile index (`ScrumNear.index` of `ScrumBlows.hostile_units`) is built once
  a tick, the first time a regrouping squad of that faction needs it, and shared, as
  `RoutFriends` keeps the rout's.
- `_walk_back`'s hold and reads are taken once for every regrouping squad, not once per
  squad: one `UnitView.hold` over all regrouping squads before the walk loop, and one release
  after. The implementing session checks that no native call runs while it is held
  (Decision 146) and records that it does not.
- **Outcomes unchanged:** digests on both engines, and the reversed-lists test with two
  regrouping squads of one side.

### Step 1c, the other passes, and the check

- **Audit** every pass that takes a view, a `column()` or an index inside a loop over squads
  or pairs. The other session's review (2026-10-21) named morale, withdraw, the shuffle and
  speed. List each in "Built" with what it reads and how it reads it once a pass, then fix
  them.
- **The check:** `check_store_reads.py` also flags `UnitView.hold`, `UnitView.read`,
  `UnitView.of` and `.column(` inside a loop, unless marked
  `# store: per squad (<why>)`, a reason the bench has shown doesn't matter.
  `UnitView.of(squad)` on a view already held for the pass is an index, not a copy: the check
  allows it, and its test proves both cases.
- **Measure:** `bench` against the base with `rounds=5` at `settle=40 fight=10` and
  `settle=100 fight=10`. The step is done when groups and regroup are each under 10 ms at
  both. If they aren't, the step records why and stops for the user.

### Step 2, the bench covers the whole battle

- **`tick_phases series=1`:** times every tick from contact until one side is gone or
  `limit` ticks (default 600). For the tick and each phase it reports the mean, the 95th and
  99th percentiles and the worst, with the tick that was worst. Spec 34 item 6.
- **Its output keeps `bench_compare.py` working.** The per-phase lines it already parses
  carry the series means, and the percentiles follow as extra lines, which `bench_compare.py`
  reports for head and base when present. The `bench` workflow needs no new input: `args`
  carries `series=1`.
- **`tick_phases terrain=mixed`:** the clash on ground with a wood and a ford across the line
  of advance, built from the existing terrain content (`FormationTerrain`) in
  `FormationBench`. The implementing session picks the layout, records it here and keeps it
  fixed, so runs compare.
- **Visited and changed, per pass:** with `counts=1`, `tick_phases` copies every store
  column before and after each phase. Per phase it reports the slots whose columns changed
  against the living units, as a share. The copying is in the tool and only under `counts=1`,
  so it changes neither the sim nor timed runs. This is the measurement for Q24 (#279): a pass
  that visits every unit but changes few is a candidate for a work list fed by events.
- **Recorded:** after it is built, the step starts three `bench` runs on `main`:
  `series=1`, `series=1 terrain=mixed` and `series=1 counts=1`. It records them in spec 34's
  "Measured" and in this spec's "Built", with the order of steps 3 and 4 re-checked against
  them.

### Step 2b, the outcome check on Rust (Decision 151)

- CI's `digests` job builds the base's Rust core in its worktree (`native/build.sh release`,
  as `bench` prepares its base) and takes the base's digest with `BREACH_NATIVE=rust`.
  `check_outcomes.py` compares it with the branch's Rust digest. The two-engine comparison of
  the branch stays as it is.
- Nothing retires in this step: it makes the guard that retiring needs. Outcomes are
  unchanged, and the check passes on a branch with no outcome change, as before.
- **Scenario:** given a branch whose Rust digest differs from the base's and no
  `Outcomes-changed:` trailer, the check fails. With the trailer, it passes and names the runs,
  as it does today for GDScript.

### Step 3, the ground and the mover in Rust, then the rout (design first)

The rout costs about 72 ms a tick while it lasts, and it is the phase that stayed slower than
before the store (+15.6%). Each router runs its flight (`RoutFlight.step`), crush
(`RoutFriends.crushed`), rally (`RoutCatch.friend_near`) and hold (`RoutCatch.holder`) in
GDScript, each with its own grid query. Its flight pays the ground through `UnitMover` and
`FormationTerrain`, and reads its route (`squad.route.heading_at`). Rust has neither the ground
nor routes. So the ground and the one mover (Decision 139) move first, and they also serve
the walk to places, the scrum walk's pay, withdraw, regroup and the pursuits. Breaking,
regrouping and re-forming a squad stay in GDScript: per-squad rules, Decision 131.

**The crush is order-independent** (checked 2026-10-17, Decision 97). A friend's
`is_alive()` reads its state, and state changes only in the deaths pass. Damage only adds.
The fell queue is drained in squad and member order (`FormationDeaths._fallen`).

**Subjects for its design session:** how terrain and routes reach Rust and stay current (spec
34 items 3 and 5, issue #268 Q12), and step 2's terrain numbers. Settled already: each port
retires its GDScript twin in its own PR (Decision 151).

### Step 4, stamina, the downed's passes and the walk to places (design first)

Ported to Rust in this order: stamina, about 22 ms (arithmetic over each unit, no queries);
wounds, recovery and deaths, about 64 ms; the walk to places, about 21 ms, after step 3,
since it pays the ground. Decision 131 already moves per-unit work to Rust. Each is its own
PR, matches the reference bit for bit on its own commit, and retires its GDScript twin in the
same PR (Decision 151).

**Subject for its design session:** step 2's counts. A pass that changes few of the units it
visits is a work list fed by events before it is a port (Q24, #279).

### Step 5, threading, then slot packing (design first)

Rust runs about 50 ms of the tick today, so threads on every kernel would save at most about
35 ms (Amdahl's law). Slot packing speeds up only native passes. Both come after steps 3 and
4 have moved more of the tick into Rust. **Subjects:** Q26 (#281) as written; each needs a
Decision the user authorises. Decision 151 reopens two options Q26 ruled out only because
they broke identity with the reference: `f32` columns and parallel reductions in a different
fixed order. Both change outcomes but not behaviour, so each is allowed under that
Decision's conditions.

## Scenarios

### Step 1a
- Given two groups of one faction at ease and in reach, when `_form_up` runs, then they meet
  exactly as before, and RANK and POSITION are each read once a pass (a counting store in the
  test).
- Given three groups that meet on one tick, with the squad lists reversed, then the same
  merges happen in the same order.

### Step 1b
- Given two regrouping squads of one side, when regroup runs, then the hostile index is built
  once for that side, and each unit's walk back is as before.

### Step 1c
- Given a `UnitView.read` inside a `for squad in squads:` loop, unmarked, when
  `check_store_reads.py` runs, then it fails, naming the line; marked
  `# store: per squad (<why>)`, it passes. `UnitView.of(squad)` in a loop passes.

### Step 2
- Given `series=1 limit=50`, then the tool reports the tick and each phase's mean, 95th, 99th
  and worst over 50 ticks, and `bench_compare.py` reads two such outputs.
- Given `counts=1`, then each phase reports changed against living, and a run without it
  is timed exactly as before.
- Given `terrain=mixed`, then the battle crosses the wood and the ford, and the digest of
  that scenario is the same on both engines.

## Test-first order

1. Step 1a: the counting-store test and the reversed-lists case, red (columns read per
   pair), then green.
2. Step 1b: the index-built-once test, red, then green.
3. Step 1c: `test_check_store_reads.py` cases for the per-squad patterns, red, then the
   check, then the audit's fixes.
4. Step 2: `test_bench_compare.py` with series output, red; then the tool.

## Rubric answers

- **single-noun-phrase:** tick performance after the unit store.
- **ocp-extension-point:** a pass reads its columns through the view it already has (Decision
  146); the check gains patterns, not exceptions.
- **lsp-contract-scope:** not applicable; no subtypes.
- **isp-fit:** `_back_row` and `_gap` take the columns they need, not the store.
- **dip-direction:** passes depend on `UnitView`, not on a backend.
- **general-over-special:** general: a pass reads once whatever the number of squads
  (Decision 150).
- **order-independent:** no order changes. Steps 1a and 1b keep each pass's snapshot. Step
  1a's pairs, gathered from a grid, are sorted by a key that orders any two meetings, with
  ties going to where squads stand, never ids.
- **bounded-work:** this round's point: groups cost pairs found by a grid, not every pair ×
  battle; regroup costs one index per faction a tick, not one per squad.

## Built

(For the implementing sessions, a section per step.)

## Open, for the implementing session to settle and record here

- Step 1a: whether the meeting key can tie, and if so the positions added to it.
- Step 2: the `terrain=mixed` layout.

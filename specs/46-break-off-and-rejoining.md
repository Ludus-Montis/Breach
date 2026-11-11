---
spec_type: code
status: active
parent_spec: ../Breach — Reverse Tower Defense Design Spec.md
---

# Spec 46: A break-off that shows, a probe of any tick, a line that settles, and units that rejoin without shoving

## Purpose

Q42 (#853), raised by the owner on the first showcase capture: `field_break_off` doesn't show
a break-off, and the defending line oscillates back to its places. The session that settled
it took in a second sighting from the owner, on `field_ford_and_flank`: units that fall and
get back up force their way through their own line, then form an L instead of a rank behind.

**Settled by:**
- **Decision 173:** breaking off is seen only when a squad that has been engaged for 3 s
  signals a withdrawal and within 3 s has moved N tiles from the foe it fought. The record is
  redrafted with a bigger wave and a longer fight.
- **Decision 174:** `breach_probe` prints any scenario's per-unit state, tick by tick.
- **Decision 175:** a line makes room where a rejoining unit can reach. Every joiner is
  covered: recovered units, returning chasers and rallied routers. A chain of units moves one
  place each, except while that file is fighting. Trades only ever put units home, and a
  router joins without a full re-form.
- **Decision 176:** `--capture` hides the HUD behind a caption band; the attackers take a
  contrasting colour and every body a dark rim.
- **The wobble needs no Decision.** Decision 110 already means a line settles. One that
  doesn't is a bug, fixed in step 3.

## Status

Agreed in the design session of 2026-11-11. Six steps, each its own PR, built with
`/run-phase` as "spec 46 step N".
- **Step 1 is a go/no-go.** It builds the probe and measures. If its measurements contradict
  the round's assumptions (see its stop conditions), stop and raise a `question` issue.
- Steps 2 and 3 need step 1, and can run side by side. Step 4a needs step 3, and 4b needs 4a.
- Step 5 needs nothing and can start at once.

## Pre-mortem

- **Regret in a month:** we redraft the record and tighten the check, the check passes, and
  the clip still doesn't read as a break-off. That can happen because the cause was partly
  visual, or because the new check is so strict that it reports "not seen" after every
  simulation change and is then ignored. Second regret: the wobble was a real bug in the
  line's hold, filed as cosmetic. Third (subject 5): the rejoin fix adds movement inside a
  standing line and makes the wobble worse, or scrambles the painted layout.
- **Riskiest assumption:** that a break-off after a real fight opens visible distance against
  Decision 103's pursuit. If the defenders keep the gap closed, no record or check shows a
  clean break-off. For the rejoin: that the shove and the L come from where a unit is sent
  (its old front-rank place), not from its path or from how the line closed up.
- **Cheapest test:** the probe, step 1, before anything builds on it. On `field_break_off`,
  the withdrawing squad's distance from its foe on every tick after the order, and the line's
  positions over ticks 150 to 290. On `field_ford_and_flank`, each rejoining unit's target
  place and whether friends stand between it and that place.

## Steps

### Step 1, the probe, and what it measures

**Needs:** none
**Touches:** `native/rust/src/bin/probe.rs`, `native/rust/src/probe.rs`,
`native/rust/src/probe/`, `native/rust/src/lib.rs`, `native/rust/Cargo.toml`,
`native/build.sh`, `native/README.md`

- Given a scenario name and `from=150 to=152`, when `breach_probe` runs, then it prints one
  JSON line per living unit per tick, with the fields Decision 174 lists, and nothing else on
  stdout.
- Given `squad=` or `unit=`, then only that squad's or unit's lines are printed.
- Given an unknown scenario, or `from` after `to`, then it says so on stderr and exits non-zero.
- Given the same arguments twice, then the output is byte-identical.
- **Then measure, and record under "Measured" in this spec:**
  - `field_break_off`: the withdrawing squad's distance from the foe it fought, tick by tick
    for 3 s after the order, on the current record and on a draft with a bigger wave
    retreating about 5 s after contact. Proposes N.
  - `field_break_off`, ticks 150 to 290: each line unit's distance from its place and its
    movement per tick. Says whether the positions settle (then the wobble is in the drawing)
    or keep moving (then in the simulation, and which: trades that undo each other, the
    0.05-cell brush, or the hold band). Proposes K and ε.
  - `field_ford_and_flank`: for each unit that rejoins, its target place on each tick,
    whether friends stand between, where it ends, and why the L forms. Proposes the
    reachability margin.
- **Stop and raise a `question`** if pursuit keeps the gap closed for every draft (Decision
  173 would then always report "not seen"), or if the shove comes from the path and not from
  the place it's sent to (Decision 175's riskiest assumption is false).

### Step 2, breaking off read from the squads, and the record redrafted

**Needs:** 1
**Touches:** `native/rust/src/scenarios/seen.rs`, `native/rust/src/scenarios.rs`,
`native/rust/src/scenarios/tests.rs`, `content/scenarios/field_break_off.record`,
`content/scenarios/README.md`

- Given a squad engaged for 3 s that signals `withdrawing`, and within 3 s is N tiles further
  from the foe it fought, then breaking off is seen at the signal's tick.
- Given the same signal while the squad keeps within N tiles of that foe for 3 s, then
  breaking off is not seen.
- Given a squad that signals `withdrawing` after less than 3 s engaged, or never engaged, then
  breaking off is not seen, however far it goes.
- Given a second foe near the withdrawing squad, then only the foe it fought counts.
- Given a scenario whose end would fall inside a behaviour's 3 s window, then it runs on to
  the window's end.
- Given the redrafted `field_break_off` (bigger wave, retreat about 5 s after contact, drafted
  against step 1's numbers), then the crate's scenario tests see engaging and breaking off,
  first contact within 15 s, and an end within 90 s.

### Step 3, the line settles

**Needs:** 1
**Touches:** `native/rust/src/trade.rs`, `native/rust/src/trade/`,
`native/rust/src/scrum/regroup.rs`, `native/rust/src/scrum/regroup/`. If step 1 places the
wobble in the drawing instead, the step touches
`presentation/skirmish/formation_2d/formation_field_scene.gd` and waits for step 5 to be
free (Decision 171), and says so in its PR.

- Given `field_break_off` replayed, when no foe has been within reach of the defending line
  for K ticks, then every line unit is on its place and moves under ε a tick, and stays so
  to the scenario's end.
- Given two units each out of its place, when they trade (Decision 175), then at least one
  ends on a place of its kind and neither leaves one. No trade undoes an earlier trade.
- The fix starts from Decision 175's trade condition, which forbids cycling. If step 1
  names another cause, the fix goes there, and the test above stays the same.
- Changes outcomes: trials both ways round (`mirror_headon`, 300 seeds an order) and a
  reversed-lists case, as `/design-check` says. Numbers in the PR.

### Step 4a, a rejoining unit takes a place it can reach, and the line makes room

**Needs:** 3
**Touches:** `native/rust/src/scrum/regroup.rs`, `native/rust/src/scrum/regroup/`,
`native/rust/src/paths/mod.rs`, `native/rust/src/paths/search.rs`

- Given a unit getting up behind its line whose own place is in the front rank, then it takes
  the nearest free place at the back, a gap in front is filled by the unit behind stepping up,
  and no unit is pushed.
- Given a unit in front of its line with a way round within the margin, then it walks round
  and takes a place at the back.
- Given a unit in front of its line with no way round, and a free place only at the back, then
  the units of the file between each move one place back, and it takes the front place opened.
- Given the same, while that file's front unit is engaged, then it waits in front as a loose
  unit and takes a place once the file is free or a gap in reach opens; it looks again at its
  re-form pace.
- Given a chain through a unit of another kind, then that unit ends out of its place and is
  brought home by trades (step 3's rule).
- Given two joiners wanting the same place on one tick, then the nearer takes it, then a
  seeded draw; the same with every list reversed.
- `field_ford_and_flank`, by the probe: no unit crosses its own ranks, and no L.
- Trials both ways round (`mirror_headon` and `mirror_flank`, 300 seeds an order) and a
  reversed-lists case. Numbers in the PR.

### Step 4b, a caught router joins without a full re-form

**Needs:** 4a
**Touches:** `native/rust/src/rout/pass.rs`, `native/rust/src/rout/tests.rs`,
`native/rust/src/joins.rs`, `native/rust/src/ranks.rs`

- Given a router caught by a steady ally (Decision 98) that joins it, then the ally gains one
  place at the back of the router's band, filling its back rank to its width before starting
  a new rank, and no other unit of the ally changes place unless a chain moves it.
- Given an ally with no band of the router's kind, then the place is at the very back, and
  the router is out of its place.
- Given the router arrives from the front with the back out of reach, then step 4a's rule
  makes room.
- Trials both ways round and a reversed-lists case. Numbers in the PR.

### Step 5, the capture's caption band, and bodies drawn to be seen

**Needs:** none
**Touches:** `presentation/skirmish/formation_2d/field_capture.gd`,
`presentation/skirmish/formation_2d/formation_field_scene.gd`,
`presentation/skirmish/formation_2d/formation_field_hud.gd`, `ci/showcase/capture.sh`,
`tests/presentation/skirmish/test_field_capture.gd`

- Given the field started with `--capture`, then the HUD is hidden and a caption band below
  the field shows the scenario's name and the time.
- Given a listed behaviour first seen at tick t, then from t the band shows it with its time,
  read from the same reading the scenario check uses.
- Given the movie's size, then the band sits below the field and covers none of it.
- Given the field in play, then the attackers are drawn in the new colour and every body has
  its rim; the same in the capture.
- The owner reviews the colour on the PR's capture.

## Not in this spec

- **The in-game inspector** (pause, step, scrub, click a unit): with Q41 (#828), amended in
  this session.
- **An outline around each squad:** fits Q41's toggles.
- **`settling` as a scenario behaviour:** step 3's test guards it.
- **Merges on a merge order** (Decision 90) taking the rejoin rule.

## Measured

To be filled by step 1.

## Test-first order

1. Step 1: red on the probe's output for a three-tick range of a scenario (Rust test of
   `probe.rs`), then the filters and errors. Then the measurements, recorded here.
2. Step 2: red on the breaking-off reading (`scenarios/tests.rs`) with each case above. Then
   the record.
3. Step 3: red on the settling test on `field_break_off`, then the trade condition.
4. Steps 4a and 4b: red on each case above (`scrum/regroup/tests.rs`, `rout/tests.rs`), then
   the rule.
5. Step 5: red on `test_field_capture.gd` (HUD hidden, band placed, behaviour shown), then
   the drawing.

## Rubric answers

- **single-noun-phrase:** the probe; the break-off reading; the line's settling; the rejoin;
  the router's join; the capture's drawing.
- **ocp-extension-point:** a behaviour's reading is one function in `seen.rs`; the probe's
  fields are one struct.
- **lsp-contract-scope:** not applicable; no subtypes.
- **isp-fit:** the probe and the checks read the replay; the rejoin reads only its own squad.
- **dip-direction:** the capture's band reads the crate's reading, never its own.
- **general-over-special:** general: one reading of breaking off for every squad (Decision
  173); one rejoin rule for every joining unit (Decision 175).
- **pre-mortem:** the riskiest assumptions are tested by step 1, with stop conditions, before
  any step builds on them.
- **order-independent:** places, chains and joiners decided from a snapshot by distance,
  fewest moves, then a seeded draw; trials both ways and reversed lists in steps 3, 4a, 4b.
- **bounded-work:** one flood per joiner over its squad's frame plus a margin; a waiting
  joiner looks again at its re-form pace; chains at most the squad's size; trades at most
  the number out of place.

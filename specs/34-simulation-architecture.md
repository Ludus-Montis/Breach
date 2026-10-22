# Spec 34: Simulation architecture agenda

Agenda, not yet a plan. This spec collects the efficiency and structure ideas from the
user's field-test notes (2026-10-13) and from another agent's review. It records which
ideas were adopted, which were declined and why, and where each one will be built.
Decisions 129-132 and 139 and the kit principle `principles/bounded-work.md` are the
rules these ideas serve.

## Adopted

1. **One mover (Decision 139).** Every change to a unit's position goes through one
   function, which pays the ground. Today twelve files write positions themselves, and
   only four of them pay terrain. That is why the rout flight and the march still ignore
   the ground. This work is the first step of Decision 132's series: the mover is the one
   writer the native field takes over. A check keeps it that way.
2. **One perception pass a tick** (the user: "one pass vs several scans"; the other agent:
   a shared spatial index). One pass over the grid gathers what each unit sees: the
   nearest foes and friends, the downed in reach, and threat (Decision 134). It stores the
   results in arrays that later passes read, rather than each pass searching the grid
   again. Decision 138's beat decides which units re-read their surroundings each tick.
3. **Routes worked out per map, not per walker** (the user: "precalculating map routes";
   the other agent: hierarchical navigation and flow fields). The plan has three layers,
   and each fits under Decision 131's flow fields:
   - A graph of route segments is built when a map loads.
   - A flow field is kept for each destination that many units share: a home, a breach,
     a rally point.
   - Local steering stays per unit.

   Terrain is held in chunks. Destroying a block marks only its chunk dirty, and only
   paths through that chunk are rebuilt.
4. **Data as arrays, not objects** (the user; the other agent). Already Decision 131's
   layout: positions, HP and stamina in packed arrays, flags as bitmasks, indices rather
   than references.
   **Settled by Decision 142 and built as spec 35 (the unit store):** the store owns
   every unit's per-tick state, as columns, with a GDScript backend and a Rust one.
5. **Structural support as events** (the user's aside on Decision 57). Supports are
   checked only where a block changes: when a block is removed, damaged or loaded. A
   collapse is reported as an event to be stored and read back. No tile is checked every
   tick.
6. **Scenario benchmarks with spreads** (the other agent). The checks report the 95th and
   99th percentile tick times as well as the mean, for named scenarios: a march, a
   melee, a rout and many waves. Today's `scaling_check` reports means only. The
   sim-checks workflow runs these on GitHub, not in an agent's session.
7. **Tactical roles** (the other agent). A formation assigns roles to its groups (front,
   reserve, flankers, rescuers) and units act within them. This meets Decision 133's
   commands and Decision 134's spare units, and belongs with spec 30 round 3's part 9
   (seeking).
8. **Impulses** (the other agent; spec 32). Knock-back and blasts follow these rules:
   - They are collected as events during a tick and resolved in one pass, so two
     impulses on one body in the same tick add by a fixed rule.
   - A fast body sweeps its path, so it can't tunnel through a wall.
   - A thrown body is airborne with simple ballistics and lands as an impact (Decision
     137).
   - Discipline sets how fast it recovers.

   These rules go into spec 32 when it is next opened.

## Declined

- **Staggering decisions by unit id.** Breaks Decision 97. Decision 138 staggers by wits
  and a seeded phase instead.
- **Level of detail.** Declined in Decision 131: every unit is simulated in full.
- **Weighted-sum steering.** The present model, in which a unit holds a place and steps
  aside, keeps outcomes exact and replayable. A weighted sum of forces would trade that
  away.
- **A higher tick rate.** It changes outcomes and costs more per second.
- **An ECS library or ORCA crowd library.** These would add a dependency outside the
  identity gate (Decision 129). The data layout above gets the same cache benefits.

## Order

Item 1 comes first, since it fixes a class of bug. Then item 2, which removes most of
the repeated grid searches. Items 3 and 5 wait for spec 29's terrain work. Item 6 goes
into the sim-checks workflow when it next changes. Items 7 and 8 follow their specs.

## Measured, 2026-10-14

Four stacks of 1,024 a side (`tick_phases engine=rust side=4096 squads=4`) on the user's
i7-12700K ran at **520 ms a tick**. A tick is 0.1 s, so real time needs under 100 ms. The
one-stack figure was 51 ms. Phases, in ms:

| Phase | ms | Phase | ms |
|---|---|---|---|
| shuffle | 156 | recovery, strays | 21 |
| wounds (tend) | 64 | scrum walk (steering) | 20 |
| slot search | 62 | stamina | 19 |
| perception | 33 | bodies snapshot | 19 |
| bodies (UnitBodies) | 32 | units walk to places | 18 |
| fight (blows) | 26 | carry | 9 |
| deaths (bury) | 24 | engage, prepare, stance | 9 |

- **#200:** made the shuffle's cost no longer grow with units × columns for a squad that is
  re-forming, and cut the cost of perception's pairing. The run above came before #200; its
  saving is still to be measured.
- **Wounds and deaths:** the one-stack snapshot had few downed units, so its small costs for
  wounds and deaths aren't a fair comparison. Measure them at the same point in the fight.

**After #200 and #202** (bench run 38643017654, both on one CI runner, about 1.2× slower
than the user's PC): **628.5 → 407.0 ms a tick.** Shuffle 209 → 37, wounds 64 → 26,
perception 32 → 17. Slot search (89) and bodies (43) are now the largest phases; the
shuffle's parts are each under 14 ms.

**Slot search's parts** (bench runs 38645791317 and 38647406879): the core 33.6 ms; its
GDScript glue about 45 ms (gathering 24.8 after #207, sync 11.0, write-back 9.5). The
glue that is left is per-unit copying, which only goes when the field owns the units'
state (item 4; `OPEN_QUESTIONS.md` Q20). It must go eventually.

**Spec 35 step 0, the unit store's skeleton** (#210; bench run on one CI runner, Xeon
8573C, against main 1cc4412): **422.8 → 430.1 ms a tick** (+7.3, 1.7%). No phase moved
by more than 1.7 ms (walk 30.8 → 32.5, units walk to places 22.2 → 23.9); the rest are
within about 0.7 ms either way. The step adds no work to any pass, only the store's
settle at the tick's end (a sum over the squads, and a walk of the lists on a tick a unit
left), so most of the difference is likely run-to-run noise. Nothing is saved until
step 4.

**Spec 35 steps 2a and 2b, the entries' keys in the unit store** (bench runs 38729571324
and 38736555180, each against its base on one CI runner; the two runners differ, so
compare percentages, not milliseconds):
- **2a, the GDScript store** (#220; AMD EPYC 7763, against main a5c8239): **398.8 → 411.6 ms
  a tick** (+12.8, 3.2%). The seek's gather fell 26.1 → 21.6 and its write-back 10.6 → 7.8
  (columns read once instead of each entry's dictionary). Units walk to places rose
  26.4 → 30.1, bodies 40.7 → 43.8 and recovery 26.2 → 28.2: scattered `put` and `value`
  calls, or run-to-run noise.
- **2b, the Rust backend** (#221; AMD EPYC 9V74, against main 7722136): **304.6 → 327.8 ms a
  tick** (+23.2, 7.6%). Bodies rose 32.1 → 36.7, units walk to places 22.0 → 24.6, the walk
  22.3 → 24.8 and blows 18.4 → 20.9. Under Rust each scattered `value` or `put` of an
  entry's key is now a call into the field, and each column fetched is one copy out of it.
  Perception (12.9 → 14.8), which reads no entry key, also rose, so part of this is
  likely noise.
- **The trajectory:** steps 1 to 3 add boundary crossings and remove none. The glue goes
  at step 4, when seek, walk and facing read the columns natively. Expect the series to
  cost about 10% until then; step 4's bench is what has to pay it back.

**Spec 35 step 4 and its fix, the scrum's passes on the unit store** (bench runs 38759449023,
38770814451, 38773725302 and 38773730557, `tick_phases engine=rust side=4096 squads=4
fight=3`; runners differ between runs, so compare percentages):
- **4a alone** (#224, rank and column in the store; against main 7733998): **370.2 → 453.9 ms
  a tick** (+22.6%). Deaths 25.9 → 54.5, blows 21.0 → 45.3, the re-form's occupancy 5.1 →
  12.8, units walking to places 27.7 → 36.0: each scattered place read was two calls into
  the field.
- **4a to 4e** (#224 to #228; against 7733998): **291.3 → 299.8 ms** (+2.9%). The scrum's
  glue went: the seek's gather 13.8 → 1.4, its sync 6.6 → 3.8, its write-back 5.4 → 3.7
  (now the walk's batch), the walk 21.0 → 5.4, the facing 1.8 → 1.2. 4a's cost stayed.
- **Places read once a pass** (#229; against #228's head): **315.9 → 283.1 ms** (−10.4%).
  Deaths 41.2 → 22.9, blows 33.3 → 18.5, the re-form's occupancy 10.3 → 6.3.
- **Step 4 with the fix** (#224 to #229; against 7733998): **353.2 → 281.6 ms** (−20.3%). Every
  phase 4a slowed is back within a millisecond of main or below it (deaths 21.9 → 22.7, the
  re-form's occupancy 5.2 → 6.2, its two tries 20.4 → 21.8); the scrum's walk (27.8 → 6.4),
  the seek's gather (17.5 → 1.4) and bodies (40.8 → 33.0) carry the gain.
- **With Decision 144's write-backs** (#229's head 64becd0, bench run 38781595574; against
  7733998, on a slower runner, AMD EPYC 7763): **378.2 → 322.1 ms** (−14.8%). Body parting
  40.5 → 23.6 (its write-back now the field's own) and units walking to places 28.1 → 16.9
  (the march's write-back committed once). Phases step 4 didn't touch came in a few percent
  above main on this runner (perception 15.9 → 17.3, stamina 24.0 → 25.4, recovery 24.9 →
  26.1), as did deaths 26.1 → 29.8 and the re-form's occupancy 5.6 → 7.5, which builds a place
  map as it goes. A second run (38783113054, AMD EPYC 9V45): **252.3 → 218.6 ms** (−13.4%), with
  the same phases above main (deaths 17.2 → 19.4, stamina 16.7 → 18.5, recovery 16.9 → 18.1,
  the re-form's occupancy 3.4 → 4.9): not noise. Spec 35 step 5b measures and fixes it.

**Spec 35 step 6, one typed view onto the store** (#239 and #240; bench runs 38980543066 and
38980545149, `tick_phases engine=rust side=4096 squads=4 fight=10`, 5 alternating rounds,
medians; `*` = the rounds' ranges don't overlap; runners differ between the two runs, so compare
percentages):
- **6b against 6a** (#240's head fc7d473 against main fe6e240; AMD EPYC 9V45): **271.1 →
  271.4 ms a tick** (+0.1%, within noise). The two phases 6b moved onto read views are faster:
  the bodies snapshot **13.85 → 10.93** (−21.1%*) and engage, prepare and stance **12.12 →
  10.29** (−15.1%*). The walk to places, which now holds its points and bearings for the
  pass, didn't move (15.05 → 15.15, +0.7%). No phase is slower beyond noise: blows (22.65 →
  25.75, +13.7%) and carry (8.03 → 8.67) swung within overlapping ranges.
- **The series: spec 35 so far against `1cc4412`, the last commit before step 0** (fc7d473
  against 1cc4412; Intel Xeon 6973P-C): **380.4 → 306.1 ms a tick** (−19.5%*). The largest
  gains: the scrum's walk 20.24 → 4.28, the seek's gather 15.28 → 2.65 and its sync 10.63 →
  4.61, body parting 41.23 → 22.23, deaths 17.24 → 11.55, the walk to places 21.35 → 17.94,
  blows 31.30 → 27.93.
- **Still slower than before step 0 (Q23):**
  - **The rout: 47.08 → 55.06** (+16.9%*), the only large one. #237 and 6a held its columns,
    but it stays above its pre-store cost: its loops now index typed arrays, and what remains
    is for Q23 to find.
  - **The shuffle's free front place: 5.96 → 6.55** (+9.9%*). It reads no store state
    (spec 35 step 6b), so the cause isn't the store.
  - **Engage's fronts: 2.44 → 3.16** (+29.5%*).
  - **The shuffle's moves under way: 0.37 → 0.47** (+27.0%*).
  - The chasers (0.02 → 0.04) are too small to read.
  - The bodies snapshot (14.80 → 13.18) and engage (13.99 → 11.82), which the series bench
    found slower before step 6, are now faster than before step 0.

**Spec 35 step 7a, a unit's state through one writer, with each squad's living and downed kept**
(#247; bench run 39026873513, `ce6eabf` against main at `a941e7d`, `fight=10`, `rounds=5`, AMD
EPYC 9V74): **282.4 → 249.6 ms a tick** (−11.6%*). The step's targets: carry 7.24 → 1.57
(−78.3%*), taking the downed 0.91 → 0.06 (−93.4%*), recovery 20.35 → 16.18 (−20.5%*), wounds
21.43 → 19.60 (−8.5%*). `living()` costing nothing to ask also shows in passes that call it
often: perception 11.15 → 5.98 (−46.4%*), blows 22.05 → 15.88 (−28.0%*), engage 10.91 → 9.49
(−13.0%*), groups 1.16 → 0.65 (−44.0%*), morale 1.18 → 0.87 (−26.3%*), the shuffle's
front-first order 1.59 → 1.12 (−29.6%*) and the rout 52.70 → 49.81 (−5.5%*). No phase is
slower beyond noise: the walk to places (+2.0%) and the free front place (+1.3%) have
overlapping ranges. An earlier run (38993065664, `4d187a6`) came before a fix that
changed outcomes (the trade's places), so it isn't counted.

**Spec 35 step 7b, the downed's and recovery's fields in the store** (#248; bench run
39030647717, `cee9142` against main at `356bb76`, `fight=10`, `rounds=5`, AMD EPYC 9V45):
**232.5 → 225.9 ms a tick** (−2.8%, ranges overlapping). Carry 2.11 → 1.50 (−28.9%*), though
7b doesn't change carry's code; recovery 17.09 → 16.68 (−2.4%), blows 18.34 → 17.26 (−5.9%)
and bury 10.70 → 10.39 (−2.9%), all within noise. No phase is slower beyond noise: the free
front places (+7.7%), morale (+2.5%), stamina (+0.8%) and the rout (+0.7%) have overlapping
ranges.

**Spec 35 step 7c, the rest of a unit's per-tick fields** (#251; bench run 39033559815,
`e0231cd` against main at `ad66bb6`, `fight=10`, `rounds=5`, AMD EPYC 7763): **329.1 → 332.2 ms a
tick** (+0.9%, ranges overlapping). No phase moved beyond noise.

**Spec 35 as a whole (Q23, issue #278)** (bench run 39043389452, `e0231cd` against `1cc4412`, the
last commit before step 0, `fight=10`, `rounds=5`, AMD EPYC 7763): **428.0 → 335.3 ms a tick
(−21.7%*).** The store paid for itself. Largest gains: the seek's gather 19.19 → 2.05 (−89.3%*),
carry 9.16 → 2.12 (−76.9%*), the scrum walk 20.22 → 5.60 (−72.3%*), the seek's sync 11.92 → 4.84
(−59.4%*), perception 13.73 → 7.51 (−45.3%*), bodies 42.95 → 26.07 (−39.3%*), blows 32.45 → 22.12
(−31.8%*), deaths 20.87 → 15.60 (−25.3%*). Still slower than before step 0: **the rout 62.54 →
72.32 (+15.6%*)**, the shuffle's free front place 8.40 → 10.04 (+19.5%*), engage's fronts 2.96 →
3.40 (+14.9%*), the shuffle's pass-through 9.54 → 10.12 (+6.1%*). Spec 38 step 3 takes the rout.

**The tick later in the fight** (bench runs 39043391384 and 39043393472, `e0231cd`, one round
each, AMD EPYC 9V45): at `settle=40 fight=10` **466.5 ms**, at `settle=100 fight=10` **385.5 ms**,
with **regroup 177.3 and 186.1 and groups 131.4 and 131.2**. Both read whole-battle columns once
per squad or per pair of squads. The bench read until now (10 ticks after contact) never showed
them. Decision 150 and spec 38 step 1.

**Spec 38 step 1a, groups bounded** (#286, bench run 39685116478 against its base, 5 rounds,
`settle=40 fight=10`, AMD EPYC 7763): tick 782.4 → 787.7 ms, **groups 208.0 → 209.4 ms** and
regroup 327.3 → 326.3, every range overlapping. Forming up wasn't where the phase's time went:
`GroupSplit.step`, timed in the same phase, copied POSITION twice and the places once for every
group at ease. Spec 38 step 1c reads them once a pass.

**Spec 38 steps 1b and 1c** (bench runs 39691303604, 39693763983 and 39693767164, each
against its base, 5 rounds, AMD EPYC 7763): no change. At `settle=40`, 1c moved the tick
781.9 → 789.7 ms, regroup 326.5 → 324.5 and groups 208.8 → 209.9. At `settle=100`, round 1 moved
the tick 648.3 → 648.0, regroup 342.0 → 340.3 and groups 204.8 → 207.6. 1b's round 1 moved
regroup 340.9 → 335.4. Every range overlaps. The per-squad copies were not the cost. Spec 38,
step 1c, "Measured", says what is left and why the step stopped.

**Next, in order:** spec 38 (`specs/38-tick-performance.md`), which replaces the list below.
Was: slot search (measure its parts first: field sync, gathering, the core,
write-back); body parting; wounds' port; blows; deaths; recovery; stamina; steering; the
walk. Each is a small PR that must reproduce the reference
digests (sim-checks). Long runs go to the `bench` workflow (`.github/workflows/bench.yml`,
`workflow_dispatch` with `tool`, `args` and `base`) or to the user's PC, never to an agent
session.

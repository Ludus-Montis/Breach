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

**Next, in order:** slot search (measure its parts first: field sync, gathering, the core,
write-back); body parting; wounds' port; blows; deaths; recovery; stamina; steering; the
walk. Each is a small PR that must reproduce the reference
digests (sim-checks). Long runs go to the `bench` workflow (`.github/workflows/bench.yml`,
`workflow_dispatch` with `tool`, `args` and `base`) or to the user's PC, never to an agent
session.

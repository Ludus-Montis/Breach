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

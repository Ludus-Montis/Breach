---
doc: principles/bounded-work
status: active
applies_to: every simulation or rules-engine source file that runs per tick, and every design of such a system
---

# Bounded Work

## Why this exists

A simulation that is fine at forty units can be unusable at four thousand. The cause is
almost never the language. It is work that grows faster than the world: a pass that
compares every thing with every other thing, a search with no edge, a check repeated
every tick on things that haven't changed. Each looks harmless when written and turns
into the whole tick's cost later. In this kit's first project, uncapped searches cost
pathing most of its speed, and five such passes cost a mass battle most of its tick:
one of them cubic, hidden in an array erased from inside a loop. So the rule is
written down here, checked on every design, and measured in CI.

## The three principles

### 1. Every per-tick query is local and bounded

What a thing looks at each tick is limited by distance or by count, never by the size
of the world.

- Find neighbours through a spatial index (a grid of cells, or a sorted structure), with
  a fixed radius or a fixed number of rings. Never scan every unit, body or squad for
  each unit.
- Searches have an edge: a planner stops at what is seen, a look-ahead at its distance,
  a ring search at its last ring. The edge is a tuned number, not "until found".
- Work that is the same for a whole group is done once for the group, not once per
  member (a squad's discipline, pace or leadership; a seeded draw used as a sort key).
- Hidden costs count: `erase`, `has`, `find`, `duplicate`, `filter` and `map` on an
  array are each a scan. Inside a loop over the same things they make it quadratic.
  Use a dictionary as a set, or mark things in a packed array.
- Comparing all pairs is allowed only where the count is bounded by design (the squads
  in one engagement, the slots of one unit), with that bound written beside it.

**Test:** If the battle doubles, does this pass at most double, or n log n at worst?
What bounds each loop: a radius, a count, or the size of the world?

### 2. Work follows change

A thing whose state hasn't changed doesn't need checking again. Where most of the world
stands still most of the time, let what changes say so, and check only what it touches.

- When something changes, it records the change (an event, or an entry on a work list):
  a tile dug out queues the tiles it supports; a unit falling queues its group to be
  checked for a split; a squad moving marks its cells dirty.
- Each tick handles only its work list, in a deterministic order (the list is ordered by
  position or seeded draw, never by when entries were added if that is creation order).
- The same records serve the log: what changed and why is kept for replay, debugging and
  analysis, instead of being reconstructed by scanning.
- Correctness first: whatever can change a checked result must queue the check. If that
  can't be made exact, keep the full check and bound it by Principle 1.

**Test:** Does this pass look at things that cannot have changed since it last looked?
If it skipped them, what would have to queue them, and does it?

### 3. State is read where it lives, once a pass

Where per-thing state lives across a boundary (a native store, a column of a packed
array held by another object), every scattered read or write of one thing is a crossing.
A loop that crosses once per thing pays the boundary for every thing, every tick. In
Breach's first native ports, rank and column read unit by unit through a helper cost a
22.6% slower tick, and fetching each column once a pass bought it all back.

- A loop over things never reads or writes the store one thing at a time. It fetches the
  columns it needs before the loop, indexes them by the thing's slot, and commits what it
  changed once after.
- Fetch into the cheapest form the language indexes. In GDScript only a typed array's
  index is cheap; a method that reads a held copy costs as much as the crossing. Breach
  holds a pass's columns as one typed view (Decision 146), and a helper that reads one
  thing's state takes that view; its scattered form, for a one-off, has a name of its own.
- Scattered reads are for events (a step-up, a join, a spawn) and a handful of things, or
  a reference implementation the fast path replaces, each marked with its reason. "The
  bench shows it doesn't matter" is not a reason: Breach's rout was measured before any
  unit routed and was 30 ms out.
- A hold on the store never spans a call into the native code that reads it.
- A fetched column is a copy as of now: a pass that writes and reads again fetches again,
  or works on its copy and commits it.
- A pass that moves state to where another pass reads it says so in the same change:
  every reader of the moved field is listed and given its fetch-once plan before the
  field moves, not after the bench finds it.

**Test:** Inside this loop, does anything call into the store per thing, directly or
through a helper? What does the bench say this pass costs against its base?

## Where this is checked

- **At design time:** `rubrics/spec-baseline.rubrics.md`'s `bounded-work` row,
  answered in the specification or Decision.
- **At run time:** `rubrics/run-baseline.rubrics.md`'s `bounded-scaling` row: the
  stack's scaling check doubles the battle and fails a doubling that costs more than the
  limit (Breach: `tools/scaling_check.gd`, run in CI).
- **Per change (Principle 3):** Breach's `ci/godot/scripts/check_store_reads.py` flags a
  scattered store call written inside a loop; `bench` (`tick_phases`) against the base
  catches one hidden in a helper.
- **When a pass is slow:** time it phase by phase (Breach: `tools/tick_phases.gd`), then
  read it for unbounded loops and hidden scans before porting it to faster code. A port
  of a quadratic pass is a faster quadratic pass.

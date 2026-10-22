---
spec_type: code
status: active
parent_spec: ../Breach — Reverse Tower Defense Design Spec.md
---

# Spec 39: The simulation in Rust

## Purpose

Move all of `sim/` into the Rust crate under `native/rust/`, rules included, so Godot only
sends commands, steps the simulation once a tick and draws what it reads back. The case is
spec 38's column count: of a 784 ms tick at `settle=40 fight=10`, the store's copies cost
about 2.5 ms and the native calls about 23 ms. The rest is GDScript running the rules for each
unit, and regroup and groups are about 70% of it (issue #292, Q33).

The other aims, in the user's words: "avoid spinning full sims when isolating issues within a
tick" and "more time-efficient unit tests". The path is the one that is fastest for sign-offs,
CI time, writing and usage.

**Settled by Decisions 152** (the whole simulation in Rust, the path), **153** (the crate owns
the content schema, and the designer writes JSON in it) **and 154** (cargo tests, one phase on
a built state, snapshots cut from the digests). Decisions 97, 130, 138, 139 and 151 still hold.
Answers #292 (Q33). Takes over spec 38's steps 3 and 4, and spec 38's step 5 (threading, slot
packing, Q26 #281) as step 10, after step 8. Q24 (#279) and Q32 (#291) stay open: they are
rule questions, and this round ports the rules as they are.

## Status

Agreed in the design session of 2026-10-22. Before step 1, spec 38 step 2b is built (the
outcome check on Rust), because every step here retires GDScript. Spec 38's steps 1d and 2 are
dropped. Their benches would time GDScript that this round deletes, and step 2's whole-battle
series is rebuilt under cargo in step 8. Step 10 (spec 38's step 5, moved here the same day)
needs a design session before it is built.

Each step is built with `/run-phase` as "spec 39 step N", as one PR. A step too large for one
session splits into lettered PRs (4a, 4b). Each lettered PR keeps the rules below, and the
session records the split here.

## Settled

- **Every port is identical.** A step's Rust reproduces the GDScript bit for bit on the digests,
  both engines, on its own commit (Decision 129's gate). The same PR then deletes the ported
  GDScript and its gdUnit tests (Decision 151). It changes no outcome, carries no
  `Outcomes-changed:` trailer and needs no Gate 2 sign-off: green CI is its proof.
- **No rule changes ride a port.** A bug found while porting is ported as it is and raised as an
  issue. It is fixed after, in its own PR, with the trailer.
- **The crate's simulation has no Godot types.** One binding module (today `lib.rs`) is all that
  imports `godot::`. Tests, benches and the digest never start Godot.
- **Squads move into the crate first** (step 1), as the units did in spec 35. The GDScript
  `SkirmishSquad` becomes a view over the crate's squad, so the ports after it read squads in
  place, with no glue that is written and then deleted.
- **Ports go in chunks of related phases,** in the order below. The order puts the mover first
  (every later chunk moves units) and the biggest costs (regroup, groups) next.
- **Content (Decision 153).** The crate's serde structs are the schema, and a generated JSON
  Schema is what the designer validates against. The simulation's definitions (units, weapons,
  armour, tuning, terrain, maps) are JSON files in that format. GDScript's
  `content/definitions/` objects are filled from what the crate loaded until their last reader
  ports.
- **Tests (Decision 154).**
  - Each step writes its scenarios as cargo tests, red first, on builders under
    `native/rust/` shared with benches.
  - A test builds its state and runs only the phase under test.
  - The reversed-lists cases become tests that build the same state with its lists reversed
    and require the same result.
  - Snapshot tests load a state the digest wrote, step one phase and compare fingerprints.
- **Order and determinism.** `DetMath` and the fingerprint already exist in Rust
  (`det_math.rs`, `ghash.rs`). No `HashMap` iteration decides anything: maps that are iterated
  are ordered (`BTreeMap`, or `Vec` by slot). No fused multiply-add (Decision 130).

## Steps

### Step 1, the squads in the crate, and the `rust` CI job

- **The squad store:** each squad's fields that a pass reads or writes (`skirmish_squad.gd`:
  state, order, route place, morale, members by slot, and the rest) live in the crate. The
  layout follows the unit store's (columns by squad slot, Decision 146). `SkirmishSquad` keeps
  its API and reads through the store. The implementing session lists every field, where it
  goes and its readers here before moving it (Decision 144's rule for moving a field).
- **The state serialises:** the crate writes and reads its whole state (units, squads, the
  tick, the seeds) through serde. `formation_digest.gd` gains `snapshot=<dir> at=<ticks>`.
- **CI's `rust` job:** `cargo fmt --check`, `cargo clippy -- -D warnings` and `cargo test`, on
  every PR that touches `native/`.
- **`CLAUDE.md`:** the test rule names `cargo test <name>` as the in-session red-then-green
  run, alongside `-a res://tests/<file>.gd`.
- Outcomes unchanged: the digests match `main`.

### Step 2, content in the crate's schema (Decision 153)

- The crate's content module: serde structs for units, weapons, armour, tuning, terrain and the
  map layout, a loader that names the field of any file that doesn't match, and a generated
  JSON Schema checked in under `content/schema/`.
- The `.tres` simulation definitions convert once to JSON (a tool, run once, then deleted).
  GDScript's definitions are filled from the crate's loaded content. A test fails if a `.tres`
  definition is still read.
- The designer exports the schema's format and validates against the JSON Schema
  (`tools/designer/`, `loadFromExport`). The map importer under `content/import/` keeps only
  what the presentation still needs.
- Outcomes unchanged.

### Step 3, the ground and the mover

The ground (`FormationTerrain`, `sim/ground/`), routes (`FormationRoute`), the one mover
(`UnitMover`, `UnitSteer`, Decision 139), pathing (`FormationPathing`) and the march
(`FormationMarch`, `FormationWalk`, `FormationSweep`, `FormationShuffle`, `FormationStaging`).
Spec 34 items 3 and 5 and Q12 (#268) are carried as they are: terrain does today what it does.

### Step 4, the scrum, regroup and the pursuits

`FormationScrum` and everything it calls: the stance, the manoeuvre, seek, the walk, facing,
blows, the contest, the trade, entries and ranks, the locks, then regroup (`ScrumRegroup`, its
walk back through the scrum walk's kernel), the pursuits, withdraw, fronts, edges, contact and
perception. The largest step. It is expected to split (4a the scrum core, 4b regroup onwards).

### Step 5, groups and the split

`FormationGroups`, `GroupSplit`, strays (`FormationStrays`), compaction (`SquadCompact`) and
rendezvous.

### Step 6, the downed and stamina

Deaths, wounds, recovery, carrying, taking, stamina and the bodies' parting (`UnitBodies`).

### Step 7, morale, the rout and orders

`FormationMorale`, `FormationRout` with its flight, crush, rally and hold, and orders and their
events. After this step no battle phase runs in GDScript.

### Step 8, the tick in Rust, and the tools under cargo

- `FormationSimulation.tick` moves to the crate: one call a tick from Godot, the events handed
  back as data. The two-engine digest comparison and `BREACH_NATIVE` retire with the last twin.
- The digest, the trials, `scaling_check` and the benches (`tick_phases` included) become
  cargo binaries that CI runs in release mode. `tick_phases` gains spec 38 step 2's series,
  `terrain=mixed` and per-pass changed counts. `check_outcomes.py` and the outcome trace (spec
  37) read their output unchanged.
- Snapshot tests for every phase, cut from the digest's battles.

### Step 9, the rest of `sim/`

The lanes, the economy, structures, the clock and commands. After it, `sim/` holds no GDScript,
and `content/definitions/` and `content/import/` keep only what presentation reads.

### Step 10, threading, then slot packing (design first)

Moved from spec 38 step 5 on 2026-10-22. It needs step 8, when the whole tick runs in Rust and
the benches run under cargo; it does not need step 9. Threads help only work in Rust: with
about 50 ms of the tick native before this round, threads on every kernel would have saved
at most about 35 ms (Amdahl's law). Slot packing, too, speeds up only native passes.

**Subjects for its design session:** Q26 (#281) as written, measured with step 8's
`tick_phases` (the series, `terrain=mixed` and per-pass changed counts). Each needs a
Decision the user authorises. Decision 151 reopens two options Q26 ruled out only because
they broke identity with the reference: `f32` columns and parallel reductions in a different
fixed order. Both change outcomes but not behaviour, so each is allowed under that
Decision's conditions.

## Scenarios

### Step 1
- Given any digest battle, when it is run with squads in the crate, then every fingerprint
  equals `main`'s.
- Given a state written as a snapshot and read back, when one tick is stepped from each, then
  they are identical.

### Steps 3 to 7 (each port)
- Given the step's commit, when the digests run under both engines, then they are identical.
  After the GDScript is deleted, the Rust digests equal the base's.
- Given a state built with the step's builders, when the step's phase runs alone, then each
  former gdUnit scenario's outcome holds.
- Given the same state with its squad and unit lists reversed, when the phase runs, then the
  result is the same.

### Step 2
- Given a content file with a field of the wrong type, when the crate loads it, then the load
  fails naming the file and the field.
- Given the designer's export of a map, when the crate loads it, then it equals the map loaded
  from the checked-in file.

### Step 8
- Given a snapshot at tick N and the fingerprint after one phase, when the test steps that
  phase, then the fingerprints match. With an intended outcome change, regenerating the
  snapshots changes only those whose battles changed.

## Test-first order

1. Step 1: the snapshot round-trip test and the squad store's unit tests, red, then green.
   Then the digest match.
2. Step 2: the loader's error test and the designer round trip, red, then green.
3. Steps 3 to 7: each chunk's cargo scenarios, red (no Rust phase yet), then green, then the
   digests identical, then the GDScript and gdUnit files deleted.
4. Step 8: the snapshot test, red, then green; the cargo digest's output compared with the
   Godot digest's on `main` before the Godot one retires.

## Rubric answers

- **single-noun-phrase:** the simulation in Rust.
- **ocp-extension-point:** a phase is a function over the crate's state. Ports add phases and
  never branch on engine.
- **lsp-contract-scope:** not applicable; no subtypes.
- **isp-fit:** each phase takes the state and setup it reads, not the whole simulation.
- **dip-direction:** the simulation crate depends on nothing in Godot. The binding depends on
  it.
- **general-over-special:** general: one rule for where the simulation runs (Decision 152).
- **order-independent:** unchanged by construction (identical ports). The reversed-lists tests
  carry over, and no iterated map is unordered.
- **bounded-work:** no per-tick work is added. Each pass keeps its bounds, and Decisions 144 and
  146 apply only while a pass is still GDScript.

## Built

Nothing yet.

## Open, for the implementing session to settle and record here

- The squad store's layout and which fields stay per-squad objects (step 1).
- The snapshot's on-disk format (serde JSON or a compact binary) and where snapshots live, kept
  small enough for the repo (step 1, used in step 8).
- A chunk's boundary may move when a dependency demands it. The session records the move here.

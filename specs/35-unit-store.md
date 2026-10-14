---
spec_type: code
status: active
parent_spec: ../Breach — Reverse Tower Defense Design Spec.md
---

# Spec 35: The unit store

## Purpose

Spec 34 item 4, "data as arrays", as a build series. Each unit's per-tick state moves
out of `SkirmishUnit` and its squad's `loose` and `fleeing` dictionaries into one store
held as packed columns. The store has two backends: GDScript, which is the reference, and
the native `BodyField`. Native passes then read and write the state where it lives, so
the GDScript glue that copies units in and out every tick goes. Today that glue is about
45 ms of slot search's 79 ms at four stacks of 1,024 a side (`OPEN_QUESTIONS.md` Q20),
and the same copying recurs in the walk, the facing and the other passes.

**Settled by Decision 142.** Read it by its line in `DECISIONS_INDEX.md`. Decisions 129
(the identity gate), 131, 132 and 139 (the one mover) still hold.

## Status

Agreed in the design session of 2026-10-14. Nothing is built yet. Build one step at a
time with `/run-phase`, as "spec 35 step N". Each step is its own PR.

## The store

- **`UnitStore`** (`sim/skirmish/formation/unit_store.gd`) is the one interface. It has
  two backends: `UnitStoreGd`, the reference, and `UnitStoreNative`, which wraps the
  battle's `BodyField`. `NativeKernels`' switch picks the backend, as it picks the engine
  today. `FormationSimulation` owns one store per battle. Under Rust it replaces
  `_field`'s role as the only owner of unit state.
- **Slots.** Each unit has a dense slot, given when the unit enrols. A slot is freed
  only when the unit is in no list that the simulation or the digest reads. Free slots
  are reused lowest first. A slot is **storage only**: no loop runs in slot order and no
  tie is broken by slot (Decision 97). Passes keep their squad and member order. Nothing
  keeps a slot across ticks except the unit's record.
- **Columns.** `column(name)` returns a packed array indexed by slot. Under GDScript
  this costs nothing; under Rust it is one copy. `commit(name, array)` writes a column
  back. A pass asks once for each column it reads and commits only the ones it wrote.
  `get(slot, name)` and `set(slot, name, value)` serve scattered use. A native pass reads
  and writes the columns inside Rust and never crosses the boundary for them.
- **What moves** (Decision 142). Today's fields are mirrored one to one, and none is
  merged in this series:
  - Where a unit stands: `position`, the loose entry's `at` and the fleeing entry's
    `offset`. A flag bit says whether a unit has a loose entry, and another whether it
    has a fleeing entry.
  - Every other key of the loose and fleeing entries: `next`, `goal` (as two columns,
    `goal_foe` and `goal_slot`, the latter -1 for none), `toward`, `foe_at`, `touch`,
    `along`, `stuck`, `caught`, `held_by`, `fanned`, `closest`, `unit` and any others
    step 2's inventory finds. A key that may be absent or null needs a presence bit, as
    well as its value, because the digest hashes which keys are present.
  - `bearing`.
  - `hp`, `attack_cooldown`, `stamina`, `speed`, the exertion fields (`exerted`,
    `breather`, `run_build`, `was_running` and the rest that change in a tick) and the
    unit's flags (alive, down, `state`).
- **What stays** on `SkirmishUnit`, as the unit's record: id, stats, traits, definition,
  command and origin, rank, column, footprint. If step 7's inventory finds a field that
  changes within a tick and isn't listed above, it moves too: the rule is "per-tick state
  moves".
- **Removed, not forwarded.** A field leaves `SkirmishUnit` in the PR that moves it, and
  every reader switches to a column or `get` in that same PR. Events carry values (id,
  point, damage) rather than a unit to read later. Presentation reads one column snapshot
  per frame. These PRs may change more pre-existing files than the OCP check's 11
  (Decision 142, the user's waiver). Each PR states its count and cites Decision 142.

## Identity

- The three reference hashes (standard, clash160 and wide in `native/README.md`) **never
  change** in this series.
- `tools/formation_digest.gd` reads units through the store, with one code path for both
  backends. It rebuilds today's bytes exactly, including the loose and fleeing entries as
  dictionaries with the same keys present.
- Each field moves in two steps, which may be two PRs or two commits of one PR:
  1. The GDScript backend owns it. Digests are unchanged on the GDScript engine.
  2. The Rust backend owns it. Digests are unchanged on both engines.
- `tests/sim/skirmish/formation/test_unit_store.gd` steps a battle on each backend and
  compares every column after each of several ticks. On a difference it names the first
  field, unit and tick.
- CI `sim-checks` is the gate: the suite under both engines, and all three digests
  reading `same`. The session runs only the test files it changed.

## Steps

Each step ends with a `bench` run (`tick_phases engine=rust side=4096 squads=4 fight=3`)
against its base. The numbers go in the PR and in spec 34's "Measured" section.

0. **The skeleton.** `UnitStore` with both backends: slots, enrolling and dropping, and
   no columns yet. The digest reads unit order and ids through it. `BodyField`'s roster
   becomes the native backend's slots: today it maps unit ids to body slots already.
1. **Where units stand.** `position`, `at` and `offset`, and the loose and fleeing flag
   bits. The mover (Decision 139) and body parting are the only writers, so the
   placement check now looks for writes to these columns. `BodyFieldSync`'s points go.
2. **The entries' other keys.** Inventory every key of `loose` and `fleeing` and move
   them. `squad.loose` and `squad.fleeing` stop holding per-unit data. If a squad still
   needs "which of my units are loose", it reads the flag column.
3. **Bearing.**
4. **Seek, walk and facing on the store.** `ScrumSeekField`, `ScrumWalkField` and
   `ScrumFaceField` stop gathering units and writing back. What the core needs from the
   unit's record (speed, rank, column, footprint, preferred position, turn rate,
   backward pace) is enrolled once or read from columns. What the rules still decide per
   squad (frame, foes, chasers, front band) is sent once per squad. `BodyFieldSync` goes,
   except for enrolling.
5. **hp and attack cooldown.** Blows (`ScrumBlowsField` and the GDScript blows), deaths,
   wounds and taking.
6. **Stamina, speed and exertion.** The stamina and recovery passes.
7. **Flags and the rest.** Alive, down and the unit's state. Then inventory
   `SkirmishUnit` for any per-tick field that is left.

Steps 1 to 3 change outcomes nowhere. The ported passes keep calling into Rust as today,
but read from the store instead of a sync. The glue is gone at step 4.

## Scenarios

- **Given** a battle on the GDScript engine, **when** a step moves a field to the
  GDScript store, **then** every digest set's per-tick file is unchanged.
- **Given** the same battle on the Rust engine, **when** the field moves to the native
  store, **then** the per-tick files equal the GDScript ones.
- **Given** two backends stepping the same battle, **when** `test_unit_store.gd`
  compares their columns at several ticks, **then** every column matches.
- **Given** a battle whose squad and unit lists are reversed (the reversed-lists tests),
  **when** units get different slots, **then** every tick's state is the same.
- **Given** units dying and new waves enrolling, **when** slots are freed and reused,
  **then** no unit reads another's state. The column test covers a battle with deaths and
  a later wave.
- **Given** a field moved in a step, **when** the tree is searched, **then**
  `SkirmishUnit` no longer declares it and no file reads it from a unit.

## Test-first order (per step)

1. A red test in `test_unit_store.gd` for the step's columns: the backends compared, and
   `get` and `set` round-trip.
2. The GDScript backend owns the columns. Run the step's touched test files plus
   `test_formation_list_order.gd`. CI checks the GDScript digests.
3. The native backend owns them (`cargo test` for any new Rust maths runs in CI). CI
   checks both engines' digests.
4. Remove the fields from `SkirmishUnit` and switch every reader in the same commit.
5. Start a `bench` run and record it.

## Rubric answers

- **single-noun-phrase:** the unit store.
- **ocp-extension-point:** later passes plug into the store's columns. The series itself
  is a broad change, waived by Decision 142.
- **lsp-contract-scope:** both backends honour one contract, which `test_unit_store.gd`
  holds them to bit for bit.
- **isp-fit:** passes fetch only the columns they use, and `get` and `set` serve
  scattered use.
- **dip-direction:** rules depend on `UnitStore`, never on a backend.
- **general-over-special:** general. One store holds all per-tick unit state.
- **order-independent:** a slot never orders a loop or breaks a tie, and the
  reversed-lists tests prove it.
- **bounded-work:** each pass fetches its columns once, and enrolling and dropping are
  incremental.

## Open, for the implementing session to settle and record here

- Where presentation's per-frame snapshot is taken, and whether it copies all columns or
  only the drawn ones. It must not change outcomes.
- Whether step 0 also moves the digest's squad order. Only if it changes no bytes.

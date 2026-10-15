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

Agreed in the design session of 2026-10-14. Build one step at a time with `/run-phase`, as
"spec 35 step N". Each step is its own PR.

- **Step 0, the skeleton: built** (2026-10-14). See "Built" below.
- **Step 1 is three PRs** (the user, 2026-10-14): 1a moves `position` to the GDScript
  store and off `SkirmishUnit`; 1b moves `at`, `offset` and the loose and fleeing flag
  bits; 1c gives all of them to the Rust backend, and `BodyFieldSync`'s points go.
- **Step 1a, `position`: built** (2026-10-14). See "Built" below.
- **Step 1b, `at`, `offset` and the flag bits: built** (2026-10-15). See "Built" below.
- **Step 1c, the Rust backend owns them: built** (2026-10-15). See "Built" below.
- **Step 2 is two PRs** (the user, 2026-10-15): 2a moves every other key of the loose and
  fleeing entries to the GDScript store; 2b gives them to the Rust backend.
- **Step 2a, the entries' other keys: built** (2026-10-15). See "Built" below.

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

## Settled for step 1 (the user, 2026-10-14)

- **How rules reach the store: through their squad.** Each squad holds the battle's store
  (`SkirmishSquad.store`, set when it is made: `spawn_squad` passes the battle's, and a
  squad made mid-battle (a stray, a split group, a lone router) passes its origin's).
  Every unit has a fixed `slot` on its record (`SkirmishUnit.slot`), set when it enrols,
  so nothing looks a slot up by id. A helper that has only a unit takes the columns
  (`UnitColumns`) or the point it needs as a parameter. A squad made alone, as tests do,
  gets its own GDScript store; squads that meet must share one.
- **The user: "the most performant".** A pass fetches a column once
  (`store.columns.column(UnitColumns.POSITION)`) and indexes it by `unit.slot`; it
  doesn't call the store per unit. Scattered use goes through `value` and `put`. On the
  CI runner a local column read cost about 0.015 µs, a `put` about 0.2 µs.
- A column fetched before a `put` doesn't see the write: GDScript's packed arrays copy
  on write. A pass that writes positions and reads them again fetches again.

## Built

### Step 0, the skeleton

- `UnitStore` (`unit_store.gd`, an abstract class) has `enrol`, `drop`, `slot_of`,
  `id_at`, `count`, `ids` (the held units by slot) and `settle(squads)`.
  `NativeKernels.unit_store(field)` gives `UnitStoreGd` with no field, else
  `UnitStoreNative` on it. `FormationSimulation.store` is the battle's store.
- **Enrolling and dropping.** `spawn_squad` enrols each new unit, which is the only way a
  unit joins a battle. At the end of `step()`, `settle` drops every unit that no squad
  lists any more (fled home, carried home, or the dead of a squad merged into another).
  It walks the lists only when they hold fewer or more units than the store, so a tick
  where nobody left costs a sum over the squads.
- **The native slots.** `BodyField`'s bodies are the store's slots (`hold`, `release`,
  `slot_of`, `unit_at`, `held_count`, `held_units`). The field's roster still holds only
  the living, as the passes read it: a held unit dropped from the field keeps its slot
  and is enrolled again in it when a sync names it. Free bodies were reused last-freed
  first; they are now reused lowest first, as the store's rule says. A seed change no
  longer resets the field; it takes every body off the field to be enrolled again, and
  keeps the slots.
- **The digest** reads each unit's id through the store
  (`store.id_at(store.slot_of(id))`), the same code on both engines, so a wrong slot map
  changes the hashes.
- **Settled here:** the digest's squad order stays on the simulation's squad list. The
  store holds units, not squads, so step 0 doesn't move it.
- `test_unit_store.gd`: the contract on both backends, a 24-a-side clash with deaths, a
  later wave and routers fleeing home (each listed unit's own slot, the same on both
  engines, every tick), and a battle whose slots are handed out the other way round
  playing the same.

### Step 1a, `position`

- `UnitColumns` (`unit_columns.gd`) holds the columns: `column`, `commit`, `value`,
  `put`, and `open` for a slot newly given (every column blank there). Both backends
  hold their columns in GDScript for now; 1c moves `position` into Rust.
- `SkirmishUnit.position` is gone. Every reader and writer in `sim/`, presentation, the
  tools and the tests switched to the store in the same PR (Decision 142). Signatures that
  changed: `WalkModes.climb` and `after_step` take the columns; `BlowRoll.side` and
  `from_flank` take where the target stands; `GroundBodies.of_lying` takes the columns
  and keeps where each body lies (`"ats"`).
- **Presentation's snapshot** (this spec's open point, settled): the field scene reads
  the position column once when it records a tick (`_snapshot`) and once a frame for
  what it draws from the store directly (the fallen, the carried). Only that column is
  read. Outcomes can't change: it only reads.
- `check_one_mover.py` flags writes to the position column (`put(..., POSITION, ...)`,
  `commit(POSITION, ...)`) as it does `.position =`. A write of a value the line before
  took from `UnitMover.` counts as the mover's.
- Tests that build squads by hand share one store through
  `tests/sim/skirmish/formation/unit_places.gd`.
- `test_unit_store.gd`: the column round-trips on both backends, `SkirmishUnit` no
  longer declares `position`, and the battle test compares each unit's slot and position
  on both engines every tick.

### Step 1b, `at`, `offset` and the flag bits

- `UnitColumns` has three more columns: `LOOSE_AT` (where a loose unit stands), `FLEE_OFFSET`
  (a fleeing unit's place off its route) and `FLAGS`, an int per slot whose bits are
  `LOOSE` (1) and `FLEEING` (2). The bits have the same values as `BodyFieldSync`'s, so 1c
  can send the column as it is. All start blank on a newly given slot.
- **`SquadEntries`** (`squad_entries.gd`) is the only place a loose or fleeing entry is given
  or taken away: `loosen`, `unloose`, `unloose_all`, `flee`, `unflee`, `unflee_all`. Each
  keeps the unit's bit and place in step with the entry, so a bit is set exactly while its
  entry exists. `loose_at` serves scattered reads. The passes over many units (the
  syncs, blows, the seek, the facing, the walk's write-back) read the column once instead.
- The entries no longer hold `"at"` or `"offset"`. Every reader and writer in `sim/`, the
  tools and the tests switched in this PR (Decision 142). The signature that changed:
  `FormationScrum._seeking_look` takes where the unit stands.
- A place column's value is meaningful only while its bit is set. Taking an entry away
  clears the bit and leaves the value, which nothing reads.
- **The digest** rebuilds each entry as it was, with `"at"` or `"offset"` read from the
  store, so its bytes are unchanged on both engines.
- `check_one_mover.py` treats writes to `LOOSE_AT` and `FLEE_OFFSET` as it treats writes
  to `POSITION`.
- `test_unit_store.gd`: the new columns round-trip on both backends and start blank;
  `SquadEntries` keeps the bits with the entries; and the battle test checks, every tick
  on both engines, that no entry still holds its place, that each listed unit's bits match
  its entries, and that its places are the same on both engines. The other-slots test
  carries every column over.
- **Checked before choosing the bit's meaning:** in a 700-tick clash with a later wave and
  routers, no entry outlived its unit's listing and no unit had an entry in two squads at
  a tick's end. So one bit per slot per kind of entry is enough.

### Step 1c, the Rust backend owns them

- **The native store's columns live on the field.** `BodyField`'s `Columns` (`field.rs`)
  holds `position`, the loose place, the flee offset and the flag bits by slot, grown with
  the slots and blank in a slot newly held. `UnitColumnsNative` is the store's columns
  on it: `column` is one copy out, `commit` one copy in, `value` and `put` one call each.
  `UnitStoreNative` uses it, and `UnitStore.field()` names the field (null in GDScript).
- **`BodyFieldSync`'s points go.** `sync` no longer takes points: the field reads where each
  unit stands from its columns (its loose place, its offset from its base, or its
  position), by the flags the sync names. The blows' sync does the same.
- **Settled here: the syncs still name each unit's flags**, from its own squad's entries,
  rather than reading the flag bits. A unit moved between squads can leave an entry, and
  so its bit, behind in its old squad: `FormationRecovery._join` moves a unit without
  taking its entries away, and `FormationContact.reinforce` leaves that to its callers.
  The bit then says loose or fleeing where the unit's own squad doesn't. Taking those
  entries away would change GDScript outcomes, which this series must not do. The bits
  become what the syncs read in step 2, when the entries leave the dictionaries and
  those moves are settled with them.
- **A native pass runs on its squads' store's field.** `UnitBodies.step` with no field
  takes the store's, and `BodyFieldSync` reports an error if the squads' store isn't on
  the field it syncs. A squad made alone (as tests make one) gets a store on the engine
  in use: under Rust, a native one on a field of its own.
- **Tests.** `test_unit_store.gd`: the native store's columns are on its field, and a sync
  of a battle 30 ticks in puts every living unit where `UnitBodies.at` does. The tests
  that build a battle under GDScript and run a pass on a native field move it onto the
  field first (`UnitPlaces.onto`: the same values, perhaps other slots), and those that
  run a pass twice from one state put the store's columns back too (`UnitPlaces.saved`,
  `restore`): before, they restored only the entries, which no longer hold the places.
  The parting tests give each run its own store, as a run left its bits in a shared one.
- **Cost to watch.** Under Rust a column fetched once a squad is one copy a squad. The
  passes fetch once a pass or once a squad, never a unit; the bench says whether a
  per-squad fetch shows.
- `check_function_length.py` now ends a function at a `static func`; it measured the
  `static func`s after a `func` as part of it.

### Step 2a, the entries' other keys

- **The inventory.** Every key the entries held across the standard digest set: loose
  `unit`, `goal` (`[foe id, slot]` or null), `next`, `foe_at`, `touch` (a bool), `toward`
  (a point or null), `stuck` and `closest`; fleeing `along`, `caught`, `held_by` (a friend's
  id or null) and `fanned`. Beyond `unit`, `goal`, `next` and `along`, an entry may lack
  each key.
- **`UnitColumns`** has a column for each: `NEXT`, `GOAL_FOE` and `GOAL_SLOT` (-1: no goal),
  `FOE_AT`, `TOUCH` (1: it touches), `TOWARD`, `STUCK`, `CLOSEST`, `ALONG`, `CAUGHT`,
  `HELD_BY` and `FANNED`, plus `KEYS`, the bits saying which keys the entry holds and
  whether `toward` and `held_by` hold a value. A missing key reads as its column's blank,
  which is the value its readers took for it (`touch` 0, `stuck` 0, `closest` INF,
  `caught` 0, `fanned` 0), so passes read the columns directly.
- **`EntryKeys`** (`entry_keys.gd`) keeps a key and its bit together: `keep`, `held` (null
  for a `toward` or `held_by` that is null or missing), `forget`, `goal` and `set_goal`. It also
  rebuilds an entry as a dictionary with exactly the keys it holds (`loose_entry`,
  `fleeing_entry`), which is what the digest hashes, so the digest's bytes are unchanged.
- **Settled with the user: `squad.loose` and `squad.fleeing` map each unit's id to the
  unit**, in the order its entry was given, and hold nothing else. Loops over them keep
  today's order (the walk back, trading places, the drilled retreat's foes), so no
  outcome can move. `SquadEntries.loosen(squad, unit, at)` gives a new entry with its next
  step at `at` and no goal (every caller gave exactly that); `flee(squad, unit, along,
  offset)` likewise. A pass over many units fetches each column once and commits it once
  (the native seek's and walk's write-backs, the facing's gather, the trade's check for stalled units);
  scattered use goes through `value`, `put` and `EntryKeys`.
- **Settled with the user: a unit moved to another squad leaves no entry behind.**
  `FormationRecovery._join`, `FormationContact.reinforce` and `GroupSplit._group` now take
  the unit's loose and fleeing entries away in its old squad as it moves. Before, each
  left them until later in the tick. Tested before choosing: all three digest sets are
  byte-identical on GDScript with the entries taken away. So each unit has at most one
  entry of each kind, in its own squad, and one set of columns per slot holds it. The
  battle test checks every tick that every entry's unit is listed in that squad.
- **Removed, not forwarded.** No entry is a dictionary any more. Signatures that changed:
  `ScrumTrade.track` takes the columns and the slot; `RoutFlight.step` and
  `RoutCatch.holder` take the unit instead of its entry; `FormationScrum._seeking_look`
  takes the columns.
- **Under Rust, the new columns are held in GDScript** (`UnitColumnsNative.ON_FIELD` names
  the four that live on the field), so a pass reads them at no cost. 2b moves them to the
  field. The syncs still name each unit's flags from its own squad's `loose` and `fleeing`;
  now that moves leave nothing behind, 2b can read the flag bits instead.
- `check_one_mover.py` treats writes to `ALONG` as it treats writes to the other place
  columns.
- **Tests.** `test_entry_keys.gd` (new): every new column round-trips and starts blank on
  both backends; an entry holds exactly the keys it was given; and a unit moved by
  `_join` or `reinforce` leaves no entry behind (red before the moves were settled).
  `test_unit_store.gd`'s battle compares every unit's entries, rebuilt as the digest
  rebuilds them, on both engines every tick. The tests that run a pass twice from one
  state put the store's columns back as well as the squads' lists (`UnitPlaces.saved`).

## Open, for the implementing session to settle and record here

- Nothing open. Presentation's snapshot was settled in step 1a (above).

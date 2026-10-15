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
- **Step 2b, the Rust backend owns them: built** (2026-10-15). See "Built" below.
- **Step 3, bearing: built** (2026-10-15), in one PR: the GDScript store and the Rust
  backend own it together. See "Built" below.
- **Step 4 is five PRs** (the user, 2026-10-15): 4a moves rank and column into the store
  (Decision 143); 4b the native seek reads its goals and places on the field and writes
  its outcome there; 4c the walk; 4d the facing; 4e the sync reads flags, next and bearing
  on the field, so `BodyFieldSync` keeps only enrolling.
- **Step 4a, rank and column: built** (2026-10-15), both backends in one PR. See "Built"
  below.
- **Step 4b, the seek on the store: built** (2026-10-15), stacked on 4a's PR (the user).
  See "Built" below.
- **Step 4c, the walk on the store: built** (2026-10-15), stacked on 4b's PR (the user:
  "keep stacking, we can merge them all down later"). See "Built" below.
- **Step 4d, the facing on the store: built** (2026-10-15), stacked on 4c's PR. See
  "Built" below.
- **Step 4e, the sync reads the store: built** (2026-10-15), stacked on 4d's PR. Step 4 is
  complete. See "Built" below.
- **Store reads held once a pass (Decision 144): built** (2026-10-15), on the same PR.
  See "Built" below.
- **Places read once a pass: built** (2026-10-15), stacked on 4e's PR, after step 4's bench
  (the user: "the aim of this spec was to improve performance, so reducing it is not
  acceptable"). See "Built" below.

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
  command and origin, footprint. Rank and column were listed here, but they change
  mid-battle, so they moved in step 4a (Decision 143). If step 7's inventory finds a field that
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

   **Steps 5 to 7 each begin with a readers list (Decision 144).** Before a field moves,
   the step's spec entry lists every reader and writer of it (grep the tree) and, for each
   loop over units, how it will fetch the column once and commit once; helpers that read
   one unit take an optional fetched table. `living()` (a filter on `is_alive()`, called in
   about 100 places) needs its plan in step 7 before the alive flags move. The step's bench
   against its base names every phase slower beyond noise, and the step fixes it in its
   own PR.
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

### Step 2b, the Rust backend owns them

- **Every column lives on the field.** The store's columns moved out of `field.rs` into
  `columns.rs` (`Columns`), which now holds the step-2 keys too, by type: points (`next`,
  `foe_at`, `toward` beside the three places), 32-bit ints (`flags`, `touch`, `keys`),
  64-bit ints (`goal_foe`, `goal_slot`, `caught`, `held_by`) and floats (`stuck`,
  `closest`, `along`, `fanned`). Each is found by its UnitColumns name and starts blank as
  UnitColumns' BLANKS say (`goal_slot` -1, `closest` INF).
- **`BodyField`'s calls, by type:** `column_points`/`commit_points`/`point`/`put_point` as
  before; `column_ints`/`commit_ints`, `column_longs`/`commit_longs`,
  `column_reals`/`commit_reals`; `int_of`/`put_int` (32- or 64-bit) and `real_of`/`put_real`.
  The flags-only calls (`column_flags`, `commit_flags`, `flags_of`, `put_flags`) are gone:
  `column_ints("flags")` and the rest replace them.
- **`UnitColumnsNative`** holds nothing in GDScript: it sends each name to the field's call
  for its kind. The 2a fix that grew GDScript-held columns for a slot the field gave in a
  sync (`UnitStoreNative.enrol`) went with them.
- **Not done here, left for step 4:** the syncs still name each unit's flags and send its
  next step from GDScript, though the field now holds both. Reading them on the field is
  the glue step 4 removes.
- **Tests.** `test_entry_keys.gd`: the native store holds every column on its field, a new
  slot reading blank there (red on the library built before this step: the calls didn't
  exist). `columns.rs`: blanks, and each column found by its name and no other. The column
  contract and the battle's per-tick comparison on both engines
  (`test_unit_store.gd`, `test_entry_keys.gd`) run unchanged.

### Step 3, bearing

- **One PR, both backends.** Since 2b the native store holds nothing in GDScript, so a
  GDScript-only half would have needed a column held in GDScript under Rust just for one
  commit. `UnitColumns.BEARING` (a float, by slot) and Rust's `Columns.bearing` (a real,
  found by the name `"bearing"`) came in together.
- **A new slot faces east** (`BLANKS[BEARING]` 90, Rust's `EAST`), as a new `SkirmishUnit`
  did. Nothing set a unit's bearing before it enrolled, so every unit starts as before.
- **`SkirmishUnit.bearing` is gone.** Every reader and writer in `sim/`, presentation, the
  tools and the tests switched in this PR (Decision 142). Signatures that changed:
  `UnitMotion.turn`, `move` and `walk` and `UnitMover.step` and `slide` take the columns
  after the unit; `UnitMotion.pace` takes the bearing (no longer optional);
  `UnitShuffle.look` takes the bearing the unit faces now; `UnitFootprint.corners`,
  `ScrumSlots.pick` and `SlotSearch.pick` take the bearing; `BlowRoll.side` and `from_flank`
  take the target's bearing instead of the target; `FormationScrum._faces` gives each
  turn with its columns.
- **Passes over many units fetch the column once:** the blows (GDScript and native
  syncs), the blows' landing, the slots round the foes, the facing's gather and its
  native write-back (one commit), the native walk's gather, and the scrum sync. The
  turns (`UnitMotion.turn`), the march's facing and the seek's pick read and write one
  value each. No pass reads a fetched bearing column after a turn in the same pass, so the
  copy-on-write rule changes nothing.
- **Presentation** reads the bearing column once a frame with the position column.
- **Not done here, left for step 4:** the syncs and the native walk and facing still send
  each unit's bearing from GDScript, though the field holds it.
- **Tests.** `test_unit_bearing.gd` (new): the column round-trips on both backends, a new
  or reused slot faces east, the native store holds it on its field, and `SkirmishUnit`
  no longer declares it. `test_unit_store.gd`'s battle compares every unit's bearing on
  both engines every tick, and the other-slots test's state reads the column. `columns.rs`:
  the blank and the name. `UnitPlaces` (tests) has `facing`, `face`, `bearing_of` and
  `turn`; its `saved` and `restore` carry the bearing with every other column, so the
  facing tests no longer save bearings themselves.

### Step 4a, rank and column

- **Why a step of its own** (Decision 143): the native seek and walk need each unit's
  place in its squad's frame, and rank and column change mid-battle (trades, re-forms,
  closing up, turning about, routs, recovery, strays, splits, reinforcements), so they
  couldn't be enrolled once.
- **`UnitColumns.RANK` and `COLUMN`** (32-bit ints, blank 0) and Rust's `Columns.rank` and
  `column`, found by the same names, came in together.
- **`UnitPlace`** (`unit_place.gd`) serves scattered use: `of`, `put` and `rank` by a squad
  (anything holding the store) and a unit; `at` and `set_at` by columns and a slot. The
  native seek's and walk's gathers, `PlaceOrder.front_first` and presentation fetch the
  two columns once.
- **`SkirmishUnit.rank` and `column` are gone.** Every reader and writer in `sim/`,
  presentation, the tools and the tests switched in this PR (Decision 142). Signatures that
  changed: `SquadFrame.corners`, `unit_rect`, `place` and `place_in` take the unit's place
  after the unit; `PlaceOrder.front_first` takes the columns; `SquadPlaces.places` takes
  the squad, and `SquadPlaces.occupancy` also returns each unit's place (`"places"`), which
  the re-form's planning reads; `FormationUnits.make` no longer takes a place:
  `spawn_squad` puts each unit's place once the squad has enrolled it.
- **`SquadCompact`** (`squad_compact.gd`) now holds `compact` and its helpers, unchanged but
  for reading places through the store; `SkirmishSquad.compact()` calls it.
  `skirmish_squad.gd` was at its 300-line limit.
- **Speed is still sent each tick** (the user, 2026-10-15): it changes every tick through
  stamina and moves to the store in step 6. Until then plan, walk and face take it from
  GDScript.
- **Cost to watch.** Rules that read one unit's place (`ScrumStance.anchor`,
  `FormationPour.place`, `lateral_span`, the re-form) read two values from the store
  each time. Under Rust each is a call into the field. The bench says whether that shows;
  4b's native seek removes the largest of them.
- **Tests.** `test_unit_place.gd` (new): the columns round-trip on both backends, a new or
  reused slot reads (0, 0), the native store holds them on its field, a spawned squad's
  units stand where its layout puts them, and `SkirmishUnit` no longer declares either.
  `test_unit_store.gd`'s battle compares every unit's place on both engines every tick.
  `UnitPlaces` (tests) has `set_place`, which enrols a hand-built unit first. The two
  crowds in `test_scrum_trade_turns.gd` share unit ids, so each now has its own store.
  `columns.rs`: the blank and the names.

### Step 4b, the seek on the store

- **`BodyField.seek` replaces `plan`.** GDScript sends only what its rules say of each
  fighting squad (`ScrumSeekField._say`): its foes, whether its units may seek (not ordered
  to retreat), its frame (heading, width, centre shift, anchor: `ScrumStance.frame`'s
  inputs) and its chasers' ids. It also sends each synced unit's speed, which stays sent
  until step 6. The field gathers the core's input itself (`seeking.rs`): each fighting
  squad's living members in list order, chasers left out; who may seek, from the front
  band and footprint; each unit's place, `SquadFrame.place_in` from the store's rank and
  column in GDScript's precision; and its goal from the columns. It then writes the
  outcome into the loose entry's columns (touch, goal, next, foe_at and their key bits),
  as `ScrumSeekField._write_back` did, and returns which squads are active.
- **The unit's record is enrolled once.** `BodyField.enrol` also takes each new unit's
  preferred band and footprint width and depth (`Field.preferred`, `foot_width`,
  `foot_depth`). None of these changes during a battle: only `FormationUnits.make` sets
  them.
- **The walk and the facing still take a batch from GDScript** (`ScrumSeekField.Batch`),
  rebuilt from `seek`'s answer: the planned squads, their units' flat indices, the codes,
  nexts and foe_ats. 4c and 4d move those onto the field. `ScrumSeekField.gathered` builds
  the same batch without planning, for the facing's test and bench.
- **A squad the sync named no living unit of is left out**, as GDScript's walk over the
  synced units never met it.
- `tick_phases` books the seek's last part as "the walk's batch", where it was "write back".
- **Tests.** `test_body_field.gd`: a synced unit's record is on the field (new, red on the
  library built before this step). The seek parity tests (the native seek against
  GDScript's at four moments of a scrum, and in reversed list order) run unchanged
  through the new path.

### Step 4c, the walk on the store

- **`BodyField.walk` gathers and writes back on the field** (`walk_store.rs`). It no longer
  takes each unit's motion and frame from GDScript. It reads the bearing, rank and column
  from the store's columns, and the turn rate, backward pace and footprint from the
  unit's record. It writes where each unit ends and looks into the loose entry's columns
  (`at`, `next`, `toward` and its key bits), as `ScrumWalkField._write_entries` did.
  GDScript still sends each squad's frame and pace, the bodies lying on the ground, the
  seek's codes and nexts, and the speeds.
- **The record grows by the paces.** `BodyField.enrol` also takes each new unit's turn
  rate and backward pace, which only `FormationUnits.make` sets.
- **The facing reads the store as the walk left it.** The walk no longer hands the facing
  a batch (`face_batch`); `ScrumFaceField` gathers from the columns, as it already did
  when no batch was given (the path `test_face_field.gd` runs). The values are the same:
  the walk wrote them. 4d moves that gather onto the field.
- **Paying the ground** (`FormationScrum._pay_walked`) notes where each unit stood only
  when there is terrain, the only case that reads it.
- **Tests.** `test_body_field.gd`: the record on the field includes the paces (red on the
  4b library). The walk parity tests (`test_walk_field.gd`: the native walk against
  GDScript's at several moments, round bodies and over the fallen, and in reversed list
  order) run unchanged through the new path.

### Step 4d, the facing on the store

- **`BodyField.face` gathers and writes back on the field** (`face_store.rs`). It takes
  only the seek's squads, their foes, the units' flat indices and speeds, and the
  `toward` valued bit. It reads where each unit stands loose, its bearing, its goal
  (next, foe_at) and its look (toward) from the store's columns, as
  `ScrumFaceField._gather` did, and the turn rate and backward pace from the record. It
  writes each bearing into the column.
- **`ScrumFaceField` holds no batch.** Its gather and write-back are gone. `spent`'s gather
  and write-back parts stay 0, so `facing_bench` reads only the call and the core.
- **Tests.** `test_face_field.gd`: the facing on the field, given only the squads, foes,
  units and speeds, turns them as GDScript does (new; red on the 4c library, whose `face`
  took the gathered arrays). The facing parity tests (at two moments, any bearing to any
  look, and in reversed list order) run unchanged through the new path.

### Step 4e, the sync reads the store

- **`BodyField.sync` takes the roster and a mode.** The roster is each squad's id,
  faction, flags and its living units' ids, in list order. The mode is `BodyFieldSync.SCRUM`
  or `BODIES`. It no longer takes each unit's flags, next point or bearing. The field reads
  how each unit stands from the store's flag bits (`LOOSE`, `FLEEING`); the scrum's
  snapshot masks out `FLEEING`, as the scrum sync sent only `LOOSE`. It reads a loose,
  non-fleeing unit's next point from the column in the bodies' snapshot, and each bearing
  from the column in the scrum's. That is what 1c left for here: since 2a, a unit's bit is
  set exactly while its own squad holds the entry, so the bits say what the squads'
  entries said.
- **What stays in GDScript: the roster and a fleeing unit's point on its route**
  (`FormationRout.route_point`). Routes are GDScript's, and whether a unit is living is
  step 7's. `ScrumBlowsField`'s own sync sends the same roster in the scrum's mode.
- **Tests.** `test_body_field.gd`: a sync that names only the roster stands a loose unit
  where it stands loose and another at its position (new; red on the 4d library, which
  had no mode). `field.rs` and `blows.rs` set the flag bits in the store before syncing.
  The seek parity tests run unchanged through the new sync; the parting's and blows'
  tests and the digests run in CI.

### Places read once a pass (after step 4)

- **Why.** Step 4's bench (`tick_phases engine=rust side=4096 squads=4 fight=3`) showed 4a
  costing 22.6% (370.2 → 453.9 ms against main), mostly in deaths (25.9 → 54.5), blows
  (21.0 → 45.3), the re-form's occupancy (5.1 → 12.8) and units walking to places
  (27.7 → 36.0). 4b to 4e won back about 33 ms of scrum glue, so step 4 as a whole was still
  2.9% slower (291.3 → 299.8). The cause was 4a's scattered reads: each `UnitPlace.of` or
  `rank` is two store reads, and under Rust each read is a call into the field.
- **`UnitPlace.table(squad)` and `read(table, squad, unit)`.** A pass over many units fetches
  the store's ranks and columns once and reads each unit's place from them; `read` with no
  table reads the store, as before. Like a position column fetched once a squad, a table
  is a copy as of now: a pass that moves units fetches again.
- **The passes that fetch once:** `fighters()`; ranged combat's marks, rows and range (and
  `SquadGeometry.unit_gap` given a table); `SquadCompact`, which works on the fetched
  ranks and commits them once, only if any changed; the march's unit distances
  (`SkirmishSquad.rank_distance(rank)` replaces `unit_distance(unit)`); `FormationPour.targets`
  and `FormationWalk.share`; the re-form's occupancy, which also hands its table to
  `PlaceOrder.front_first` (now taking a table, not the columns); `SquadEdges`' bounds, edge
  units, reach and overlap; `SquadGeometry.lateral`; the trade's places; `SquadRanks.close`;
  contact's and groups' back rows; recovery's back row; the re-form's widening. `lateral_span`,
  `ScrumStance.anchor`, `FormationPour.place` and `FormationWalk.place_of` take an optional
  table.
- **Left scattered:** reads once per event (a step-up's rank, a trade's two places, a swap
  that ends, a unit placed when it spawns, joins or strays).
- **Tests.** `test_unit_place.gd`: a table reads each unit's place as the store holds it, on
  both backends, and `read` with no table reads the store (red on 4e's `UnitPlace`, which had
  neither). No outcome changes; the digests hold that.
- **Measured** (spec 34's "Measured"): step 4 with this fix against main before step 4,
  353.2 → 281.6 ms a tick (−20.3%); this fix alone, 315.9 → 283.1 (−10.4%).

### Store reads held once a pass (Decision 144)

- **The rule** is `principles/bounded-work.md` Principle 3, recorded as Decision 144 and
  added to the `design-check` skill: no loop over units reads or writes the store one unit
  at a time, outside marked exceptions.
- **`check_store_reads.py`** (pre-commit and CI, on `sim/`, tests in
  `test_check_store_reads.py`) flags a scattered store call written in a loop's body,
  unless the line or its loop carries `# store: scattered (<why>)`.
- **Fixed, every tick:** body parting's write-back is the field's own
  (`BodyField.commit_part`, called once GDScript has loosened the units parting made loose;
  `field_parting::commit`); the march's write-back (`FormationMarch.sync_units`) puts the
  routers', loose and placed points into fetched columns and commits them once, then the
  units in their places walk (each reads only its own place, the ground and the lying); the
  native walk's paying for the ground (`ScrumWalkField.pay`, moved from FormationScrum)
  fetches once and commits once.
- **Marked, with the reason the check now requires:** per-event loops (step-ups, joins,
  splits, spawns, a swap ending, closing ranks, turning about, production); the rout's
  routers, pursuit's chasers and regrouping units, each stepped by UnitMover (the rout about
  1.5 ms at the bench, pursuit and regroup under 0.1 ms); the GDScript reference's own seek,
  walk and facing, which the Rust engine replaces with native passes.
- **Not seen by the check, left for a later port:** the march's walk itself
  (`FormationWalk.walk` → `WalkModes`, `UnitMover`, `UnitMotion`) reads and writes each
  unit's position and bearing per unit, inside helpers; it is spec 34's "the walk" port.

## Open, for the implementing session to settle and record here

- Nothing open. Presentation's snapshot was settled in step 1a (above).

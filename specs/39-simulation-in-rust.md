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
- **Split** (2026-10-22, the user's call): **2a** the content module, its JSON Schema and the
  loader's errors, with units, weapons, armour and the tuning converted and GDScript's
  definitions filled from them; **2b** terrain and the map layout, the designer's export and
  validation, and the importer kept to what the presentation needs. 2b's PR closes the step.
- **Split again** (2026-10-23, the user's call, so each PR's schema change carries its
  designer change, as `CLAUDE.md` requires): **2b** the terrain library in the crate's schema,
  converted once, `TerrainLibraryDef` filled from it, the designer's library page on the
  schema and the Godot library import retired; **2c** the whole map in one file in the
  crate's schema (`content/maps/<name>.json`: the layout and the nodes, lanes, factions and
  loss groups, the user's call), converted once, `MapDef` filled from it, the designer's map
  export and validation against the JSON Schema, the round trip, and the importer kept to what
  the presentation needs. 2c's PR closes the step.
- **Where the designer's translation lives** (2026-10-23, the user's call, building 2c): in the
  crate, not Python, for the map and for 2b's terrain library alike. The structs are the
  schema, so a translated map is checked by serde, the round trip is a cargo test, and the
  rules are ported once into the language this round moves to. The designer's server runs a
  binary of the crate (`designer`).
- **All of it in Rust** (2026-10-23, the user's call, "make it all Rust", building 2c): the
  map's rules (`MapDef.validate()` and what it called) move to the crate too, and so do the
  designer's server (`serve.py`, `designer_repo.py`, `content_library.py`) and the GDScript that
  filled the definitions from the crate's content (`ContentFill`, `MapFill`, `MapJson`), since
  steps 3 to 9 only shrink that fill and never retire it (what presentation draws stays). The
  designer's page stays JavaScript; moving it to Rust (WebAssembly) is a stretch goal, Q36
  (#566).

### Step 3, the ground and the mover

The ground (`FormationTerrain`, `sim/ground/`), routes (`FormationRoute`), the one mover
(`UnitMover`, `UnitSteer`, Decision 139), pathing (`FormationPathing`) and the march
(`FormationMarch`, `FormationWalk`, `FormationSweep`, `FormationShuffle`, `FormationStaging`).
Spec 34 items 3 and 5 and Q12 (#268) are carried as they are: terrain does today what it does.

- **Split** (2026-10-23, the implementing session, as Status allows: about 3,300 lines of
  GDScript in some 29 classes): **3a** the ground (`FormationTerrain`, `TerrainObscurance`,
  `sim/ground/`); **3b** routes and the mover (`FormationRoute`, `RoutePatch`, `UnitMotion`,
  `UnitMover`, `TerrainStep`, `UnitSteer`); **3c** pathing (`FormationPathing`, `PathSearch`,
  `PathFrontier`, `PathMemory`, `PathSight`, `TerrainPaths`, `SquadPathfinder`,
  `WalkerShares`, `ClimbBudget`); **3d** the march (`FormationMarch`, `FormationWalk`,
  `FormationPour`, `WalkModes`, `FormationSweep`, `FormationShuffle`, `ShuffleFront`,
  `FormationStaging`). 3d's PR closes the step. `TerrainWalker` (a unit's walker, built from
  its definition) stays GDScript until the units' rules port; the crate takes its fields.
  `FormationStaging` moved to step 5 in 3d (Built, "Step 3d").

### Step 4, the scrum, regroup and the pursuits

`FormationScrum` and everything it calls: the stance, the manoeuvre, seek, the walk, facing,
blows, the contest, the trade, entries and ranks, the locks, then regroup (`ScrumRegroup`, its
walk back through the scrum walk's kernel), the pursuits, withdraw, fronts, edges, contact and
perception. The largest step. It is expected to split (4a the scrum core, 4b regroup onwards).

- **Split** (2026-10-23, the user's call, building 4a: about 3,500 lines of GDScript in some
  27 classes): **4a** the per-unit scrum: the seek (`ScrumSeek`'s plan with `SlotSearch`,
  `ScrumSlots`, `ScrumNear`, `FoeIndex` and `BodyGrid` where only it reads them), the walk,
  the facing, the blows' picks and the landing's counts, `ScrumContest` and `ScrumReach`;
  **4b** the squad-level scrum: engage, the stance, the manoeuvre, the locks, ranks, entries,
  the trade and the stall; **4c** regroup onwards (regroup, the pursuits, withdraw, fronts,
  edges, contact and perception). 4c's PR closes the step.
- **The squads' records in the store** (2026-10-23, the user's call, building 4b): the locks,
  entries, manoeuvre and stall are rules over SkirmishSquad's dictionaries, so `loose`,
  `fleeing`, `stance`, `flank_contacts` and `chasers` move into the crate's squad store in
  4b, and the four port with the rest, not with the tick (step 8).
- **The field in both engines** (2026-10-23, the user's call, building 4a): the seek, walk,
  facing and blows already ran in Rust on the battle's `BodyField` (Decision 129), which the
  `gdscript` engine didn't make. To retire their twins, every battle now has a field and the
  native unit store under both engines, and `UnitStoreGd` retires. `BREACH_NATIVE` then picks
  only the twins still left (body parting, `UnitBodies`, until step 6).

### Step 5, groups and the split

`FormationGroups`, `GroupSplit`, strays (`FormationStrays`), compaction (`SquadCompact`) and
rendezvous, with the staging that times its departure by it (`FormationStaging`, moved here
from step 3d: Built, "Step 3d").

- **Split** (2026-10-24, the implementing session, as Status allows: about 860 lines of
  GDScript in seven classes, with `FormationContact.reinforce` carried from 4c): **5a** groups
  forming up and the split (`FormationGroups`, `GroupPairs`, `GroupSplit`), the costly part;
  **5b** compaction (`SquadCompact`) and reinforcing (`FormationContact.reinforce`), strays
  (`FormationStrays`), rendezvous (`FormationRendezvous`) and the staging
  (`FormationStaging`). 5b's PR closes the step.

### Step 6, the downed and stamina

Deaths, wounds, recovery, carrying, taking, stamina and the bodies' parting (`UnitBodies`).

- **Split** (2026-10-24, the user's call: about 1,650 lines of GDScript in eight classes):
  **6a** stamina (`FormationStamina`) and the bodies' parting, its GDScript twin retired;
  **6b** the downed: the fallen (`FormationDeaths`), the takers and surrender
  (`FormationWounds`), recovery and regeneration (`FormationRecovery`,
  `FormationRegeneration`); **6c** carrying (`FormationCarry`) and taking
  (`FormationTaking`). 6c's PR closes the step.
- **The last twin** (2026-10-24, the user's call): the parting was the last pass with a
  GDScript twin, so 6a also retires `BREACH_NATIVE`, the engine switch and the digests' run
  under each engine, which step 8 had listed "with the last twin". The digests then compare
  only with the base (`check_outcomes.py`).

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

### Step 1, the squads in the crate, and the `rust` CI job

- **The squad store** (`native/rust/src/squads.rs`, `Squads`; GDScript's `SquadColumns`,
  the reference, and `SquadColumnsNative` on the BodyField). Columns by squad slot, as the
  unit store's (Decision 146), named by index in one order on both sides (`NAMES`): 16 ints,
  4 longs, 6 reals and the position, and each squad's members, its units' unit-store slots in
  its list's order. A squad takes a slot as it is made and keeps it for the battle: squads are
  few, and one that has left the lists may still be read (an event, the presentation).
  `SkirmishSquad` extends `StoredSquad` (`stored_squad.gd`), whose fields are properties
  over the columns, so every reader keeps its code. A squad set to another store carries its
  state there. `UnitState` writes the members as a squad's lists are rebuilt after a join or
  a leaving (one write a rebuild).
- **Which fields stay per-squad objects** (open item settled): the dictionaries and lists
  that hold units, nested entries or routes stay GDScript until the step that ports their
  pass: `loose`, `stance`, `flank_contacts`, `taking`, `chasers`, `pursuit`, `withdraw`,
  `staging`, `fleeing`, `painted`, `swaps`, `units`, `joined`, `kept`, `command`, `route`,
  and `faction_id` (a string; the field already interns factions on its syncs) and `id`.
- **Every field moved, its column and its readers** (Decision 144; files under `sim/`,
  `presentation/` and `tools/` that read or write it on a squad):

| Field | Column (kind) | Readers and writers (outside `stored_squad.gd`) |
|---|---|---|
| `direction` | `DIRECTION` (int) | skirmish_squad_layer, formation_contact, formation_groups, formation_march, formation_pathing, formation_pursuit, formation_rout, formation_simulation, formation_strays, formation_sweep, group_split, scrum_turn, skirmish_squad, squad_compact |
| `width` | `WIDTH` (int) | formation_contact, formation_groups, formation_production, formation_shuffle, formation_staging, formation_sweep, scrum_seek_field, scrum_stance, scrum_walk_field, shuffle_front, skirmish_squad, squad_memo, squad_ranks, squad_turn |
| `order` | `ORDER` (int) | formation_skirmish_readout, foe_index, formation_contact, formation_groups, formation_march, formation_pursuit, formation_rout, formation_simulation, formation_staging, formation_stamina, formation_withdraw, group_split, scrum_blows, scrum_blows_field, scrum_regroup, scrum_seek, scrum_seek_field, scrum_stance, skirmish_squad, blows_bench, digest_state, formation_replay |
| `state` | `STATE` (int) | formation_skirmish_readout, skirmish_squad_layer, formation_field_hud, formation_field_scene, battle_trials, body_field_sync, foe_index, formation_carry, formation_combat, formation_contact, formation_deaths, formation_edges, formation_groups, formation_joins, formation_locks, formation_manoeuvre, formation_march, formation_morale, formation_pathing, formation_perception, formation_recovery, formation_rout, formation_scrum, formation_shuffle, formation_simulation, formation_staging, formation_stamina, formation_strays, formation_taking, formation_withdraw, formation_wounds, group_split, rout_catch, rout_friends, scrum_blows, scrum_blows_field, scrum_engage, scrum_regroup, scrum_seek, scrum_seek_field, scrum_slots, scrum_stance, shuffle_front, unit_bodies, digest_state, facing_bench, formation_digest, formation_replay, rout_bench, shuffle_phases, steering_bench, tick_phases |
| `fought` | `FOUGHT` (int (bool)) | formation_groups, formation_scrum |
| `patched_count` | `PATCHED_COUNT` (int) | formation_pathing |
| `stall_ticks` | `STALL_TICKS` (int) | formation_scrum, digest_state |
| `manoeuvre` | `MANOEUVRE` (int) | formation_manoeuvre |
| `morale` | `MORALE` (int) | formation_field_scene, formation_morale, formation_rout, formation_simulation, formation_strays, group_split, digest_state, formation_replay, rout_bench |
| `rally_ticks` | `RALLY_TICKS` (int) | formation_rout |
| `blocked` | `BLOCKED` (int (bool)) | formation_field_hud, formation_march |
| `painted_width` | `PAINTED_WIDTH` (int) | — |
| `narrow_ticks` | `NARROW_TICKS` (int) | formation_manoeuvre |
| `wait_ticks` | `WAIT_TICKS` (int) | formation_simulation |
| `reforming` | `REFORMING` (int (bool)) | formation_deaths, formation_joins, formation_locks, formation_rout, formation_shuffle, squad_turn, shuffle_phases |
| `combat_width` | `COMBAT_WIDTH` (int) | formation_simulation, shuffle_front |
| `engaged_with` | `ENGAGED_WITH` (long) | foe_index, formation_fronts, formation_locks, formation_manoeuvre, formation_morale, scrum_engage, scrum_pursuit, scrum_seek, scrum_stance, digest_state |
| `fight_since` | `FIGHT_SINCE` (long) | formation_scrum, digest_state |
| `taking_since` | `TAKING_SINCE` (long) | formation_taking |
| `took_until` | `TOOK_UNTIL` (long) | formation_taking |
| `home_distance` | `HOME_DISTANCE` (real) | formation_march, formation_pathing, formation_pursuit, formation_rout, formation_staging, formation_strays, formation_withdraw, group_split, rout_catch, skirmish_squad |
| `front_distance` | `FRONT_DISTANCE` (real) | formation_contact, formation_march, formation_pathing, formation_pursuit, formation_rehearsal, formation_rout, formation_simulation, formation_staging, formation_strays, formation_sweep, formation_withdraw, group_split, scrum_turn, skirmish_squad, squad_compact, digest_state, formation_replay, rout_bench |
| `heading` | `HEADING` (real) | formation_march, formation_morale, formation_pour, formation_pursuit, formation_rout, formation_scrum, formation_strays, formation_sweep, formation_walk, ground_bodies, group_split, scrum_regroup, scrum_seek_field, scrum_stance, scrum_turn, scrum_walk_field, squad_edges, squad_geometry, squad_memo, digest_state, steering_bench |
| `looked_at` | `LOOKED_AT` (real) | formation_pathing |
| `painted_shift` | `PAINTED_SHIFT` (real) | — |
| `centre_shift` | `CENTRE_SHIFT` (real) | formation_contact, formation_production, formation_shuffle, scrum_seek_field, scrum_stance, scrum_walk_field, shuffle_front, skirmish_squad, squad_memo, squad_turn |
| `position` | `POSITION` (point) | formation_field_scene, fatigue_trials, formation_pathing, formation_pour, formation_pursuit, scrum_engage, scrum_seek_field, scrum_stance, scrum_walk_field, squad_edges, squad_geometry, squad_memo, digest_state, formation_replay, tick_phases |

- **Read once a pass** (Decision 144, the user's call of 2026-10-22: every loop, not only
  those a bench names). A scan of `sim/` for loops over units that read a squad's field,
  directly or through a helper that takes the squad, found about 40 sites. Each now reads it
  once: the frame (`StoredSquad.frame`, or `ScrumStance.frame` with `SquadFrame.place_in`)
  before the loop for anchors (the scrum's walk, regroup, the trade, the pursuits), the
  march's places (`FormationPour.place`, `FormationWalk.place_of`), edges and spans
  (`SquadEdges`, `SquadGeometry`, `SquadFrame.span`) and the ranged picks
  (`FormationCombat._rows`); the order, the home distance, the width and the state as locals;
  and a squad's morale band (`FormationMorale.interval` now takes the band) and a rally's
  reads (`RoutCatch.read_once`) remembered for the pass where they were read lazily, so a
  band's first read, which may set the morale, falls where it did.
- **The state serialises** (`snapshot.rs`, `ordered.rs`): the tick, the battle seed, the
  field (the store's slots and columns, the bodies) and the squads, through serde in bincode.
  **Format** (open item settled): bincode, not JSON: it keeps every f64 bit for bit, and JSON
  can't hold the infinities the columns use (`closest`, `looked_at`). The field's hash maps
  and free-slot heap are written in key order, so the same state always writes the same
  bytes. **Where snapshots live** (open item settled for now): the digest writes them on
  demand (`formation_digest.gd snapshot=<dir> at=<tick>,...`, Rust engine only) to a
  directory it is given; none is committed in this step. Step 8 commits the ones its tests
  read, cut from the digest's battles, and judges their size then.
- **The scenario "one tick stepped from each"**: the tick is GDScript's until step 8, and a
  snapshot holds only the crate's state, so step 1's cargo test steps the crate's own pass
  (the bodies' parting) from a state and from it read back, and requires the same moves and
  the same bytes after. Step 8's snapshot tests step whole phases.
- **Files opened:** 34 pre-existing `.gd` files, over the OCP check's 11, under Decision 157
  (spec 39's ports may open more, each PR stating its count).
- **Builders** (`build.rs`, `crowd`): test-only for now; step 8's benches share them.
- **CI's `rust` job** (`.github/workflows/rust.yml`): `cargo fmt --check`, `cargo clippy
  --all-targets -- -D warnings` and `cargo test` on PRs that touch `native/`. The crate is
  formatted with `use_small_heuristics = "Max"` (`rustfmt.toml`), the nearest to how it was
  written, and clippy's four findings on `main` are fixed.
- **Tests:** `squads.rs` (a new squad's blanks, each column by name and by index),
  `snapshot.rs` (read back, the same bytes; a pass stepped from each, the same; bytes that
  aren't a state refused), `test_squad_store.gd` (both backends, one contract: slots and
  blanks, each field its column, members, carrying a squad to another store, the crate's
  snapshot read back). Outcomes unchanged: CI's digests compare with `main` on both engines.

### Step 2a, units, weapons, armour and the tuning in the crate's schema

- **The content module** (`native/rust/src/content/`): `Unit`, `Weapon`, `Armour` (their shared
  `ItemDef` fields as `Item`, under `"item"`) and `BattleTuning`, named field for field as
  GDScript's properties. A unit, weapon or armour file lists only what it changes, and the
  rest take GDScript's defaults (`#[serde(default)]` over `Default` impls that copy them). The
  tuning names every field (none defaulted, as its `.tres` did). `deny_unknown_fields` on all.
  A list of reals may hold `"inf"` (`reals.rs`): the leash steps end in one.
- **Loader** (`Content::load`): every `.json` under `units/`, `weapons/`, `armour/` by id (the
  name), and `tuning/battle_tuning.json`; a mismatch names the file and the field
  (`serde_path_to_error`, `units/grem.json: slots[1]: ...`), and a unit's item that isn't
  there names `items[N]`. Units name items by path (`weapons/grem_bite`).
- **JSON Schema** (`schema.rs`, schemars): one file a kind under `content/schema/`, with a
  README. A cargo test fails while one is stale; `BLESS_SCHEMA=1 cargo test schema` rewrites
  them.
- **Converted once:** `tools/convert_tres_content.py` copied each `.tres` field's text into
  JSON (numbers as written, so no float passed through a formatter), then was deleted with the
  `.tres` in the same PR. Before they went, every field of every filled definition was checked
  equal to its `.tres` with `==` (512 fields).
- **The fill** (`CrateContent`, `content/definitions/crate_content.gd`): the binding's
  `ContentStore` hands the loaded content to GDScript as dictionaries (ints as ints, reals as
  reals), and `UnitDef`, `WeaponDef`, `ArmourDef` and `BattleTuning` are filled by name. Each is
  made once and shared, as `load()` shared a `.tres`. `BattleTuning.current()` and every reader
  (scenes, trials, benches, tests) take them from it. The fill is a bridge: it shrinks as steps
  3 to 9 port the readers, and after step 9 only what presentation draws stays.
- **The core is always loaded** (open point settled, the user's call): every Godot run (the
  game, both engines, gdUnit) loads the content through the core, so it must be built.
  `NativeLibrary` loads it for the content and for the kernels; `BREACH_NATIVE` picks only the
  kernels. CI's `ci.yml` builds it once (`native` job) for the gdUnit shards and the scaling
  stages; the `rust` job also runs on changes to the content it loads and its schema.
- **The designer:** its content library (`tools/designer/content_library.py`) reads the units'
  and items' traits and tags from the JSON. The designer doesn't write units yet; its export
  and validation are 2b's.
- **Tests:** cargo (`content/mod.rs`: a wrong type and an unknown field name the file and
  field; a missing item names it; omitted fields take GDScript's defaults; the tuning names
  every field; the checked-in content loads; `schema.rs`: the schemas are current), and
  `test_crate_content.gd` (the crate's defaults equal GDScript's; a unit, its shared items and
  an infinite real filled from the files; no `.tres` definition left or read).
- **Files opened:** 36 pre-existing files over the OCP check's 11, under Decision 157: every
  reader of a `.tres` definition changes how it gets it, and nothing else.

### Step 2b, the terrain library in the crate's schema

- **The schema** (`native/rust/src/content/terrain.rs`): `TerrainLibrary` (terrains, features,
  materials, `road_move_multiplier`), `Terrain`, `LiquidBody`, `Stratum`, `TerrainFeature`,
  `Material` and `HeatTransition`, named field for field as GDScript's properties, each entry
  defaulting to GDScript's (`#[serde(default)]`), `deny_unknown_fields` on all. Lists keep the
  designer's order. A colour is `#rrggbb` or `#rrggbbaa` (`Colour`), and anything else names
  its field. `TerrainLibraryDef.source_hash` went: nothing is imported, so nothing goes stale.
- **Loader:** `content/terrain/terrain_library.json`, required beside the tuning. A material a
  stratum, a liquid body or a heat transition names that isn't in the library names its field
  (`terrains[0].strata[1].material_id`). The GDScript `validate()` still runs on the filled
  library.
- **JSON Schema:** `content/schema/terrain_library.schema.json`.
- **Converted once,** from the designer's `content/designer/terrain.json` by the designer's own
  translation (`tools/designer/terrain_schema.py`, `to_schema`), which holds the rules the Godot
  library import had (saves from before Decisions 62 to 66 included). Numbers are copied as the
  designer wrote them. Before `terrain_library.tres` was deleted, every field of the filled
  library was checked equal to it with `==` (a one-off script, deleted). The designer's file
  read back (`from_schema`) equals the old `terrain.json`, so nothing the designer had is lost.
- **The fill:** `CrateContent.terrain_library()` fills `TerrainLibraryDef` and what it holds.
  The fill by name moved to `ContentFill` (`content_fill.gd`; `CrateContent` was over the ISP
  check's method count), and it now fills a typed list of definitions one by one and reads a colour from its hex. It is
  made once and shared. `MapLayoutDef.terrain_library` is no longer saved with a map: it starts
  as that shared library (a test may set its own), and the map `.tres` files lost their
  reference to the library's `.tres`. The map importer takes the same library.
- **The designer:** the server (`designer_repo.py`) reads the library file into the designer's
  shape and writes it back in the schema's, so the designer's pages keep their shape
  (`repo.js` only reports the refusal below). A
  save that drops a terrain or feature a saved map uses is refused and writes nothing (the
  check the Godot import made, now in Python). Saving a map no longer imports the library
  first. Validating the designer's output against the JSON Schema is 2c's, with the map export.
- **Retired:** `content/import/designer_library_import.gd`, `designer_library_importer.gd`,
  `tools/import_designer_library.gd`, their gdUnit tests (their scenarios are the Python tests
  now), `content/designer/terrain.json` and `content/terrain/terrain_library.tres`.
- **Tests:** cargo (`content/mod.rs`: a terrain field of the wrong type, a colour that isn't
  hex and a material that isn't there each name the field; omitted fields take GDScript's
  defaults; the checked-in library loads; `schema.rs`: current), `test_crate_content.gd` (the
  crate's defaults equal GDScript's for every terrain kind; the library filled from its file
  and valid; every layout shares it; no terrain `.tres` left or read), and
  `tools/designer/test_terrain_schema.py` and `test_designer_repo.py` (the translation both
  ways, the old saves, the round trip of the checked-in file, the in-use refusal).

### Step 2c, the whole map in one file, and the designer's translation in the crate

- **The schema** (`native/rust/src/content/map.rs`): `Map` and what it holds (`Node`,
  `GarrisonUnit`, `Subnode`, `StructurePlan`, `StructureFace`, `Lane`, `MapEdge`, `Faction`,
  `FactionRelation`, `LossGroup`, `MapLayout`, `Tile`, `Bridge`, `RoadSegment`, `Route`,
  `Channel`), named field for field as GDScript's, defaulting to GDScript's,
  `deny_unknown_fields`. Every node is listed once under `nodes` and a lane names its nodes by
  id, so the home shared by two lanes is one node, as the `.tres` shared one sub-resource; a
  node on no lane is off-lane, in `nodes`' order. Enums by name (`"FORT"`, `"north"`), cells as
  `[col, row]`, structure cells `[x, y, level]`, a plan's keyed cells as `[cell, value]` pairs
  in the designer's order, `null` for no layout, plan or bridge. Files are written with lists
  of plain values on one line (`content/write.rs`).
- **Loader:** every `content/maps/<name>.json` by name. A lane naming a node that isn't there,
  or a default terrain, tile terrain or feature not in the library, names its field
  (`lanes[0].nodes[1]`, `layout.tiles[1].terrain_id`). `MapDef.validate()` still runs in
  GDScript; a test holds every checked-in map valid.
- **JSON Schema:** `content/schema/map.schema.json`.
- **The designer's translation in the crate** (`native/rust/src/designer/`): the designer
  export into `Map` (the rules of Godot's import: nodes and their designer fields, lanes from
  links, edges, off-lane nodes, the layout, plans, loss groups, the format check and the
  warnings), and 2b's terrain library both ways with the in-use refusal (`terrain_schema.py`'s
  rules). A plan face on a side that isn't one is now an error; Godot's import stored side -1.
  The binary (`src/bin/designer.rs`, `native/bin/breach_designer` from `native/build.sh`) has
  `map`, `terrain-read` and `terrain-write`, and prints the lines the server already read.
- **Converted once:** demo_map, skirmish_p_c and skirmish_two_lanes by the crate's
  translation of their saved designer exports (the designer's own path), p_f_F_c from its
  `.tres` through `MapJson`. Before the `.tres` files were deleted, every field of each filled
  map was checked equal to its `.tres` (592 fields, a one-off script, deleted), except each
  designer map's node `tile`: the `.tres` were imported before Decision 68 gave nodes a tile,
  and nothing in `sim/` or `presentation/` reads it. The terrain library was rewritten once by
  the crate's writer; its values are unchanged.
- **The fill:** `MapFill` builds a `MapDef` from the crate's fields (nodes by id, lanes, the
  off-lane rest), `ContentFill` now fills vectors, cell-keyed dictionaries and a definition
  held alone, and `CrateContent.map(name)` makes each once and shares it. `main.gd`, the
  formation skirmish and the map viewer (`--map=<name>`, read from its file on each Reload)
  take maps from it. `MapJson` writes a `MapDef` in the schema through the crate
  (`ContentStore.write_map`): the map scene's export (`MapSceneRoot`, "Export to JSON") uses it.
- **The importer, kept to what the presentation needs:** `DesignerMapImporter.import_map` and
  `.plan` hand a designer export or plan to the crate (`ContentStore.map_from_export`,
  `plan_from_export`) and fill the result, for the presentation's and the load cases' tests,
  which build maps and plans as the designer draws them. The GDScript builders, the import
  resource, `tools/import_designer_map.gd` and their gdUnit tests are gone; their scenarios are
  cargo tests.
- **The designer:** saving a map writes the export, then the tool writes
  `content/maps/<name>.json`, without Godot; the terrain library is read and written by the
  tool. Godot is needed only for View in Godot. `serve.py`'s API is 5 (`tool_found` in health).
- **Tests:** cargo (`content/mod.rs`: a map field of the wrong type, a node type that isn't one,
  a lane naming a missing node and a tile of a missing terrain each name the field; omitted
  fields take GDScript's defaults; the checked-in maps load; `designer/tests.rs`: the import's
  scenarios, the terrain translation's, and each saved designer map read equals the checked-in
  map), `test_crate_content.gd` (the crate's map defaults equal GDScript's; a map filled from
  its file; a node on two lanes is one node; every map there and valid; a map written by
  `MapJson` reads back the same; no `.tres` under `content/maps/` or read),
  `test_designer_map_importer.gd` (what reaches GDScript), and `tools/designer/`'s Python tests
  (the server runs the tool; the real tool when it is built).

- **And then all of it in Rust** (the user's call above), in the same PR:
  - **The map's rules** (`native/rust/src/content/map_check.rs`): `MapDef.validate()` and what
    it called (lanes, nodes, footprints, subnodes, edges, faction relations, garrison units,
    loss groups, the layout's grid, tiles, bridges, roads, routes and channels), with
    GDScript's messages. The crate checks them when it loads a map (the load fails naming the
    map), when the designer saves one, and when a map file is written. The GDScript
    `validate()` methods of the map's definitions and their gdUnit tests went; their
    scenarios are cargo tests (`map_check_tests.rs`). `FactionDef.validate()`, which nothing
    called, went with them. The terrain library's, units' and items' `validate()` stay
    GDScript: they are 2a's and 2b's, and the user picked the map's.
  - **The fill in the binding** (`ContentStore`, `lib.rs`): the crate hands GDScript its
    definitions made and filled (vectors, cell-keyed dictionaries, enums by name, typed lists,
    a unit's shared items, a map's shared nodes), each made once and shared. `ContentFill`,
    `MapFill` and `MapJson` went; `CrateContent` only calls the store, and `MapFiles` reads and
    writes a map file through it (the map viewer, the map scene's export). Every unit, weapon,
    the tuning, the terrain library and every map, filled this way, was checked equal to the
    `.tres` it was converted from (1637 fields, a one-off script, deleted).
  - **The designer's server** (`native/rust/src/designer/http.rs`, `repo.rs`, `registry.rs`;
    `native/bin/breach_designer`): the same API (version 6), the same local-only and
    whitelisted-name checks, standard library only. It saves a map and the terrain library
    in-process. The Python server and its tests went: the server's scenarios are cargo tests
    (`server_tests.rs`), and the page's JavaScript tests, which Python only drove, run with
    `node --test` (`tools/designer/test/`, in pre-commit when Node is installed).

### Step 3a, the ground in the crate

- **The ground** (`native/rust/src/ground/mod.rs`, `Ground`): the cells (cost, height,
  depth, climb, flows and blocking sight), painting them, a walker's crossing (cost, slope,
  liquid by its height, climbing and swimming by the pair rule), its mode, the high ground
  and the fastest ground painted, with the tuning's `ground_*` read when it is made. The
  cells keep the GDScript's precision (cost, depth and obscurance in f32, as its packed
  arrays did); points are f32 (`Vector2`), shares f64.
- **Sight** (`ground/sight.rs`): TerrainObscurance's cells, fog, sums along a line and the
  budget's test, its box in f32 as `Rect2` computes it.
- **The surface** (`ground/surface.rs`): GroundSurface, GroundRelief and GroundChannels on
  the crate's `MapLayout` and terrain library. Nothing in Godot read them (only their
  tests), so they have no binding.
- **The binding** (`TerrainGround`, `lib.rs`): `FormationTerrain` is a view over it under
  both engines, its API unchanged, so its 33 readers keep their code. A walker is passed as
  its fields. `TerrainObscurance` is a view over the same ground (`terrain.obscurance`).
  `walker_shares` stays on the view for `WalkerShares` until 3c.
- **Identical:** before the GDScript was deleted, a one-off script (deleted) painted 60
  random grounds and compared old and new on 180,060 queries (crossing, mode, factor,
  height, sight blocking, the high ground, the fastest, obscurance at a point and along a
  line, with and without a limit, and the budget's test), all equal bit for bit; and the
  old surface's values on 25 random layouts (relief, channels, liquids; 3,750 columns) were
  equal to the crate's. Outcomes unchanged: CI's digests compare with `main`.
- **Retired:** `sim/ground/` and `tests/sim/ground/`; the GDScript bodies of
  `FormationTerrain` and `TerrainObscurance`; the pure scenarios of
  `test_formation_terrain.gd` and `test_sight_obscurance.gd` (now cargo tests). Their
  scenarios through a battle or a path plan stay gdUnit until those port.
- **Tests:** cargo (`ground/tests.rs`: cost, slope and cliffs, water by height, the pair rule
  for swimming and climbing, modes a walker may not use, the fastest ground, blocking sight,
  obscurance's sums, fog, either way and mirrored, paint order and cell edges; the surface's
  levels, shapes, slopes, digging, cliffs, ceiling, relief, channels, liquids and banks; the
  relief noise's bits as Godot gave them).

### Step 3b, routes and the mover in the crate

- **Stacked on 3a** (the user's call, 2026-10-23): built on `feature/ground-in-the-crate`
  (PR #570) before it merged; its PR is based on that branch.
- **Routes** (`native/rust/src/route.rs`): FormationRoute's length, point, heading and the
  distance of a point along it, and RoutePatch's detour and patching, waypoints in f32 and
  distances in f64 as the GDScript had them.
- **Motion** (`motion.rs`): UnitMotion's turn (and whether it then faces), move and walk
  join its bearing, pace and look. `UnitMotion.vector` is `maths::bearing_vector`, which
  gives the quarter bearings exactly, so the GDScript's lookup and memo of vectors went
  with it (and the benches' clearing of the memo).
- **The mover** (`mover.rs`): TerrainStep's way and pay and UnitMover's step, slide, pay and
  share on the crate's ground (`None`: open ground). `TerrainStep.returns` stays GDScript
  until FormationPour ports (3d).
- **Steering** (`steer.rs`): UnitSteer's rule as a `Way` that weighs each body and steps
  past the nearest; the scrum's native walk (`walking.rs`) uses the same kernel. A unit's
  draw (ScrumContest.draw) is made in the crate from its id. The bodies near the way are
  still found in GDScript (BodyGrid, the scrum's, step 4).
- **The binding** (`UnitMoves`, `lib.rs`): stateless, one shared (`CrateMoves`). A unit's
  motion is passed as [bearing, turn rate, backward pace], a walker as its five fields
  (`TerrainWalker.fields`), a placed step back as [x, y, bearing], the ground as its
  `TerrainGround` (`FormationTerrain.core`). The six classes are views, their APIs
  unchanged, so their readers keep their code.
- **Identical:** before the GDScript was deleted, a one-off script (deleted) ran the old
  GDScript and the views on 273,792 random queries over 40 painted grounds (vector,
  bearing_to, pace, turn, move, walk with and without a look, step, slide, pay, share,
  way, steering with and without a crowd, a route's length, point, heading and distance,
  detours and patching with duplicates and overlaps), all equal bit for bit. Outcomes
  unchanged: CI's digests compare with `main`.
- **Retired:** the GDScript bodies of the six classes; `test_formation_route.gd`,
  `test_unit_motion.gd` and `test_unit_steer.gd`; the pure scenarios of
  `test_route_patch.gd` and `test_one_mover.gd`; `test_det_math.gd`'s check of the
  vector lookup. The scenarios through a battle or a path memory stay gdUnit.
- **Tests:** cargo (`route/tests.rs`, `mover/tests.rs`, `steer/tests.rs`, `motion.rs`):
  the gdUnit scenarios above, plus patches at one place whatever the list, bodies level
  along the way whatever the list, sliding along a bank, staying where nothing is open,
  and the quarter bearings exact.

### Step 3c, pathing in the crate

- **The search** (`native/rust/src/paths/search.rs`, `frontier.rs`): PathSearch's A* and
  PathFrontier's heap, with its precision: costs in f64, each cell's estimate and distance
  aside in f32 as its packed arrays kept them, `Geometry2D.get_closest_point_to_segment` in
  f32 as Godot computes it.
- **The shares** (`shares.rs`): WalkerShares, one per kind of walker (its height by its
  bits, its levels and modes), held beside the ground in `TerrainGround` and dropped when
  it is painted, as `FormationTerrain.walker_shares` was. A share worked out just now is
  given back unrounded and a cached one rounded to f32, as the GDScript did; ported as it
  is (spec 39's rule) and raised as #575. Removing it changed no answer on 25,920
  queries.
- **Sight, memory and climbing** (`sight.rs`, `memory.rs`, `climb.rs`): PathSight's shape
  and test, PathMemory's cells (the GDScript keeps it between plans and hands its bytes
  over), ClimbBudget, and the two rules it calls, ClimbStamina's `standable` and
  ModeStamina's `step_cost`.
- **Plans** (`paths/mod.rs`): TerrainPaths' plan, rejoin, cells, cost, straightening and
  legs.
- **The look** (`pathing.rs`): FormationPathing's look ahead, its group walker and
  SquadPathfinder's pick, as one call over the group's living units.
- **Boundary moves** (Open's rule): ClimbStamina's `standable` and ModeStamina's
  `step_cost` became views now, not in step 6, since the climb budget calls them and one
  rule should not have two homes. Which groups look, `_follow` and the commands' patches
  stay in FormationPathing until the tick ports (step 8): a squad's route and its
  command's patches are not in the crate's store yet. `TerrainWalker.kind` went with
  WalkerShares, its only reader; its test is a cargo test of the shares.
- **The binding** (`GroundPaths`, `lib.rs`): stateless, one shared (`CratePaths`). A
  walker for a path is passed as its seven fields and the climb's tuning as it stands at
  the call, a sight as its nine fields, a memory as its bytes (null: none), a group's
  living units as sixteen numbers each. The seven classes are views, their APIs unchanged.
- **Identical:** before the GDScript was deleted, a one-off script (deleted) ran the old
  GDScript and the views on 77,745 random queries over 60 painted grounds (cells, plans
  with and without sight and memory, replans from the edge of sight, rejoins, the search's
  cells, expanded and reaches, costs, sight and reach, standing, a step's stamina, the
  pathfinder and the look ahead, 303 of which found a way round), all equal bit for bit,
  the shares' caches warming in the same order. Outcomes unchanged: CI's digests compare
  with `main`.
- **Retired:** the GDScript of WalkerShares, ClimbBudget and PathFrontier, and the bodies
  of TerrainPaths, PathSearch, PathSight, SquadPathfinder, `ClimbStamina.standable`,
  `ModeStamina.step_cost` and FormationPathing's look and group walker;
  `test_terrain_paths.gd`, `test_walker_shares.gd`, `test_route_rejoin.gd` and
  `test_squad_pathfinder.gd`; the pure scenarios of `test_path_sight.gd`,
  `test_sight_obscurance.gd`, `test_climb_stamina.gd`, `test_formation_pathing.gd` and
  `test_terrain_walker.gd`. The scenarios through a battle stay gdUnit.
- **Tests:** cargo (`paths/tests.rs`, `pathing.rs`): the gdUnit scenarios above, plus the
  shares looked up twice, heights an ulp apart kept apart, a group walking its route
  backwards, and the pathfinder and the way round the same whatever the order of the
  units.

### Step 3d, the march in the crate

- **Stacked on 3c** (the user's call, 2026-10-23): built on `feature/pathing-in-the-crate`
  (PR #576) before it merged; its PR is based on that branch.
- **The march** (`native/rust/src/march/mod.rs`, `sweep.rs`): FormationMarch's toward, travel
  sign, its front rank's pace and its facing, and FormationSweep's rate and turn. Godot's
  `is_equal_approx`, `is_zero_approx`, `signf`, `clampf`, `minf`, `maxf` and `lerpf` are in
  `maths.rs` (motion's `clampf` and `maxf` moved there).
- **The walk** (`walk.rs`, `modes.rs`): FormationWalk's walk, for a squad's units in one call
  (each reads only its own columns, the ground and the bodies lying), with WalkModes'
  climbing, swimming, the spent's start and the fall (DetPow's `pow`, now `det_pow.rs`); its
  share, slack and rule. A fall's hurt is handed back, and GDScript applies it (UnitHealth,
  step 6) after the walk, in the units' order. The bodies underfoot (`lying.rs`,
  GroundBodies.underfoot) are the scrum walk's, now shared. The GDScript skipped the walker
  among the lying; a walker is alive and the lying aren't (`sync_units` walks only units
  that are), so the crate has no skip.
- **Pouring** (`pour.rs`): FormationPour's place and targets: the squeeze, the queue (front
  first, then nearest the centre line, then by draw), the filing in, and one out taking a
  downed foe making for beside it.
- **Re-forming** (`shuffle.rs`, `front.rs`): the moves a re-forming squad starts
  (FormationShuffle's planning, ShuffleFront, SquadPlaces' occupancy), and
  PlaceOrder.front_first. Two units on one place, which a frame never holds, keep their
  order: the crate's sort is stable, where GDScript's `sort_custom` left such ties to its
  sort past 16 units.
- **Boundary moves** (Open's rule): `FormationStaging` moved to step 5. It is a state
  machine over a squad's staging dictionary and its events, and its rules are sight
  (FormationSight, step 4) and the rendezvous' timing (`FormationRendezvous.ticks_to`, step
  5), with no arithmetic of its own worth a native call. The swaps under way (their
  countdown, ends and widening: FormationShuffle's `step`, `offset` and `movers`), the
  arrivals (`check_ends`, which calls FormationWithdraw, step 4) and the placing of routers
  and loose units in `sync_units` (FormationRout, step 7) stay GDScript until those port or
  the tick does (step 8): the swaps and the staging are not in the crate's store. ModeStamina's
  `rate` and `can_start` went into the walk modes, their only reader. PlaceOrder became a
  view now, not in step 5, since the re-form calls it (SquadCompact reads it too).
  `TerrainStep.returns` stays a call of the pour's targets.
- **The binding** (`GroundMarch`, `lib.rs`): stateless, one shared (`CrateMarch`). A squad's
  frame is passed as five numbers, a unit making for a point as seventeen, a walker of the
  march as eleven with its columns as nine, a re-forming unit as seven, and the bodies lying
  as where each lies with its radius and area, worked out once a tick. The views' APIs are
  unchanged, and `FormationWalk.walk_all` is new: a squad's walk in one call, over the
  march's hold.
- **Identical:** before the GDScript was deleted, a one-off script (deleted) ran the old
  GDScript and the views on 76,720 random queries over 400 grounds (streams with fords,
  cliffs, climbs, woods and water, or open ground):
  - toward, travel sign, the pace (with its blocked event), facing, sweeps and rates;
  - targets with and without swaps and loose units (84 sets with units squeezed), returns,
    places, shares, slack and rules;
  - 28,221 walks with and without bodies lying (10,421 on a climb or a swim, 24 falls),
    1,200 whole syncs and 400 placings;
  - 3,200 re-form plannings (2,369 started moves, 1,434 through) and 2,400 front-first
    orders.

  All were equal bit for bit. Outcomes unchanged: CI's digests compare with `main`.
- **Retired:** WalkModes, ShuffleFront, SquadPlaces and `tools/shuffle_phases.gd`
  (`tick_phases` times the shuffle as one phase now); the bodies of FormationMarch's pace,
  toward, travel sign and face, FormationSweep, FormationWalk's walk, share, slack and rule,
  FormationPour, FormationShuffle's planning, PlaceOrder, and ModeStamina's `rate` and
  `can_start`; `test_walk_modes.gd` and `test_place_order.gd`; the pure scenarios of
  `test_formation_walk.gd`, `test_formation_pour.gd` and `test_mode_stamina.gd`. The
  scenarios through a battle stay gdUnit (`test_formation_sweep.gd`,
  `test_formation_shuffle.gd`, `test_formation_staging.gd`, the walk's and the pour's).
- **Tests:** cargo (`march/tests.rs`, `det_pow.rs`): the gdUnit scenarios above, plus a walk
  over a climb face climbing before it moves on, a unit part way through a swap, the front
  stopping at its target, travel signs, the pace's worst ground, a sweep's turn, a free
  front place, passing through, spreading within the combat width in a fight, DetPow's
  powers, and the targets, the pace and the moves planned the same whatever the order of
  the units.

### Step 4a, the per-unit scrum in the crate

- **The field in both engines** (the user's call, Steps, "Step 4"): `NativeKernels.body_field`
  makes a `BodyField` under either engine, and `unit_store` always the native store (on a
  new field if none is given). `UnitStoreGd` is gone. `BREACH_NATIVE` picks only body
  parting's engine (`NativeKernels.parts_natively`, read by `UnitBodies.step`), the last
  pass with a GDScript twin; CI's two-engine digests compare that one.
- **The seek, the walk, the facing and the blows** already ran in Rust on the field
  (Decision 129: `scrum.rs`, `seeking.rs`, `walking.rs`, `facing.rs`, `blows.rs`). Their
  GDScript twins are gone: ScrumSeek's own plan and `SlotSearch`, ScrumSlots' slots,
  `FoeIndex`, ScrumNear's gaps and its `only`, FormationScrum's `_walk`, `_footing`,
  `_faces` and `_seeking_look`, ScrumBlows' search and BlowLanding's `_pressed`. Each is a
  phase in the crate that `BodyField` calls (`seeking::seek`, `walk_store::walk`,
  `face_store::face`, `blows::picks`). A rolled tick with no melee search (shots only)
  now syncs the field for the landing's count (`ScrumBlowsField.sync`), where GDScript
  counted it before: the same count, from the same points.
- **The contest and the reach** (`contest.rs`, `reach.rs`): ScrumContest's draws, squad
  draws and rolls, and ScrumReach's radius, gap, front and area, bound by `ScrumRules`
  (stateless, one shared: `CrateScrum`). ScrumContest and ScrumReach are views over it,
  their APIs unchanged; `ScrumContest.before` (a key compare other passes use) and
  `ScrumReach.at`/`at_in` (where a unit stands, from its squad's entries) stay GDScript.
  The seek's contest order, the steering's and the march's draws (`steer::draw`, moved),
  and the facing's, blows' and seek's touch and front tests use them, one rule in one
  place.
- **Kept GDScript for 4b and 4c**: ScrumNear (engage, regroup, the rout), BodyGrid,
  ScrumSlots' bodies (regroup, pursuit), ScrumBlows' touches, hostiles, nearest touching
  (regroup), ticks and clearing of targets, and ScrumSeek's foe ids.
- **Identical:** before the GDScript was deleted, a one-off script (deleted) ran the old
  ScrumContest and ScrumReach and the views on 280,000 random queries (draws, squad draws
  and keys with seeds and ids across their range, radii, gaps with bodies apart, touching
  and on one point, fronts at any bearing and on the quarter ones, and areas turned every
  way), all equal bit for bit. The seek, walk, facing and blows were the Rust engine's
  already, identical to GDScript on CI's digests since Decision 129. Outcomes unchanged:
  CI's digests compare with `main`.
- **Retired:** `slot_search.gd`, `foe_index.gd`, `unit_store_gd.gd`; the GDScript bodies
  above; `tools/facing_bench.gd`, `steering_bench.gd` and `blows_bench.gd` (each timed a
  twin against the core); `test_scrum_slots.gd`, `test_formation_mirror.gd`,
  `test_walk_field.gd`, `test_face_field.gd`, `test_blows_field.gd` and
  `test_foe_index.gd` (its draw case is `test_nearest_touching.gd`, for regroup's
  GDScript); the twin halves of `test_scrum_search.gd` and `test_body_field.gd`; the
  GDScript backend of the store's contract tests. `test_face_field.gd`'s DetMath check is
  in `test_det_math.gd`.
- **Tests:** cargo, on a scrum built by `build::scrum` (and `build::reversed`):
  `scrum/tests.rs` (slots round a grem and a brute, the nearest open from the foe's front,
  a slot under a body, claimed or past the leash, waiting a breadth short, touching and
  standing, mirror slots about as often, the list order), `facing/tests.rs` (the foe in
  front before a nearer one at its side at its turn rate, nothing to face, the walk's
  look, the list order), `walking/tests.rs` (a step to a slot, none touching, back to its
  place, round a body, slowed underfoot, the list order), `blows.rs` (the list order),
  `contest.rs` (draws pinned to Godot's, the die, the order, the list order) and
  `reach.rs`. gdUnit: a battle has a field under either engine (`test_body_parting.gd`).

### Step 4b, the squad-level scrum in the crate

- **Stacked on 4a** (the user's call, 2026-10-23): built on `feature/scrum-units-in-the-crate`
  (PR #581) before it merged; its PR is based on that branch.
- **The squads' records** (`squads.rs`; Steps, "Step 4"): each squad's loose and fleeing units
  (their ids, in the order each entry was given), its stance (anchor, heading, foe), the foes
  on its edges (edge, foe, since, in the order each began; an edge set again keeps its place,
  as a dictionary's key does) and its chasers (unit, foe, from, leash, and whether it has
  turned back), all in the snapshot. SkirmishSquad's dictionaries of the same names are views
  (`SquadRecords`), taken once and kept until a call changes a record (the field's
  `records_edits`), so none is changed in place: `flank`, `unflank_all`, `chase`,
  `chase_back` and `unchase` on the squad, the entries and the locks do it. A squad carried
  to another store carries its records (`SquadRecords.carry`). A chase's turning back was a
  key set in its dictionary (`returning`); it is the record's flag now, read back the same.
- **The entries and the locks** (`entries.rs`, `locks.rs`): SquadEntries' giving and taking
  away, EntryKeys' `keep` and `forget` (the reads stay GDScript, for the digest and the
  seek's batch), FormationLocks' lock and release, FormationContact.can_engage and
  ScrumSeek.foe_ids.
- **The phases** (`engage.rs`, `stance.rs`, `manoeuvre.rs`, `ranks.rs`, `trade.rs`, with
  Godot's `Rect2` in `rect.rs` and SquadFrame's `basis` and `place_in` in `frame.rs`):
  ScrumEngage's search and locks; ScrumStance's threat, commitment and letting go, the turn's
  anchor staying GDScript (SquadEdges.reach, step 4c's edges) and landing after the crate's
  call, as a stance moves no frame; FormationManoeuvre's step; FormationScrum's stall;
  SquadRanks' closing; ScrumTrade's track and trade. What the crate doesn't hold is handed in:
  where units stand (ScrumReach.at), each squad's bounds, discipline and the hostiles it
  detects (FormationSight, perception being 4c's), withdrawing and skirmishing (4c's), and
  footprints and kinds. A free squad's sight is now looked at for a squad committed to its
  stance too (it used to stop first); sight decides nothing there.
- **Holds** (Decision 144): a pass holding columns the crate now writes (the rout's `KEYS`,
  `ALONG` and `FLEE_OFFSET`, the regroup's places and loose points) lends them to each call
  (`ColumnsLent`): one slot's touched columns for an entry or a key, every held column for a
  trade or a lock. The regroup no longer holds `STUCK`, which the crate notes.
- **Identical:** each phase reproduces the GDScript's order and precision (`Vector2` and
  `Rect2` in f32, Godot's `snappedf`, `fposmod`, `roundi` and `posmod`, the draws' ties).
  Unlike 3d and 4a there was no one-off comparison script; the proof is CI's digests,
  which compare with the base on both engines. Outcomes unchanged.
- **Retired:** the GDScript bodies of the classes above; `test_scrum_trade.gd` and
  `test_scrum_trade_turns.gd` (their scenarios are `trade/tests.rs`). The scenarios through a
  battle stay gdUnit (`test_formation_manoeuvre.gd`, `test_formation_engage_rule.gd`,
  `test_stance_heading.gd`, `test_entry_keys.gd`).
- **Tests:** cargo: `entries.rs` (a loose entry's start, its place in the order, fleeing, keys
  held with no value), `locks.rs` (a lock's first blows, releases front and flank, foes in
  the lists' order), `engage.rs` (reach whatever the faces, friends and the unfree, the
  nearest and the draw whatever the lists), `stance.rs` (turning to a flank, no reason to,
  letting go unless holding, the draw), `manoeuvre.rs` (priorities, a narrowing's re-form, a
  stall), `ranks.rs` (closing up, a wide and a deep unit, the lists' order), `trade/tests.rs`
  (the three GDScript scenarios, a chaser and a stance, trades in turn whatever the lists),
  `frame.rs` and `rect.rs`.

### Step 4c, regroup onwards in the crate

- **The roster** (`roster.rs`, `SquadRoster`): GDScript keeps who is living (UnitState,
  Decision 147), so each phase is given the squads as `CrateRoster` fills them once a tick:
  per squad its slot, id, faction, whether units joined it and whether it merges, and per
  living unit what the crate doesn't hold (id, slot, footprint, band, ranged reach, kind,
  turn rate, backward pace, detection, most stamina, and its walker on terrain). Where each
  stands, faces and its place are the store's columns. FormationScrum makes one after the
  seek for regroup, the pursuits and the withdrawal; FormationSimulation one in `_engage`
  for perception, the locks, skirmishing and the march's limits (nobody falls or joins
  between them).
- **The phases**: ScrumRegroup's walk back, a drilled retreat's foes faced and the trade
  (`regroup.rs`); ScrumPursuit's chase and breaking ranks, and FormationPursuit's lag and
  spread (`pursuit.rs`); FormationWithdraw's engaged and safe tests, each unit's flight and
  whether the squad is home (`withdraw.rs`); the squads' geometry - SquadFrame's corners and
  spans, SquadGeometry's overlap and facing off, SquadEdges' edges, reach and bounds
  (`geometry.rs`); FormationContact's limit, skirmishing and joining, FormationFronts' and
  FormationEdges' locks (`contact.rs`); FormationSight and FormationPerception
  (`perception.rs`). The bodies a walker steps round and a faction's foes indexed where they
  stand are `near.rs`, shared; Godot's `Rect2.merge`, `grow` and `intersects` are in
  `rect.rs`, `DetMath.rotated` and `move_toward` in `maths.rs`.
- **Kept GDScript, with the orders (step 7)** (the boundary moved, "Open"): each squad's
  `pursuit` and `withdraw` records and what they drive - ScrumPursuit.retreat,
  FormationPursuit's give-up, going back and arriving, FormationWithdraw.begin, its
  `safe_ticks` and its re-form - with ScrumTurn, the events and the morale shocks. Also
  FormationContact.reinforce (the groups', step 5), SquadGeometry.gap and unit_gap (joining,
  ranged combat), ScrumNear and BodyGrid (the rout, wounds, recovery, groups),
  FormationStamina.squad_gives_up (step 6) and RoutFlight.fan (the rout's, handed in).
- **Identical:** each port follows the GDScript's order and precision (`Vector2` and `Rect2`
  in f32, Godot's `minf`, `snappedf`, `fposmod`, `roundi`, `lerp`). A search through a grid
  looks at a superset of what qualifies in the lists' order, so it picks as a look at every
  body did (`near.rs`, and a cargo test steering both ways). Before each commit, a one-off
  run of nine digest battles (the mirrors, the five field logs, two unrolled) on `main` and
  on the branch gave the same hashes, every tick. The proof is CI's digests. Outcomes
  unchanged.
- **Retired:** `scrum_slots.gd`, `unit_steer.gd`, `unit_shuffle.gd`, `scrum_trade.gd`,
  `squad_memo.gd`; ScrumBlows' touches, hostiles and nearest touching, FormationStamina's
  `gives_up`, EntryKeys' `set_goal`, the passes' GDScript bodies above, and
  the bindings nothing calls (`UnitMoves.steer`, `BodyField.trade_track` and `trade`).
  `test_scrum_regroup.gd`, `test_nearest_touching.gd`, `test_squad_memo.gd` and
  `test_squad_heading.gd` (its scenarios are `geometry/tests.rs`), the steering half of
  `test_scrum_search.gd`, and the unit scenarios of `test_formation_engage_rule.gd` (now
  `contact/tests.rs`). The scenarios through a battle stay gdUnit (`test_formation_retreat`,
  `test_formation_withdraw`, `test_retreat_home`, `test_formation_contact`,
  `test_formation_perception`, `test_formation_sight`, `test_formation_terrain_sight`).
- **Tests:** cargo: `regroup/tests.rs` (the walk back at the re-form pace, holding while
  regrouping, rejoining in place, a stance keeps them loose, chasers and the withdrawing left
  out, a drilled retreat backing away, an undrilled one turning, steering round a body,
  steering through the crowd as through every body, the lists' order), `pursuit/tests.rs`
  (after the nearest, turning back at the leash, out of sight, tired or its quarry gone,
  rejoining, a fallen chaser, breaking ranks by chance and nearness, lagging, spread, the
  lists' order), `withdraw/tests.rs` (fleeing at full pace, disengaging at the re-form pace,
  the fan and its limit, home, safe, the lists' order), `geometry/tests.rs`,
  `contact/tests.rs` (front locks, out of reach, one getting away, the draw whatever the
  order, limits at a front and behind a friend, skirmishing, joining, flank locks and the
  nearer holding an edge, who may be engaged, a rout met where its units are),
  `perception/tests.rs` (in range, screened by a wood, the tick's picture, none living),
  `near.rs` and `rect.rs`.
- **Step 4 is done** with this PR.

### Step 5a, groups and the split in the crate

- **The split** (`split.rs`): which of a group's placed units (not loose, fleeing or out
  taking) are fallen behind their places and cut off in clusters, the clusters' order by
  their units' least draws, each new group's units lined up across its heading and its
  front along the route. Nobody moves until every split is found, so the crate decides them
  all at once. `GroupSplit` hands it the squads at ease on a route (`CrateRoster`), whether
  each waits for every unit (FormationWalk.rule_of), its route's points, who is out taking
  and each unit's swap offset (FormationShuffle.offset, the swaps staying GDScript), and
  makes the new groups from its answer: the SkirmishSquad, the places in rank 0, the
  entries let go and the "split" events.
- **Forming up** (`groups.rs`): the pairs of groups near enough (GroupPairs' grid), whether
  two may form up, the gap between their nearest bodies (exact up to `group_join`, through a
  grid for large groups), the meetings nearest first with the squads' draws for ties, and
  each meeting's rule - taken in by the larger, a leader that gathers or joins, merging, and
  led groups sending units back to the command they started under. Meetings are decided in
  turn on the groups as the ones before left them: the crate moves the units of a group
  taken in or sent back between its own lists, as `reinforce` and `enlist` move them.
  `FormationGroups` hands it each group's command and route (codes) and each unit's
  leading, its "joins" and "gathers" traits and the command it started under, and lands the
  meetings in order: `_take_in` (the event, `reinforce`, re-forming) and the units' return
  (their rank behind the home group's back row, `enlist`, the "rejoined" events).
- **Kept GDScript for 5b**: `FormationContact.reinforce` and the compaction it ends with,
  and `_back_row`, which a returning unit's rank reads after earlier meetings' reinforces
  have moved ranks.
- **Identical:** each port follows the GDScript's order and precision (`Vector2` in f32,
  `snappedf`, the draws). The proof is CI's digests, which compare with the base on both
  engines. Outcomes unchanged.
- **Retired:** `group_pairs.gd`; FormationGroups' `_meet`, `_taker`, `_larger`, `_worth`,
  `_together`, `_leader_has`, `_led`, `_merging`, `_back_row_in` and `_gap`; GroupSplit's
  `_behind` and `_clusters`. `test_group_pairs.gd` (its scenarios are `groups/tests.rs`), the
  gap case of `test_formation_near_passes.gd` (now `groups/tests.rs`), and
  `test_formation_groups.gd`'s two read counts (Decision 150's reads once a pass: the
  GDScript reads no columns there now). The scenarios through a battle stay gdUnit
  (`test_formation_groups.gd`, `test_formation_list_order.gd`).
- **Tests:** cargo: `split/tests.rs` (fallen far behind and lined up across, no man left
  behind, every cluster behind, one near its place keeping its cluster, the loose and those
  out taking left out, clusters in their draws' order, the lists' order) and
  `groups/tests.rs` (one command's groups, a leader and the leaderless, gathering, joining,
  not having fought, merging, too far apart, led groups sending units back, a group taken in
  met no more and the taker meeting with what it took, the lists' order, the pairs in list
  order and as every pair finds them, the gap exact within `group_join`).

### Step 5b, closing up, strays, rendezvous and staging in the crate

- **Closing up and taking in** (`compact.rs`): SquadCompact's step-up (front first, a unit
  looked at again only when a place in its columns changed, the moves under way holding
  where they head, the re-anchoring when the front has all fallen) and
  FormationContact.reinforce's moves (the leader's frame shifted to centre both lines on the
  wider, the joining units behind its back rank, striking no sooner than next tick, no longer
  loose or fleeing, the leader's width and centre shift, then closing up over both). The
  units' lists and commands (`enlist`, `joined`) stay GDScript; so does the squad's position,
  which its front's setter places (`SquadCompact.placed`).
- **Strays** (`strays.rs`): a stray's setting off (fleeing home from where it stands along
  its formation's route, walking straight, in its squad's only place, its front where it
  stands) and the strays' numbering by their draws. The stray's squad, its command and orders
  stay GDScript (`FormationStrays.strand`).
- **Rendezvous** (`rendezvous.rs`): the predicted march, tick by tick, and the waits.
  FormationRehearsal stays GDScript (not this round's).
- **Staging** (`staging.rs`): which friends fire a staged wave's trigger and whether a
  coordinated leader's timing lets it go, on the tick's roster (`_engage`'s). Holding at the
  staging point, the fallback, giving up or turning back and the events stay GDScript with
  the orders (step 7), as 4c's order handling did.
- **Identical:** each port follows the GDScript's order and precision (`Vector2` in f32,
  `rad_to_deg`, `fposmod`, `clampf`, the draws). The native calls that write the unit store
  are lent its held columns (ColumnsLent.all), as FormationRout's rallying calls reinforce
  under a hold. The proof is CI's digests, which compare with the base on both engines.
  Outcomes unchanged.
- **Retired:** SquadCompact's `Held` and its loop, FormationContact's `_back_rows` and
  reinforce's moves, FormationStrays' setting off and sort, FormationRendezvous' `_tick` and
  `_places`, FormationStaging's `_triggered`, `_time_to_go` and `SLACK_TICKS`.
  `test_skirmish_squad.gd`'s four step-up cases (now `compact/tests.rs`). The scenarios
  through a battle stay gdUnit (`test_formation_rendezvous`, `test_formation_staging`,
  `test_formation_leaders`, `test_formation_command`, `test_formation_wounds_order`,
  `test_formation_recovery`, `test_entry_keys`).
- **Tests:** cargo: `compact/tests.rs` (closing a column, a wide unit waiting for its whole
  footprint, passes and the units behind following, a move under way's place, only the front
  band into the front rank, re-anchoring, the lists' order, falling in behind centred on the
  wider, a narrower squad centred and stepping up, the half cell left over, the entries
  cleared), `strays/tests.rs` (fleeing from where it stands, walking straight, numbered by
  the draws whatever the order), `rendezvous/tests.rs` (a straight march, setting out part
  way, a wheel, a tighter slack, slow ground, the waits) and `staging/tests.rs` (seeing the
  partner, only the named one, a friend on another route and none on its own or hostile,
  seeing a fight, a coordinated leader waiting and going, the lists' order).
- **Step 5 is done** with this PR.

### Step 6a, stamina in the crate, and the last twin retired

- **Stamina** (`stamina.rs`): who runs (fleeing; pursuing or retreating unless its leader or
  all its units keep to a march; hurried, while not fighting), a unit chasing on its own
  running, a run building up and a pursuer reacting first by its wits, the spent unable to
  run, running, a laden march and blows spending stamina and restarting the breather,
  recovering after it (none on a climb face) by constitution and condition, the pace by
  stage, and a formation too tired to keep up a chase. `FormationStamina` hands it each
  squad's slot, whether it pursues and is on its way back and whether it is hurried (the
  pursuit record and the command stay GDScript), and per living unit its id, slot, fresh
  speed, most stamina, tiring, run pace, wits, constitution, load stage, leading and its two
  "at a march" traits. A unit made by hand still takes its fresh speed from its SPEED column
  first. `stage_at` is `ScrumRules.stamina_stage`, which the landing's skill shift reads.
- **The bodies' parting**: its Rust (`parting.rs`, `field_parting.rs`) was already the
  battle's under `BREACH_NATIVE=rust` (Decision 129). UnitBodies' GDScript passes are gone, so
  every battle parts on its field. With the last twin gone, `BREACH_NATIVE`, the project
  setting `breach/native/engine` and `NativeKernels.use`, `engine` and `parts_natively` retire
  (Open, "The last twin"). The tools lose their `engine=` argument (an old base still reads
  it, so the bench's default keeps `engine=rust`); `native_bench`'s engines are `rust` and
  `rust_threads`. CI's suite and scaling jobs drop the variable, and each digest job takes
  one digest and compares it with the base's (`BREACH_NATIVE=rust` is still set on the base's
  run, for a base from before this step).
- **Identical:** stamina follows the GDScript's order and precision (f64 throughout, Godot's
  `maxf`, `minf`, `clampf`; `stamina_run * tiring * seconds` and `fresh_speed * gait * pace`
  in that order). The native calls are lent held columns (ColumnsLent.all). The parting's
  Rust is the code the `rust` digests already ran. The proof is CI's digests against the
  base. Outcomes unchanged.
- **Retired:** FormationStamina's `_step_squad`, `runs`, `_build`, `_marches`, `_spend`,
  `_breathe`, `_march_cost` and its held columns; UnitBodies' `_passes`, `_drawn`,
  `_bodies`, `_pairs`, `_framed`, `_push`, `_keeps`, `_mass`, `_move`, `BUCKET` and
  `EPSILON`. `test_unit_bodies.gd` (now `parting/tests.rs`), `test_formation_stamina.gd`'s
  stamina cases (now `stamina/tests.rs`; its load cases stay), and the engine comparisons
  in `test_body_parting.gd`, `test_field_test_record.gd`, `test_formation_pour.gd`,
  `test_terrain_toll.gd`, `test_unit_store.gd` and `test_unit_walk_columns.gd`. The
  scenarios through a battle or the store stay gdUnit (`test_unit_stamina.gd`,
  `test_body_parting.gd`'s kept field and list order, `test_formation_pour.gd`).
- **Tests:** cargo: `stamina/tests.rs` (running tires and a light march doesn't, a run
  building up and a pursuer reacting, a chaser running on its own, the breather and a
  hardy unit's shorter one, a heavy load and a poor condition, no breather on a face, who
  runs, a routing formation and a returning pursuit, the spent can't run, the stages'
  paces, a mostly tired formation giving up, the lists' order) and `parting/tests.rs` (a
  body's radius, mass, foes stepping back half, a framed unit giving way and loosened, a
  brush, holding its space against another squad's loose unit, two squads' frames and one
  squad's, bodies on one spot, the lists' order on a field). The parting's were written
  against Rust that already passed them: they had no red run.

## Open, for the implementing session to settle and record here

- ~~The squad store's layout and which fields stay per-squad objects (step 1).~~ Settled in
  "Built", step 1.
- ~~The snapshot's on-disk format and where snapshots live (step 1, used in step 8).~~ The
  format is settled in "Built", step 1. Which snapshots are committed, and their size, is
  step 8's.
- A chunk's boundary may move when a dependency demands it. The session records the move here.
- **Step 4c's boundary moved** (2026-10-24): the squads' `pursuit` and `withdraw` records,
  and the order handling around them (a retreat's start, a pursuit's giving up, going back
  and arriving, a withdrawal's record and re-form), stay GDScript with step 7's orders: they
  drive ScrumTurn, the events and the morale shocks, all step 7's. Their per-unit rules port
  here. Recorded in "Built", step 4c.

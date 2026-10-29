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

- **Split** (2026-10-24, the user's call: about 1,700 lines of GDScript in nine classes):
  **7a** morale's pressure and recovery (`FormationMorale.step`) and the blows on routers
  (`RoutBlows`); **7b** the rout: breaking, the flight, the crush, the rally and the hold,
  re-forming and reaching home (`FormationRout`, `RoutFlight`, `RoutFriends`, `RoutCatch`);
  **7c** orders: applying them, the retreat, and the pursuit's and the withdrawal's records
  and their events (`FormationSimulation._apply_orders`, `ScrumPursuit.retreat`,
  `FormationPursuit`, `FormationWithdraw`'s record and re-form). 7c's PR closes the step.

### Step 8, the tick in Rust, and the tools under cargo

- `FormationSimulation.tick` moves to the crate: one call a tick from Godot, the events handed
  back as data. The two-engine digest comparison and `BREACH_NATIVE` retire with the last twin.
- The digest, the trials, `scaling_check` and the benches (`tick_phases` included) become
  cargo binaries that CI runs in release mode. `tick_phases` gains spec 38 step 2's series,
  `terrain=mixed` and per-pass changed counts. `check_outcomes.py` and the outcome trace (spec
  37) read their output unchanged.
- Snapshot tests for every phase, cut from the digest's battles.
- **Split** (2026-10-28, the user's call): **8a** the rules still GDScript in the tick, into the
  crate: ranged combat and the blows' landing (**8a1**), then the rest (**8a2**: ScrumTurn, the
  joins, staging, the bodies' drag, the shuffle and what else the tick still decides in
  GDScript); **8b** the tick itself: spawning, the order queue, the routes and the ground held
  by the crate, one `step()` call, the events handed back as data; **8c** the digest, the
  trials, `scaling_check` and the benches as cargo binaries (`tick_phases` with spec 38 step
  2's series, `terrain=mixed` and per-pass changed counts), the Godot digest retired after the
  cargo one matches it on `main`; **8d** the snapshot tests for every phase, cut from the
  digest's battles. Each is an identical port, proved on the digests against the base. 8d's
  PR closes the step. **8a2 split further** (2026-10-28, under the user's standing leave to
  split): **8a2-i** ScrumTurn and the joins' order; **8a2-ii** staging's hold and fallback
  and the bodies' drag (GroundBodies); **8a2-iii** the shuffle's swaps under way, their ends
  and the widening, and whatever else the tick still decides in GDScript. **8a2-iii split
  further** (2026-10-28, under the same leave): **8a2-iii** the shuffle's swaps' ends, the
  widening and the closing up after; **8a2-iv** whatever else the tick still decides in
  GDScript (surveyed by its session). **8a2-iv split further** (2026-10-28, under the same
  leave), on its survey: **8a2-iv** morale's ceiling, bands and shocks; **8a2-v** the march's
  ends (an arrival, a retreat's return home) and who may shoot (8a1's kept). What else the
  tick decides in GDScript is the tick itself, 8b's: which squads march, hold or wait, which
  groups look ahead and their commands' patches, landing the hurts, and the events.

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

### Step 6b, the downed in the crate

- **The fallen** (`deaths.rs`): which of the fallen die (past minus their constitution) and
  which are downed, their hp put to 0, the wounds they take (1 and their last blow's) and
  their time downed left to reckon. FormationDeaths keeps taking the fell queue and finding
  each fallen unit's squad (`_fallen`, `_place`: the lists are GDScript's), and lands the
  fates: the states, the wounds, the morale, the step-ups and a squad destroyed.
- **The takers and surrender** (`wounds.rs`): the standing and the downed in their draws'
  order, a body guarded by a friend within `wounds_guard_reach`, the nearest unguarded foe's
  body within `wounds_reach` and a taker with no foe of its own near, a body playing dead
  fooling a taker, a captor, a leader's messenger (the messengers left counted down within
  the pass) and the strikes through death's door on each taker's interval (its cooldown,
  target and the body's hp, a strike that takes a body from above 0 putting it on the fell
  queue); and the struck router's surrender roll. FormationWounds hands it every squad's
  living and lying bodies, and lands each body's taking in order: the states, the messenger
  spent, the morale shocks and the events. Which struck routers it asks about (routing,
  standing, no "never_surrenders") and a squad's destruction stay GDScript.
- **Recovery and regeneration** (`recovery.rs`, `regeneration.rs`): condition, its drain
  (onto the fell queue when it crosses 0), regeneration's mending, the time downed, playing
  dead with a foe near (its own cunning or its leaders'), and those coming to - each finds
  the nearest joinable friendly formation in sight, then they rise nearest first, ties by
  their draws, at 1 HP at least. The nearest is a search of the grid within the unit's
  detection; its key ([gap, the squad's draw, the unit's place]) is a total order, so it
  equals the ring search it replaces. FormationRecovery hands it the squads, and lands the
  deaths, those playing dead, and those coming to: `_join` (the lists, `enlist`, the place
  at the back and the entry), or a stray for FormationStrays.
- **Kept GDScript**: `FormationRegeneration.scorch`, which the blows' landing calls while it
  holds REGENERATION_HALT (it ports with the landing).
- **Identical:** each port follows the GDScript's order and precision (`Vector2` distances in
  f32, `snappedf`, Godot's `maxf`, `minf`, `clampf`, `lerpf`, `int()` of a constitution,
  `floori(x + 0.000001)`, the seeded rolls through `ghash::uniform` with the same keys). The
  grid's cells around the downed that `tend` filtered the standing by is gone: every unit it
  let in or out was beyond any reach the rules test. The native calls are lent held columns
  (ColumnsLent.all). The proof is CI's digests against the base. Outcomes unchanged.
- **Retired:** FormationDeaths' `depth` and the fates' rule; FormationWounds' `STRUCK`,
  `_cells_around`, `_by_draw`, `_within_reach`, `_fooled`, `_guarded`, `_take`, `_messenger`
  and the surrender roll; FormationRecovery's `HELD`, `_lies_still`, `_led_cunning`,
  `_index`, `condition_of`, `_unclamped`, `_drain`, `_wakes`, `_rise` and `_nearest`;
  FormationRegeneration's `mend`. From the gdUnit tests: test_formation_wounds.gd's struck
  and guarded cases, test_formation_recovery.gd's condition, drain, regeneration, downed
  regenerator, playing dead and fooled cases, and test_formation_near_passes.gd's nearest
  formation (each now a cargo test). The scenarios that land what the crate decides or run
  through a battle stay gdUnit (test_formation_wounds.gd, test_formation_recovery.gd,
  test_formation_wounds_order.gd, test_unit_downed.gd, test_unit_health.gd).
- **Tests:** cargo: `deaths/tests.rs` (downed within the constitution and dead past it, a
  wounding blow's wounds), `wounds/tests.rs` (struck through death's door, a captor, guarded
  and pressed, a messenger, which body a messenger takes whatever the lists' order, a
  thorough foe fooled less, a router's surrender) and `recovery/tests.rs` (condition and
  hardening, drain, coming to and rejoining, alone, the nearest not its own nor a routed one,
  the nearest as a look at all finds it, regeneration's limit and max, a halt, a downed
  regenerator, playing dead and the cunning, getting up once foes are gone, two rising
  whatever the lists' order). The Rust was drafted before them, so they had no red run.

### Step 6c, carrying and taking in the crate

- **The pick-up** (`carry.rs`): every free unit of a formation that tends its downed (told
  to, and not fighting or routing), within `wounds_reach` of a friendly body lying that no
  foe stands within `wounds_guard_reach` of, and able to move bearing it (its load stage
  under 3: its gear and the body's cells times `wounds_body_weight`, against its strength
  and "hauler"), may pick it up: [gap, the body's draw, the bearer's draw] first, each body
  and bearer once. FormationCarry hands it every squad's faction, whether it tends, its
  living and its bodies lying, and lands each pick in order: the states, `carried_by` and
  `carrying`, the bearer's pace, tiring and dodge (`_bear`), the event and those setting off
  home.
- **The takers sent** (`taking.rs`): each free unit of a group that sends (not loose,
  fleeing or out already) and each downed foe lying that it sees within `take_reach`, by
  [gap, the unit's draw, the body's draw], each unit and body once, the bodies its takers
  claimed already left. FormationTaking keeps which groups send (at ease, a command that
  takes, not "fall behind, left behind", not threatened, not given up), the recall and the
  hold, and lands each taker sent: the taking, its start and the event. All the recalls run
  before the one call: a recall reads and writes only its own group, and none makes an
  event, so the events keep their order.
- **Kept GDScript**: a body following its bearer, dropped when the bearer falls, set down
  (`_follow`, `_hold`) and coming home with it (`_bring_home`), and a bearer's pace for its
  load (`_bear`, through `UnitArms.stage_of`, which the units' gear reads too): they are
  the states, the lists and per-unit fields the crate doesn't hold. They port with step 7's
  orders and step 8's tick.
- **Identical:** both follow the GDScript's order and precision (`Vector2` distances in f32,
  `snappedf`, the stage's sums in f64, the keys compared item by item). The pick-up's grids
  (all the standing, each tending formation's living) give exactly what a look at all finds.
  The native calls are lent held columns (ColumnsLent.all). The proof is CI's digests
  against the base. Outcomes unchanged.
- **Retired:** FormationCarry's `_pairs` and `_unguarded`, and the pick-up's sort;
  FormationTaking's pairing. From the gdUnit tests: test_formation_carry.gd's cases of a
  formation that leaves its downed or has a foe near and of a unit too weak, and
  test_formation_near_passes.gd's bearers paired (each now a cargo test). The scenarios that land what the crate decides or
  run through a battle stay gdUnit (test_formation_carry.gd, test_formation_taking.gd,
  test_formation_wounds_order.gd's who bears whom).
- **Tests:** cargo: `carry/tests.rs` (picked up, none by a formation leaving its downed or
  with a foe near, a foe beyond guard reach, beyond reach or bearing already, too weak, a
  body's weight and a hauler, the nearest pairs first and each once, who bears whom whatever
  the lists' order, the pairs a look at all finds) and `taking/tests.rs` (sent within reach,
  none beyond reach or sight or to a friend, a claimed body left, each unit one body nearest
  first, two groups to one body, who takes which whatever the lists' order). The Rust was
  drafted before them, so they had no red run.
- **Step 6 is done** with this PR.

### Step 7a, morale's pressure and recovery, and the blows on routers, in the crate

- **Pressure and recovery** (`morale.rs`): once a second, each formation not routing or
  destroyed is fought on its front while locked and on each edge it is flanked on, each
  once; a left or right edge counts one less when a friendly formation (not destroyed) has
  its box within `morale_support_cells` of the squad's and its centre out beyond that edge.
  It loses `morale_pressure` by the sides left (at least one), or out of contact gains
  `morale_recovery` times its units' mean condition (held to 0..`condition_cap`, rounded)
  plus its leadership; either way up to its ceiling. `FormationMorale.step` hands it each
  squad's index, morale (its ceiling the first time it is read), leadership and ceiling, and
  sets what it returns through `_set_morale` in the lists' order (the 0..100 hold, the band
  and its event).
- **The blows on routers** (`rout_blows.rs`): each router stands at its route's point that
  far along plus its offset (`where_on`, FormationRout.where_in; on no route, the line y =
  0); each fighter given takes the nearest hostile router within `rout_strike_reach`,
  [the snapped gap, its draw] first, as its target, runs its cooldown down and at 0 strikes
  and starts again. `RoutBlows` hands it each routing squad's route and the fighters of the
  squads free to fight (`FormationContact.can_engage`, `SkirmishSquad.fighters`) with the
  cooldown each starts again at (`interval` times its melee seconds), and makes the blows
  (`FormationCombat.damage`, a flank blow).
- **Kept GDScript**: the ceiling, leadership, the bands, `interval`, `shock` and `losses`.
  The landings of the deaths, the wounds, the edges, the rout and the retreat deal the shocks,
  and the blows and the rallies read the bands: they port with the passes that call them
  (7b, 7c and step 8's tick).
- **Identical:** both follow the GDScript's order and precision (`Rect2` in f32 with
  `grow`, `intersects` and `get_center`, `Vector2` distances and dots in f32, the condition
  summed in f64 in the living's order, `roundi` as `round`, `snappedf`). Nothing in the
  pressure reads another squad's morale, so stepping them together decides as one by one did.
  The routers' grid visits a superset of those in reach, and the key is total, so the nearest
  is what a look at all finds. The native calls are lent held columns (ColumnsLent.all). The
  proof is CI's digests against the base. Outcomes unchanged.
- **Retired:** FormationMorale's `_condition`, `_sides_engaged` and `_supported`, and
  step's loop; RoutBlows' `_nearest` and its search. From the gdUnit tests:
  test_formation_morale.gd's pressure growing with the sides, a friend beside a fought side
  and recovery to the ceiling (each now a cargo test). The scenarios of the ceiling, the
  shocks, the bands and a battle stay gdUnit (test_formation_morale.gd, test_formation_rout.gd).
- **Tests:** cargo: `morale/tests.rs` (pressure growing with the sides, a flank alone, a
  friend beside a fought side, a friend on the other side, a foe or one too far, a destroyed
  friend, a friend on each flank, recovery by leadership to the ceiling, a poor and a hardened
  condition, the lists' order) and `rout_blows/tests.rs` (the nearest router struck and the
  cooldown started again, the cooldown running down, none beyond reach or of its own side, a
  router on its route plus its offset, two equally near by their draws whatever the lists'
  order, once a tick). Both ran red on a stub, then green.

### Step 7b, the rout in the crate

- **The rout** (`rout.rs`): who breaks (standing, not arrived, a unit living, at 0 morale),
  decided before the flight; each break in the lists' order releases its locks
  (`locks::release`), drops its flanks, routs with its rally ticks at 0, gives each living
  unit a fleeing entry where it stands (`front_distance` cells along, its offset from that
  point) and names the friends of its faction with a unit living that detect it
  (`perception::detects`). Each routing squad's routers not held flee a step, crush the
  nearest friend's unit nearer than a cell (`panic` the first time a routing squad crushes
  that friend, by its id) and, reaching home, lose their entries; a squad left with none is
  destroyed. The rallies: a router with a led friend within `rout_rally_reach` it has got
  behind joins it, else the friend holding it (the one it was held by while that one stands,
  else the nearest it has got behind within its breadth) counts its time while steady, keeps
  it while not, and after `rout_steady_rally_seconds` steady joins it; all are decided before
  any join, which go nearest first, [the snapped gap, its draw]. A routing squad whose leader
  lives counts its ticks with no enemy within `rout_enemy_near` of its routers and after
  `rout_rally_seconds` re-forms round its best leader ([-leadership, its draw]): its front
  there, its position on its route, its entries gone, facing the enemy's end along its route,
  holding, at `rout_reformed_morale`, re-forming.
- **The flight** (`rout/flight.rs`, RoutFlight.step): its pace by its speed, out to the side by
  its fan angle (`fan`, RoutFlight.fan) unless fanned `rout_fan_cells` out or the ground to the
  side is barred, or with a refuge (the standing friend's unit nearest it between it and home)
  turned towards that friend's ranks by at most the fan's sine; the step paid by the ground
  (`mover::share`).
- **The searches** (`rout/friends.rs`, RoutFriends and RoutCatch): per faction, the crush index
  (the living of its squads not routing, at their points) and the stand index (of its squads
  neither routing nor destroyed, where their bodies are), taken as each call begins. A faction
  is alone with no squad but its routing ones. `refuge` looks ring by ring; `short_of` takes
  the least and most of (where - at)·side over the friend's living, which is what the old
  look at the units within 0.1 of each edge found.
- **Four calls a tick**, each lent the held columns (ColumnsLent.all): `rout_flee` (the
  breaking and the flight), `rout_rallies`, `rout_regroup` and `rout_break`. Between them
  GDScript lands in the old order: the crushes' hurts (UnitHealth.hurt_one), events and panic
  shocks, the routers home (out of `units`, UnitState.regather) and the scattered; the bands
  first read (`morale` below 0 is handed in with its ceiling, and FormationMorale.band sets
  it on those named) and the joins (`_join`, FormationContact.reinforce); the reformed events;
  each break's staging, swaps, event and the shocks of those that saw it. The roster and the
  leadership are made again after routers leave or join. `step` returns at once when no squad
  routs and none is at 0 morale.
- **Kept GDScript**: `where`, `where_in` and `route_point_in` (their readers port with the
  march's sync and step 8), RoutFlight.fan (FormationWithdraw's, 7c), `_join`, the bands, the
  shocks and the ceiling.
- **Identical:** each follows the GDScript's order and precision (`Vector2` in f32,
  `Vector2 * float` narrowed first, `snappedf`, DetMath's `sin`, `ghash::uniform` for the fan,
  `move_toward`, `is_equal_approx`). The squads the rout passes stand where they are through
  it, and a crush's hurt changes no unit's state within the tick, so indexes taken as a call
  begins find what the old ones, gathered the first time asked, found; who is alone is the same
  through the tick, as it was kept for it. The proof is CI's digests against the base.
  Outcomes unchanged.
- **Retired:** RoutCatch, RoutFriends and ScrumNear (GDScript), RoutFlight.step, and
  FormationRout's `_flee`, `_crush`'s search, `_rallies`, `_key`, `_regroup` and `_reform`, and
  its held view (`FLED`, `HELD`). From the gdUnit tests: test_rout_friends.gd (each search as a
  look at every unit), test_one_mover.gd (a router in a wood at half pace) and
  test_formation_rout.gd's re-forming facing a slanting route (each now a cargo test). The
  battles stay gdUnit (test_formation_rout.gd, test_formation_rally.gd,
  test_formation_rout_order.gd).
- **Tests:** cargo: `rout/tests.rs` (who breaks, a break ending its fight and its units
  fleeing from where they stand, the friends that see it, a lone router's fan, one fanned out
  as far as it may, a router in a wood at half pace, a refuge, a held router, the nearest
  friend crushed and panicked once, no foe or routing friend crushed, home and scattering, a
  led friend got behind, a steady friend's hold and join, a shaken one's hold, a holder kept
  while it stands, a band first read, the joins' order whatever the lists', re-forming facing
  a slanting route, an enemy near and a leaderless rout) and `rout/friends/tests.rs` (crushing,
  catching, a refuge and turning aside, and an enemy near, each as a look at every unit; who is
  alone). Both ran red on a stub, then green.

### Step 7c, orders, the retreat, and the pursuit's and the withdrawal's records, in the crate

- **Orders** (`orders.rs`): the orders given since the last tick, in the order given, each to
  its squad if it is listed and not destroyed; who is fighting (locked, or a foe on an edge)
  is read before any lands. A retreat out of a fight finds its enemies (another faction,
  fighting it front to front or on an edge of either), ends its locks (`locks::release`),
  withdraws (its record at 0 safe ticks, its stance dropped, its units not yet loose loosened
  where they stand), and for each enemy in the lists' order: one that may pursue locks onto it
  and, with no pursuit yet, keeps its post (front, point, home, direction, order) and takes up
  its quarry's route where that passes within 2 cells, travelling towards its quarry (Decision
  113; a route taken is that pursuer's for the rest of the tick); then its units may break
  ranks (`pursuit::tempt`). `FormationOrders` hands it the roster, and only when an order
  retreats each squad's route (a code per route object, its points), whether it pursues and
  its units' chances and leashes (FormationDiscipline), and lands what it returns in order:
  the events, the ragged retreat's shock (FormationMorale.shock) and a pursuer's routes.
- **The pursuit as a whole** (`pursuit/whole.rs`, FormationPursuit.step): each pursuer in the
  lists' order: going back, once its march home is over, it takes up its post, home and order
  again (ScrumTurn then faces it the way it held its post); else it gives up when its quarry
  is gone, destroyed or not retreating, out of its sight (open ground, as before), most of it
  is too tired, or its frame is its leash from its post: its locks end and it marches back
  (home its post along the route it is on, ordered to retreat); else it sweeps round its
  route's bends and advances after its quarry's units, no nearer than a cell, while none of it
  lags (`lagging`, `spread`). GDScript hands it each pursuer's two routes and its leash
  (FormationDiscipline.pursuit_leash) and lands the routes, the turns and the events.
- **The withdrawal's record and re-form** (`withdraw.rs`): those routing or destroyed withdraw
  no more; who is pursued is read from the records; after the flight each counts its safe
  ticks (back to 0 when not safe) and after `rout_rally_seconds`, or with its units all home,
  re-forms: its front at its units' mean along its route, kept between its home and its front,
  facing home, or home and holding there facing out. GDScript lands the turn (ScrumTurn) and
  the events.
- **The records** (`squads.rs`): `pursuit` ({foe, post, post_at, home, direction, order,
  returning}) and `withdraw` (its safe ticks) are the squad store's. `OrderRecords` reads them
  as the dictionaries they were (SkirmishSquad's `pursuit` and `withdraw`) and keeps the route
  a pursuer held its post on until routes port; it was not split into SquadRecords, which
  would have outgrown its interface (`check_isp`).
- **Kept GDScript**: the routes (FormationRoute objects), ScrumTurn, the discipline and the
  leashes, the shocks and the events. Each lands after the crate's call, in its order. What a
  turn changes (its squad's heading, direction, front and loose entries) is read by no later
  decision of the same call: a pursuer back at its post is no other's quarry still pursued (a
  quarry's order is to retreat, and one ordered to retreat does not arrive), and a re-form
  reads only its own squad.
- **Identical:** each follows the GDScript's order and precision (`Vector2` in f32,
  `distance_to` and `dot` in f32, the mean along the route summed in f64 in the living's order,
  `clampf`, `is_zero_approx`, a front set moving its point as StoredSquad._place does). The
  proof is CI's digests against the base. Outcomes unchanged.
- **Retired:** ScrumPursuit.retreat, `_fights` and `_tempt`; FormationPursuit.begin, `_take`,
  `_given_up`, `_advance`, `_ahead`, `_lagging`, `_go_back`, `_arrive` and `_spread`;
  FormationWithdraw.begin, `_pursued` and `_reform_here`; FormationSimulation's order loop; and
  the bindings `pursuit_tempt`, `pursuit_lagging` and `pursuit_spread`. No gdUnit test called
  them directly: the battles stay gdUnit (test_formation_retreat.gd, test_formation_withdraw.gd,
  test_retreat_home.gd, test_formation_simulation.gd).
- **Tests:** cargo: `orders/tests.rs` (an order to its squad and none to one gone or destroyed,
  a retreat out of no fight, a retreat ending its fight and withdrawing from where its units
  stand, a pursuer locking on and keeping its post, one ordered not to and one pursuing
  already, taking its quarry's route and not when it passes far or is its own, a foe on an
  edge and a friend, breaking ranks, orders given together whatever the lists' order),
  `pursuit/whole/tests.rs` (advancing at its pace, a cell short, waiting for its rearmost,
  giving up on a quarry not retreating, gone or destroyed, out of sight, most tired and at its
  leash, going back and arriving, whatever the lists' order) and `withdraw/tests.rs` (the safe
  ticks, re-forming where its units stand, between its home and its front, at home holding,
  routing or destroyed, pursued). All ran red on a stub, then green.

### Step 8a1, ranged combat and the blows' landing in the crate

- **The shots** (`shots.rs`, FormationCombat.ranged_blows): each shooter in the order handed
  in finds the nearest enemy unit within its range in ranks: a rank's gap goes by how far it
  stands ahead of the shooter's squad's front along its heading (SquadGeometry.unit_gap), less
  the shooter's own rank's, snapped to 0.000001; of the nearest, one it overlaps across its
  squad's columns first, then the targets' draws (ScrumContest), then their place in the
  listing of every squad not arrived. It takes that one as its target, its cooldown runs down,
  and at 0 it shoots and starts again. GDScript picks the shooters (squads standing and not
  arrived, their living with a range, a fighting squad's front rank fighting as melee), hands
  in each one's cooldown and makes the shots.
- **The landing** (`landing.rs`, BlowLanding, BlowRoll and BlowDamage): each blow in order,
  melee then shots. With rolls, its margin is the striker's skill plus a shift (its band's,
  the high ground's, the surrounded's for each presser past the first, the skill past the
  floor's, the stamina stage's, added in that order) plus its roll across the die; read
  against parry (melee from the front), dodge (not from the rear), defence (a graze) and the
  hit band, else a critical; a shot's flank is whether it comes from outside its target's
  front (ScrumReach.in_front). What it deals is its weapons' parts scaled by the roll between
  the floor skill lifts and the full and by how it landed, each part by its damage type,
  armour off the mundane and ward off the magical, at least 1 if any. Without rolls every
  blow is a hit at full damage, and a shot marks no wounds, as before. A blow dealing damage
  of a type that stops the target's regeneration halts it (FormationRegeneration.scorch).
  GDScript hands in each unit's arms once a call (skills, critical, floors, most stamina,
  agility, dodging, defence, armour, ward, wounding, whether it parries, its damage types as
  bit sets of codes given in the order met, and its parts), who presses each target and the
  striker's band (ScrumBlowsField.pressed, FormationMorale.band, both still GDScript), and
  lands the damage and how it landed (the events' names).
- **The rolls' hash** (`ghash.rs`): a bool's element hash is its 0 or 1 (Godot 4.7.2's
  `hash([true])` is 2714831339, `hash([false])` 1771887473), so BattleRolls' keys
  `[tick, striker, "blow", ranged]` hash in the crate as in GDScript. BattleRolls stays for
  the flight's and the bodies' draws.
- **Kept GDScript**: who may shoot, the morale band, the pressed counts, applying the damage
  (UnitHealth) and the events, FormationWounds.surrender and FormationStamina.strike, each
  where it was in FormationSimulation._fight.
- **Identical:** each follows the GDScript's order and precision (points and dots in f32,
  gaps, shifts and damage in f64, `snappedf`, `lerpf`, `clampf`, `roundi`, DetMath's cosine).
  The proof is CI's digests against the base. Outcomes unchanged.
- **Retired:** BlowRoll, BlowDamage and FormationRegeneration (their files), FormationCombat's
  `_marks`, `_rows` and `_in_range`, BlowLanding's `_land`; test_blow_roll.gd and
  test_blow_damage.gd, whose rule scenarios are now cargo tests. Their content tests (a unit
  parries with a held weapon or a shield, a unit in the field carries its skills, strength
  scaling, proficiency, armour worn) moved to tests/content/test_unit_arms.gd;
  test_formation_recovery.gd's regeneration test now sets the halt itself. The battles stay
  gdUnit (test_blow_landing.gd, test_formation_ranged.gd).
- **Tests:** cargo: `landing/tests.rs` (the margin through parry, dodge, defence and hit into
  a critical; flank, rear and range; the margin; the side; the floor, the lift and the spill;
  how it landed; weakness, resistance and immunity by part; armour and ward; a unit without
  parts; without rolls a plain hit, and a shot marking no wounds; regeneration stopped;
  wavering, pressed and tired strikers landing fewer or more; the same seed; whatever the
  arms' order), `shots/tests.rs` (a shooter behind its front striking the enemy front, out of
  range holding fire, its cooldown, the nearest rank, one it overlaps first, friends and
  arrived squads not shot at, whatever the lists' order) and `ghash.rs` (bools against the
  engine). All ran red on a stub, then green.

### Step 8a2-i, the turn and the joins' order in the crate

- **The turn** (`turn.rs`, ScrumTurn.begin and SquadTurn.reverse_ranks): a squad about to
  move against the way it faces turns about first: its places reversed (ranks from the old
  back by each unit's footprint depth, columns from the old right by its width), its centre
  shift flipped, it re-forms, its front moves back by its depth in ranks and is placed on its
  route (or the straight lane), its direction flips and its heading turns half round. Then,
  on a route, its heading is the route's there, by its direction. Unless its heading is the
  same (`is_equal_approx`) and it kept its direction, every living unit not already loose is
  loosened where it stands, in the lists' order. GDScript hands in the squad's roster
  (CrateRoster of the one squad) and its route's points, and on the answer (kept, turned,
  turned about) cancels the swaps under way after an about-face and adds the "turning" and
  "turned" events.
- **The joins' order** (`contact.rs`, `Field::joins`, FormationJoins.join): each advancing
  mover that has reached a friend's back rank (`joinable`), as (friend, mover), nearest first
  by its gap to that friend snapped to 0.000001, then by the movers' draws (ScrumContest).
  GDScript joins each in that order (FormationContact.reinforce, already the crate's), adds
  the events and takes the joined squad out of the lists.
- **Kept GDScript**: the swaps (a per-squad list until the shuffle ports, 8a2-iii), the
  events, the lists.
- **Identical:** the same operations in the same order and precision (the front's shift
  `(direction * depth) * RANK_DEPTH`, the route's heading scaled as `Vector2 * int`,
  `fposmod`, `bearing_to`, the sort by `[gap, draw]`). One roster serves every mover's join,
  as nothing changes between them. The proof is CI's digests against the base. Outcomes
  unchanged.
- **Retired:** SquadTurn.reverse_ranks (SquadTurn keeps the wheel's and the about-face's
  times), ScrumTurn's own turn and loosening, FormationJoins' own sort.
- **Tests:** cargo: `turn/tests.rs` (an about-face puts the back rank in front and turns the
  columns; a wide or deep unit's footprint kept inside; going back turns it about, its front
  back by its depth; a turn loosens every unit in its frame where it stands, in the lists'
  order, one already loose kept; on a bend it faces the route; facing its way it keeps its
  heading; turning about on a route faces back along it) and `contact/tests.rs` (waves
  reaching one friend join nearest first whatever their order; of two as near the lower draw
  first, one short not joining). All ran red on a stub, then green. gdUnit:
  test_squad_turn.gd's about-face now goes through ScrumTurn.begin (the swaps cancelled, the
  events), and one facing its way doesn't turn.

### Step 8a2-ii, staging's hold and fallback and the bodies' drag in the crate

- **Staging's hold and fallback** (`staging.rs`, `hold`, FormationStaging.holds): a staged
  squad whose trigger fired goes (`signalled`, as before); else, short of its staging point
  (its front less its home, in cells, under `at`) it marches on; else it takes the HOLDING
  state, staged if it first did so this tick, and holds while its fallback lasts past this
  tick, else it gives up. GDScript keeps the order (`staging`, which only an advancing squad's
  holds are asked of), counts the fallback down, and adds the events ("signalled", "staged",
  "gave_up") and the turn back. The answer is a code: what it does, plus STAGED.
- **The bodies' drag** (`lying.rs`, `Lying::drag`, GroundBodies.drag): the share of its pace
  a marching squad keeps for the bodies its front rank steps onto, half a cell ahead along
  its heading: the least of each front-rank unit's footing (`underfoot`, already the
  crate's). GDScript still finds which bodies lie and where (GroundBodies.ground, read by the
  walk too) and hands them in as the walk has them (CrateMarch.lying); the tick's roster
  names the squad.
- **Identical:** the same operations in the same order and precision (the step
  `Vector2 + vector * 0.5` in f32, the footprint area and weights in f64, `minf`; the
  staging distance `front * CELLS_PER_TILE` less the home's, `absf`, `< at`; the fallback
  `- 1 > 0`). The roster is the one the tick decided on, and nobody falls before the joins.
  The proof is CI's digests against the base. Outcomes unchanged.
- **Retired:** GroundBodies.underfoot (the walk's footing was already the crate's) and
  BodyGrid, which only it searched; FormationStaging's own hold. test_scrum_search.gd
  (BodyGrid's rings) and test_formation_near_passes.gd (the near search against a look at
  all, now `lying/tests.rs`), and test_ground_underfoot.gd's two rule scenarios.
- **Tests:** cargo: `lying/tests.rs` (a body slows by its weight against the walker's, one
  too heavy blocks, the bodies a near search finds are those a look at all finds; a front
  over its own weight halves its pace and a great one blocks it, only the front rank steps
  over them and the worst step counts, whatever the order) and `staging/tests.rs` (short of
  its point it marches on; at it, it holds, staged first; its last tick of fallback gives up,
  staged too if it just arrived; a fired trigger goes before it holds). The drag's and the
  hold's ran red on stubs, then green. gdUnit: test_ground_bodies.gd and
  test_formation_staging.gd unchanged, now through the crate.

### Step 8a2-iii, the shuffle's swaps' ends and the widening in the crate

- **The swaps' ends** (`swaps.rs`, `end`, FormationShuffle.step): last first, a swap any of
  whose units has fallen is cancelled where they stand; the rest count down, and one whose
  time is up ends: its units take the places it was heading for, the squad widens to take in
  any living unit now outside its columns (the first end always looks; after it, only an end
  with a unit outside), and it closes up (`compact`, already the crate's) round the swaps
  still under way. The widening renumbers every unit's column, the fallen's too, and the
  swaps still under way, and shifts the centre by `left + (right - left - width) / 2.0`.
  GDScript hands in the swaps as numbers (`_under_way`: ticks, then each unit's slot, whether
  it stands, its place to and footprint) and the squad's units; the answer is the swaps kept
  with their ticks left, how far the widenings moved the columns, and each swap that ended
  with the units that stepped up after it and their ranks. GDScript keeps the list (its
  `from`, `to`, mover and passed, renumbered by the answer), makes the "swapped" and
  "stepped_up" events in the order they happened, and starts new moves (already the
  crate's).
- **Kept GDScript:** the list of swaps under way, read as it stands by the offsets
  (`offset`, `movers`) of the march, the pour, the split and the presentation, until the
  crate holds the tick's state (8b).
- **Identical:** the same operations in the same order: swaps last first, a swap's units
  in its `to` order (the mover first), compact's movers the swaps still listed in the list's
  order, each widening's bounds over the living, the centre shift in f64. The events carry
  ids and the rank each stepped up to as it did. The squad's position follows its front once
  after (`SquadCompact.placed`): nothing reads it between. The proof is CI's digests against
  the base. Outcomes unchanged.
- **Retired:** FormationShuffle's own ends, `_inside` and `_widen_to_fit`.
- **Tests:** cargo: `swaps/tests.rs` (a swap with time left counts down and nobody moves;
  one ending puts its units in place and the gap closes; one with a fallen unit is cancelled;
  ending outside the columns widens the squad and renumbers every place, the fallen's and a
  later-ending swap's too; a widening on the right keeps the columns while a swap under way
  counts down). Red on a stub, then green. gdUnit: test_formation_moves.gd and
  test_formation_spread.gd unchanged, now through the crate.

### Step 8a2-iv, morale's ceiling, bands and shocks in the crate

- **The survey** (what the tick still decided in GDScript after 8a2-iii): morale's ceiling,
  bands and shocks (here); the march's ends and who may shoot (8a2-v); and the tick's own
  choices, 8b's (the split bullet in step 8).
- **Morale's rules** (`bands.rs`, on `ScrumRules`; FormationMorale): a formation's ceiling
  is its living's mean courage, rounded, plus `morale_per_leadership` for each point of its
  best living leader's leadership, at most 100, and 0 with none living; its band is steady
  from `morale_steady_at`, shaken from `morale_shaken_at`, wavering above 0 and routing at 0;
  a wavering one's attack interval is half again as long; a shock's morale is held to 0..100;
  the tick's dead shock it by `morale_loss` each and `morale_leader_loss` per point of each
  fallen leader's leadership. GDScript hands in the living's courage and leadership, the
  morale and the tuning (`_rules`: [per leadership, steady at, shaken at, loss, leader
  loss]); it keeps each squad's morale (first read at its ceiling), and makes the
  "morale_band" and "leader_fell" events.
- **Identical:** the same operations in the same order: the courage summed as ints and
  divided in f64, rounded half away from zero (`roundi`, `f64::round`), `mini(100, ...)`;
  the bands' comparisons; `roundi(usual * 1.5)`; `clampi(value, 0, 100)`; the losses' sum.
  The proof is CI's digests against the base. Outcomes unchanged.
- **Retired:** FormationMorale's own arithmetic.
- **Tests:** cargo: `bands/tests.rs` (the ceiling is the mean courage rounded plus the best
  leader's steps; at most 100 and 0 with none living; the best living leader leads; the
  bands fall by morale, routing only at 0; a wavering formation strikes half again more
  slowly; morale is held to 0..100; the dead shock by each loss and each leader's
  leadership). Red on stubs, then green. gdUnit: test_formation_morale.gd unchanged, now
  through the crate.

### Step 8a2-v, the march's ends and who may shoot in the crate

- **The march's ends** (`march::end`, FormationMarch.check_ends): an advancing squad whose
  front is at the enemy's end (`is_equal_approx`) arrives there; a retreating one whose front
  is home, or on a route all of whose units stand within half a cell of home
  (`withdraw::home`, asked only then), returns and holds there as it faces. GDScript hands
  in the squad (`_along`), its route's points and, for a retreat, its roster; on the answer
  ([marching on, arrived or returned; the front]) it sets the front, the state and the
  order and adds the "arrived" and "returned" events. The binding `march_end` replaces
  `withdraw_home`.
- **Who may shoot** (`shots::shooters`, FormationCombat.ranged_blows' choice): in the lists'
  order, the living with a range of every squad not arrived, but a fighting squad's front
  rank, which fights as melee; each starts again at its ranged time in ticks,
  `max(1, round(interval * seconds))`. GDScript hands in each living unit's ranged time,
  asks nothing when none has a range, and makes the shots from the answer, four numbers a
  shot (the shooter's squad and place in its living, its target's).
- **Identical:** the same comparisons in the same order (the advance first, the front's
  test before the units'), the home in cells `home * CELLS_PER_TILE`, the rounding half away
  from zero; the shooters in the order GDScript listed them, so each target and cooldown is
  set as before. The proof is CI's digests against the base. Outcomes unchanged.
- **Retired:** FormationWithdraw._home, FormationMarch's own end tests, FormationCombat's
  own choice of shooters.
- **Tests:** cargo: `march/tests.rs` (an advance at the enemy's end arrives, from either
  end and within the tolerance; one short of it marches on; a retreat whose front is home
  returns without asking about its units; one whose units are all home returns wherever its
  front stands; off a route only its front brings it home; a holding squad does neither)
  and `shots/tests.rs` (the living with a range may shoot in the lists' order; each starts
  again at its ranged time in ticks, rounded, at least 1; a fighting squad's front rank
  fights as melee and its rear shoots; an arrived squad holds its fire). Red on stubs, then
  green. gdUnit: test_retreat_home.gd and test_formation_ranged.gd unchanged, now through
  the crate.
- **After it**, what the tick still decides in GDScript is the tick itself (8b).

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
  here. Recorded in "Built", step 4c; the records and their order handling ported in 7c.

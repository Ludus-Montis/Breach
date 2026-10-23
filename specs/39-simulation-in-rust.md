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

## Open, for the implementing session to settle and record here

- ~~The squad store's layout and which fields stay per-squad objects (step 1).~~ Settled in
  "Built", step 1.
- ~~The snapshot's on-disk format and where snapshots live (step 1, used in step 8).~~ The
  format is settled in "Built", step 1. Which snapshots are committed, and their size, is
  step 8's.
- A chunk's boundary may move when a dependency demands it. The session records the move here.

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
  groups look ahead and their commands' patches, landing the hurts, and the events. **8b split
  further** (2026-10-28, under the same leave): **8b1** the move's choices, which squads march,
  hold or wait, and every move landed together; **8b2** landing the hurts and the fight's
  events; **8b3** which groups look ahead and their commands' patches; **8b4** the squads'
  objects still GDScript (routes, commands, staging, the swaps under way, the painted places,
  the taking) held by the crate, with spawning and the order queue; **8b5** one `step()` call,
  the events handed back as data. A later part may split again on its own survey. **8b4
  split further** (2026-10-28, under the same leave), on its survey: **8b4-i** the squads'
  routes held by the crate, and the bindings that take one squad's own route read it there
  (staging's hold, a march's end, the turn, a stray setting off); **8b4-ii** the bindings
  that take many squads' routes (the withdrawal, the rout, the rout's blows, the split, the
  orders, the pursuit) and the march's face and sweep read the held routes; **8b4-iii** the
  commands (their route, home, standing options and patches) held by the crate; **8b4-iv**
  staging, the swaps under way, the painted places and the taking held by the crate;
  **8b4-v** spawning and the order queue. **8b4-iv split further** (2026-10-28, under the
  same leave), on its survey: **8b4-iv** staging held by the crate, and the painted places
  retired (nothing writes them); **8b4-v** the swaps under way and the taking held by the
  crate; **8b4-vi** spawning and the order queue. **8b4-v split further** (2026-10-28, under
  the same leave), on its survey: **8b4-v** the swaps under way held by the crate, with the
  units' state mirrored in the store; **8b4-vi** the taking held by the crate; **8b4-vii**
  spawning and the order queue. **8b5 split** (2026-10-28, under the same leave), on its
  survey (the tick still runs about 18 phases' glue in GDScript, each unit is a GDScript object of 61
  fields, and every phase builds its events as dictionaries): **8b5-i** the units' remaining
  fields held by the crate, SkirmishUnit a view of them; **8b5-ii** the squads' remaining
  fields (their units, the joined, faction, the living and downed lists) held by the crate;
  **8b5-iii** orders, engagement, pathing and the move as one crate pass, their events built
  as data; **8b5-iv** the scrum, the fight and deaths, likewise; **8b5-v** taking, wounds,
  recovery and strays; **8b5-vi** morale, the rout and carrying; **8b5-vii** groups, stamina,
  the bodies, the shuffle and the march's sync; **8b5-viii** one `step()` binding,
  FormationSimulation.step that one call and the events decoded from its data. **8b5-i split
  further** (2026-10-28, under the same leave), on its survey (61 fields: 42 numbers, the rest
  words, lists and objects): **8b5-i** the units' numbers held by the crate; **8b5-ii** their
  words, lists and objects (faction, tags, tactics, attributes, traits, the weapons' parts,
  damage type, weaknesses, resistances, immunities, what stops regeneration, the walker, the
  command and origin, the definition), coded for the crate. The rest follow, renumbered:
  **8b5-iii** the squads' remaining fields; **8b5-iv** orders, engagement, pathing and the move;
  **8b5-v** the scrum, the fight and deaths; **8b5-vi** taking, wounds, recovery and strays;
  **8b5-vii** morale, the rout and carrying; **8b5-viii** groups, stamina, the bodies, the
  shuffle and the march's sync; **8b5-ix** one `step()` binding. **8b5-ii split further** (2026-10-28, under the
  same leave), on its survey (twelve words and lists, plain data, against four objects whose
  coding each needs its own table): **8b5-ii** the units' words and lists (faction, tags,
  tactics, attributes, traits, the weapons' parts, damage type, weaknesses, resistances,
  immunities, what stops regeneration) coded for the crate; **8b5-iii** their objects (the
  walker, the command and origin, the definition) coded for the crate. The rest follow,
  renumbered: **8b5-iv** the squads' remaining fields; **8b5-v** orders, engagement, pathing
  and the move; **8b5-vi** the scrum, the fight and deaths; **8b5-vii** taking, wounds,
  recovery and strays; **8b5-viii** morale, the rout and carrying; **8b5-ix** groups,
  stamina, the bodies, the shuffle and the march's sync; **8b5-x** one `step()` binding.
  **8b5-v split further** (2026-10-28, under the same leave), on its survey (no event is
  data yet, and the four passes still land their events, shocks, routes and the move's
  glue in GDScript: the move alone is staging, the march's ends, the turn, the sweep, the
  walk, the drag and the joins): **8b5-v** the events as data (a crate log, decoded by
  GDScript into the same dictionaries) and the orders' pass reading its discipline from
  the units' sheets and building its shock and events there; **8b5-vi** engagement and
  pathing likewise; **8b5-vii** the move likewise. The rest follow, renumbered:
  **8b5-viii** the scrum, the fight and deaths; **8b5-ix** taking, wounds, recovery and
  strays; **8b5-x** morale, the rout and carrying; **8b5-xi** groups, stamina, the bodies,
  the shuffle and the march's sync; **8b5-xii** one `step()` binding. **8b5-vi split
  further** (2026-10-28, under the same leave), on its survey (the fronts and edges already
  decide in the crate and only their events and shocks are GDScript's, while pathing's look
  reads each unit's wits and its definition's sight, still GDScript's): **8b5-vi**
  engagement's events and shocks built in the crate; **8b5-vii** pathing (the follow, the
  look, "detoured") likewise. The rest follow, renumbered: **8b5-viii** the move;
  **8b5-ix** the scrum, the fight and deaths; **8b5-x** taking, wounds, recovery and
  strays; **8b5-xi** morale, the rout and carrying; **8b5-xii** groups, stamina, the bodies,
  the shuffle and the march's sync; **8b5-xiii** one `step()` binding. **8b5-viii split
  further** (2026-10-28, under the same leave), on its survey (each march's glue reads what
  the crate already holds, but the walk's share reads each unit's pour, its place, its swap
  and its walker, and the joins rebuild GDScript's lists): **8b5-viii** the move's march as a
  crate pass (staging, the march's ends, the hold, the turn, the sweep, the pace, the drag and
  the landing), its events built as data, each squad's share handed in; **8b5-ix** the walk's
  share read in the crate, and the joins and their step-ups as data. The rest follow,
  renumbered: **8b5-x** the scrum, the fight and deaths; **8b5-xi** taking, wounds, recovery
  and strays; **8b5-xii** morale, the rout and carrying; **8b5-xiii** groups, stamina, the
  bodies, the shuffle and the march's sync; **8b5-xiv** one `step()` binding. **8b5-ix split
  further** (2026-10-28, under the same leave), on its survey (the share reads only what the
  crate holds, but a join changes GDScript's lists, its units' command and the squads' list,
  each its own rebuild): **8b5-ix** the walk's share read in the crate; **8b5-x** the joins
  and their step-ups as data. The rest follow, renumbered: **8b5-xi** the scrum, the fight and
  deaths; **8b5-xii** taking, wounds, recovery and strays; **8b5-xiii** morale, the rout and
  carrying; **8b5-xiv** groups, stamina, the bodies, the shuffle and the march's sync;
  **8b5-xv** one `step()` binding. **8b5-xi split further** (2026-10-29, under the same
  leave), on its survey (the scrum alone is a dozen GDScript phases, each gathering its own
  arrays; the fight builds its events from GDScript's blows; deaths bury through GDScript's
  lists): **8b5-xi** the scrum's engaging, manoeuvres and stall over one roster, their events
  in the log; **8b5-xii** the scrum's preparing and stance ("faced"); **8b5-xiii** the seek,
  the walk and the facing; **8b5-xiv** the regroup, the chasers, the withdrawal and the
  pursuit, their events; **8b5-xv** the fight (blows, shots, their landing and hurts, "hit"
  and "spat", the surrender and the strike's stamina); **8b5-xvi** deaths. The rest follow,
  renumbered: **8b5-xvii** taking, wounds, recovery and strays; **8b5-xviii** morale, the
  rout and carrying; **8b5-xix** groups, stamina, the bodies, the shuffle and the march's
  sync; **8b5-xx** one `step()` binding. **8b5-xiv split further** (2026-10-29, under the
  same leave), on its survey (the regroup's and the chasers' say all lies in the crate already, but the
  withdrawal's re-form and the pursuit's return each turn their squad (ScrumTurn), rebuild
  GDScript's route and build their events): **8b5-xiv** the regroup and the chasers;
  **8b5-xv** the withdrawal and the pursuit, their turns and events. The rest follow,
  renumbered: **8b5-xvi** the fight; **8b5-xvii** deaths; **8b5-xviii** taking, wounds,
  recovery and strays; **8b5-xix** morale, the rout and carrying; **8b5-xx** groups,
  stamina, the bodies, the shuffle and the march's sync; **8b5-xxi** one `step()` binding.
  **8b5-xv split further** (2026-10-29, under the same leave), on its survey (the
  withdrawal's say all lies in the crate, but the pursuit's post route is still GDScript's,
  kept by OrderRecords, and must become a squad column the snapshot carries first):
  **8b5-xv** the withdrawal, its turns and events; **8b5-xvi** the pursuit (its leash and
  its post's route held in the crate, its turns, "pursuit_ended", GDScript's route mirrored
  on its return). The rest follow, renumbered: **8b5-xvii** the fight; **8b5-xviii**
  deaths; **8b5-xix** taking, wounds, recovery and strays; **8b5-xx** morale, the rout and
  carrying; **8b5-xxi** groups, stamina, the bodies, the shuffle and the march's sync;
  **8b5-xxii** one `step()` binding. **8b5-xvii split further** (2026-10-29, under the same
  leave), on its survey (whom each strikes is the crate's already, in three calls glued by
  GDScript, but the landing reads each unit's arms from GDScript's objects and the events,
  the surrender and the stamina are built from GDScript's blows): **8b5-xvii** the blows and
  shots chosen in one crate pass (targets cleared, the scrum's search, cooldowns and the
  morale's interval, the blows on routers, the shots, each blow's damage from the sheets);
  **8b5-xviii** their landing (the arms read in the crate), hurts, "hit" and "spat", the
  surrender and the strike's stamina. The rest follow, renumbered: **8b5-xix** deaths;
  **8b5-xx** taking, wounds, recovery and strays; **8b5-xxi** morale, the rout and carrying;
  **8b5-xxii** groups, stamina, the bodies, the shuffle and the march's sync; **8b5-xxiii**
  one `step()` binding. **8b5-xviii split further** (2026-10-29, under the same leave), on
  its survey (the landing's arms, pressing and bands all lie in the crate, but the hurts'
  events, the surrender and the stamina are built from GDScript's blows, and the landing's
  own tests hand it blows of their own): **8b5-xviii** the landing read in the crate (each
  unit's arms from its sheet and words, who presses each target, each striker's band);
  **8b5-xix** the fight as one `fight_step` pass (choosing and landing together, the hurts,
  "hit" and "spat" in the log, the surrender and the strike's stamina). The rest follow,
  renumbered: **8b5-xx** deaths; **8b5-xxi** taking, wounds, recovery and strays;
  **8b5-xxii** morale, the rout and carrying; **8b5-xxiii** groups, stamina, the bodies, the
  shuffle and the march's sync; **8b5-xxiv** one `step()` binding.
  **8b5-xix split further** (2026-10-29, under the same leave), on its survey (the hurts,
  "spat", "hit" and the stamina follow from the landing alone, but the surrender sets unit
  and squad states GDScript copies, releases locks and needs the test-only callers of the
  landing, the choice, the hurts and the strike moved): **8b5-xix** the fight as one
  `fight_step` pass (choosing and landing together, the hurts, "spat" and "hit" in the log,
  the strike's stamina); **8b5-xx** the surrender in that pass (TAKEN, "surrendered",
  "destroyed" and the locks' release, with GDScript's copies mirrored), and the pieces left
  only to tests retired (BlowLanding.land, FormationMelee.blows, UnitHealth.hurt_landed,
  FormationStamina.strike and the `fight_choose`, `fight_land`, `fight_hurts` and
  `stamina_strike` bindings), their tests moved onto the pass or into the crate. The rest
  follow, renumbered: **8b5-xxi** deaths; **8b5-xxii** taking, wounds, recovery and strays;
  **8b5-xxiii** morale, the rout and carrying; **8b5-xxiv** groups, stamina, the bodies, the
  shuffle and the march's sync; **8b5-xxv** one `step()` binding.
  **8b5-xx split further** (2026-10-29, under the same leave), on its survey (the surrender
  follows from the pass's own blows, but retiring the test-only pieces moves six test files
  onto the pass or the crate): **8b5-xx** the surrender in `fight_step` (routers struck for
  damage while standing, none "never_surrenders", rolled by `wounds::surrenders`; TAKEN,
  "surrendered", "destroyed" and the locks' release via `locks::release`, GDScript's unit
  states mirrored); **8b5-xxi** the pieces left only to tests retired (BlowLanding.land,
  FormationMelee.blows, UnitHealth.hurt_landed, FormationStamina.strike,
  FormationWounds.surrender and the `fight_choose`, `fight_land`, `fight_hurts`,
  `stamina_strike` and `wounds_surrenders` bindings, with BodyField's `fight_synced`), their
  tests moved onto the pass or into the crate. The rest follow, renumbered: **8b5-xxii**
  deaths; **8b5-xxiii** taking, wounds, recovery and strays; **8b5-xxiv** morale, the rout
  and carrying; **8b5-xxv** groups, stamina, the bodies, the shuffle and the march's sync;
  **8b5-xxvi** one `step()` binding.
  **8b5-xxiii split further** (2026-10-29, under the same leave), on its survey (each of the
  four is its own GDScript pass gluing crate calls, and each lands states, lists and events
  of its own): **8b5-xxiii** taking as one crate pass (`taking/pass.rs`, `taking_step`: the
  takers called back, the hold and `took_until`, which squads send, the sending and
  "sent_to_take" in the log); **8b5-xxiv** the wounds' tending (FormationWounds.tend) as one
  pass; **8b5-xxv** recovery and strays. The rest follow, renumbered: **8b5-xxvi** morale,
  the rout and carrying; **8b5-xxvii** groups, stamina, the bodies, the shuffle and the
  march's sync; **8b5-xxviii** one `step()` binding.
  **8b5-xxv split further** (2026-10-29, under the same leave), on its survey (recovery lands
  states, lists and events over squads that exist, while the strays make new squads, which
  the crate can't yet open for GDScript): **8b5-xxv** recovery as one crate pass
  (`recovery/pass.rs`, `recovery_step`: the living and downed read in the crate, the deaths
  of wounds, those playing dead, those coming to joining a formation or setting off alone,
  "died_of_wounds", "playing_dead" and "came_to" in the log); **8b5-xxvi** the strays
  (FormationStrays.adopt) made in the crate. The rest follow, renumbered: **8b5-xxvii**
  morale, the rout and carrying; **8b5-xxviii** groups, stamina, the bodies, the shuffle and
  the march's sync; **8b5-xxix** one `step()` binding.
  **8b5-xxvi as built** (2026-10-29): `strays::adopt` (binding `strays_adopt`) takes each
  stray out of its squad, opens its squad's slot with the battle's next id (`issue_squad`),
  copies its old squad's faction, way, home, command, route and heading, sets it ROUTING
  with no morale and sets it off; GDScript wraps the slot it opened (`SquadColumns.opened`,
  which the next `open` takes). The stray's squad was numbered in GDScript only before, so
  the crate held it as squad 0 (its draws and its log): now it holds the stray's own id.
  GroupSplit numbers its groups the same way and is left to 8b5-xxviii.
  **8b5-xxvii split further** (2026-10-29, under the same leave), on its survey (morale's
  second, the rout and carrying are three GDScript passes, the rout alone landing hurts,
  shocks, joins, homecomings and a single squad per rallying unit): **8b5-xxvii** morale's
  second as one crate pass (`morale/pass.rs`, the `morale_step` binding now taking the roster,
  the tuning, BandTuning's and the tick: each squad's morale, its best leader's leadership and
  its ceiling read from the sheets, its morale set, "morale_band" in the log); **8b5-xxviii**
  the rout (FormationRout.step) as one pass, its rallying unit's single squad opened in the
  crate as the strays' are; **8b5-xxix** carrying (FormationCarry.step) as one pass. The rest
  follow, renumbered: **8b5-xxx** groups, stamina, the bodies, the shuffle and the march's
  sync (GroupSplit's groups made in the crate as the strays are); **8b5-xxxi** one `step()`
  binding.
  **8b5-xxviii built** (2026-10-29): the rout as one crate pass (`rout/pass.rs`, the
  `rout_step` binding in place of `rout_flee`, `rout_rallies`, `rout_regroup` and
  `rout_break`): the flight's crushes hurt and panic in the crate, a router home leaves its
  squad's members ("fled_home" with its definition's code, which CrateEvents decodes given
  the store), and each rally's join goes through a single squad and command the crate opens
  for it, as FormationRout's did, so outcomes are unchanged; "crushed", "fled_home",
  "scattered", "rallied", "reformed" and "routed" are logged. FormationRout mirrors only its
  unit lists and the friends' `joined`.
  **8b5-xxix built** (2026-10-29): carrying as one crate pass (`carry/pass.rs`, the
  `carry_step` binding in place of `carry_pick_up`): the bodies whose bearers fled home this
  tick come home out of their squads' members ("fled_home" with the definition's code), the
  borne follow their bearers or are dropped, bearers set their loads down, and the free units
  of the formations that tend their downed pick bodies up ("borne": unit, body), each bearer's
  load stage, pace, tiring and dodge set in its sheet, its pace a share of its definition's
  speed (which UnitWrites now hands the crate, `put_speed`, beside its sight), so outcomes are
  unchanged. FormationCarry mirrors the states, the carrying and the loads, and its unit
  lists, and returns the bearers setting off home for FormationStrays.
  **8b5-xxx split further** (2026-10-29, under the same leave), on its survey (groups, stamina,
  the bodies, the shuffle and the march's sync are five GDScript passes; the groups' alone
  split, form up, open squads and land meetings): **8b5-xxx** the groups (FormationGroups.step:
  GroupSplit and the forming up) as one crate pass, each split's group opened in the crate with
  the battle's next id as the strays' are; **8b5-xxxi** stamina (FormationStamina.step) and
  the bodies' parting (UnitBodies.step) as crate passes; **8b5-xxxii** the shuffle
  (FormationShuffle.step); **8b5-xxxiii** the march's sync (FormationMarch.sync_units);
  **8b5-xxxiv** one `step()` binding.
  **8b5-xxx built** (2026-10-29): the groups as one crate pass (`groups/pass.rs`, the
  `groups_step` binding in place of `group_split` and `group_form_up`): the groups at ease on
  a route whose units fell behind split off, each new group's squad opened in the crate with
  the battle's next id ("split": from), then the groups at ease meet and each meeting lands
  there: a group taken in ("merged") or units gone back to the command they started under,
  each behind its new squad's back row ("rejoined": unit). A route is still GDScript's object,
  so each squad's route is handed over as a code (the same code, the same route). A split's
  group used to be opened as squad 0 and numbered only in GDScript, so the crate held it as 0;
  now it holds the real id (outcomes change, as for the strays). FormationGroups wraps the
  slots opened and mirrors its lists; GroupSplit is gone.
  **8b5-xxxi built** (2026-10-29): stamina and the bodies' parting as crate passes.
  `stamina/pass.rs` (the `stamina_step` binding now takes the listed squads' slots): each
  squad's pursuit and its command's hurry, and each living unit's fresh speed, most stamina,
  tiring, run pace, load stage, leadership, wits and constitution and its march traits, read
  in the crate; a unit made by hand, with no fresh speed, takes its speed column's, and
  FormationStamina mirrors it. `parting/pass.rs` (the `bodies_part` binding in place of `part`
  and `commit_part`): the field synced to the listed squads' living units as the bodies stand
  (a router's base its point on its squad's route), new units enrolled from their sheets, the
  bodies parted on a way rolled in the crate from the battle's seed and their draws
  (UnitBodies.part_way is gone), those pushed from their places made loose and the moved
  written into the columns. BodyFieldSync.bodies stays for its tests. Outcomes don't change.
  **8b5-xxxii built** (2026-10-29): the shuffle as one crate pass over every squad
  (`swaps/pass.rs`, the `shuffle_step` binding in place of `squad_swaps_end` and
  `squad_start_moves`): each listed squad's swaps under way end (the squad's place following
  its front if it moved), "swapped" (unit, passed) and "stepped_up" (unit, rank) in the log,
  then a re-forming squad starts every move it can, each living unit's place, footprint, band,
  priority, pace and whether it joined read in the crate, and stops re-forming once none starts
  and none is under way. "passed" is the log's first list field: how many, then each
  (`events::LISTS`, `Log::squad_listed`; CrateEvents decodes it to an Array). FormationShuffle.step
  now takes the squads, not one squad. Outcomes don't change.
  **8b5-xxxiii built** (2026-10-29): the march's sync as one crate pass over every squad
  (`march/sync.rs`, the `march_sync` binding): each listed squad's living units get their
  distance along the route (a swap under way included), a router stands on its point on the
  route (`rout_blows::where_on`), facing the heading unless loose, a loose one where its own
  moves took it; placing, each unit in its frame is set on its place (`pour::place`), facing
  the heading; otherwise each walks a tick (`walk::walk`) to where it makes for
  (`pour::targets`), its walker, stamina, turn and footprint read from its sheet, over the
  bodies lying as the tick found them (`moving::pass::Bodies::listed`), and a fall from a
  climb face lands its hurt in the crate, onto the fell queue. FormationMarch.sync_units keeps
  its signature and makes the one call; its WALKED and SYNCED holds and FormationWalk.place_of
  are gone. Outcomes don't change.
  **8b5-xxxiv split further** (2026-10-29, under the same leave), on its survey (the tick is
  still some twenty GDScript passes, each one crate call between GDScript's own mirrors, and
  a few still hand the crate what only GDScript holds: the routes as objects, the
  perception's picture, the lists of squads and units): **8b5-xxxiv** the scrum
  (FormationScrum.step: engaging, readying, the stances, the manoeuvres, the seek, the
  regroup, the withdrawal, the pursuit and the stall) as one crate call; **8b5-xxxv** the
  tick's opening (the orders, the perception, the fronts and edges, the pathing and the
  move with its joins) as one call, the routes and the perception held by the crate;
  **8b5-xxxvi** the fight through the groups (the fight, deaths, taking, the wounds,
  recovery, the strays, morale, the rout, carrying and the groups) as one call, each pass's
  mirror after it; **8b5-xxxvii** the tick's end (stamina, the bodies, the shuffle and the
  march's sync), then one `step()` binding running them all, FormationSimulation.step that
  one call, GDScript's copies mirrored after it and the events decoded from its data.
  **8b5-xxxiv built** (2026-10-29): the scrum as one crate call (`scrum/step.rs`, the
  `scrum_step` binding): its nine phases in their order over one roster, each phase's events
  in the log in that order, the stall quiet unless the seek found its squad or a foe active,
  and the pursuers back at their posts handed back, whose routes GDScript mirrors
  (FormationPursuit.mirror). Each phase's class hands over its own rules (`rules`), and its
  own binding stays for its tests and `tick_phases`, which times each phase. Outcomes don't
  change.
  **8b5-xxxv split further** (2026-10-29, under the same leave), on its survey (the opening
  is six crate calls between GDScript's mirrors; the routes reach the crate as codes of
  GDScript's objects, made twice a tick and changed between the calls by the pursuers'
  `enemy.route = quarry.route` and pathing's re-routes, and the taking reads the perception
  as GDScript's dictionary): **8b5-xxxv** the tick's opening (the orders, the perception, the
  fronts and edges, the pathing and the move with its joins) as one crate call, the routes'
  codes handed over once and kept by the crate as GDScript's mirrors would leave them, the
  perception handed back; **8b5-xxxvi** the routes and the perception held by the crate (the
  routes' codes the crate's own, for the opening and the groups, the pursuers' and
  re-routed groups' routes no longer mirrored by hand, the taking reading the perception the
  crate kept); **8b5-xxxvii** the fight through the groups as one call (was 8b5-xxxvi);
  **8b5-xxxviii** the tick's end and the one `step()` binding (was 8b5-xxxvii).
  **8b5-xxxv built** (2026-10-29): the tick's opening as one crate call (`opening.rs`, the
  `opening_step` binding): the orders queued, each squad's perception, the fronts and then the
  edges, the pathing (on ground only) and the move with its joins, over one roster, every
  pass's events in the log in that order. Each squad's route comes over once as a code
  (FormationOrders.route_codes), and the crate keeps the codes as GDScript's mirrors left
  them between the calls: a pursuer that took its quarry's route holds it and its code, and a
  group that re-routed a code of its own. FormationOpening makes the call and mirrors after
  it: the pursuers' and re-routed groups' routes, the perception (handed back) and the joins.
  The per-pass bindings stay for their tests and `tick_phases`. Outcomes don't change.
  **8b5-xxxvi split further** (2026-10-29, under the same leave), on its survey (the codes
  are one change through the crate's route setter and four passes, the mirrors another that
  makes GDScript's route a view of the crate's, and the perception a third): **8b5-xxxvi**
  the routes' codes the crate's own; **8b5-xxxvii** the routes and the perception held by the
  crate (the pursuers' and re-routed groups' routes no longer mirrored by hand, the taking
  reading the perception the crate kept); **8b5-xxxviii** the fight through the groups as
  one call (was 8b5-xxxvii); **8b5-xxxix** the tick's end and the one `step()` binding (was
  8b5-xxxviii).
  **8b5-xxxvi built** (2026-10-29): the routes' codes the crate's own (`Squads::route_code`,
  -1 none): SkirmishSquad.route's setter hands the crate the route's instance id with its
  points (`squad_put_route`), and the crate gives each route it is handed a code of its own
  (`Squads::issue_route`; the same route, the same code). The passes that hand a squad
  another's route hand it the code too (a pursuer its quarry's, `opening::took`, which
  `apply_orders` now runs as well; a split's group and a stray their own squad's), and a
  group that re-routes onto its command's way round takes a fresh code (`pathing::pass`).
  `opening_step`, `apply_orders`, `move_step` and `groups_step` read the codes there, and
  FormationOrders.route_codes, FormationStaging.route_codes and FormationGroups' codes are
  gone. Outcomes don't change: the codes fall into the same groups as GDScript's routes.
  **8b5-xxxvii built** (2026-10-29): the routes and the perception held by the crate.
  SkirmishSquad.route is a view of the crate's route and code (SquadRoutes, on the store):
  it reads the route kept for its squad's code, and a route the crate made (a group
  re-routed, a pursuit's post route handed over unnamed) is made once from the crate's
  points and corridor (`squad_route_half_width`) and named by its object (`name_route`). The
  crate keeps the post route's code as a pursuit begins (`Squads::post_route_code`) and the
  squad takes it up again back at its post; one handed over by `put_squad_pursuit` is issued
  a fresh code there. So FormationOpening's `_pursued`, FormationOrders.apply's quarry copy,
  FormationPathing's `_follow`/`rerouted` and FormationPursuit.mirror are gone, and a squad
  carried to another store takes its route along (SquadRoutes.carry). The perception keeps
  each squad's threat (`Squads::threatened`, `perception::keep`, from the opening and
  `perception_see`), which `taking_step` reads instead of a list handed to it
  (SkirmishSquad.threatened; FormationTaking.step takes no perception, and
  FormationPerception.threatened is gone). Outcomes don't change, except where a pursuer
  back at its post now holds the code of the very route it held it on, not one of the same
  points (the old mirror's command's route or a new copy).
  **8b5-xxxviii split further** (2026-10-29, under the same leave), on its survey (the fight
  through recovery runs over one squad list that doesn't change, but the strays, the rout's
  rallies and the groups open and drop squads partway, so the list must grow and shrink in
  the crate): **8b5-xxxviii** the fight and its aftermath (the fight, deaths, taking, the
  wounds' tending and recovery) as one call; **8b5-xxxix** the strays through the groups (the
  strays, morale, the rout, carrying, the strays again and the groups) as one call, the
  squads listed growing and shrinking in the crate; **8b5-xl** the tick's end and the one
  `step()` binding (was 8b5-xxxix).
  **8b5-xxxviii built** (2026-10-29): the fight and its aftermath as one crate call
  (`aftermath::step`, binding `aftermath_step`): `fight::step`, `deaths::bury`,
  `taking::pass`, `wounds::pass` and `recovery::pass` in that order over one roster, each
  pass's events in the log in that order. It returns each pass's result, its count first
  (the surrender's units now by id, not by their index in the living), and FormationAftermath
  mirrors them in the same order (FormationDeaths.mirror, FormationWounds.mirror,
  FormationRecovery.mirror, split out of their passes), then takes the events once.
  FormationSimulation.step makes the one call; the per-pass bindings and `_fight` stay for
  their tests and `tick_phases`. Outcomes don't change.
  **8b5-xxxix built** (2026-10-29): the strays through the groups as one crate call
  (`mustering::step`, binding `mustering_step`): `strays::adopt` over recovery's strays,
  `morale::pass` on a second's tick, `rout::pass` when a listed squad routs or is at 0
  morale, `carry::pass` (its homes the routers the rout brought home), `strays::adopt` over
  the bearers it set off, then `groups::pass`, each pass's events in the log in that order.
  The squads listed grow in the crate as GDScript's lists did (`mustering::listing`): each
  squad opened is listed after the rest, its faction coded as a listed squad of the same
  one, its merging its command's, and every listed squad's living read again from the
  members, each unit as the roster first gave it. It returns each pass's result, its count
  first; FormationMustering mirrors them in the same order (FormationStrays.mirror,
  FormationRout.mirror, FormationCarry.mirror, FormationStrays.mirror,
  FormationGroups.mirror), then takes the events once. A mirror now runs after the later
  passes, so one must not write back what they changed: a stray's squad keeps the heading,
  state, morale, front and place the crate holds for it (a stray home or rallied in the
  same call is DESTROYED there), not those it was set off with. `groups_step` (and so
  FormationGroups.mirror) names a split's units by id, not by their index in the living.
  FormationSimulation.step makes the one call; the per-pass bindings stay for their tests
  and `tick_phases`. Outcomes don't change.
  **8b5-xl split further** (2026-10-29, under the same leave): **8b5-xl** the tick's end
  (stamina, the bodies' parting, the shuffle and the march's sync) as one call; **8b5-xli**
  the one `step()` binding running the opening, the scrum, the aftermath, the mustering and
  the end, FormationSimulation.step that one call, GDScript's copies mirrored after it and
  the events decoded from its data.
  **8b5-xl built** (2026-10-29): the tick's end as one crate call (`ending::step`, binding
  `ending_step`): `stamina::pass`, `parting::pass`, `swaps::pass` and `march::sync` in that
  order over the squads listed, the shuffle's events in the log. No mirror between them fed
  the next pass, and none changes the list. It returns the parting's [usec, passes, pairs],
  then the ids of the units made by hand that took their speed as fresh;
  FormationEnding.step mirrors those (FormationStamina.mirror, split out of its pass) and
  takes the events. FormationSimulation.step makes the one call; the per-pass bindings and
  GDScript passes stay for their tests, `tick_phases` and `native_bench`. Outcomes don't
  change.
  **8b5-xli split further** (2026-10-29, under the same leave): with the whole tick in one
  call, every GDScript mirror runs after the last pass, and several write into the crate
  (`UnitState.put` the STATE column, `unit.wounded` the sheet, FormationCommand.enlist a
  unit's place), so a mirror can overwrite what a later pass in the same call did (a body
  downed by the deaths' pass and picked up by carrying's). **8b5-xli** the whole tick as one
  crate pass and its binding: `tick::step` runs the opening, the scrum, the aftermath, the
  mustering and the end over one roster of every unit the squads hold, listing the squads
  afresh between the passes as CrateRoster.of did (the squads a join took in dropped, the
  factions coded again in the lists' order, each squad's living read from its members, the
  strays recovery set off handed to the mustering, the squads it opened and took in listed
  for the end). **8b5-xlii** FormationSimulation.step that one call, with GDScript's copies
  mirrored after it, no mirror writing over the crate, and the events decoded once from its
  data.
  **8b5-xli built** (2026-10-29): the whole tick as one crate pass (`tick::step`, binding
  `tick_step(roster, ground, opening, scrum, aftermath, mustering, ending)`, each of the five
  an Array of that pass's own binding's arguments past the roster and the ground, the
  mustering's past its strays). The roster holds every unit the squads hold; `tick::relisted`
  lists the squads before the opening and after each pass but the last as CrateRoster.of
  did, dropping the squads the opening's joins took in and coding the factions again in the
  list's order; the strays recovery set off (`Came::Alone`, [its squad's slot, its unit's
  slot]) go to the mustering; the end runs over `tick::ended_listing` (the mustering's list,
  the adopted squads, the splits, less the groups taken in). It returns each pass's flat
  result as its own binding does, each part its length first (the scrum's [seek usec, the
  pursuers back...]). The five bindings share their rules and results with it (lib.rs
  `opening_rules`/`opened_flat`, `scrum_rules`, `aftermath_rules`/`aftermath_flat`,
  `mustering_rules`/`mustered_flat`, `ending_rules`/`ended_flat`). GDScript doesn't call it
  yet. Outcomes don't change.
  **8b5-xlii built** (2026-10-30): FormationSimulation.step is one crate call
  (FormationTick.step, `tick_step`) over `CrateRoster.of(squads, terrain, true)` (every unit
  of `squad.units`, after `squad.living()` freshens the members). Each pass's GDScript class
  hands its part of the arguments (`args`: FormationOpening, FormationScrum,
  FormationAftermath, FormationMustering, FormationEnding, whose own `step` uses them too)
  and mirrors its part of the result (`mirror`) in the passes' order. The mirrors run under
  CrateMirroring: while it is on, `UnitState.put` gives the unit the crate's STATE and
  writes nothing, a unit's number setter takes the crate's sheet value (`UnitWrites.number`),
  and its words and objects, a squad's columns, route, faction, command and joined list,
  and an admitted unit's sheet and state are left as the crate holds them. So no mirror
  writes over what a later pass in the call did (a body downed by the deaths and borne by
  carrying stays CARRIED). The events are decoded once (`CrateEvents.taken(field, store)`),
  then `orders_clear` and `store.settle`. The per-pass GDScript classes and bindings stay
  for their tests and the tools. Outcomes don't change.
- **8c split further** (2026-10-30, under the user's standing leave to split): a cargo binary
  can only play a battle the crate can set up, order and step with nothing from GDScript.
  **8c-i** the tick's rules read in the crate: each pass's rules built from the battle's
  tuning the crate loaded, so FormationTick hands the call only the tick's own numbers.
  **8c-ii** a battle set up in the crate: a squad spawned from its units' definitions, the
  ground made from a map, and the digest's battles (BattleTrials' mirrors,
  FormationBench.clash, the field logs' fields) made in Rust; the binding's rule parsers moved
  off Godot's arrays so a cargo binary can use them. **8c-iii** a battle run end to end in
  the crate: the rest of FormationSimulation.step and the field's actions (the field logs'
  orders by tick), the digest's stop rules (a side broken, TAIL ticks, ticks past contact).
  **8c-iv** the digest as a cargo binary: each tick's events and squads' and units' state
  hashed as `formation_digest.gd` hashes them (Godot's `var_to_bytes`, SHA-256), its output
  compared with the Godot digest's in CI; the Godot digest retired once they match on
  `main`. **8c-v** the trials and `scaling_check` as cargo binaries. **8c-vi** the benches,
  `tick_phases` among them with spec 38 step 2's series, `terrain=mixed` and per-pass
  changed counts.
  **8c-i built** (2026-10-30): `tick::given` (`tick::given::of(tuning, At, Paces)`) builds
  every pass's arguments past the roster and the ground from the battle's tuning
  (`content::BattleTuning`) and the tick's numbers, value for value as FormationOpening,
  FormationScrum, FormationAftermath, FormationMustering and FormationEnding's `args` build
  them. The binding `tick_run(content, roster, ground, [tick, seed, ticks a second, 1 if
  blows roll, 1 if threaded], [travel scale, seconds, route length, cells a second])` reads
  the tuning from the ContentStore (`CrateContent.store()`, which replaces
  `CrateContent.defaults()`) and runs `tick_step` on them; `tick_given` returns them for the
  test that holds them equal to GDScript's. FormationTick.step makes that call;
  `FormationTick._args` is removed. Outcomes don't change.
  **8c-ii split further** (2026-10-30, under the same leave): **8c-ii** the tick's rules
  parsed in the crate: lib.rs's rule parsers (`opening_rules`, `scrum_rules`,
  `aftermath_rules`, `mustering_rules`, `ending_rules`, `fight_rules` and the rest) moved off
  Godot's Packed arrays and VarArrays into `tick::rules`, so `tick::rules::of(&Given)` gives
  `tick::step` its rules with nothing from Godot; **8c-iii** a squad spawned in the crate from
  its units' definitions; **8c-iv** the ground made in the crate from a map; **8c-v** the
  digest's battles (BattleTrials' mirrors, FormationBench.clash, the field logs' fields) made
  in Rust. The rest renumber: **8c-vi** a battle run end to end in the crate (was 8c-iii),
  **8c-vii** the digest as a cargo binary (was 8c-iv), **8c-viii** the trials and
  `scaling_check` (was 8c-v), **8c-ix** the benches (was 8c-vi).
  **8c-iii split further** (2026-10-30, under the same leave): a squad spawned from its units'
  definitions first needs each unit made from its definition, and none of FormationUnits.make,
  UnitArms or TerrainWalker.of was in the crate. **8c-iii** a unit made in the crate from its
  definition; **8c-iv** a squad spawned in the crate from those units (its squad and command
  opened, the units admitted with their words coded and objects set, placed, at full hp,
  regeneration, stamina and pace, on its route, facing along it, at its morale's ceiling, and
  stood on their places). The rest renumber: **8c-v** the ground made from a map (was 8c-iv),
  **8c-vi** the digest's battles made in Rust (was 8c-v), **8c-vii** a battle run end to end
  in the crate (was 8c-vi), **8c-viii** the digest as a cargo binary (was 8c-vii), **8c-ix**
  the trials and `scaling_check` (was 8c-viii), **8c-x** the benches (was 8c-ix).
  **8c-iii built** (2026-10-30): `arms` (`Arms::of(content, unit)`) reads what a unit strikes
  with and wears from its definition in the crate's content, as UnitArms and UnitDef do: the
  weapons it can use (innate ones only where no carried item holds their body parts), its
  best ranged weapon, its parts, floor shares and skills, its melee interval, whether it
  parries, its armour and ward, its load stage and the traits its items grant. `made::unit`
  makes a unit's sheet, words (coded in the field's lexicon in UnitWrites.all's order) and
  walker from those, as FormationUnits.make and TerrainWalker.of do. The binding
  `ContentStore.made(id, faction, direction)` returns them for the gdUnit test that holds every
  unit in `content/units/` equal to FormationUnits.make's. Nothing in the tick calls them yet.
  Outcomes don't change.
  **8c-iv built** (2026-10-30): `spawn::squad` spawns a squad in the crate as
  FormationSimulation.spawn_squad does, every write the one GDScript makes: its units' ids
  and its squad's issued, its command opened on its route, the squad opened with its faction
  (coded before its units' words), direction, facing, home and width; each unit made by
  `made::unit` and admitted (sheet with its squad, words, walker, its command as command and
  origin, its definition's code given by the caller with its sight and speed kept), then
  placed at full hp, all its regeneration to come, full stamina and its pace fresh; the squad
  on its route (its code given, or issued), placed and faced along it (`face_held`), with its
  wait, its morale at its ceiling and its combat width, and its units stood on their places
  (`march::sync`, placing). The binding `BodyField.spawn_squad(content, units, codes, places,
  faction, [width, at the player's end, wait, combat width], points, half_width, object)`
  codes the route as `squad_put_route` does. The gdUnit test holds the crate's whole state
  (`snapshot`) after three spawns equal, byte for byte, to GDScript's. Nothing in the tick
  calls it yet. Outcomes don't change.
  **8c-v built** (2026-10-30): the map is FormationField's: the formation battles' only
  painted ground (the digest's field logs and BattleTrials fight on it; nothing makes a
  formation ground from a MapLayout). `ground::map` holds a `Map` (its size and its areas
  painted in order, each a rect and a `Paint`) and makes its `Ground` on the battle's tuning
  (`Map::ground`); `ground::map::field(tuning)` is FormationField._ground's map (the wood,
  the stream, the ford, the hill's eight rings), and `ground::map::of(name, tuning)` names it
  "field". The binding `TerrainGround.map(content, name)` makes it; `TerrainGround.snapshot()`
  gives the ground's cells and rules as bytes. The gdUnit test holds the crate's field equal,
  byte for byte, to the one FormationField paints. The trials' grounds (PathTrials' crag,
  SightTrials' wall) are added as maps with the trials (8c-ix). Nothing in the tick calls it
  yet. Outcomes don't change.
  **8c-vi built** (2026-10-30): `battles` sets up the digest's battles in the crate, every
  write GDScript's set-up makes: BattleTrials' mirrors (head-on, and on a flank with the
  line's hold queued; its sides swapped if asked), FormationBench.clash (its back ranks
  spitters if asked) and the field the field logs play on (FormationField's kingdom line,
  captained or not, and its reserve, each holding with its front set where it stands on
  route A). Each squad is spawned by `spawn::squad`, its definitions coded in the order
  first met and its routes as first used. `battles::named(name, numbers)` names them as the
  digest does ("mirror_headon", "mirror_flank", "clash", "field"), and `set_up` returns
  what the simulation is made with (its route's length, tick, combat width and map). The
  field's waves are its actions, built and sent tick by tick (8c-vii). The binding
  `BodyField.battle(content, name, numbers)` sets one up; the gdUnit test holds the crate's
  whole state after each digest battle's set-up equal, byte for byte, to GDScript's. Nothing
  calls it yet. Outcomes don't change.
  **8c-vii split further** (2026-10-30, under the same leave): the field's actions are a
  system of their own (DomainProduction's builders, FormationProduction's waves with their
  templates, staging and departures, FormationRehearsal's timed marches for a rendezvous, and
  the record's commands: send, retreat, auto, wait, tend, hurry, route, pursue, and the routers
  home banked), while the mirrors and the clash need none of it. **8c-vii** a digest battle
  without the field's actions played end to end in the crate: the rest of
  FormationSimulation.step and the digest's stop rules; **8c-viii** the field's actions in the
  crate (the waves built and sent tick by tick, and the field logs' commands by tick). The
  rest renumber: **8c-ix** the digest as a cargo binary (was 8c-viii), **8c-x** the trials and
  `scaling_check` (was 8c-ix), **8c-xi** the benches (was 8c-x).
  **8c-vii built** (2026-10-30): `play` plays a digest battle in the crate. `Play::new` sets
  it up (`battles::set_up`) on its map's ground, if any, and `Play::step` is
  FormationSimulation.step: the tick counted; the squads listed with every unit each holds as
  CrateRoster.of(squads, terrain, true) lists them (`play::listed`: each unit's id, footprint,
  band, reach, turn rate, backward pace, detection and most stamina from its sheet, its kind
  its definition's code, its walker from its objects on ground and the open ground's
  otherwise); the whole tick (`tick::step`) on the rules `tick::given` and `tick::rules` build
  from the battle's tuning, on the ground with the paths' cache the play keeps; the orders
  queued cleared; the squads listed as the tick leaves them (`Ticked::listed`, the end's
  listing); and the units no listed squad holds let go (UnitStore.settle). `play::Watch` holds
  the digest's stop rules (`_run`: the limit, ticks past first contact, TAIL ticks after a
  side no longer stands) and `play::run` plays to them. The binding `BodyField.battle_play(
  content, name, numbers, [seed, rolls, threaded, limit, after])` returns the ticks played;
  the gdUnit test holds the ticks and the crate's whole state equal, byte for byte, to
  GDScript's run of the same battle (a mirror head-on and one flanked and swapped to a side
  broken and the tail, a clash to 30 ticks past contact, the captained field to its limit).
  Nothing calls it yet. Outcomes don't change.
  **8c-viii split further** (2026-10-30, under the same leave): the field's actions are three
  systems: the waves built and sent (DomainProduction's builders, FormationProduction's
  templates and sends) with FormationField.step's tick; the record's commands
  (FormationFieldActions.apply and FormationRecord.parse: retreat, auto, wait with B's staging,
  tend, hurry, route, the kingdom's pursue); and the rendezvous sent together (`send A+B`:
  FormationRehearsal's private march and FormationRendezvous' waits). **8c-viii** the field's
  waves in the crate: the builders and waves and FormationField.step, a wave sent by hand;
  **8c-ix** the record's commands but `send A+B`, so field logs 1, 2 and 4 play in the crate;
  **8c-x** the waves sent together, so all five do. The rest renumber: **8c-xi** the digest as
  a cargo binary (was 8c-ix), **8c-xii** the trials and `scaling_check` (was 8c-x),
  **8c-xiii** the benches (was 8c-xi).
  **8c-viii built** (2026-10-30): `waves` holds a lane's wave as GDScript does: `Template`
  (WaveTemplate's painting, front-first order and layout; `Template::line` is
  WavePresets.line), `Wave` (its places filled front-first, announced full once, departing when
  full if `auto`) and `Builders` (DomainProduction: builders by type in the order set, each
  claiming a lane by the sharing rule or the reserve under its cap, the reserve filling the
  lanes first, `Produced` telling what each did as GDScript's events do). `field_play`'s
  `FieldPlay` sets the field up (`Play`, `Battle::Field`; `battles::set_up_coded` keeps the
  definitions' codes for the waves' units) with FormationField's routes (A coded as the
  kingdom's, B and C as first used), its waves and its builders, and `step` is
  FormationField.step: the player's squads hurried and tending by their routes' waves, the
  builders, each wave announced (and sent, if automatic), the battle's tick (`Play::step`), and
  the player's "fled_home" units banked (`play::starts` walks the log `Log::since` gives from a
  `Log::mark`). `FieldPlay::send` is FormationProduction.send: `spawn::squad`, a partial wave
  as wide as its front band and closed up (`ranks::close`, its swaps cleared), centred, and
  stood on its places (`spawn::stand`, split out of `spawn::squad`). The binding
  `BodyField.field_play(content, [captained], [seed, rolls, threaded, ticks], sends)` returns
  the ticks and how many production events there were; the gdUnit test holds both and the
  crate's whole state equal, byte for byte, to GDScript's field sent the same waves on the
  same ticks. Nothing calls it yet. Outcomes don't change.
  **8c-ix built** (2026-10-30): `record::parse` is FormationRecord.parse (each version's
  header, the commands in order, a version 0 log's words through `record::from_words`, lines
  that aren't commands skipped). `FieldPlay::apply` (`field_play/actions.rs`) is
  FormationFieldActions.apply: the kingdom's pursue on its line and reserve (`FieldPlay`'s
  `kingdom`, kept from the set-up), a wave sent, a retreat queued for each player squad
  listed on the wave's route (matched by route code), auto, hurry and tend as FormationField
  holds them, a wave put on route A, B or C, and wait (`FieldPlay::set_wait`: B's `Wave`
  `staging`, the `staging::Stage` FormationField.set_wait gives, which `send` sets on the
  squad). `field_play::actions::replay` plays a record as the digest's `_field` does: each
  tick's commands, then the tick, for `extra` ticks past the last command; a record sending
  the waves together (`send A+B`) isn't replayed yet (8c-x). The binding
  `BodyField.field_replay(content, log, [rolls, threaded, extra])` returns the ticks and the
  production events, or [-1]; the gdUnit test holds the crate's whole state equal, byte for
  byte, to GDScript's replay of the digest's field logs 1, 2 and 4. Nothing calls it yet.
  Outcomes don't change.

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

### Step 8b1, the move's choices in the crate

- **Which squads march, hold or wait** (`moving::choose`, FormationSimulation._move's first
  test): one fighting, routing, arrived or destroyed stands, and so does one regrouping
  (`moving::regrouping`, FormationScrum.regrouping: any loose unit not out chasing on its
  own); else one still waiting waits; else it may march. GDScript asks once a tick for every
  squad of the tick's roster (`move_choices`), and runs down the waits after the joins, as
  before.
- **Whether one free to march holds** (`moving::hold`): ordered to hold, or skirmishing
  (`contact::skirmishing`); its state becomes holding or moving. GDScript asks after staging
  and a retreat's end (`move_holds`), where it decided before.
- **The landing** (`moving::land`, FormationSimulation._march): each march's front as it
  lands, in the order planned; an advancing squad's limit (`contact::limit`) judged before
  any lands, with every advancing squad marching. GDScript hands in [its index in the
  roster, where it wants to get] a march (`move_land`), sets each front (its route places
  it), then checks the ends and joins the advancing, in that order.
- **Identical:** each squad's first choice reads only its own state, loose units, chasers and
  wait, which nothing earlier in the loop changes, so asking once up front is the same as
  asking in the loop; the hold's tests in the same order (the order first); the limits from
  the same squads advancing, before any front moves. The proof is CI's digests against the
  base. Outcomes unchanged.
- **Retired:** FormationSimulation.STANDING, FormationContact.limit and its binding
  `contact_limit`, FormationScrum.regrouping's own rule (now the crate's, for the tools and
  tests that ask it).
- **Tests:** cargo `moving/tests.rs` (a fighting, routing, arrived or destroyed squad stands
  and the rest may go; a unit out of its place makes a squad regroup, a chaser doesn't;
  standing and regrouping come before waiting; an order to hold holds and a squad free to
  march moves; a ranged line with an enemy in reach holds to skirmish; two hostiles
  advancing at each other each close half the gap as they stood, however listed; a retreat
  lands where it wanted and isn't counted marching; an advance alone stops at reach). Red on
  stubs, then green. gdUnit: the battles through the move unchanged, now through the crate.
- **After it**, 8b2: landing the hurts and the fight's events.

### Step 8b2, landing the hurts and the fight's events in the crate

- **The hurts** (`hurts::land`, FormationSimulation._fight's UnitHealth.hurt over the shots
  and blows): each shot, then each blow, in the order BlowLanding landed them, takes what it
  dealt from its target's hp (`wounds::hurt`); a unit taken from above 0 to 0 or below goes
  on the fell queue in the order it fell, once. Each hurt answers its target's hp after it.
  GDScript hands them in through `UnitHealth.hurt_landed` (hp's one writer, Decision 145;
  the binding `fight_hurts`, lent any held columns, ColumnsLent) and builds the "spat" and
  "hit" events from the answers, the "hit" with its blow's answer as `target_hp`.
- **The events** stay GDScript dictionaries, in the same order (every "spat", then every
  "hit"), till the tick hands them back as data (8b5).
- **Identical:** the same hurts in the same order on the same column, the fall's test the
  same (`was > 0 and now <= 0`), the hp each "hit" reports read right after its own blow, as
  before. The proof is CI's digests against the base. Outcomes unchanged.
- **Retired:** _fight's held view of hp, and UnitHealth.heal (no caller left).
- **Tests:** cargo `hurts/tests.rs` (each hurt takes its damage and answers the hp left;
  hurts on one target land in order; a unit taken to 0 or below falls once, in the order it
  fell; one already down doesn't fall again, and nothing dealt fells nobody). gdUnit
  `test_unit_health.gd`: the fight's hurts land in order and fell the fallen once. Red on
  stubs, then green. The battles through the fight unchanged, now through the crate.
- **After it**, 8b3: which groups look ahead and their commands' patches.

### Step 8b3, which groups look ahead and follow their commands' patches in the crate

- **Following** (`detours::follows` and `detours::follow`, FormationPathing._follow): a group
  whose command has a route re-routes onto it, patched (`route::Patch::patched`), when the
  command carries ways round it doesn't (its `patched_count` differs from theirs). It stands
  where it stood along the new route (its front from its position), and one bound for the far
  end (`home_distance` above 0) keeps the new route's end as home. GDScript asks
  (`path_follows`, -1 for a command with no route), hands in the command's route and patches
  (`path_follow`, the patches flat as `RoutePatch.flat`) and sets the group's route from the
  points it answers.
- **Looking** (`detours::looks`, FormationPathing._looks): a group marching on a route looks
  ahead once its front is `path_relook` cells or more from where it last looked, either way;
  it looks from there (`looked_at`). GDScript asks (`path_looks`) after every group has
  followed, as before, and calls the look (`pathing::look_ahead`, step 3c) for those that do.
- **The patches' list** stays on the GDScript command (FormationCommand.patches), and a look's
  way round is still made a patch there (`RoutePatch.of_detour`, the crate's since 3b), till
  the crate holds the commands (8b4).
- **Identical:** the same tests in the same order (the command's route, then the count; the
  state, the route, then the distance), the same route maths over the same points, and
  `looked_at` written when the group looks, before the look reads anything it changes. The
  proof is CI's digests against the base. Outcomes unchanged.
- **Retired:** FormationPathing._looks and _follow's own re-anchoring; RoutePatch.patched's
  flattening is now `RoutePatch.flat`, shared.
- **Tests:** cargo `detours/tests.rs` (a command without a route leaves its groups be; a group
  follows only ways round it doesn't carry yet; one following stands where it stood along the
  new route; one bound for the far end keeps it as home, one not stays so; only a marching
  group on a route looks; a group looks again only once its front has moved on, back along it
  too). Red on stubs, then green. gdUnit `test_formation_pathing.gd` and `test_route_patch.gd`
  unchanged and green, now through the crate.
- **After it**, 8b4-i: the squads' routes held by the crate.

### Step 8b4-i, the squads' routes held by the crate

- **The route each squad follows** is a squad column (`Squads::route`, none: a straight
  lane), written whenever GDScript sets SkirmishSquad.route (`squad_put_route`: its points
  and corridor's half-width), read back by `squad_route`. The snapshot carries it.
- **Read there now:** the bindings that took one squad's own route as points: staging's
  hold (`staging_hold`, the meeting's timing along it), a march's end (`march_end`, by the
  squad's slot: whether it is on a route, and its units' test for a retreat), the turn
  (`squad_turn`, its roster's only squad), and a stray setting off (`stray_set_off`, the
  stray's route, its formation's).
- **Identical:** the same points in each call (the route held is the one GDScript set, and no
  rule reads a route's half-width); `march_end`'s "on a route" now true for a squad on one
  whatever its order, which only a retreat reads, as before. The proof is CI's digests
  against the base. Outcomes unchanged.
- **Retired:** those four bindings' route arguments, and GDScript's points for them.
- **Tests:** gdUnit `test_skirmish_squad.gd`: the crate holds the route a squad follows, and
  none once it has none. Red on stubs, then green. Cargo: a state with a route read back
  writes the same bytes (`snapshot.rs`). The battles through staging, the ends, the turn and
  the strays unchanged, now through the held routes.
- **After it**, 8b4-ii: the bindings that take many squads' routes, and the march's face and
  sweep, read the held routes.

### Step 8b4-ii, the held routes read by every binding

- **Read there now:** the bindings that took many squads' routes as points: the withdrawal
  (`withdraw`), the rout's flight, rallies, re-forming and breaks (`rout_flee`,
  `rout_rallies`, `rout_regroup`, `rout_break`, each routing squad now one number, its index
  in the roster), the rout's blows (`rout_blows`), the split (`group_split`), a retreat's
  orders (`apply_orders`: each squad's route code only, still GDScript's, since two squads
  share a code only when they share the route object) and the pursuit (`pursue`: the route
  a squad is on; the one it held its post on stays GDScript's, in its pursuit record, till
  8b4-iv).
- **The march's face and sweep** (`march::sweep::face_held` and `sweep_held`) turn a squad's
  frame along the route it holds, its heading written in the crate (`march_face`,
  `march_sweep` on the field). CrateMarch's `face` and `sweep`, which took the points, retire.
- **Identical:** each call reads the same points GDScript passed before, at the same moment
  (nothing in the crate changes a route), and a squad with no route reads none, as before.
  The proof is CI's digests against the base. Outcomes unchanged.
- **Tests:** cargo `march/tests.rs` (a squad faces its held route where its front stands, and
  as it is with none; a squad sweeps towards its held route no faster than its rate, and as
  it is with none). Red on stubs, then green. gdUnit `test_formation_sweep.gd` green. The
  battles through the rout, the withdrawal, the split, the orders and the pursuit unchanged.
- **After it**, 8b4-iii: the commands held by the crate.

### Step 8b4-iii, the commands held by the crate

- **The commands** (`commands.rs`, FormationCommand): each one's guide route, home, standing
  options (hurrying, merging, pursuing and taking as flags; what it does with its own downed
  as its word) and the ways round its groups have learned, in the order learned, in a table
  the squad store keeps (`Squads::commands`, so the snapshot carries it). Each squad names
  the command it follows (`Squads::command`), written whenever SkirmishSquad.command is set.
- **FormationCommand is a view** of its slot on the battle's field: `FormationCommand.new`
  takes the field and opens the command there, and every field reads or writes the crate.
  Units still carry the views they follow and started under (object identity, which the
  groups' codes read), till the tick ports.
- **The patches** are learned in the crate (`Commands::learn`, a look's way round made a
  patch to the command's route, `route::Patch::of_detour`); following reads the command's
  route and patches there (`detours::command_patches`, `command_route`), so `path_follows`
  and `path_follow` take the squad only.
- **Identical:** the same defaults (pursuing and taking, nothing else, home as given), the
  same patch from the same route and way, and the patched route from the same points and
  patches in the same order. The proof is CI's digests against the base. Outcomes unchanged.
- **Retired:** FormationCommand's own fields and `patches` list (now `patch_count()` and
  `learn()`), and the patches' flattening for the follow.
- **Tests:** cargo `commands/tests.rs` (a new command has no route and pursues and takes; an
  order given and taken back; a way round is kept as a patch, in the order learned) and
  `detours/tests.rs` (a group reads its command's route and ways round). Red on stubs, then
  green. gdUnit `test_formation_command.gd`: the crate holds a command's orders, and its
  groups read them there. `test_formation_pathing.gd` green through the crate's patches.
- **After it**, 8b4-iv: staging held by the crate.

### Step 8b4-iv, staging held by the crate

- **A squad's hold-until order** (`staging::Stage`, FormationStaging's `staging`): its
  staging point, whether it waits to see a fight or its partner, its partner, its fallback,
  what it does then, and its meeting point, a squad column (`Squads::staging`) the snapshot
  carries. SkirmishSquad.staging is a view of it, read and written whole (`squad_staging`,
  `put_squad_staging`); the two tests that changed one key of it in place now write it back.
- **The hold reads it** (`staging::hold_staged`): `staging_hold` takes the squad, the route
  codes and, if a coordinated leader times its meeting, its units' slack; its fallback
  counts down in the crate while it holds.
- **The painted places retire:** nothing wrote SkirmishSquad.painted, so the scrum's test of
  it was always true.
- **Identical:** the same order read the same way (a trigger other than "sees_partner"
  waits to see a fight, no partner is 0, the fallback an integer), the meeting only for a
  coordinated leader with a meeting point, and the fallback counted down only while it holds,
  as before. The proof is CI's digests against the base. Outcomes unchanged.
- **Tests:** cargo `staging/tests.rs` (a held order counts its fallback down while it holds,
  not on its way; a held order's trigger reads its partner). Red on stubs, then green. gdUnit
  `test_formation_staging.gd`: the crate holds a squad's staging order, read back whole, and
  none once cleared (red against the library before, then green); the file and
  `test_formation_leaders.gd` green.
- **After it**, 8b4-v: the swaps under way held by the crate.

### Step 8b4-v, the swaps under way held by the crate

- **The swaps** (`swaps::Held`, FormationShuffle's swaps): each squad's swaps under way, in
  the order they began, each its ticks left and in all, its units (the mover first) by id
  and store slot with the place each heads for, the one it stood in and its footprint, and
  the ids of the units it passed: a squad column (`Squads::swaps`) the snapshot carries.
- **Their ends** (`swaps::end_held`): the crate builds what `swaps::end` reads from them,
  keeps those still under way (counted down, moved left with the widenings) and drops the
  rest, and hands back each that ended: its mover, the units it passed and those that
  stepped up after it, for the "swapped" and "stepped_up" events.
- **Their start** (`swaps::claims`, `swaps::begin`): `squad_start_moves` reads the places the
  held swaps claim and their living units, starts the moves (`march::shuffle::start_moves`)
  and holds each. CrateMarch's `start_moves` retires.
- **Read there:** how far each unit is through its move (`swaps::offsets`, FormationShuffle
  `movers` and `offset`, now by unit id), and the moves compact and the taking-in close up
  round (`swaps::moving`; `squad_compact` and `squad_reinforce` lose their `movers`).
  `FormationShuffle.swapping` and `clear` ask and cancel them; SkirmishSquad.swaps is a view,
  read and written whole.
- **Who still stands:** a swap with a fallen unit is cancelled. A unit's state is now
  mirrored in the unit store (`UnitColumns.STATE`, `Columns::state`), written by UnitState,
  its one writer (Decision 147), and when a unit takes its slot; the crate reads a unit as
  standing while its state is before DEAD, as `is_alive()` does.
- **Identical:** the same swaps in the same order with the same units, places and ticks; the
  same progress (`1 - ticks / total`) from the same places; the "swapped" event's keys in the
  same order, its unit by id as before. The proof is CI's digests against the base. Outcomes
  unchanged.
- **Tests:** cargo `swaps/tests.rs` (a held swap with time left counts down and stays; one
  ending reports its mover and passed and is dropped; one whose unit is dead is cancelled;
  moves begun are held and claim their places; a unit's offset is how far it is through its
  first swap). Red on stubs, then green. gdUnit `test_formation_moves.gd`: the crate holds a
  move under way by its units' ids (red against the library before, then green);
  `test_squad_turn.gd`, `test_formation_spread.gd`, `test_unit_state.gd` and
  `test_unit_store.gd` green.
- **After it**, 8b4-vi: the taking held by the crate.

### Step 8b4-vi, the taking held by the crate

- **Who is out taking which body** (FormationTaking): each squad's units out taking the
  downed, [taker's id, its body's store slot] in the order they set out, a squad column
  (`Squads::taking`) the snapshot carries. SkirmishSquad.taking is a view of it, {taker's id:
  its body's store slot}, read and written whole (`squad_taking`, `put_squad_taking`).
- **Read by slot:** a taker's recall reads its body's state from the store's STATE column
  (8b4-v's mirror) where it read the body object; the send claims bodies by store slot;
  the pour finds the body where it lies by its slot. Callers that ask per unit take the view
  once a squad. The test that moves a body in from another battle mirrors its state, as a
  unit's admission now does.
- **Identical:** the same takers in the same order with the same bodies (one slot a body),
  the same state read (UnitState, its one writer, mirrors every write). The proof is CI's
  digests against the base. Outcomes unchanged.
- **Tests:** gdUnit `test_formation_taking.gd`: the crate holds who is out taking which body,
  and none once cleared. Red with the crate's write stubbed (it and the file's sending tests
  fail), then green; `test_formation_wounds_order.gd`, `test_formation_carry.gd`,
  `test_formation_recovery.gd`, `test_ground_bodies.gd` and `test_ground_underfoot.gd`
  green. Cargo: a state with units out taking read back writes the same bytes.
- **After it**, 8b4-vii: spawning and the order queue.

### Step 8b4-vii, spawning's ids and the order queue in the crate

- **The order queue** (`orders::queue`, FormationSimulation.order): orders given for the
  next tick, [squad id, order] in the order first given, a second order for a squad before
  then replacing the first where it stands, as a dictionary's key keeps its place: a squad
  column set (`Squads::pending`) the snapshot carries. `apply_orders` takes the queue itself;
  GDScript reads it (`orders_pending`) only to see whether any retreats, and drops it after
  the tick (`orders_clear`), as before when no squad was left to take it.
- **Spawning's ids** (`Squads::issue_squad`, `issue_units`): the battle's squad and unit ids,
  from 1 in the order issued, counted by the crate (`squads_issued`, `units_issued`, carried
  by the snapshot). `spawn_squad` asks for its unit ids together, then its squad id; the
  strays and the groups still number theirs from the next squad id and hand the count back.
  Building the squad's and units' views, and entering them, stays where it is till 8b5.
- **Identical:** the same ids in the same order, and the same orders applied in the same order
  with the same retreat test. The proof is CI's digests against the base. Outcomes unchanged.
- **Tests:** cargo `orders/tests.rs` (orders queue in the order given, and a second replaces
  the first in place) and `squads.rs` (ids are issued from 1, in order, squads and units
  apart). Red on stubs, then green. gdUnit `test_formation_simulation.gd`: the crate issues
  the ids and queues the orders (red with the queue stubbed: it and the file's ordering tests
  fail; then green, with `test_formation_command.gd` and `test_formation_staging.gd`).
- **After it**, 8b5-i: the units' remaining fields held by the crate (8b5's split is in
  Step 8's bullet).

### Step 8b5-i, the units' numbers held by the crate

- **The sheet** (`sheet::Sheet`, `Columns::sheet`): each unit's 42 numbers, every int, float
  and bool of SkirmishUnit but its id, slot and state (the state is 8b4-v's column): its hp at
  most and strikes, its detection, courage, leadership, height, initiative and discipline, its
  turn rate and backward pace, its direction and squad, its footprint and band, its ranged
  reach and damage, its skills, defence, critical and parrying, its floors, armour and ward,
  its stamina at most, pace, tiring, dodging, run and load, its regeneration, its wounds and
  wounding, playing dead, whom it carries or is carried by, its gear's weight, and its blows'
  and shots' seconds. By store slot, blank at SkirmishUnit's own defaults, carried by the
  snapshot. The crate names them (`sheet_names`), in its order; GDScript asks once.
- **Written:** whole when the store admits a unit (`UnitStore.admit`, which now gives it its
  slot: `put_sheet`), and one at a time after by each of SkirmishUnit's setters
  (`put_sheet_value`). A unit the store lets go (`settle`) writes no more, as its slot may be
  another's. A unit built and not yet admitted writes nothing.
- **Read:** SkirmishUnit keeps the copy it sets and reads it without a call: no crate pass
  writes these yet, and a read through the binding per field would cost a call in every pass
  that reads them per unit per tick until it moves. A later part whose crate pass writes one
  reads it from the store.
- **Identical:** nothing reads the sheet yet; the same values are set in the same order. The
  proof is CI's digests against the base. Outcomes unchanged.
- **Tests:** cargo `sheet.rs` (every number reads back what was set at its index; the names are
  unique and past the last reads nothing; a blank sheet has the unit's defaults; red with the
  write stubbed, then green) and `snapshot.rs` (a state with sheets read back writes the same
  bytes). gdUnit `test_unit_numbers.gd`: every number the crate names is one of the unit's own;
  an admitted unit has its numbers in the crate; one set after is written there; one let go
  writes no more to its old slot (red against the library with the bindings stubbed, then
  green); `test_unit_state.gd`, `test_unit_store.gd` and `test_skirmish_squad.gd` green.
- **After it**, 8b5-ii: the units' words, lists and objects held by the crate.

### Step 8b5-ii, the units' words and lists held by the crate

- **The words** (`words::Words`, `Columns::words`, `Field::lexicon`): each unit's twelve
  words and lists, SkirmishUnit's: its faction and damage type (a word each); its tags,
  tactics, weaknesses, resistances, immunities and what stops its regeneration (lists of
  words, in order); its attributes and traits ({word: level}, in order, the level a float as
  content may give one); and its melee and ranged weapons' parts ([damage, type, magical]).
  Each word is coded once for the field (`Lexicon`, the order first seen, "" is 0), so a
  crate pass compares codes. By store slot, blank at SkirmishUnit's own defaults, carried
  by the snapshot with the lexicon. The crate names them by shape (`word_names`).
- **Written:** whole when the store admits a unit (with its numbers, `UnitWrites.all`), and
  one at a time after by each of SkirmishUnit's setters (`put_word`, `put_word_list`,
  `put_word_levels`, `put_word_parts`). A trait set in place goes through `set_trait`, which
  writes the traits whole (the messenger sent home, FormationWounds). A unit let go writes
  no more. `UnitWrites` holds the writing, numbers and words, so the unit keeps to its size.
- **Read:** SkirmishUnit keeps the copy it sets and reads it without a call, as in 8b5-i.
- **Identical:** nothing reads the words yet; the same values are set in the same order. The
  proof is CI's digests against the base. Outcomes unchanged.
- **Tests:** cargo `words.rs` (words are coded in the order first seen and read back; each
  named field is found by its shape and no other; a field set reads back what was set; red
  on stubs, then green) and `snapshot.rs` (a state with words read back writes the same
  bytes, lexicon too). gdUnit `test_unit_words.gd`: every word the crate names is one of the
  unit's own; an admitted unit has its words in the crate; one set after, or a trait set in
  place, is written there; one let go writes no more to its old slot (red against the
  library with the writes stubbed, then green). `test_unit_numbers.gd`,
  `test_formation_wounds.gd`, `test_formation_groups.gd` and `test_formation_taking.gd` run
  in CI (the last two now set their traits through `set_trait`).
- **After it**, 8b5-iii: the units' objects (the walker, the command and origin, the
  definition) coded for the crate.

### Step 8b5-iii, the units' objects coded for the crate

- **The objects** (`objects::Objects`, `Columns::objects`): each unit's four objects,
  SkirmishUnit's, as codes: its walker as its fields (TerrainWalker.fields: height, swimmer,
  climber, swims, climbs; none for a unit made without one, which crosses as a grounded one
  of its height), the command it follows and the one it started under as their slots in the
  squad store's commands (step 8b4-iii), and the definition it was made from as its code in
  the unit store's own list (`UnitStore.definition_code`, the order first seen). None is -1.
  By store slot, blank at SkirmishUnit's own defaults, carried by the snapshot. The crate
  names them (`object_names`).
- **The definitions stay GDScript's:** a definition is a Resource, so the unit store keeps
  them by code (`UnitStore.definition_of`) for a later part's events to decode. A snapshot
  carries the codes, not the definitions.
- **Written:** whole when the store admits a unit (with its numbers and words,
  `UnitWrites.all`, now given the store), and one at a time after by each of SkirmishUnit's
  setters (`put_walker`, `put_object`). A unit let go writes no more.
- **Read:** SkirmishUnit keeps the copy it sets and reads it without a call, as in 8b5-i.
- **Identical:** nothing reads the objects yet; the same values are set in the same order.
  The proof is CI's digests against the base. Outcomes unchanged.
- **Tests:** cargo `objects.rs` (a blank unit has none; each named code is found and reads
  back what was set; a walker reads back its fields, and none is none; red on stubs, then
  green) and `snapshot.rs` (a state with objects read back writes the same bytes). gdUnit
  `test_unit_objects.gd`: every object the crate names is one of the unit's own; an admitted
  unit has its objects in the crate, one definition one code; one set after is written
  there; one let go writes no more to its old slot (red against the library with the writes
  stubbed, then green). `test_unit_words.gd` and `test_unit_numbers.gd` run in CI.
- **After it**, 8b5-iv: the squads' remaining fields (their units, the joined, faction, the
  living and downed lists).

### Step 8b5-iv, the squads' remaining fields held by the crate

- **The fields** (`Squads::faction`, `joined`): each squad's faction, coded in the unit
  store's lexicon (8b5-ii's), so a crate pass compares it with a unit's; and the units that
  joined it as reinforcements (FormationContact), their ids in the order they joined. Blank
  ("", none) for a new slot, carried by the snapshot.
- **Its units:** its members (its units' store slots, in its order) have been the crate's
  since step 1, listed at the squad's next read of its lists after a join or a leaving
  (UnitState). Its **living and downed** units are not held twice: the crate reads them from
  its members and the STATE column (8b4-v's mirror) as UnitState does (`Squads::living`,
  `downed`; `squad_living`, `squad_downed`), standing before DEAD, downed DOWNED or CARRIED.
- **Written:** the faction by SkirmishSquad.faction_id's setter (`squad_put_faction`); the
  joined whole by its setter (`put_squad_joined`), set by FormationContact.reinforce, its
  one writer. A squad moved to another store (a test's) starts them blank there, as its route and
  command do.
- **Read:** SkirmishSquad keeps the copy it sets and reads it without a call, as in 8b5-i.
- **Identical:** nothing reads them yet; the same values are set in the same order (the
  reinforcements enlist, then join, where they did both a unit at a time: enlisting reads
  nothing joined). The proof is CI's digests against the base. Outcomes unchanged.
- **Tests:** cargo `squads.rs` (a squad's living and downed are its members by state, in its
  order; a new squad has no faction and none joined; red on stubs, then green) and
  `snapshot.rs` (a state with a faction and joined units read back writes the same bytes).
  gdUnit `test_squad_fields.gd`: a squad's faction is held by the crate; units that joined are
  held by their ids in their order, and a wave that reinforces a fight is; its
  living and downed are read from its members and their states (the first three red against
  the library with the writes stubbed, the last, which no write feeds, red in cargo; then
  green). `test_formation_contact.gd` runs in CI.
- **After it**, 8b5-v: orders, engagement, pathing and the move as one crate pass, their
  events built as data (split further: Step 8's bullet).

### Step 8b5-v, the events as data and the orders' pass in the crate

- **The events** (`events::Log`, `KINDS`): what a crate pass reports, built by the crate
  and handed back flat: each is its kind's index, the tick, its squad's id and faction
  (coded in the unit store's lexicon), then a number per field its kind names. The kinds
  are added as their passes move here; this part's are the orders': "order_applied"
  (order), "disengaged", "withdrawing", "pursuing" (of) and "morale_band" (band, morale).
  GDScript asks for the kinds once (`event_kinds`) and decodes each (CrateEvents, through
  `events_take` and `lexicon_words`) into the dictionary FormationEvents made, its keys in
  the same order; an order or a band is named from its number. The log isn't carried by
  the snapshot: a pass's events are taken in the same tick.
- **The orders' pass** (`orders::pass`): the queue applied (step 7c's), then each order
  landed in the crate: its event, and for a retreat out of a fight its events, its shock
  and its pursuers' events, in the order FormationOrders made them. What a retreat reads is
  read there: whether a squad may pursue (its command's, step 8b4-iii), and its living
  units' chances to break ranks and leashes, from their sheets (`discipline.rs`,
  FormationDiscipline's unit rules, step 8b5-i's numbers). The shock (`shocks.rs`) is
  FormationMorale.shock: the squad's morale, first read as its ceiling, drained and held,
  a change of band its event. The routes its pursuers keep or take stay GDScript's to land
  (the routes it holds are objects), from the four numbers per pursuer the binding returns.
- **Identical:** the same orders, shocks and routes in the same order, and the same events
  in the same order with the same keys and values. The proof is CI's digests against the
  base. Outcomes unchanged.
- **Tests:** cargo `events.rs` (events are taken flat in the order built, once; each kind
  is named once), `discipline.rs` (a formation's mean, rounded, bolstered by its best
  leader; its disorder; a unit's own and its leash), `shocks.rs` (a shock drains morale, a
  change of band its event; a squad with none yet is shocked from its ceiling) and
  `orders/tests.rs` (the pass takes the queue and reports each order; a ragged retreat is
  shocked and its pursuers reported in order; enemies break ranks by their sheets'
  discipline); red on stubs, then green. gdUnit `test_formation_orders.gd`: an order applied
  is the event it always was, keys in order; a ragged retreat is shocked by its disorder and
  reported in order (red with the GDScript's taking of the events stubbed, then green).
  `test_formation_retreat.gd` and `test_formation_simulation.gd` run in CI.
- **After it**, 8b5-vi: engagement (perception, the fronts, the edges and their shocks) and
  pathing (the follow, the look) as crate passes, their events built as data.

### Step 8b5-vi, engagement's events and shocks built in the crate

- **The passes** (`contact::pass`): the fronts and the edges decide in the crate already
  (step 4c); now they report there too. Each front lock picked is its attacker's "engaged"
  event (with: its foe's id), in the lists' order; each flank lock landed is its victim's
  "flanked" event (by: the attacker's id, edge: SquadEdges' number), then the victim's
  shock (`shocks.rs`, 8b5-v's): `morale_rear_impact` for its rear, `morale_side_impact`
  for a side. FormationFronts and FormationEdges hand the crate the tick, the impacts and
  morale's tuning (BandTuning, now shared with FormationOrders and FormationMorale) and
  take the events (CrateEvents). Perception reports nothing, and stays the binding it was:
  taking reads its picture till taking moves (8b5-x).
- **Identical:** the same locks, shocks and events in the same order with the same keys
  and values. The proof is CI's digests against the base. Outcomes unchanged.
- **Tests:** cargo `contact/pass.rs` (each front lock is its attacker's event in the lists'
  order; a flank lock is the victim's event then the shock of its edge; the rear's is the
  rear's impact) and `events.rs` (the new kinds named once); red on stubs, then green.
  gdUnit `test_formation_morale.gd`: a rear hit is reported, keys in order, then the band
  it shocks the line into; `test_formation_engage_rule.gd`: a front lock is its attacker's
  "engaged" event with its foe, keys in order (red with the GDScript's taking of the events
  stubbed, then green).
- **After it**, 8b5-vii: pathing (the follow, the look, "detoured") as a crate pass, its
  event built as data; the look reads each unit's wits and its definition's sight from the
  crate.

### Step 8b5-vii, pathing as a crate pass

- **The pass** (`pathing::pass::step`): FormationPathing hands the crate its squads' slots,
  in the lists' order, the ground and the tuning (`path_step`). Every group re-routes onto
  its command's patched route where it has changed (`detours::follows`), standing where it
  stood along it: the crate now sets the squad's route and its place too. Then each
  marching group that has moved on looks ahead (`detours::looks`); a way round it keeps is
  learned by its command and reported as its "detoured" event (cells: the way's waypoints),
  built in the crate's log. GDScript takes the events (CrateEvents) and gives each group
  that re-routed its FormationRoute afresh from the crate's (`squad_route`).
- **What a look reads** (`pathing::pass::members`): each living unit, in its members'
  order, where it lives: its walker from its objects (8b5-iii; a grounded one of its sheet's
  height for none), its wits from its attributes (8b5-ii; AVERAGE unlisted), its id and
  place from the store, and its definition's sight from the field's `sights`, by the
  definition's code: UnitWrites writes a definition's detection range and sight shares as
  it codes it (`put_sight`). CratePaths.members is a read of the same (`path_members`), for
  FormationPathing.group_walker and SquadPathfinder. The bindings the pass replaces
  (`path_follows`, `path_follow`, `path_looks`, GroundPaths' `look_ahead`) are gone.
- **Identical:** the same follows, looks, ways round and events in the same order with the
  same keys and values. The proof is CI's digests against the base. Outcomes unchanged.
- **Tests:** cargo `pathing/pass/tests.rs` (a look reads each unit's walker, wits, place
  and definition's sight where they live; a barred route ahead is kept on the command and
  reported with its cells; every group of the command follows the patched route on the
  next tick; an open route, or a group that hasn't moved on, makes no detour), red on
  stubs, then green. gdUnit `test_formation_pathing.gd`: a look reads each unit's wits and
  its definition's sight from the crate (red with UnitWrites' sight write stubbed, then
  green).
- **After it**, 8b5-viii: the move (staging, the march's ends, the turn, the sweep, the
  walk, the drag, the joins) as a crate pass, its events built as data.

### Step 8b5-viii, the move's march as a crate pass

- **The pass** (`moving::pass::step`): FormationSimulation hands the crate the tick's
  roster, each squad's share of its pace (FormationWalk.share, worked out for each squad that
  may march and isn't ordered to hold; read only where it steps), its route's code
  (FormationStaging.route_codes) and the tuning (`move_step`). Each squad, in the lists'
  order, decides from the tick's start (`moving::choose`): a staged one holds till its
  trigger (`staging::hold_staged`; a coordinated leader's slack from its discipline, read
  from the sheets, and its tactic from the lexicon), a retreat home already ends there, one
  free to march holds or goes (`moving::hold`), one going back the way it faces turns first
  (`turn::begin`; an about-face cancels its swaps), and the rest step: at their slowest
  unit's pace within the share handed in, the frame swept round any bend (`sweep_held`),
  slowed by the ground its front rank steps into (`march::pace`, each unit's walker from its
  objects) and the bodies lying on the field (`lying`, the downed and the dead found in the
  store once a tick). Every move lands together (`moving::land`), each front placed on its
  route; arrivals and returns after; the waiting count down. The events ("staged",
  "signalled", "gave_up" with its word coded in the lexicon, "arrived", "returned",
  "turning" and "turned" with the facing and the heading's bits, "blocked") go to the log,
  in the order GDScript built them. It returns the squads that advanced, which GDScript joins
  (FormationJoins) as before.
- **Gone:** `move_holds`, `move_land`, `staging_hold`, `march_sweep`, GroundMarch's
  `toward`, `travel_sign` and `pace`; FormationStaging.holds, FormationSweep.step and
  FormationMarch's `length`, `pace`, `toward` and `travel_sign`. ScrumTurn.begin,
  FormationMarch.check_ends and GroundBodies.drag stay for the withdrawal, the pursuit and
  their tests.
- **Identical:** the same holds, turns, steps, landings, ends and events in the same order
  with the same keys and values. The proof is CI's digests against the base. Outcomes
  unchanged.
- **Tests:** cargo `moving/pass/tests.rs` (a squad steps at its slowest unit's pace within
  its share and advances; one out of its slack waits; one reaching the enemy's end arrives;
  one going back the way it faces turns first and reports its heading; a staged squad holds,
  then gives up and turns back as its order says; the waiting count down and one ordered to
  hold holds), red on a stubbed pass, then green. gdUnit `test_formation_staging.gd`: the
  move's events read as FormationEvents made them (red with CrateEvents' word and heading
  decoding stubbed, then green).
- **After it**, 8b5-ix: the walk's share (each unit's pour, place, swap and walker, read
  where they live) and the joins with their step-ups, in the crate.

### Step 8b5-ix, the walk's share read in the crate

- **The share** (`moving::share::of`): the share (0 to 1) of its step a squad's frame takes
  for its units lagging behind their places, each unit read where it lives. None while any
  of its units is out taking (the squad's taking); all of it under "fall behind, left
  behind" (the rule from its living units' traits, by their lexicon codes, and their
  sheets' leadership). Otherwise its living units, less the loose and the fleeing (the
  squad's lists, by id), are poured as `march::pour` reads them: where each stands, its
  place (rank, column) from the store, its footprint from its sheet, its walker from its
  objects (a grounded one of its sheet's height for none), how far it is through a swap
  from the squad's swaps under way (`swaps::offsets`); the slack from its discipline
  (`share::slack`, read from the sheets, which the staged hold now reads too).
- **The pass** (`moving::pass`): works out each listed squad's share as the tick starts,
  for each that may march (`choose`) and isn't ordered to hold, as `_keeping` did; `move_step`
  takes the fight's seed and wounds_reach and walk_lost instead of the shares.
  FormationWalk.share reads the same for one squad (`walk_share`), for its callers and tests.
- **Gone:** FormationSimulation._keeping and MOVE_GOES, the `move_choices` binding,
  GroundMarch's `share`, FormationWalk.RULE_CODES.
- **Identical:** the same shares from the same units in the same order. The proof is CI's
  digests against the base. Outcomes unchanged.
- **Tests:** cargo `moving/share/tests.rs` (every unit at its place keeps the whole step;
  one lagging three quarters of its slack halves it, one at its slack stops it; one fallen
  behind is waited for only if no man is left behind; a leader who leaves the fallen behind
  never waits; a unit part way through a swap is judged from its place part way there; the
  loose and the fleeing aren't waited for and one out taking holds the frame; a drilled squad
  has more slack) and `moving/pass/tests.rs` (the share the pass works out sets the step),
  red on a stubbed share, then green. gdUnit `test_formation_walk.gd`: a unit lagging three
  quarters of its slack halves its frame's step (red with FormationWalk.share stubbed, then
  green).
- **After it**, 8b5-x: the joins and their step-ups as data (who joins whom, the moves, the
  lists, the command and the events in the crate; GDScript drops the joined squads).

### Step 8b5-x, the joins and their step-ups as data

- **The joins** (`joins::step`, from `move_step` after the move lands): each squad that
  advanced and has reached a friend's back rank joins it, nearest first, a tie to the
  squads' draws (`contact::Field::joins`, as before). For each, in that order: "reinforced"
  (the leader fighting) or "merged", on the joining squad, `into` the leader; the joining
  squad taken in (`joins::take_in`); "stepped_up" on the leader for each unit that stepped
  up, with its unit's id and its rank once all have moved; the leader reforming. The events
  go to the log as FormationEvents made them (`events::KINDS` gains the three kinds).
- **Taking in** (`joins::take_in`, FormationContact.reinforce): each squad read where it
  lives: the leader's members shift with its frame (`compact::reinforce`), each living
  unit's id from the store, its footprint and band from its sheet, the leader's moves under
  way from its swaps. The joining units then follow the leader (FormationCommand.enlist):
  their sheet's `squad_id`, their command the leader's, their origin too if they started
  under none; the leader's members and joined list take them in, in the joining squad's
  order; the joining squad's members are emptied. FormationContact.reinforce, for the rout's
  rejoining and the groups' taking, calls the same (`squad_take_in`).
- **GDScript** keeps its own lists, which mirror the crate's till they go: `move_step`
  returns the joins (leader, joining, as listed indices), and FormationJoins.join hands each
  to FormationContact.took_in (the units enlisted in GDScript's copies, the joined list, the
  joining squad's units cleared) and drops the joining squad from the list.
- **Gone:** the `contact_joins` and `squad_reinforce` bindings; FormationJoins' events and
  its call to the crate's order.
- **Identical:** the same joins, moves, lists, commands and events, in the same order with
  the same keys and values. The proof is CI's digests against the base. Outcomes unchanged.
- **Tests:** cargo `joins/tests.rs` (a wave reaching a fighting friend's back rank is taken
  in behind it, enlisted under its command, the friend's members and joined list taking it
  in and its own emptied, and "reinforced" logged; a merging wave steps up where the
  friend's front has fallen, "merged" and "stepped_up" with the rank logged; a joining unit
  keeps the command it started under; a wave short of its friend, or one not merging with a
  marching friend, joins nobody; taking in hands back the living closed up and which stepped
  up), red on a stubbed pass, then green. gdUnit `test_formation_contact.gd`: a joined wave's
  event reads as FormationEvents made it, and GDScript drops the wave, its units the
  leader's under its command (red with FormationJoins.join stubbed, then green).
- **After it**, 8b5-xi: the scrum, the fight and deaths.

### Step 8b5-xi, the scrum's engaging, manoeuvres and stall over the roster

- **One roster for the scrum:** FormationScrum.step makes the squads' roster (CrateRoster)
  as it starts and hands it to every phase: nobody falls, joins or leaves in the scrum, so
  it is the roster the regroup, the chasers, the withdrawal and the pursuit read before.
- **Engaging** (`scrum::pass::engage`, `scrum_engage`): each listed squad's living units
  read from the roster, each standing where ScrumReach.at has it (its loose point while
  loose), its body's radius from its footprint; the locks as `engage::step` decides them;
  each lock's "engaged" event, with its foe's id, in the log in the lists' order.
- **Manoeuvres** (`scrum::pass::manoeuvre`, `scrum_manoeuvre`): each squad's withdrawal read
  from its record, whether it fights from its locks, whether it skirmishes asked only where
  neither applies (`contact::Field::skirmishing`), a narrowed squad's living units from the
  roster; then `manoeuvre::step` as before.
- **The stall** (`scrum::pass::stall`, `scrum_stall` now taking the roster and the tick):
  the listed squads and their factions from the roster, each release's "disengaged" event in
  the log, in the lists' order.
- **Gone:** the `engage` and `manoeuvre_step` bindings; ScrumEngage's and
  FormationManoeuvre's gathering; the stall's events built in GDScript; the scrum's second
  roster.
- **Identical:** the same locks, manoeuvres, releases and events, in the same order with the
  same keys and values. The proof is CI's digests against the base. Outcomes unchanged.
- **Tests:** cargo `scrum/pass/tests.rs` (squads whose units come within reach lock, each
  lock its squad's "engaged" event; a loose unit engages from where it stands loose; each
  squad takes the manoeuvre of the highest priority that applies, a narrowed one's units
  loosened; an advancing squad of archers with a foe in its reach skirmishes; a fight quiet
  for its ticks is released and each side's "disengaged" logged), red on stubbed passes,
  then green. gdUnit `test_formation_scrum.gd`: a scrum lock and a stalled fight's release
  read as FormationEvents made them (red with the GDScript's taking of the log stubbed, then
  green).
- **After it**, 8b5-xii: the scrum's preparing and stance ("faced" in the log).

### Step 8b5-xii, the scrum's preparing and stance over the roster

- **Preparing** (`scrum::prepare::prepare`, `scrum_prepare(roster, tick)`, FormationScrum._prepare):
  each listed squad, in the lists' order. A routing or destroyed one lets its loose units,
  its stance and its fight go. Otherwise each loose unit no longer standing (its STATE
  column) leaves its entry; a fight beginning sets `fight_since` to the tick, `fought` and
  `stall_ticks` 0; one ending closes the squad's ranks over its dead (`ranks::close` over
  the roster's living, their preferred bands and footprints) and ends its swaps under way;
  a squad fighting, or holding a stance, sends each living unit not yet loose loose where it
  stands (its POSITION).
- **Stance** (`scrum::prepare::anticipate`, `scrum_stance(roster, ground, rules, tick,
  seed)`, ScrumStance.anticipate_all): each listed squad's bounds read from its shape
  (`geometry::Shape`), whether it meets threats as a line from its state, its front and its
  discipline (`discipline::of` against `discipline_meets_threats`), and the hostiles it
  detects (`perception::detects`, on the ground); then `stance::anticipate` as before. Each
  new stance's anchor is the face of its footprints towards its heading, as ScrumStance._turn
  worked it out, and its "faced" event (heading as its f64 bits, then facing) goes to the
  log (`events::KINDS` gains it). The scrum's one roster serves: ScrumStance's own is gone.
- **Gone:** the `anticipate` binding; ScrumStance's gathering (`_looks`, `_sees`, `_turn`)
  and FormationScrum._prepare's GDScript body.
- **Identical:** the same entries, places, marks, stances and events, in the same order with
  the same keys and values. The proof is CI's digests against the base. Outcomes unchanged.
- **Tests:** cargo `scrum/prepare/tests.rs` (a routing squad lets its loose units, stance
  and fight go; a fight beginning is marked, the fallen unloosed and the living sent loose
  where they stand; a fight ending closes the ranks over the dead and sends nobody loose; a
  squad holding a stance sends its units loose out of a fight; a disciplined squad takes a
  line facing an enemy closing on its flank, "faced" logged; one short of the discipline
  meets it unit by unit, letting its line go), red on stubbed passes, then green. gdUnit
  `test_formation_scrum.gd`: a line's facing reads as FormationEvents made it (red with the
  GDScript's taking of the log stubbed, then green).
- **After it**, 8b5-xiii: the seek, the walk and the facing.

### Step 8b5-xiii, the scrum's seek, walk and facing over the roster

- **One call** (`scrum::seek::step`, `scrum_seek(roster, ground, ints, floats)`,
  ScrumSeek.step from FormationScrum._seek): the field brought to the scrum's snapshot of
  the roster's squads, in the lists' order (BodyFieldSync.scrum's sync; a unit new to the
  field enrolled with its radius, footprint, draw, initiative and record); then the seek,
  the walk and the facing as before (`seeking::seek`, `walk_store::walk`,
  `face_store::face`).
- **Read in the crate:** each fighting squad's foes (`locks::foe_ids` over the roster),
  whether its units may seek (its order isn't to retreat), its frame (its stance's heading
  and anchor, else its own heading, width, centre shift and position), its chasers, each
  unit's speed (its SPEED column); the walk's pace (crowded out of a pursuit) and the bodies
  lying on the field (GroundBodies.ground, as the move's `Bodies`); on terrain, whether a
  unit may stand on a point (`Ground::crossing` with its walker) and each step paid for the
  ground it crosses (`mover::pay`). The fighting squads active this tick come back by id,
  for the stall.
- **Gone:** the `seek`, `walk`, `face` and `squad_foe_ids` bindings; ScrumSeekField,
  ScrumWalkField and ScrumFaceField; ScrumSeek.plan and foe_ids; the scrum's own snapshot of
  the bodies lying (ctx["lying"]). tools/tick_phases.gd and tools/native_bench.gd time the
  one call.
- **Identical:** the same slots, steps, bearings and active squads, in the same order. The
  proof is CI's digests against the base. Outcomes unchanged.
- **Tests:** cargo `scrum/seek/tests.rs` (a fighting squad's units make for slots beside
  their foes and step towards them, both squads active; one ordered to retreat seeks no
  slot; a unit touching a foe turns towards it; squads out of a fight are not walked), red
  on a stubbed pass, then green. gdUnit `test_formation_scrum.gd`: units within reach touch
  and keep their fight active (red with the GDScript's call stubbed, then green);
  `test_body_field.gd`'s list-order test runs the one call.
- **After it**, 8b5-xiv: the regroup, the chasers, the withdrawal and the pursuit, their
  events.

### Step 8b5-xiv, the scrum's regroup and chasers over the roster

- **One call** (`scrum::regroup::step`, `scrum_regroup(roster, ground, rules, seed)`,
  ScrumRegroup.step(field, roster, ctx) from FormationScrum._regroup): the regroup
  (`regroup::step`), then the chase (`pursuit::chase`), as ScrumRegroup.step and
  ScrumPursuit.step ran them.
- **Read in the crate:** which squads are out of the fight (neither fighting nor routing,
  by the state column), whether each has units loose, whether it is a drilled retreat (its
  order, and FormationDiscipline.of from its living units' sheets against
  `discipline_meets_threats`), whether it withdraws (its record), its re-form pace
  (FormationDiscipline.reform_pace: its discipline over `discipline_march_pace_at`,
  clamped), which squads have chasers; on terrain, out of a stance, where its living units
  none fleeing walk back to (TerrainStep.returns: `march::pour::targets` in its frame, each
  unit's swap under way and the downed foe it is out taking read from the squad,
  `moving::share::pourers`), the chasers' read after the regroup has walked.
- **Gone:** the `regroup` and `pursue_chasers` bindings; ScrumPursuit.step (ScrumPursuit
  keeps `away` and the rules' account); TerrainStep.returns. tools/tick_phases.gd times the
  one call ("scrum: regroup and chasers").
- **Identical:** the same walks, trades, chases and returns, in the same order. The proof
  is CI's digests against the base. Outcomes unchanged.
- **Tests:** cargo `scrum/regroup/tests.rs` (a squad out of the fight walks its loose
  units back; a fighting squad's are not walked back; a chaser walks after its quarry; a
  disciplined retreat with units out regroups drilled at its re-form pace), red on a
  stubbed pass, then green. gdUnit `test_scrum_regroup.gd`: a squad out of the fight walks
  its loose unit back (red with the GDScript's call stubbed, then green);
  `test_formation_list_order.gd`'s regroup test runs the one call.
- **After it**, 8b5-xv: the withdrawal and the pursuit, their turns and events.

### Step 8b5-xv, the scrum's withdrawal over the roster

- **One call** (`scrum::withdraw::step`, `scrum_withdraw(roster, ground, rules, tick,
  seed)`, FormationWithdraw.step(field, roster, ctx, events) from FormationScrum._withdraw):
  those routing or destroyed withdraw no more (`withdraw::ended`); the rest flee
  (`withdraw::step`), count their safe ticks and re-form (`withdraw::records`); each
  re-form is landed in order: its turn (`turn::landed`: ScrumTurn.begin with the route it
  holds, an about-face cancelling its swaps under way, "turning" and "turned"), then
  "regrouping", and "returned" for one home with nowhere further to go.
- **Read in the crate:** which squads withdraw (their record and state), each one's
  disorder (FormationWithdraw.disorder, `discipline::disorder`) and re-form pace
  (FormationDiscipline.reform_pace, `discipline::reform_pace`, which the regroup now shares)
  from its living units' sheets, its route and home (`home_distance`), and its living units'
  flight angles (RoutFlight.fan, `rout::flight::fan`).
- **Gone:** the `withdraw` binding; FormationWithdraw's `_reformed`. The move's turn logs
  through the same `turn::landed`. FormationWithdraw keeps `disorder`; ScrumTurn.begin stays
  for the pursuit. "regrouping" joins `events::KINDS`. tools/tick_phases.gd follows.
- **Identical:** the same flights, records, re-forms, turns and events, in the same order.
  The proof is CI's digests against the base. Outcomes unchanged.
- **Tests:** cargo `scrum/withdraw/tests.rs` (a withdrawing squad's units flee home; safe
  long enough it about-faces for home, "turning", "turned", "regrouping"; an about-face
  cancels the swaps under way; with all its units home it holds, "regrouping", "returned";
  one routing withdraws no more; its disorder, re-form pace, home and fans are read in the
  crate), red on a stubbed pass, then green. gdUnit `test_scrum_withdraw.gd`: a withdrawal
  safe long enough re-forms, its "regrouping" as FormationEvents made it (red with the
  GDScript's taking of the log stubbed, then green).
- **After it**, 8b5-xvi: the pursuit, its turns and events.

### Step 8b5-xvi, the scrum's pursuit over the roster

- **The post's route held** (`Squads::post_route`, a squad column the snapshot carries): the
  orders' pass keeps the route a pursuer held its post on as its pursuit begins
  (`orders::begin_pursuit`). `squad_pursuit`/`put_squad_pursuit` carry it after the
  pursuit's eight numbers ([half width, then the points' x and y]), so OrderRecords.carry
  carries it with the pursuit. OrderRecords' `hold_route`/`_routes` and the pursuit view's
  "route" are gone; FormationOrders lands only the route a pursuer takes.
- **One call** (`scrum::pursue::step`, `scrum_pursue(roster, rules, tick)`, from
  FormationPursuit.step): each pursuing squad (a record set, a unit living) is read in the
  crate: the route it is on, its `post_route`, and its leash (FormationDiscipline
  .pursuit_leash: `discipline::of` from its living units' sheets, a step out for a living
  unit that "pursues", a step in for one "cautious", `discipline::leash`). Then
  `pursuit::whole::step`, and each end landed in order: "pursuit_ended" for one that gave
  up; for one back at its post, its `post_route` taken up as its route and its turn
  (`turn::landed`: "turning", "turned"). Returns the listed indices back at their posts.
- **GDScript's route mirrored** on the return (FormationPursuit._mirror): the route it holds
  if the crate's has the same points (it never took its quarry's), else its command's if
  that has them (a lane's squads share their command's route), else one made from the
  crate's points with its command's corridor.
- **Gone:** the `pursue` binding, FormationPursuit's ENDED/ARRIVED and its turn through
  ScrumTurn. ScrumTurn.begin and `squad_turn` stay: test_squad_turn.gd uses them.
  "pursuit_ended" joins `events::KINDS`. FormationPursuit.reach and
  FormationDiscipline.pursuit_leash stay for the presentation.
- **Identical:** the same pursuits, ends, routes, turns and events, in the same order. The
  proof is CI's digests against the base. Outcomes unchanged.
- **Tests:** cargo `scrum/pursue/tests.rs` (it advances along the route it is on; its leash
  is its discipline's step moved by its leaders' tactics; at its leash it gives up,
  "pursuit_ended"; back at its post it takes up its own route and turns the way it held it,
  "turning", "turned"; a destroyed pursuer pursues no more) and `orders/tests.rs` (a pursuer
  keeps the route it held its post on as it begins, and only then), red on a stubbed
  line, then green; `snapshot.rs` carries `post_route`. gdUnit `test_scrum_pursue.gd`: a
  pursuer whose enemy is gone says "pursuit_ended" as FormationEvents made it; one back at
  its post holds its own route again, GDScript's copy too, and turns (red with the GDScript
  mirror and the taking of the log stubbed, then green).
- **After it**, 8b5-xvii: the fight.

### Step 8b5-xvii, the fight's blows and shots chosen in the crate

- **One call** (`fight::choose`, `fight_choose(roster, [interval, seed], [reach_contact,
  front dot, rout_strike_reach], band tuning)`, from FormationMelee.blows, which now returns
  [melee blows, shots]): every living unit's target cleared; if some squad may strike
  (ScrumBlowsField._any_strike, now `fight::any_strike`), the field synced to the scrum
  (`scrum::seek::sync`) and each pick of `blows::picks` taken in the sync's order: its target
  set, its cooldown run down, and at 0 started again at its melee time (ScrumBlows.ticks),
  half again while its squad wavers (FormationMorale.band, read at its first blow through
  `shocks::morale`), a blow unless it retreats and has turned away; then the blows on
  routers from the front ranks, by column, of the squads free to fight (RoutBlows,
  `rout_blows::blows`); then the shots (FormationCombat.ranged_blows, `shots::shooters` and
  `shots::shots`, each unit's ranged time from its sheet). Each blow's damage is its
  striker's sheet's `dmg`, a shot's its `ranged_dmg`. Returns whether it synced, which
  ScrumBlowsField.mark records for the landing's count.
- **Gone:** the `blows`, `rout_blows` and `shots` bindings; ScrumBlowsField.blows, `_any_strike`,
  `_struck` and `spent`; RoutBlows, ScrumBlows and FormationCombat (scrum_blows.gd,
  rout_blows.gd, formation_combat.gd).
- **Identical:** the same targets, cooldowns, blows and shots, in the same order. The proof
  is CI's digests against the base. Outcomes unchanged.
- **Tests:** cargo `fight/tests.rs` (no foe in reach: every target cleared, none strikes;
  fighting units strike the foes they touch and start again at their melee time; a wavering
  squad half again more slowly; a squad with no morale yet reads its band from its ceiling;
  one still cooling down takes its target but strikes not; one retreating with its back to
  its foe strikes no blow; hostiles free to fight strike routers from behind; a ranged unit
  shoots for its ranged damage), red on a stubbed pass, then green. gdUnit
  `test_formation_melee.gd`: two units locked in a duel strike each other, their targets
  set; a wavering formation's cooldown half again longer (red with the GDScript decoding
  stubbed, then green).
- **After it**, 8b5-xviii: the blows' landing, hurts, "hit" and "spat", the surrender and the
  strike's stamina.

### Step 8b5-xviii, the fight's landing read in the crate

- **One call** (`fight::land::land`, `fight_land(roster, blows, rolls, landing tuning,
  reach_contact, band tuning, ground)`, from BlowLanding.land, which hands it each blow as
  [striker slot, target slot, damage, 1 if a shot, 1 if from a flank]): each unit's arms
  read from its sheet (skills, critical, floors, most stamina, dodging, defence, armour,
  ward, wounding, parries) and its words (its melee and ranged parts, the damage types it
  is weak to, resists, is immune to and that stop its regeneration, coded afresh for the
  tick in the order first met as BlowLanding._Arms coded them, so a tick may name 64
  whatever the lexicon holds; agility from its attributes, else UnitDef.AVERAGE). With
  rolls on, the field as `fight_choose` synced it this tick (the BodyField keeps whether it
  did, and `fight_land` takes it), or synced now (`scrum::seek::sync`); then
  `blows::pressed` over every living unit's target: each striker's squad, whose morale band
  is `shocks::morale` and `bands::band`, and the foes pressing each target. Then
  `landing::land`. FormationMelee.blows now also returns the roster it chose over, which
  FormationSimulation._fight hands BlowLanding.land.
- **Gone:** the `land_blows` and `pressed` bindings and `arms_of`; BlowLanding's `_Arms`;
  ScrumBlowsField (scrum_blows_field.gd: `mark`, `synced`, `sync`, `pressed`).
- **Identical:** the same arms, bands, counts, codes and blows in the same order, so the
  same rolls and damage. The proof is CI's digests against the base. Outcomes unchanged.
- **Tests:** cargo `fight/tests.rs` (a blow lands with the arms its units' sheets and words
  hold, a damage type coded past the lexicon's 64th word still found; a wavering
  formation's blows land less often; a unit both its touching foes fight is struck more
  surely), red on a stubbed pass, then green. gdUnit `test_blow_landing.gd`: a blow lands
  with the arms the crate holds for its units (red with the GDScript write stubbed, then
  green); `test_unit_view.gd`'s hold check now through BodyFieldSync.scrum.
- **After it**, 8b5-xix: the fight as one `fight_step` pass: the hurts, "hit" and "spat" in
  the log, the surrender (TAKEN, "surrendered", "destroyed", the locks' release) and the
  strike's stamina.

### Step 8b5-xix, the fight as one crate pass

- **One call** (`fight::step::step`, `fight_step(roster, [interval, seed, tick, 1 if blows
  roll], [reach_contact, front dot, rout_strike_reach, stamina_blow], band tuning, landing
  tuning, ground)`, from FormationMelee.fight, which FormationSimulation._fight calls): the
  blows and shots chosen (`fight::choose`), landed melee then shots (`fight::land::land`, on
  the field as the choice synced it), each at the damage it dealt; the hurts, each shot then
  each blow (`hurts::land`), the fallen onto the fell queue; "spat" for each shot (its
  shooter's squad, its damage type from the lexicon, how it landed) and "hit" for each blow
  in the log; then each striker's stamina spent, blows then shots (`stamina::strike`, its
  tiring from its sheet). Returns the melee blows, [striker, target, dealt], for the
  surrender, which stays GDScript's (FormationWounds.surrender) till 8b5-xx.
- **Events told of a unit:** "hit" has no squad: `events::UNIT_LED` names such kinds,
  `Log::unit` writes the unit's id and its own faction in the squad's place, `event_kinds`
  says which, and CrateEvents names the field "unit". A damage type decodes from the
  lexicon, a flank to a bool, how a blow landed to BlowLanding.LANDED's name.
- **Gone:** FormationEvents.hit. BlowLanding.land, FormationMelee.blows,
  UnitHealth.hurt_landed and FormationStamina.strike, and their bindings, stay for their
  tests till 8b5-xx; BlowLanding.tuning is now public, for FormationMelee.fight.
- **Identical:** the same blows, landings, hurts, events and stamina, in the same order. The
  proof is CI's digests against the base. Outcomes unchanged.
- **Tests:** cargo `fight/step/tests.rs` (each blow hurts its target, says "hit" and tires
  its striker; a shot hurts first and says "spat" with its shooter's damage type; a unit
  struck to nothing falls onto the fell queue), red on a stubbed pass, then green;
  `events.rs` (a unit-led event carries its unit's id and faction). gdUnit
  `test_formation_fight.gd`: a duel's blows hurt, say "hit" as FormationEvents made it, key
  for key, and tire their strikers (red with the GDScript call stubbed, then green).
- **After it**, 8b5-xx: the surrender in the pass, and the test-only pieces retired.

### Step 8b5-xx, the surrender in the fight's pass

- **In the pass** (`fight::step::step`, after the strike's stamina): each melee blow, in the
  order chosen, whose target is a living unit of a ROUTING squad, dealt damage and doesn't
  have "never_surrenders" is caught; `wounds::surrenders` rolls each (wounds_surrender, now
  the fifth of `fight_step`'s floats). Each that gives up and still stands (one struck twice
  gives up once) is TAKEN in the STATE column and "surrendered" (its squad; the unit) goes in
  the log; a squad left with no one standing, and not yet DESTROYED, is DESTROYED, released
  with `locks::release` over the roster's squads, and "destroyed" goes in the log. Both kinds
  are new in `events::KINDS`.
- **Back to GDScript:** `fight_step` returns the units taken, [listed, member] each, and
  FormationMelee.fight hands them back; FormationSimulation._fight mirrors each through
  UnitState.put (its own copy, the squad's kept lists) before CrateEvents takes the log.
  The squads' states are the crate's already (StoredSquad.state).
- **Gone:** FormationWounds.surrender's call from the tick. It, and the other pieces left
  only to tests, stay for their tests till 8b5-xxi.
- **Identical:** the same caught units, rolls, states, releases and events, in the same
  order. The proof is CI's digests against the base. Outcomes unchanged.
- **Tests:** cargo `fight/step/tests.rs` (a routing unit struck surrenders and says so; its
  squad, left with no one, is destroyed and frees its foe; one with "never_surrenders"
  doesn't), red on a stubbed pass, then green. gdUnit `test_formation_fight.gd`: a router
  struck in the duel is TAKEN in GDScript's copy as in the crate, and "surrendered" and
  "destroyed" come after "hit" (red with the GDScript mirror stubbed, then green).
- **After it**, 8b5-xxi: the pieces left only to tests retired.

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

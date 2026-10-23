# Breach's native core (Rust)

Decision 129: the simulation's hot core moves to Rust, through GDExtension
(godot-rust/gdext). It owns its state, and GDScript keeps the rules and stays the
reference behind one switch. The first production piece is in place:

- **`BodyField`**: the bodies' state, owned natively and kept across ticks.
- **Body parting** (UnitBodies) runs on the field.
- **The scrum's slot search** (ScrumSeek with SlotSearch, ScrumSlots and ScrumNear) runs
  on the field.
- **The scrum's facing** (FormationScrum._faces and the turns: ScrumBlows.nearest_touching,
  UnitShuffle.look, UnitMotion's bearing_to, pace and turn) runs on the field, after the
  walk, for the units the slot search was given.
- **The scrum's walk** (FormationScrum._walk: UnitSteer.toward and _round, SquadFrame.place,
  UnitShuffle.look, UnitMotion.move, GroundBodies.underfoot) runs on the field, between
  the slot search and the facing, for the same units.

All three reproduce the GDScript reference bit for bit. The C++ kernel from the spike
(#174) is retired.

| | |
|---|---|
| Switch | `sim/skirmish/formation/native_kernels.gd`: `BREACH_NATIVE=gdscript\|rust` (env), else project setting `breach/native/engine`, else `gdscript`. `BREACH_NATIVE_THREADS=1` lets body parting use Rayon. It picks only the kernels: the library is always loaded (`content/definitions/native_library.gd`), because the content is (below). |
| Rust | `native/rust/` - `godot` 0.5.5 (`api-4-7`), classes `BodyField` and `ContentStore` |
| Content | `native/rust/src/content/` (Decision 153, spec 39 step 2): the serde structs for units, weapons, armour and the tuning are the schema of `content/{units,weapons,armour}/*.json` and `content/tuning/battle_tuning.json`. A file that doesn't match fails naming its field. The JSON Schema under `content/schema/` is generated from them (`BLESS_SCHEMA=1 cargo test schema` rewrites it; a test fails while it is stale). GDScript's definitions are filled from what the crate loaded (`CrateContent`, through `ContentStore`) until their last reader ports |
| GDScript side | `body_field_sync.gd` (the sync), `body_parting.gd` (parting on the field), `scrum_seek_field.gd` (the slot search on the field), `scrum_face_field.gd` (the facing on the field), `scrum_walk_field.gd` (the walk on the field) |
| Owner | `FormationSimulation._field`: one field a battle, made when the battle is (null under GDScript). Its bodies are the slots of the battle's unit store (`FormationSimulation.store`, `UnitStoreNative`; spec 35) |
| Benchmarks | `tools/native_bench.gd` (parting and the slot search, split), `tools/facing_bench.gd` (the facing pass alone, and DetMath), `tools/steering_bench.gd` (the walk alone), `tools/tick_phases.gd` (a whole tick, phase by phase), `tools/formation_bench.gd` (whole ticks) |
| Identity | `tools/formation_digest.gd` (every set, both engines); `tests/sim/skirmish/formation/test_unit_store.gd`, `test_body_field.gd`, `test_body_parting.gd`, `test_face_field.gd` and `test_walk_field.gd`; the whole suite under `BREACH_NATIVE=rust` |

## The native state and its boundary

```
GDScript (rules, reference)                    Rust: BodyField (one a battle)
-------------------------------------------    ---------------------------------------------
FormationSimulation._field ------------------> roster (persistent, incremental)
UnitStoreNative: hold, release, slot_of,         unit id -> body slot (the store's slot);
  unit_at, held_count, held_units ---------->    squad id -> squad;
UnitColumnsNative: column/commit, value/put ->   the store's columns by slot: position,
  (one copy a column, one call a value)            loose place, flee offset, flag bits;
BodyFieldSync.scrum / .bodies  --sync------->     per body: radius, area, initiative, draw,
  one call, flat over squads in list order:       faction, squad; bodies in draw order
  ids, mode (scrum/bodies) [, bases]             where each stands (as of the last sync,
                                                 read from the columns by the flag bits
                                                 there; next and bearing by the mode):
  <- flat indices of units new to the field        at, offset/base (fleeing), next, flags,
  enrol(radius, area, draw, initiative,         bearing; squads' members in list order;
    record, paces) -------------------------->     record: preferred band, footprint,
                                                   turn rate, backward pace
                                               passes on the field
BodyParting.step --------- part(tuning, way) -> parting (spike kernel, draw order)
  <- moved bodies (flat), points, nexts, loosened in order, stats
ScrumSeekField.plan ------ seek(rules' say) ---> seek (seeking.rs, scrum.rs): who may
  per fighting squad: its foes' squad ids,         seek and its place (SquadFrame.place_in
  may it seek, its frame, its chasers;             from the store's rank and column), its
  per unit: speed                                  goal from the columns; touch, contest
                                                   order, kept slots, rings, picks, presses;
                                                   the outcome written into the columns
  <- planned units (flat), codes, nexts, foe_ats, active squads (for the walk and facing)
  within(unit, point) <- asked only on terrain
ScrumFaceField.face ------ face(after the walk) -> facing (face_store.rs, facing.rs): where
  the seek's units, their squads' foes,            each stands, faces and makes for (next,
  their speeds                                     foe_at, toward) from the columns, its
                                                   turn rate and backward pace from its
                                                   record; the touching foe (front, distance,
                                                   draw), UnitShuffle.look, bearing_to, the
                                                   turn at its rate (motion.rs, DetMath's
                                                   atan2 and acos); the bearing written into
                                                   the column
  <- every unit's bearing after its turn (tests)
ScrumWalkField.walk ------ walk(after seek) ----> walk (walk_store.rs, walking.rs): each
  the seek's units and answer (codes, nexts),      unit's bearing, rank and column from the
  speeds, a frame and pace per squad, the lying    columns, turn rate, backward pace and
                                                   footprint from its record;
                                                   SquadFrame.place, UnitShuffle.look,
                                                   UnitSteer (the body in the way, least by
                                                   [along, draw]; _round), UnitMotion.move,
                                                   GroundBodies.underfoot over a grid of the
                                                   lying; where each ends and looks written
                                                   into the columns
  <- where each stands after its step, its look (tests)
```

**What the field owns.** Everything about a body that doesn't change tick to tick:

- its unit id, radius, footprint area and initiative
- its seeded draw, `ScrumContest.draw`, which used to be hashed and sorted every tick
- its faction and squad

It also keeps the bodies in draw order and each squad's members in list order. The
roster changes incrementally. A sync names each squad's living units:

- a unit new to the field is enrolled once, with GDScript's own values, so no rule is
  copied to Rust
- a unit no squad names is dropped (the dead, the taken)
- a unit named under another squad moves with it

**The unit store's slots** (spec 35 step 0). A body is also the slot the battle's unit
store gives a unit (`hold`), from when it is spawned until it is in no squad's list
(`release`). A held unit dropped from the field (dead, downed) keeps its slot and is
enrolled again in it if a sync names it again. Free slots are reused lowest first. A
slot is storage only: no pass loops in slot order or breaks a tie by it.

A battle's field lives as long as its simulation.

**Where units stand lives here** (spec 35 step 1c). The unit store's columns - each
unit's position, its loose place, its fleeing offset and its flag bits - are the field's
`Columns`, by slot. GDScript reads and writes them through `UnitColumnsNative` (a whole
column is one copy; one value is one call), and a sync no longer sends points: the field
reads where each unit stands from its columns, by the flags the sync names (as the
unit's own squad's entries say). A native pass runs on the field its squads' store lives
on (`UnitStore.field`); a squad made alone, as tests make one, gets a store on a field of
its own under Rust. The passes still write their results back through GDScript, which
puts them into the columns; the passes reading and writing the columns themselves is
spec 35 step 4.

**The squads' state lives here too** (spec 39 step 1). Each squad's scalar state - its
state, order, front distance, heading, position, morale and the rest - is the crate's
`Squads` (`squads.rs`), columns by a squad slot kept for the battle, with its members as
unit-store slots. GDScript reads and writes them through `SquadColumnsNative`, one call a
value; `SkirmishSquad` (a `StoredSquad`) keeps its fields as properties over them, and each
pass reads a squad's fields once, not unit by unit (Decision 144). The squads' entries and
lists that hold units (loose, fleeing, chasers and the rest) stay GDScript until their pass
ports.

**The state writes and reads back** (`snapshot.rs`): the tick, the battle seed, the field
and the squads, through serde in bincode, bit for bit and in a fixed byte order.
`BodyField.snapshot(tick, seed)` gives the bytes and `restore(bytes)` reads them back;
`tools/formation_digest.gd snapshot=<dir> at=<ticks>` writes them during its battles (Rust
engine only). The crate's tests build their state with `build.rs` and never start Godot.

**What GDScript still decides and hands in:** the rules.

- who may seek (front band, footprint, orders, chasers)
- a unit's place (ScrumStance.anchor)
- which squads a squad fights
- speed, which stamina changes every tick
- the ground: `terrain.factor`, called back only where there is terrain

**What comes back:** only what changed - the bodies parting moved, and each seeker's
goal, next point and foe point - written into the same dictionaries the GDScript path
writes, in the same order.

**The next ports share the field.** The spatial grid (BodyGrid, ScrumNear) is already
`grid.rs` over the field's points. Pathfinding and steering (UnitSteer) would read
`at`, `radius` and `squad` the same way. Each port that writes positions should write
them into the field first and then retire a GDScript writer, until the per-pass sync is
gone.

## Identity

The rules for matching Godot bit for bit (`native/rust/src/maths.rs`):

- `Vector2` maths is f32 (`real_t`) and GDScript `float` is f64; each operation is done
  in its own precision. `distance_to`, `length` and `dot` are f32 and widened when they
  meet an f64; `Vector2 * float` narrows the float first.
- `snappedf` is `floor(v / step + 0.5) * step`.
- `UnitMotion.vector` is `bearing * (PI / 180)` through `DetMath.sin` and `DetMath.cos`;
  `bearing_to`, `pace` and UnitShuffle's turning go through `DetMath.atan2` and
  `DetMath.acos` (`det_math.rs` has sin, cos, asin, acos and atan2, the same operations
  in the same order; `cargo test` checks them against bits printed by Godot, and
  `test_face_field.gd` against the GDScript on 20,000 inputs each). `motion.rs` has
  Godot's `fposmod`, `clampf`, `maxf` and `rad_to_deg` as the engine computes them. sim/ calls no platform transcendental function: the C
  library's differ between glibc, the Windows CRT and Apple's libm, so a battle wouldn't
  replay across machines. DetMath and DetPow use only IEEE 754's exact operations;
  `test_det_math.gd` pins their bits and `tools/platform_probe.gd` prints them.
- No fused multiply-add. Rust never contracts, and the library targets baseline x86-64.
- The same order everywhere: pairs, pushes, contest order, slot keys compared item by
  item, a full tie going to the slot listed first, foes in list order.
- The seeded draws reproduce Godot's `hash()` exactly (`ghash.rs`: MurmurHash3 over the
  elements' own hashes, `hash_one_uint64` for ints, djb2 for Strings, `Variant::ARRAY` as
  the seed). `test_body_field.gd` proves it against the engine's `hash()` on 10,000 keys
  of every shape the battle hashes. A slot's mirror-image draw is only computed when a
  tie needs it, which doesn't change the order of comparisons. The rare "two bodies on
  one spot" way still calls back `UnitBodies.part_way`.

### Fingerprints (`tools/formation_digest.gd`, every tick hashed)

The per-tick files are byte-identical between `BREACH_NATIVE=gdscript` and `rust`
(sha256 of each per-tick file):

| set | runs | ticks hashed | per-tick file (GDScript = Rust) |
|---|---:|---:|---|
| standard | 26 | 14,082 | `14878d70c9800c31bc37ab2173343b8900758f933e0f4a0a5c4b590c28d8663a` |
| clash160 | 1 | 210 | `846075a85ef4f9b1ebd6eb8f75205daab8f6dc03cf29382131e4b283d65b58d0` (run `85b79694...`) |
| wide | 2 | 553 | `0cb41776ecb08e35cccef399d15aa6a3285f9615da043aec9a28cd7a90fd27a9` (runs `a65b5863...`, `af991eff...`) |

These are with DetMath (sim/ calls no platform transcendental function), so every
platform gives them (checked on Windows). The standard and wide sets' hashes are after the
field-test fixes: units squeezed at a gap file in behind, bodies of one frame part,
chasing, regrouping and fighting units pay the ground, and a retreat whose units are
home holds there. clash160 has no ground and no such crowding, so it is unchanged. Before
those fixes they were `63ae199a...` (standard) and `e2752a95...` (wide). Before that, the
standard set's hash was `4caaed38...` with DetMath alone; it changed when groups found
their way round barred ground (#177), which changed its terrain battles. Before DetMath they were `bf2111e4...` (standard),
`a0b69851...` (clash160) and `62361719...` (wide), the same as GDScript at
`origin/feature/movement-integrate` (f4a6490) before the Rust core, and they differed on
Windows.

Each set gives the same file under the release and the dev build, with and without
`BREACH_NATIVE_THREADS=1`.

The suite proves the same:

- `test_body_parting.gd`: a crowd parted the same as GDScript, reversed, on a field kept
  across ticks while units die and change squad, and a 40-a-side battle.
- `test_body_field.gd`: the hash, the roster, and the slot search planning a mid-fight
  scrum the same as GDScript at four moments and whatever the list order.
- `test_face_field.gd`: DetMath's bits in Rust, and a mid-fight scrum planned, walked and
  turned to the same bearings as GDScript at four moments, with every unit given a
  bearing of its own and its looks moved, and whatever the list order.
- `test_walk_field.gd`: a mid-fight scrum planned and walked to the same points, looks and
  next points as GDScript at four moments; with ways pushed through the crowd, bodies laid
  underfoot (some heavy enough to stop a grem) and units held back to walk to their
  places; and whatever the list order.
- `test_formation_pour.gd`, `test_terrain_toll.gd` and `test_field_test_record.gd`: bodies
  of one frame parted, a fight in a wood walked (each step paid for the ground in GDScript
  after the core's walk), and the user's field-test record replayed, the same as GDScript.
- The whole suite under `BREACH_NATIVE=rust`, including its reversed-list tests:
  1015 of 1015 pass, as they do on GDScript.

## Results

Linux x86_64, Godot 4.7.2 official, release build, 4 cores shared with another job, so
whole ticks carry a few % of noise. FormationBench's clash (grems v militia, seed 1),
mid-fight: 10 ticks after 10 ticks of contact. Figures are ms a tick, the median of 3
runs.

### The two ported passes (`tools/native_bench.gd`)

| a side (wide) | engine | whole tick | body parting | slot search (ScrumSeek.plan) |
|---|---|---:|---:|---:|
| 160 (10) | GDScript | 127.3 | 28.9 | 65.0 |
| | Rust | **28.2** | **1.39** | **3.27** |
| 512 (32) | GDScript | 565.4 | 101.0 | 293.5 |
| | Rust | **162.7** | **5.67** | **14.3** |
| 1024 (32) | GDScript | 899.6 | 168.2 | 424.0 |
| | Rust | **280.2** | **9.10** | **23.8** |

Where the Rust passes' time goes, at 1,024 a side:

| pass | sync (GDScript) | the rules' say (GDScript) | Rust's own work | crossing the boundary | write back (GDScript) |
|---|---:|---:|---:|---:|---:|
| body parting (9.1) | 4.47 | - | 2.98 | 0.03 | 1.46 |
| slot search (23.8) | 3.39 | 9.18 | 7.73 | 0.06 | 3.23 |

- **Body parting: 168 -> 9.1 ms.** The spike's pass was 18 ms. The per-tick draws, the sort
  and the radius, area and index lookups are gone. Rust's work is 3 ms. Most of the rest
  is the position sync, which goes once the field owns positions (see the boundary).
- **The slot search: 424 -> 24 ms (18x).** Rust's work is 7.7 ms. GDScript now spends
  about 16 ms around it: the sync, the rules' say (anchors, chasers, foe ids), and
  writing back.
- **Rayon** (`BREACH_NATIVE_THREADS=1`, parting only) is within noise or slower at every
  size: 3 ms isn't worth a thread pool.

### The facing pass (`tools/facing_bench.gd`, 1,024 a side, 32 wide)

DetMath (#186) made the early fight's "scrum: face foes" phase 33 -> 49 ms: about 0.6-0.8
us a call in GDScript, and the pass makes about 3,900 atan2, 3,700 acos and 2,900 sin+cos
calls. Timed alone, 20 repetitions on one snapshot (tick 100), the bearings restored
between them; ms a pass, median:

| engine | before (GDScript facing) | after |
|---|---:|---:|
| GDScript | 42.4 | 44.8 (the same code; noise) |
| Rust | 44.9 | **4.5** (gather 3.7, call 0.4 of which the core 0.4, write back 0.4) |

The GDScript pass split: foe grids 5.6, the touching foe 18, looks and bearings 22 (DetMath
about 10 of it), turns 1.9. DetMath alone: about 650-860 ns a call in GDScript, 13-20 ns
in Rust (one call over 100,000 inputs, the boundary included). What remains under Rust is
GDScript reading each unit's loose entry (3.7 ms): it goes when the field owns positions.

`tools/tick_phases.gd engine=rust side=1024 stage=fight fight=5`: the whole tick 204-220
-> 148-162 ms, "scrum: face foes" 45-51 -> 5 ms (below the 181 ms of before DetMath).

### The walk (`tools/steering_bench.gd`, 1,024 a side, 32 wide)

FormationScrum._walk alone, on one snapshot after the tick's slot search, the entries'
at, next and toward restored between repetitions; ms a walk, median. Tick 100 (10 ticks
into the fight: 1,982 walkers, 958 to slots, nothing lying) and tick 130 (40 ticks in:
1,017 walking back to their places, 36 bodies lying):

| moment | engine | before | after |
|---|---|---:|---:|
| tick 100 | GDScript | 31.5-37.0 | 30.8 (the same code) |
| | Rust | 31.4 (GDScript walk) | **4.0** (gather 2.0, call 0.3 of which the core 0.27, write back 1.7) |
| tick 130 | GDScript | 36.4 | 35.7 |
| | Rust | 35.5 | **2.3** (gather 1.2, core 0.24, write back 0.75) |

The GDScript walk split, tick 100 / tick 130: places 2.3 / 1.6, looks 2.3 / 7.0, steering
(UnitSteer.toward) 23 / 8.3, footing (GroundBodies.underfoot, a loop over every lying
body) 3.2 / 13.7, steps 6.0 / 1.3. In Rust the lying bodies are found through a grid
(a least over them, so the order can't matter). The walk's batch also hands the facing
where each unit now stands, its bearing and turning and what it looks at, so the facing
no longer reads the entries again (its gather, 3.5 ms, goes).

`tools/tick_phases.gd engine=rust side=1024 stage=fight fight=5`: the whole tick 152-160
-> 119-128 ms; "scrum: walk" 32-32.3 -> 4.4-5.2 ms; "scrum: face foes" 5.2-5.6 -> 0.8-1.0
ms. Later in the fight (`settle=40 fight=3`): the whole tick 126 -> 112 ms, "scrum: walk"
33.0 -> 2.6 ms, "scrum: face foes" 1.9 -> 0.2 ms. What remains is GDScript reading each
unit's turning and frame and writing its entry back: it goes when the field owns
positions.

### The melee blows (`tools/blows_bench.gd`, 1,024 a side, 32 wide)

The fight's melee part (FormationMelee.blows, BlowLanding.land) runs its search on the
field: `ScrumBlowsField` syncs the scrum as the fight begins (clearing each unit's target
as it goes) and `BodyField.blows` (`blows.rs`) gives each striking unit its foe -
ScrumBlows._pick's least [front, distance, draw], the same `facing::touching` the facing
uses - with whether its blow falls on the foe's flank and whether a retreating unit has
turned away. Cooldowns, the morale's interval, the blows, rolls, damage, hit points,
events, surrender and stamina stay GDScript's, taken in ScrumBlows' order. For rolled
blows, `BodyField.pressed` counts the foes pressing each target (BlowLanding._pressed) on
the same sync. RoutBlows and the ranged shots stay GDScript. A tick no squad can strike
in (marching, or only a rout left) makes no sync. `test_blows_field.gd` checks the blows,
their landing and every target and cooldown against GDScript at four moments, with
bearings and cooldowns stirred, a squad retreating, wavering or striking only a retreat,
and whatever the list order.

Ten consecutive ticks from tick 101 (one attack interval), each timed 10 times from the
same state; ms a tick, the mean of the ticks' medians:

| blows | GDScript | Rust before | Rust after |
|---|---:|---:|---:|
| unrolled (as the bench's clash) | 22.5-24.8 | 21.7 | **2.7-2.8** (gather and sync 2.3, the core 0.14, write back 0.09) |
| rolled | 25.8-26.1 | (as GDScript) | **3.6-3.8** |

The GDScript split (unrolled): clearing targets 1.1, foe lists 1.3, foe grids 4.2, the
grid search 9.2, the pick 9.9, cooldowns and blows 1.3; rolled, the landing adds 2.9
(who's whose 1.5, pressed 1.3) and 0.25 for the blows themselves. `tick_phases.gd
engine=rust side=1024`: "fight (blows)" 25-29 -> 6.0-6.1 ms mid-fight (the rest: the
shots' search 2.8, the gather 2.3), 5.1 -> 3.1 marching, and 8.4-8.7 -> 8.5-8.8 at
settle=40 (a rout: RoutBlows 3.5, of it 2 ms placing the routers on their routes).

### Where a tick's time goes (`tools/tick_phases.gd`, 1,024 a side, 32 wide, before facing)

| phase | GDScript | Rust |
|---|---:|---:|
| deaths, wounds, morale (FormationDeaths, Taking, Wounds, Recovery, Morale) | 86.1 | 66.4 |
| scrum: face foes (FormationScrum._faces: ScrumNear, ScrumBlows.nearest_touching) | 37.5 | 37.0 |
| scrum: walk (UnitSteer.toward, GroundBodies.underfoot) | 34.8 | 29.8 |
| fight (ScrumBlows.blows, BlowLanding) | 33.5 | 26.9 |
| scrum: seek (the slot search) | 445.9 | 26.5 |
| shuffle, units to their places (FormationShuffle, FormationMarch.sync_units) | 23.3 | 15.6 |
| rout, carry, groups, stamina | 12.6 | 11.9 |
| bodies (UnitBodies) | 161.0 | 9.6 |
| scrum: the bodies snapshot (ScrumSlots.bodies, BodyGrid) | 7.6 | 7.0 |
| everything else | 10.0 | 9.5 |
| **whole tick** | **852** | **240** |

`tools/formation_bench.gd` mid-fight, average and worst, GDScript -> Rust:

- 160 a side: 124 -> 48 ms
- 512 a side: 481 -> 157 ms
- 1,024 a side: 938 -> 313 ms (worst 1,260 -> 487)

**Marching is unchanged**: 318-354 ms at 1,024 a side, before any contact. Neither
ported pass is what a marching tick spends.

Reading it: at 1,024 a side the tick is about 3.5x faster, but it is still 2.4x the
100 ms budget. What is left is GDScript rule code over every unit. The worst of it:

- the deaths/wounds/morale phase (66 ms, the largest now)
- the scrum's facing and walking (67 ms together: foe searches and steering per unit)
- blows (27 ms)
- the per-tick syncs and write-backs around the two ports (about 20 ms)
- marching

The next ports, in order of payoff:

1. Positions owned by the field, which removes the syncs.
2. ScrumNear and steering (facing is done).
3. Blows.
4. A profile of the deaths/wounds phase, which may be a GDScript fix rather than a port.

## Building

```
bash native/build.sh          # release (thin LTO): what to measure and ship
bash native/build.sh dev      # optimised without LTO: rebuilds in seconds
```

This puts `native/bin/[lib]breach_rust.<platform>.<ext>` in place, and the map designer's
tool, `native/bin/breach_designer[.exe]` (`src/bin/designer.rs`, spec 39 step 2c), which
`tools/designer/serve.py` runs to save maps and the terrain library. They are git-ignored, as
is `native/rust/target`. Results are the same bit for bit under either profile.
`cargo test` in `native/rust` runs the crate's own tests (the hash against values printed
by Godot, the squad store, the snapshot read back), with no Godot. CI's `rust` job runs it
with `cargo fmt --check` and `cargo clippy --all-targets -- -D warnings` on every PR that
touches `native/`.

`native/` holds a `.gdignore`, so the editor never scans it. `breach_rust.gdextension`
is loaded at run time by NativeKernels only when Rust is chosen and built. An export
would need the library and the `.gdextension` added.

- **Windows:** `powershell -ExecutionPolicy Bypass -File native\build.ps1` (or
  `bash native/build.sh` in Git Bash). It needs Rust from rustup: either the default
  MSVC toolchain with the Visual Studio Build Tools' C++ workload, or the GNU one
  (`rustup default stable-gnu`), which needs no Visual Studio. The crate also
  cross-builds from Linux (`--target x86_64-pc-windows-gnu`, linker
  `x86_64-w64-mingw32-gcc`); that DLL has no fused multiply-adds and needs only system
  DLLs.
- **macOS:** the Xcode command-line tools. Build `aarch64-apple-darwin` and
  `x86_64-apple-darwin` and join them with `lipo` into the `.universal.dylib`. Sign it
  ad hoc (`codesign -s -`) so it loads on Apple silicon.
- **CI:** a matrix (ubuntu, windows, macos) running `bash native/build.sh`, caching
  `~/.cargo` and `native/rust/target`, and uploading `native/bin/*`. The gdUnit job then
  runs the suite twice, plain and with `BREACH_NATIVE=rust`, plus
  `tools/formation_digest.gd` under each, and diffs the files. The identity check is
  what lets the core be trusted.

## Running the benchmarks locally

`tools/run_benches.ps1` runs everything above on the machine it is on and writes one
report to `reports/bench/` (git-ignored), to compare machines:

1. **Identity:** the per-tick digests of every set under each engine, against the
   reference build's hashes. The same battles must replay bit for bit on every machine,
   OS and engine. A `DIFFERS` line is a finding, not noise.
2. **A tick, phase by phase** (`tools/tick_phases.gd`), mid-fight and marching, under
   each engine.
3. **The native core's passes** (`tools/native_bench.gd`).

On Windows, with Godot 4.7.2's console build (`Godot_v4.7.2-stable_win64_console.exe`),
which passes Godot's output through to the script:

```
powershell -ExecutionPolicy Bypass -File native\build.ps1
powershell -ExecutionPolicy Bypass -File tools\run_benches.ps1 -Godot C:\path\to\Godot_v4.7.2-stable_win64_console.exe
```

Options: `-Side 1024` (the phase timings' size), `-SkipDigests`, `-SkipGdscript` (GDScript
at 1,024 a side takes about a second a tick here). Elsewhere the same script runs under
PowerShell 7 (`pwsh tools/run_benches.ps1 -Godot <godot>`), after `native/build.sh`.

The reference box: an Intel Xeon at 2.1 GHz with 4 cores and no hyperthreading, often
shared with other jobs. Its own report is the baseline to compare against.

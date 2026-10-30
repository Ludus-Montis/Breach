# Breach's native core (Rust)

Decision 129: the simulation's hot core moves to Rust, through GDExtension
(godot-rust/gdext). It owns its state, and GDScript keeps the rules and stays the
reference behind one switch. The first production piece is in place:

- **`BodyField`**: the bodies' state, owned natively and kept across ticks.
- **Body parting** (UnitBodies) runs on the field.
- **The scrum's slot search** (ScrumSeek; once SlotSearch, ScrumSlots and ScrumNear) runs
  on the field.
- **The scrum's facing** (FormationScrum._faces and the turns: ScrumBlows.nearest_touching,
  UnitShuffle.look, UnitMotion's bearing_to, pace and turn) runs on the field, after the
  walk, for the units the slot search was given.
- **The scrum's walk** (FormationScrum._walk: UnitSteer.toward and _round, SquadFrame.place,
  UnitShuffle.look, UnitMotion.move, GroundBodies.underfoot) runs on the field, between
  the slot search and the facing, for the same units.

All three reproduced the GDScript reference bit for bit, and since spec 39 step 4a they run
under either engine: every battle has a field, and the scrum's twins (and `UnitStoreGd`)
are gone. Body parting was the last pass with a GDScript twin; spec 39 step 6a retired it,
and the engine switch with it. The C++ kernel from the spike (#174) is retired.

| | |
|---|---|
| Switch | None since spec 39 step 6a: every battle runs on the core, whose library is always loaded (`content/definitions/native_library.gd`), because the content is (below). `sim/skirmish/formation/native_kernels.gd` makes a battle's field and store; `BREACH_NATIVE_THREADS=1` lets body parting use Rayon. `BREACH_NATIVE` is ignored (CI still sets it for a base from before 6a). |
| Rust | `native/rust/` - `godot` 0.5.5 (`api-4-7`), classes `BodyField`, `ContentStore`, `TerrainGround`, `UnitMoves`, `GroundPaths`, `GroundMarch`, `ScrumRules` and `SquadRoster` |
| Ground | `native/rust/src/ground/` (spec 39 step 3a): the ground a battle is on, its cells and rules (how fast a walker crosses them, its mode, the high ground) and its obscurance (`sight.rs`), and a map's surface with its relief and channels (`surface.rs`). `TerrainGround` holds it; `FormationTerrain` and `TerrainObscurance` are views over it, under both engines, and their GDScript is gone. Its scenarios are cargo tests (`ground/tests.rs`) |
| Moves | `native/rust/src/route.rs`, `motion.rs`, `mover.rs`, `steer.rs` (spec 39 step 3b): routes and their learned detours, a unit's turning and pace by facing, the one mover and the ground's toll on a step (Decision 139), and steering round bodies (shared with the scrum's walk). `UnitMoves` binds them, stateless and shared (`CrateMoves`); `FormationRoute`, `RoutePatch`, `UnitMotion`, `UnitMover` and `TerrainStep` are views over it, under both engines (steering is the crate's phases' own since step 4c). Their scenarios are cargo tests (`route/`, `mover/`, `steer/`, `motion.rs`) |
| Content | `native/rust/src/content/` (Decision 153, spec 39 step 2): the serde structs for units, weapons, armour and the tuning are the schema of `content/{units,weapons,armour}/*.json` and `content/tuning/battle_tuning.json`. A file that doesn't match fails naming its field. The JSON Schema under `content/schema/` is generated from them (`BLESS_SCHEMA=1 cargo test schema` rewrites it; a test fails while it is stale). GDScript's definitions are filled from what the crate loaded (`CrateContent`, through `ContentStore`) until their last reader ports |
| GDScript side | `body_field_sync.gd` (the sync), `body_parting.gd` (parting on the field), `scrum_seek_field.gd` (the slot search on the field), `scrum_face_field.gd` (the facing on the field), `scrum_walk_field.gd` (the walk on the field) |
| Owner | `FormationSimulation._field`: one field a battle, made when the battle is, under either engine. Its bodies are the slots of the battle's unit store (`FormationSimulation.store`, `UnitStoreNative`; spec 35) |
| Paths | `native/rust/src/paths/`, `pathing.rs` (spec 39 step 3c): the A* search over the ground (`search.rs`, its ties to geometry in `frontier.rs`), each kind of walker's speed shares (`shares.rs`, held by `TerrainGround` and dropped when it is painted), a pathfinder's sight and memory, climbing on stamina (`climb.rs`), plans, rejoining a route and a way's cost (`mod.rs`), and a marching group's look ahead, its walker and its pathfinder (`pathing.rs`). `GroundPaths` binds them, stateless and shared (`CratePaths`); `TerrainPaths`, `PathSearch`, `PathSight`, `SquadPathfinder`, `ClimbStamina.standable` and `ModeStamina.step_cost` are views over it, and `FormationPathing` keeps only which groups look and their commands' patches. Their scenarios are cargo tests (`paths/tests.rs`, `pathing.rs`) |
| March | `native/rust/src/march/`, `lying.rs`, `det_pow.rs` (spec 39 step 3d): a squad's march along its route (where its front gets, which way its order takes it, the pace of the ground under its front rank; `mod.rs`), sweeping round bends (`sweep.rs`), its units walking to their places (`walk.rs`) with their climbing, swimming and falls (`modes.rs`), pouring through a gap (`pour.rs`), the frame's slack and wait, and the moves a re-form starts (`shuffle.rs`, `front.rs`), over the bodies lying underfoot (`lying.rs`, shared with the scrum's walk) and DetPow's `pow` (`det_pow.rs`). `GroundMarch` binds them, stateless and shared (`CrateMarch`); `FormationMarch`, `FormationSweep`, `FormationWalk`, `FormationPour`, `FormationShuffle` and `PlaceOrder` are views over it, `WalkModes`, `ShuffleFront` and `SquadPlaces` are gone, and the swaps under way, arrivals and the routers' and the loose's placing stay GDScript. Since step 8b5-xxxiii the units' placing and walking is one call over every squad, `march_sync` (`march/sync.rs`): each living unit's distance along the route, a router on its route, a loose one where it stood, the rest placed or walked a tick to where it makes for, a fall's hurt onto the fell queue. Its scenarios are cargo tests (`march/tests.rs`, `march/sync/tests.rs`) |
| Scrum | `native/rust/src/scrum.rs`, `seeking.rs`, `walking.rs`, `facing.rs`, `blows.rs`, `contest.rs`, `reach.rs` (spec 39 step 4a): the seek, walk, facing and blows' search on the field, each a phase (`seeking::seek`, `walk_store::walk`, `face_store::face`) that `BodyField` calls, under either engine; ScrumContest's draws and rolls and ScrumReach's bodies, gaps and fronts, which `ScrumRules` binds, stateless and shared (`CrateScrum`), and ScrumContest and ScrumReach are views over. The builders `build::scrum` and `build::reversed` make a scrum for the scenarios, which are cargo tests (`scrum/tests.rs`, `walking/tests.rs`, `facing/tests.rs`, `blows.rs`, `contest.rs`, `reach.rs`) |
| Squad scrum | `native/rust/src/squads.rs`, `entries.rs`, `locks.rs`, `engage.rs`, `stance.rs`, `manoeuvre.rs`, `ranks.rs`, `trade.rs`, `frame.rs`, `rect.rs` (spec 39 step 4b): each squad's records in the squad store (its loose and fleeing units, its stance, the foes on its edges, its chasers), which `SquadRecords` and SkirmishSquad's dictionaries are views of; the entries and their keys, the locks and who may be engaged, engaging, the stance, the manoeuvre, the stall, closing ranks and the trade, each a phase `BodyField` calls. `SquadEntries`, `EntryKeys`' writes, `FormationLocks`, `ScrumEngage`, `ScrumStance.anticipate_all`, `FormationManoeuvre.step` and `SquadRanks` are views over them (the trade is regroup's since step 4c); a pass holding the columns lends them to each call (`ColumnsLent`). Their scenarios are cargo tests (`entries.rs`, `locks.rs`, `engage.rs`, `stance.rs`, `manoeuvre.rs`, `ranks.rs`, `trade/tests.rs`) |
| Regroup onwards | `native/rust/src/roster.rs`, `near.rs`, `regroup.rs`, `pursuit.rs`, `withdraw.rs`, `geometry.rs`, `contact.rs`, `perception.rs` (spec 39 step 4c): the squads a phase is given (`SquadRoster`: each squad's living units and what of them the crate doesn't hold, filled by `CrateRoster` once a tick), the bodies and foes near a unit, regrouping, the chasers and breaking ranks, a pursuing formation's lag and spread, the withdrawal's flight, the squads' geometry (frames, gaps, overlaps, edges, bounds), the march's limits, skirmishing, joining, the front and flank locks, and sight and the tick's perception. `ScrumRegroup`, `ScrumPursuit`, `FormationWithdraw`, `FormationContact`, `FormationFronts`, `FormationEdges`, `FormationSight`, `FormationPerception` and `SquadEdges` are views over them; each squad's pursuit and withdrawal records, the orders' re-forms and the events stayed GDScript until step 7c. Since step 8b5-xiv the regroup and the chasers are one call, `scrum_regroup` (`scrum/regroup.rs`), each squad's say (out of the fight, drilled, withdrawing, its re-form pace, where its units walk back to on the ground) read in the crate. Since step 8b5-xv the withdrawal is one call, `scrum_withdraw` (`scrum/withdraw.rs`), each squad's disorder, re-form pace, home and its units' flight angles read in the crate, and its re-form's turn (`turn::landed`), "regrouping" and "returned" in the crate's log. Since step 8b5-xvi the pursuit as a whole is one call, `scrum_pursue` (`scrum/pursue.rs`), each pursuer's routes (the one it held its post on is a squad column, `post_route`, kept by the orders' pass) and leash read in the crate, its return's route and turn and "pursuit_ended" landed there. Their scenarios are cargo tests (`regroup/`, `pursuit/`, `withdraw/`, `scrum/withdraw/`, `scrum/pursue/`, `geometry/`, `contact/`, `perception/`, `near.rs`, `rect.rs`) |
| Groups | `native/rust/src/split.rs`, `groups.rs` (spec 39 step 5a): which of a group's units fallen behind split off, in what order and where each new group's front is, and which groups at ease meet (the pairs near enough, the gap between their bodies) and what each meeting does - a group taken in, or a led group's units going back to the command they started under - decided in turn on the groups as the ones before left them. Since step 8b5-xxx both are one pass (`groups/pass.rs`, binding `groups_step`) that also lands them: each new group's squad opened with the battle's next id, groups taken in, units sent back, the events in the log; `FormationGroups` wraps the slots opened and mirrors its lists (`GroupSplit` and `GroupPairs` are gone). Their scenarios are cargo tests (`split/tests.rs`, `groups/tests.rs`, `groups/pass/tests.rs`) |
| Closing up, strays and staging | `native/rust/src/compact.rs`, `strays.rs`, `rendezvous.rs`, `staging.rs` (spec 39 step 5b): a squad closing up as units fall and taking a joining squad in behind its back rank, a unit on its own setting off home and strays numbered by their draws, the predicted march a planned rendezvous waits by, and a staged wave's trigger and its coordinated leader's timing. `SquadCompact`, `FormationContact.reinforce`, `FormationStrays`, `FormationRendezvous` and `FormationStaging` are views over them (a joining squad's units enlisted by `joins.rs`, spec 39 step 8b5-x); the stray's squad, holding at the staging point, its fallback and the events stay GDScript. Their scenarios are cargo tests (`compact/`, `strays/`, `rendezvous/`, `staging/`) |
| Benchmarks | `tools/native_bench.gd` (parting and the slot search, split), `tools/tick_phases.gd` (a whole tick, phase by phase), `tools/formation_bench.gd` (whole ticks). The facing, walk and blows benches retired with their twins (spec 39 step 4a); their results below are history |
| Stamina | `native/rust/src/stamina.rs` (spec 39 step 6a): gaits, runs building up and a pursuer reacting, running, marching laden and blows tiring a unit, its breather and recovery, its pace by how tired it is, and a formation too tired to keep up a chase. `FormationStamina.step` is one crate pass (`stamina/pass.rs`, spec 39 step 8b5-xxxi) that reads each squad's pursuit and hurry and each unit's numbers there; `ScrumRules.stamina_stage` gives the landing a unit's stage. Its scenarios are cargo tests (`stamina/tests.rs`, `stamina/pass/tests.rs`), and the parting's are `parting/tests.rs` and `parting/pass/tests.rs` |
| The downed | `deaths.rs`, `wounds.rs`, `recovery.rs` and `regeneration.rs` (spec 39 step 6b): which of the fallen die and which are downed with the wounds they take; the takers of the downed, guards, playing dead and being fooled, captors, messengers and the strikes through death's door, and a struck router's surrender; condition, its drain, regeneration, the time downed, playing dead, and those coming to joining the nearest formation in sight or setting off alone. The burial is one pass, `deaths/bury.rs` (spec 39 step 8b5-xxii): the fell queue taken, the fates, states and wounds, "died" and "downed", the fallen leaders and the losses' shock, the step-ups and the destroyed, their events in the log; `FormationDeaths` mirrors GDScript's copies of the units' states and wounds. The wounds' tending is one pass too, `wounds/pass.rs` (spec 39 step 8b5-xxiv): the living and the bodies lying read in the crate, the takings, the bodies captured, sent home (the messenger spent, the side shocked) or finished, their states and "captured", "sent_home" and "finished" in the log; `FormationWounds` mirrors the states and the messengers. Recovery is one pass, `recovery/pass.rs` (spec 39 step 8b5-xxv): the living and the downed read in the crate, condition, the drain and regeneration, the deaths of wounds, those playing dead, and those coming to joining a formation (out of their own squad, a place behind its rearmost, loose where they lie) or setting off alone, their states and "died_of_wounds", "playing_dead" and "came_to" in the log; `FormationRecovery` mirrors the states, the playing dead and the squads' lists. Their scenarios are cargo tests (`deaths/tests.rs`, `deaths/bury/tests.rs`, `wounds/tests.rs`, `wounds/pass/tests.rs`, `recovery/tests.rs`, `recovery/pass/tests.rs`) |
| Carrying and taking | `carry.rs` and `taking.rs` (spec 39 step 6c): which free unit of a formation tending its downed picks up which friend's body, no foe near it and its load not too much to move under, and which free unit of a group out of a fight walks out to which downed foe it sees, nearest first, each once. Since step 8b5-xxix the carrying is one call, `carry_step` (`carry/pass.rs`): the bodies following their bearers, dropped and coming home with those that fled home, the bearers' loads set down or taken up, the pick-up and "fled_home" and "borne" in the crate's log; `FormationCarry` mirrors the states, carrying and loads, and its unit lists. The taking is one pass, `taking/pass.rs` (spec 39 step 8b5-xxiii): the takers called back, the hold and where a group gave up, the squads that send, the sending and "sent_to_take" in the log; `FormationTaking` is the one call. Their scenarios are cargo tests (`carry/tests.rs`, `carry/pass/tests.rs`, `taking/tests.rs`, `taking/pass/tests.rs`) |
| Morale and the blows on routers | `morale.rs` and `rout_blows.rs` (spec 39 step 7a): each second, the pressure on a formation by the sides it fights on, less for a flank a friend covers, or out of contact its recovery by its units' condition and its leadership, up to its ceiling; and which router a hostile front-rank fighter strikes from behind, its target and cooldown. `FormationMorale` and `RoutBlows` are views over them that keep picking the fighters and the blows' damage. Their scenarios are cargo tests (`morale/tests.rs`, `rout_blows/tests.rs`) |
| The rout | `rout.rs`, `rout/flight.rs` and `rout/friends.rs` (spec 39 step 7b): who breaks at 0 morale, the break's release, flanks and fleeing entries, and the friends that see it; each router's flight along its route, fanned out or turned towards a refuge and paying the ground; the friend it crushes; the rallies to a led friend or a steady one that has held it long enough, nearest first; re-forming round its best leader with no enemy near; and reaching home. Since step 8b5-xxviii the rout is one call, `rout_step` (`rout/pass.rs`): the crushes' hurts and panic, the routers home leaving their squads, the rallies' joins, the re-forming, the breaks' staging, swaps and shocks, and their events in the crate's log; `FormationRout` mirrors its unit lists. Its scenarios are cargo tests (`rout/tests.rs`, `rout/friends/tests.rs`, `rout/pass/tests.rs`) |
| Orders | `orders.rs`, `pursuit/whole.rs` and `withdraw.rs`'s records (spec 39 step 7c): the orders given since the last tick, each applied to its squad if it stands, who is fighting read before any lands; a retreat out of a fight ending its locks and withdrawing from where its units stand, its enemies that may pursue locking on, keeping their posts and taking up the route their quarry flees by, and their units tempted to break ranks; a formation pursuing as a whole advancing after its quarry, giving up and marching back to its post, and taking up its post and order again; and a withdrawal's safe ticks and its re-forming where its units stand, or at home. Each squad's pursuit and withdrawal are records in the squad store, read by `OrderRecords`; `FormationOrders`, `FormationPursuit` and `FormationWithdraw` are views over them that keep the routes (a pursuer's post route is the crate's since step 8b5-xvi), the ragged retreat's shock (the crate's since step 8b5-v), the turns (ScrumTurn) and the events. Their scenarios are cargo tests (`orders/tests.rs`, `pursuit/whole/tests.rs`, `withdraw/tests.rs`) |
| Combat | `shots.rs` and `landing.rs` (spec 39 step 8a1): each ranged unit's pick of the nearest enemy unit within its range in ranks (one it overlaps across its columns first, then the seeded draws), its target and cooldown; and how each blow lands: its roll against the target's parry, dodge and defence, shifted by morale, high ground, the foes pressing its target, skill and tiredness, what it deals by its weapons' range, damage types, armour and ward, and the regeneration it stops and the wounds it marks (`ghash.rs` hashes the rolls' keys, bools included, as Godot does). `FormationCombat` and `BlowLanding` are views that hand in each unit's ranged time and arms and make the shots and the events. Their scenarios are cargo tests (`shots/tests.rs`, `landing/tests.rs`) |
| Turns and joins | `turn.rs` and `contact.rs`'s `joins` (spec 39 step 8a2-i): a squad about to move against the way it faces turning about (its places reversed, its front moved back by its depth) and facing its route where it stands, its units loosened to re-form; and the order waves that reached a friend's back rank join it, nearest first, then the seeded draws. `ScrumTurn` is a view over the turn that keeps the swaps under way. The joins themselves are `joins.rs` (spec 39 step 8b5-x): each join's moves, the members, joined list and command the joining units take, and its events ("merged" or "reinforced", "stepped_up") in the crate's log; `FormationJoins` mirrors GDScript's lists and drops the joined squads. Their scenarios are cargo tests (`turn/tests.rs`, `contact/tests.rs`, `joins/tests.rs`) |
| Staging and the bodies' drag | `staging.rs`'s `hold` and `lying.rs`'s `drag` (spec 39 step 8a2-ii): a staged wave at its staging point holding while its fallback lasts, then giving up (its trigger, `signalled`, was already the crate's); and the share of its pace a marching squad keeps for the bodies its front rank steps onto. `FormationStaging` and `GroundBodies` are views over them that keep the staging order, the events and which bodies lie where. Their scenarios are cargo tests (`staging/tests.rs`, `lying/tests.rs`); BodyGrid retired with GDScript's footing |
| The shuffle's swaps' ends | `swaps.rs`'s `end` (spec 39 step 8a2-iii): a re-forming squad's swaps under way cancelled when one of their units falls, counted down, and ended, its units in their new places, the squad widened to take in any outside its columns and closed up round the swaps still under way. Since step 8b5-xxxii the shuffle is one call over every squad, `shuffle_step` (`swaps/pass.rs`): the swaps' ends, the moves a re-forming squad starts, and "swapped" (its `passed` the log's first list field) and "stepped_up" in the crate's log; `FormationShuffle` takes the events. Its scenarios are cargo tests (`swaps/tests.rs`, `swaps/pass/tests.rs`) |
| Morale's ceiling, bands and shocks | `bands.rs` (spec 39 step 8a2-iv), on `ScrumRules`: a formation's ceiling (its living's mean courage, rounded, plus a step per point of its best leader's leadership, at most 100), its band by its morale, the slower strikes of a wavering one, a shock held to 0..100 and the shock of the tick's dead. `FormationMorale` is a view over them that keeps each squad's morale and makes the events. Their scenarios are cargo tests (`bands/tests.rs`) |
| The march's ends and who may shoot | `march::end` and `shots::shooters` (spec 39 step 8a2-v): an advance whose front reaches the enemy's end arriving, and a retreat whose front is home, or on a route all of whose units are home, holding there as it faces; and who may shoot, the living with a range of every squad standing and not arrived but a fighting squad's front rank, each starting again at its ranged time in ticks. `FormationMarch` and `FormationCombat` are views over them that keep the states, the orders and the events. Their scenarios are cargo tests (`march/tests.rs`, `shots/tests.rs`) |
| The move's choices | `moving.rs` (spec 39 step 8b1): which squads march as the move begins (not one fighting, routing, arrived or destroyed, nor one regrouping, its units out of their places but for its own chasers), which still wait to set out, whether one free to march holds (ordered to, or skirmishing), and every move landed together, each advance stopped at contact or behind a friend as they all stood at the tick's start. The move's pass (`moving/pass.rs`, spec 39 step 8b5-viii) runs each march between (staging, the ends, the hold, the turn, the sweep, the pace, the drag) and the landing, its events in the crate's log; Each squad's share of its pace is worked out there too (`moving/share.rs`, 8b5-ix), and the squads that advanced join their friends (`joins.rs`, 8b5-x); `FormationSimulation` calls `move_step`, takes the events (`CrateEvents`) and drops the joined squads. Its scenarios are cargo tests (`moving/tests.rs`, `moving/pass/tests.rs`) |
| The fight's hurts | `hurts.rs` (spec 39 step 8b2): each shot, then each blow, in the order landed, takes what it dealt from its target's hp; a unit taken from above 0 to 0 or below falls, onto the fell queue in that order. Each answers its target's hp after it, which the "hit" event reports. The fight's pass (`fight::step`, spec 39 step 8b5-xix) lands them and logs "spat" and "hit". Its scenarios are cargo tests (`hurts/tests.rs`) |
| The groups' ways round | `detours.rs` (spec 39 step 8b3): which groups re-route onto their command's route with its ways round (one it doesn't carry yet), standing where they stood along it, a group bound for the far end keeping it as home; and which look ahead (marching, on a route, its front moved on `path_relook` cells since it last looked). The commands keep the patches (`commands.rs`, step 8b4-iii) and the pass that calls these and the look is the crate's (`pathing/pass.rs`, step 8b5-vii). Its scenarios are cargo tests (`detours/tests.rs`) |
| The squads' routes | `squads.rs`'s `route` column (spec 39 step 8b4-i): the route each squad follows, written whenever SkirmishSquad.route is set (`squad_put_route`) and carried by the snapshot. Every binding that follows a squad's route reads it there (step 8b4-ii: the rout, the withdrawal, the split, a retreat's orders, the pursuit, and the march's face and sweep, `march::sweep::face_held` and `sweep_held`) rather than taking points. Since step 8b5-xxxvi each route is named by a code the crate holds (`route_code`, -1 none: the same route, the same code): the setter hands it the route's instance id, the crate gives each route it is handed a code, the passes that hand a squad another's route hand it the code too, and a re-route takes a fresh one; the staging, the orders, the opening and the groups read the codes there. Its tests are gdUnit's (`test_skirmish_squad.gd`) and cargo's (`march/tests.rs`, `opening/tests.rs`) |
| The commands | `commands.rs` (spec 39 step 8b4-iii): each command's guide route, home, standing options and learned ways round, in a table the squad store keeps (the snapshot carries it), and which command each squad follows. `FormationCommand` is a view of its slot. Groups follow their command's patched route from there (`detours::command_route`). Its scenarios are cargo tests (`commands/tests.rs`) |
| Staging's order | `staging.rs`'s `Stage` (spec 39 step 8b4-iv): each squad's hold-until order, a squad column the snapshot carries; `SkirmishSquad.staging` is a view of it, read and written whole, and `staging::hold_staged` holds by it, counting its fallback down. Its scenarios are cargo tests (`staging/tests.rs`) |
| The swaps under way | `swaps.rs`'s `Held` (spec 39 step 8b4-v): each squad's swaps under way, by unit id and slot, a squad column the snapshot carries. `swaps::end_held` ends them, `claims` and `begin` start them round those held, `offsets` and `moving` answer how far each unit is through its move and what compact closes up round. A unit's state is mirrored in the store's `state` column for who still stands. Its scenarios are cargo tests (`swaps/tests.rs`) |
| The taking | `squads.rs`'s `taking` column (spec 39 step 8b4-vi): each squad's units out taking the downed, [taker's id, its body's store slot] in the order they set out, carried by the snapshot; `SkirmishSquad.taking` is a view of it, read and written whole. Its test is gdUnit's (`test_formation_taking.gd`) |
| Spawning's ids and the order queue | `orders.rs`'s `queue` and `squads.rs`'s `issue_squad`, `issue_units` (spec 39 step 8b4-vii): the orders given for the next tick, in the order first given, which `apply_orders` takes, and the battle's squad and unit ids, counted by the crate; both carried by the snapshot. Their scenarios are cargo tests (`orders/tests.rs`, `squads.rs`) |
| The units' numbers | `sheet.rs` and `columns.rs`'s `sheet` column (spec 39 step 8b5-i): each unit's numbers (its hp at most, its strikes, skills, defence and armour, its footprint and band, its courage, its stamina at most and pace, its wounds and whom it carries, and the rest of SkirmishUnit's ints, floats and bools), by store slot, carried by the snapshot. `UnitStore.admit` writes them whole (`put_sheet`) and each of SkirmishUnit's setters writes its own after (`put_sheet_value`); the unit keeps a copy it reads, as no crate pass writes them yet. Its tests are cargo's (`sheet.rs`) and gdUnit's (`test_unit_numbers.gd`) |
| The units' words and lists | `words.rs`, `columns.rs`'s `words` column and the field's `lexicon` (spec 39 step 8b5-ii): each unit's faction and damage type, its tags, tactics, weaknesses, resistances, immunities and what stops its regeneration, its attributes and traits with their levels, and its weapons' parts, by store slot, each word coded once for the field in the order first seen; carried by the snapshot. `UnitStore.admit` writes them whole and each of SkirmishUnit's setters writes its own after (`put_word`, `put_word_list`, `put_word_levels`, `put_word_parts`, through `UnitWrites`), a trait set in place too (`set_trait`); the unit keeps a copy it reads. Its tests are cargo's (`words.rs`) and gdUnit's (`test_unit_words.gd`) |
| The units' objects | `objects.rs` and `columns.rs`'s `objects` column (spec 39 step 8b5-iii): each unit's walker (TerrainWalker's fields), the command it follows and the one it started under (their command slots) and its definition (its code in the unit store's own list, the order first seen, read back by `UnitStore.definition_of`), by store slot; carried by the snapshot. `UnitStore.admit` writes them whole and each of SkirmishUnit's setters writes its own after (`put_walker`, `put_object`, through `UnitWrites`); the unit keeps a copy it reads. Its tests are cargo's (`objects.rs`) and gdUnit's (`test_unit_objects.gd`) |
| The squads' remaining fields | `squads.rs`'s `faction` and `joined` columns, `living` and `downed` (spec 39 step 8b5-iv): each squad's faction, coded in the lexicon a unit's is so the two compare, and the units that joined it as reinforcements, their ids in the order they joined, both carried by the snapshot; its living and downed units read from its members and the unit store's STATE column. `SkirmishSquad.faction_id`'s and `joined`'s setters write them; the squad keeps a copy it reads. Its tests are cargo's (`squads.rs`) and gdUnit's (`test_squad_fields.gd`) |
| The events as data, and the orders' pass | `events.rs`, `orders/pass.rs`, `discipline.rs` and `shocks.rs` (spec 39 step 8b5-v): the events a crate pass reports, built by the crate in a log and taken flat (`events_take`): each its kind (`event_kinds`), the tick, its squad's id and faction code, then its kind's fields, decoded by `CrateEvents` into the dictionaries `FormationEvents` made. The orders' pass applies the queue and lands each order there: what a retreat reads (whether a squad may pursue, its units' chances to break ranks and leashes, from their sheets: FormationDiscipline's unit rules), a ragged retreat's shock (FormationMorale.shock) and the events; `FormationOrders` lands the routes its pursuers take. Its tests are cargo's (`events.rs`, `discipline.rs`, `shocks.rs`, `orders/tests.rs`) and gdUnit's (`test_formation_orders.gd`) |
| Engagement's events and shocks | `contact/pass.rs` (spec 39 step 8b5-vi): the front locks' "engaged" events, in the lists' order, and each flank lock's "flanked" event and its victim's shock (the rear's impact or the side's, `shocks.rs`), built in the crate's log; `FormationFronts` and `FormationEdges` take them (`CrateEvents`), and `BandTuning` hands the crate morale's tuning. Its tests are cargo's (`contact/pass.rs`) and gdUnit's (`test_formation_morale.gd`, `test_formation_engage_rule.gd`) |
| Pathing's pass | `pathing/pass.rs` (spec 39 step 8b5-vii): the groups' follows of their commands' patched routes, setting each one's route and place, then the marching groups' looks, each way round kept learned by the command and reported as its "detoured" event in the crate's log; a look reads each unit's walker, wits, place and definition's sight where they live (`members`, the field's `sights` written by `UnitWrites` as it codes a definition). `FormationPathing` hands it the squads (`path_step`) and takes the events (`CrateEvents`) and the re-routed groups' routes. Its tests are cargo's (`pathing/pass/tests.rs`) and gdUnit's (`test_formation_pathing.gd`) |
| The scrum's engaging, manoeuvres and stall | `scrum/pass.rs` (spec 39 step 8b5-xi): over the scrum's one roster, engaging (each squad's living units where they stand, loose or in place), each lock's "engaged" event in the crate's log; each squad's manoeuvre, its withdrawal, locks and skirmishing read in the crate; and the stall, each release's "disengaged" event in the log. `ScrumEngage`, `FormationManoeuvre.step` and `FormationScrum._stall` call them (`scrum_engage`, `scrum_manoeuvre`, `scrum_stall`) and take the events (`CrateEvents`). Its tests are cargo's (`scrum/pass/tests.rs`) and gdUnit's (`test_formation_scrum.gd`) |
| The scrum's preparing and stance | `scrum/prepare.rs` (spec 39 step 8b5-xii): over the scrum's one roster, each squad readied for the tick (a routing one letting its loose units, stance and fight go, the fallen unloosed, a fight's beginning marked and its end closing the ranks over the dead, a fighting squad's or a stance's units sent loose where they stand), and each squad meeting the threats it sees, its bounds, discipline and sight read in the crate, each new line's "faced" event in the log. `FormationScrum._prepare` and `ScrumStance.anticipate_all` call them (`scrum_prepare`, `scrum_stance`) and take the events (`CrateEvents`). Its tests are cargo's (`scrum/prepare/tests.rs`) and gdUnit's (`test_formation_scrum.gd`) |
| The scrum as one call | `scrum/step.rs` (spec 39 step 8b5-xxxiv): FormationScrum.step is one call, `scrum_step`, its nine phases in their order over one roster (engaging, readying, the stances, the manoeuvres, the seek, the regroup and chasers, the withdrawal, the pursuit, the stall), each phase's events in the log in that order; it returns the pursuers back at their posts, whose routes FormationPursuit.mirror sets in GDScript. Each phase's class hands over its rules (`rules`); the per-phase bindings stay for their tests and `tools/tick_phases.gd`. Its tests are cargo's (`scrum/step/tests.rs`: the one pass against the phases one by one) and gdUnit's (`test_scrum_pursue.gd`) |
| The fight and its aftermath as one call | `aftermath.rs` (spec 39 step 8b5-xxxviii): the fight with its surrender (`fight::step`), the burial (`deaths::bury`), the takers sent (`taking::pass`), the wounds' tending (`wounds::pass`) and recovery (`recovery::pass`) in their order over one roster, one call, `aftermath_step`, each pass's events in the log in that order. It returns each pass's result, its count first; FormationAftermath mirrors them in the same order (FormationDeaths.mirror, FormationWounds.mirror, FormationRecovery.mirror) and returns recovery's strays. The per-pass bindings stay for their tests and `tools/tick_phases.gd`. Its tests are cargo's (`aftermath/tests.rs`: the one call against the passes one by one) and gdUnit's (`test_formation_aftermath.gd`) |
| The strays through the groups as one call | `mustering.rs` (spec 39 step 8b5-xxxix): the strays adopted (`strays::adopt`), morale's second (`morale::pass`), the rout (`rout::pass`), carrying (`carry::pass`), the bearers adopted and the groups (`groups::pass`) in their order, one call, `mustering_step`, each pass's events in the log in that order; the squads the passes open are listed after the rest for the passes after (`mustering::listing`). It returns each pass's result, its count first; FormationMustering mirrors them in the same order, a stray's squad keeping what the crate holds for it. The per-pass bindings stay for their tests and `tools/tick_phases.gd`. Its tests are cargo's (`mustering/tests.rs`: the one call against the passes one by one) and gdUnit's (`test_formation_mustering.gd`) |
| The tick's end as one call | `ending.rs` (spec 39 step 8b5-xl): stamina (`stamina::pass`), the bodies' parting (`parting::pass`), the shuffle (`swaps::pass`) and the march's sync (`march::sync`) in their order over the squads listed, one call, `ending_step`, the shuffle's events in the log. It returns the parting's counts, then the units made by hand that took their speed as fresh, which FormationEnding mirrors (FormationStamina.mirror). The per-pass bindings stay for their tests, `tools/tick_phases.gd` and `tools/native_bench.gd`. Its tests are cargo's (`ending/tests.rs`: the one call against the passes one by one) and gdUnit's (`test_formation_ending.gd`) |
| The whole tick as one call | `tick.rs` (spec 39 step 8b5-xli): the opening (`opening::step`), the scrum (`scrum::step`), the aftermath (`aftermath::step`), the mustering (`mustering::step`) and the end (`ending::step`) in their order, one call, `tick_step`, each pass handed its own binding's arguments, every pass's events in the log in that order. It takes a roster of every unit the squads hold and lists the squads afresh between the passes as CrateRoster.of did (`tick::relisted`: the squads a join took in dropped, the factions coded again, each squad's living read from its members), hands the mustering the strays recovery set off, and runs the end over the squads the mustering's mirrors would list (`tick::ended_listing`). It returns each pass's result as its own binding does, each part its length first. FormationSimulation.step makes it its one call (FormationTick, `tick_run`, step 8b5-xlii; since step 8c-i the crate builds each pass's rules from the battle's tuning it loaded, `tick::given`, value for value as each pass's GDScript `args` builds them): CrateRoster.of lists every unit, each pass's GDScript class mirrors its part (`mirror`) in order, under CrateMirroring, so a mirror takes the crate's final values instead of writing over what a later pass did, and the events are decoded once. Its tests are cargo's (`tick/tests.rs`: the one step against the five calls one by one, GDScript's lists made afresh between them) and gdUnit's (`test_formation_tick.gd`: a clash with a rout tick by tick against the five calls, a body downed and borne in one tick, and each pass's rules the crate builds equal to its `args`) |
| Identity | `tools/formation_digest.gd`, against the base's (`check_outcomes.py`, Decision 151); until spec 39 step 6a also under each engine, while body parting had its twin |

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
BodyParting.step --------- bodies_part(listed) -> parting::pass::step (spec 39 step
  the listed squads' slots, the seed, tuning       8b5-xxxi): the sync and enrolment from
                                                   the squads' records and sheets, then
                                                   parting (spike kernel, draw order), the
                                                   pushed made loose, the moved committed
  <- stats
ScrumSeek.step ----------- scrum_seek(roster) -> scrum::seek::step (spec 39 step 8b5-xiii):
  the scrum's roster, the ground, the tuning       the sync and enrolment from the roster;
                                                   each fighting squad's say read in the
                                                   crate (foes, may it seek, its frame, its
                                                   chasers, speeds); then, in order:
                                                 seek (seeking.rs, scrum.rs): who may seek
                                                   and its place (SquadFrame.place_in), its
                                                   goal; touch, contest order, kept slots,
                                                   rings, picks, presses; on terrain, where
                                                   it may stand (Ground::crossing)
                                                 walk (walk_store.rs, walking.rs): a frame
                                                   and pace per squad, the bodies lying;
                                                   UnitSteer, UnitMotion.move,
                                                   GroundBodies.underfoot; on terrain each
                                                   step paid for the ground (mover::pay)
                                                 facing (face_store.rs, facing.rs): the
                                                   touching foe, UnitShuffle.look, the turn
                                                   at its rate (DetMath's atan2 and acos)
                                                 every outcome written into the columns
  <- the fighting squads active this tick (for the stall)
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
  `test_det_math.gd` against the GDScript on 20,000 inputs each). `motion.rs` has
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
  one spot" way is rolled in the crate too (`parting::pass::way`, spec 39 step 8b5-xxxi).

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

The suite proved the same while the twin lived (history: since spec 39 step 6a there is one
engine, and these tests keep only what doesn't compare engines):

- `test_body_parting.gd`: a crowd parted the same as GDScript, reversed, on a field kept
  across ticks while units die and change squad, and a 40-a-side battle.
- `test_body_field.gd`: the hash, the roster, and the slot search planning a mid-fight
  scrum the same whatever the list order. Until spec 39 step 4a it and `test_face_field.gd`
  and `test_walk_field.gd` also held the seek, the facing and the walk to the GDScript's at
  four moments of a scrum; those twins are gone, and their scenarios are cargo tests.
- `test_formation_pour.gd`, `test_terrain_toll.gd` and `test_field_test_record.gd`: bodies
  of one frame parted, a fight in a wood walked (each step paid for the ground in GDScript
  after the core's walk), and the user's field-test record replayed, the same as GDScript.
- The whole suite under `BREACH_NATIVE=rust`, including its reversed-list tests:
  1015 of 1015 pass, as they do on GDScript.

## Results

Linux x86_64, Godot 4.7.2 official, release build, 4 cores shared with another job, so
whole ticks carry a few % of noise. FormationBench's clash (grems v militia, seed 1),
mid-fight: 10 ticks after 10 ticks of contact. Figures are ms a tick, the median of 3
runs. They predate spec 39 step 4a, which retired the facing, walk and blows benches with
the GDScript twins they compared against, and spec 39 step 8b5-xiii, after which the bench
times the seek, the walk and the facing together as one call (ScrumSeek.step).

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

The fight's choice is one crate pass over the squads' roster since spec 39 step 8b5-xvii
(`fight::choose`, inside FormationMelee.fight's `fight_step` since 8b5-xix): every living unit's target
is cleared; if some squad may strike, the field is synced to the scrum (`scrum::seek::sync`)
and `blows.rs` gives each striking unit its foe - ScrumBlows._pick's least [front,
distance, draw], the same `facing::touching` the facing uses - with whether its blow falls
on the foe's flank and whether a retreating unit has turned away; its target and cooldown
are set, the cooldown starting again at its melee time, half again while its squad wavers
(its band read from the squad's morale, `shocks::morale`). Then the blows on routers
(`rout_blows.rs`) and the shots (`shots.rs`), each blow's damage from its striker's sheet.
Their landing is the crate's too (`fight/land.rs`, spec 39 step 8b5-xviii):
each unit's arms read from its sheet and words, and for rolled blows each striker's band
and the foes pressing each target (`blows::pressed`) counted on the same sync (a tick no
squad strikes in syncs for the count when blows are rolled). The hurts, events, stamina
and surrender follow in the same pass (`fight::step`, spec 39 steps 8b5-xix and 8b5-xx).
A tick no squad can strike
in (marching, or only a rout left) makes no sync for the search. `test_blows_field.gd`
checked the blows (until spec 39 step 4a retired the twin),
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
server, `native/bin/breach_designer[.exe]` (`src/bin/designer.rs`, `src/designer/`, spec 39
step 2c). They are git-ignored, as
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

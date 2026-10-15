# Open questions

The register of design questions still open, and of reminders the owner asked for
(`AI_First_Development_Kit/principles/agent-sessions.md`, rule 5). Add an entry when a
question is raised and not settled in the same session. Before starting work, check the
entries whose **Raise when** matches it. An agent may add **Possibly settled by**; only
the owner closes an entry, by moving it to Closed with the Decision that answered it.

Entry shape:

```markdown
### Q<n>. <the question, short>
**Raised:** <date, where> · **Raise when:** <spec, feature, date, or next design session>
**Possibly settled by:** <Decision N: why> (optional, for the owner to confirm)

<the detail>
```

## Open

### Q1. Abstracted lane combat vs full spatial placement
**Raised:** before 2026-10-14, in the design spec's open-questions list · **Raise when:** Next design session
**Possibly settled by:** Decision 4 chose abstracted lanes for the vertical slice; Decisions 52 and 132 since moved to 3D cell structures and fully positioned units.

Abstracted lane combat vs full spatial tower placement (Structures section) — the single biggest scope decision.

### Q2. One overlord for v1, or data for many armies
**Raised:** before 2026-10-14, in the design spec's open-questions list · **Raise when:** The faction/overlord spec

One overlord/playstyle for v1, or design the unit data now for multiple armies later? Recommend one now, data-driven for later.

### Q3. A depleted Harvester: ravageable, or a permanent trickle
**Raised:** before 2026-10-14, in the design spec's open-questions list · **Raise when:** The economy spec

Does a depleted Harvester become ravageable for a reduced lump, or just floor at a low trickle permanently?

### Q4. Where fusion happens
**Raised:** before 2026-10-14, in the design spec's open-questions list · **Raise when:** The Lair spec

Does fusion happen only in the Lair, or also mid-map at a fortified position?

### Q5. Map resources to Lair currency exchange rate
**Raised:** before 2026-10-14, in the design spec's open-questions list · **Raise when:** The Lair economy spec

Exchange rate for map resources → Lair currency — needs actual numbers once Lair economy is drafted.

### Q6. Research and goals per map
**Raised:** before 2026-10-14, in the design spec's open-questions list · **Raise when:** The campaign progression spec

Research/goals per map (mentioned but not yet specced): is this a fixed unlock tree per map, or player-chosen objectives within a map?

### Q7. Suspicion decay rate and response-unit stats
**Raised:** before 2026-10-14, in the design spec's open-questions list · **Raise when:** Tuning, once the suspicion state machine is built

Suspicion decay rate and per-tier response-unit stats — needs a tuning pass once the state machine is implemented.

### Q8. Is the map-creator tool in scope for v1
**Raised:** before 2026-10-14, in the design spec's open-questions list · **Raise when:** Next design session
**Possibly settled by:** Decision 30 made the designer a local app in the repo, so it is being built now.

Does the map-creator tool get scoped at all for v1, or purely a post-launch investment (recommended)?

### Q9. Roster sizes and training rates per unit type
**Raised:** before 2026-10-14, in the design spec's open-questions list · **Raise when:** Tuning, once the Task Force system is built

Roster sizes and training rates per unit type (Builder/Guard/Militia/Hero) — needs tuning once the Task Force system is implemented.

### Q10. Multiple critical assets per faction and loss criteria
**Raised:** before 2026-10-14, in the design spec's open-questions list · **Raise when:** The win/loss spec
**Possibly settled by:** Decision 32 made loss groups real map data.

Multiple critical assets per faction with per-faction loss criteria (raised while iterating on the tile designer prototype): should "is player home"-style node marking generalize to an "is_critical_asset" flag independent of node type — any faction could have several (lose any one = that faction is knocked out) plus ancillary, non-critical bases? For the player this could mean multiple defensible cores; for the enemy, multiple bases where only specific one(s) are the real objective. Would need a new per-faction loss-criteria config, and reconciling with `LaneDef.player_home_index`'s existing single-index model — not designed, ties to the still-unbuilt generalized win/loss system (Phase 4 item 5). **Candidate shape, now mocked up in the prototype (v7, prototype-only):** per faction, a list of named *loss groups*. Each group is a set of critical-asset node ids plus a rule: `ANY` (losing any one member triggers it) or `ALL` (only losing every member does). Groups are OR'd, so a faction is knocked out when any one of its groups triggers. That covers "lose if the home falls" (one `ANY` group), "lose only if both twin keeps fall" (one `ALL` group) and mixtures. The prototype warns about critical nodes in no group, which makes flagging them pointless.

### Q11. A faction design one-pager, and whether relations change at runtime
**Raised:** before 2026-10-14, in the design spec's open-questions list · **Raise when:** The faction/overlord spec

Faction "design one-pager" (raised while iterating on the tile designer prototype): a dedicated view (separate from map authoring) for defining a faction's default suspicion ramp rate, available unit types, and AI strategy presets — reusable content, not per-map. Also raised: should faction relations be static per-map authored data, or something that can change at runtime based on save-game state (e.g. a faction the player recruits mid-campaign, previously hostile)? Not designed — a real extension to `FactionDef`/`FactionRelationDef` once there's an actual consumer. The prototype (v7) now gives this its own top-level "Factions" view, separate from the map. Its fields are still unconsumed stubs. It also holds **library-level default relations** (v8, prototype-only): a default stance per faction pair that fills in automatically when both factions are added to a map, and that each map can still override. Real relations remain per-map `FactionRelationDef`s; defaults would be a new `FactionDef`-level concept.

### Q12. What terrain does mechanically
**Raised:** before 2026-10-14, in the design spec's open-questions list · **Raise when:** Next design session
**Possibly settled by:** Decision 85 sets speed per cell from ground, slope and liquid, and Decision 139 makes every move pay it.

Terrain tiles (water, forest, mountain, ravine, etc.) — raised while iterating on the tile designer prototype, added there as purely cosmetic (no mechanical effect), and layered independently of structures (a Fort can sit in a Forest tile, a Resource node on a Mountain). Revisit what they should actually do once movement/combat design has a use for terrain — including a concrete idea raised alongside this: **terrain-based movement resistance** (a base map-wide terrain speed multiplier; roads as a buildable/upgradeable modifier that reduces resistance, built by workers with an ongoing upkeep cost; some terrain outright blocking certain unit types, e.g. siege weapons needing roads/fields; units pathing the least-resistant route). Not designed — a real alternative to (or refinement of) today's "lanes as fixed tracks" model, worth its own pass. **Partly mocked up in the prototype (v7):** a top-level "Terrain" view holds a cross-map terrain-type library. Each type has label, glyph, colour, whether it can be a map's base ground, a stub movement-cost multiplier, stub "blocks unit classes" text, and default stability/footprint (see "Future direction: layered tile model"). The view also holds a natural-feature library and a stub road movement multiplier, and roads are drawn as explicit cell-to-cell connections, with bridges required on water and ravine. None of it is simulated. Designer-view roadmap recorded alongside: Lanes / Factions / Terrain now, with Units, Squads and Static defenses as planned further views.

### Q13. Why attack a fort instead of going around it
**Raised:** before 2026-10-14, in the design spec's open-questions list · **Raise when:** Structures reaching the sim (barricades first)

Why attack a fort instead of going around it? (Raised while designing structure interiors in the tile designer prototype.) Decision 4's Option 2 already says forts "bar a path" and the player routes "around or through", and Infiltrators bypass structures. Once routes are pathfound over terrain (Decision 26), a fort needs a reason to matter. Proposed combination, not yet designed:
  - **Terrain chokepoints**: forts placed where the route is forced, such as bridges (drawbridges especially), passes or ravine crossings.
  - **Zone of control**: static-defense range plus sorties and patrols cover nearby tiles, so passing within range is costly.
  - **Rear threat**: a bypassed garrison raids workers, messengers and supply, and retakes undefended ground ("ground must be held").
  - **Objective value**: critical assets and loss groups, resources, capture rewards, suspicion.

  Goal: bypassing is a real choice with a cost, not always or never worthwhile.

### Q14. Undiscovered, unlinked resource nodes as side objectives
**Raised:** before 2026-10-14, in the design spec's open-questions list · **Raise when:** Movement and discovery being built

Undiscovered/unlinked resource nodes as side objectives (raised while iterating on the tile designer prototype) — a resource node authored with no `MapEdgeDef`/lane connection at all could act as a hidden objective the player must physically route to themselves, tying naturally into the base spec's existing Scout/fog-of-war concept. Likely achievable with the current graph-topology schema as-is (an unlinked node is already a valid, if unusual, authored shape) rather than needing new mechanics — worth confirming once movement/discovery is real.

### Q15. Structure upgrade progression
**Raised:** before 2026-10-14, in the design spec's open-questions list · **Raise when:** Structures reaching the sim

Structure upgrade progression (raised while iterating on the tile designer prototype) — a resource deposit gaining a "mine" upgrade with a small garrison; a Fort's fortification level gating modular internal capacity (barracks for defender capacity, kitchens/stores for siege endurance, defense-point emplacements like arrow slits requiring a specific unit up to a limit, echoing the "static defenses" idea already noted above); saved prefabs for reuse; and faction-specific structure mechanics (e.g. an eldritch faction establishing summoning circles, or defenders acting unilaterally to open unpatrolled entry points). Large, not designed — a real future extension of the Combatant-unification/structure work already flagged.

### Q16. Captives and conversion
**Raised:** before 2026-10-14, in the design spec's open-questions list · **Raise when:** Not for now: raise only if the owner brings it up

Captives/conversion mechanic (raised while iterating on the tile designer prototype, explicitly flagged as "likely not for now") — taking prisoners in combat and converting them to the player's side, or consuming them as a resource (food/blood/sacrifice/knowledge). Not designed.

### Q17. Resource replenishment and depletion redesign
**Raised:** before 2026-10-14, in the design spec's open-questions list · **Raise when:** The economy spec

Resource replenishment/depletion redesign (raised while iterating on `NodeDef`'s resource fields, `specs/13-faction-def-and-garrison-unit.md`): natural baseline replenishment independent of workers; unit skills/traits that increase a specific resource type's replenishment at their tile (no skill/trait system exists yet); workers who both extract *and* replenish (e.g. a farmer granting +2/tick replenishment while harvesting 3/tick), with a priority rule needed between banking replenishment locally vs. hauling extracted output away; a cap on standing "available" resource (e.g. wheat in the field) distinct from the underlying reserve/capacity cap; neglect-decay — an untended, unharvested resource losing material over time, the *inverse* direction from today's harvest-pressure decay (`EconomySystem.decayed_yield()` decays *extraction rate* under sustained harvesting, not standing material under neglect) — and whether/how a tick-represents-real-calendar-time concept (e.g. 6 hours/tick, 3-day seasons) layers season and time-of-day effects on top. Not designed yet; `NodeDef.is_inexhaustible`/`total_reserves` (schema round 4) deliberately leave room for this without committing to any of it.

### Q18. A JSON-to-.tscn converter for the designer's export
**Raised:** before 2026-10-14, in the design spec's open-questions list · **Raise when:** Next design session
**Possibly settled by:** Decision 29 chose JSON to typed .tres instead, built in spec 16.

A real JSON→`.tscn` converter for the Lane Tile Designer prototype's export (offered during this work, not yet built): would generate `MapSceneRoot`/`MapLaneRoot`/`MapNodeMarker`/`MapEdgeMarker` scene text from a pasted-out export JSON, reusing the already-tested `MapSceneConverter` (`specs/10-map-scene-authoring.md`) for the rest of the pipeline — the practical path from the prototype's clipboard-only export (an artifact cannot write to the repo/filesystem directly) to an actual loadable map. Not started.


### Q19. Should `FactionDef` store its own colour
**Raised:** 2026-09-29, building the map viewer (PR #30) · **Raise when:** Faction colours are next touched (viewer, designer or sprites)

The viewer colours factions by roster order and the designer by library order, so one faction can show in two colours. A stored `FactionDef.color` would fix both. Related: the owner wants recolourable sprite parts for faction allegiance (2026-10-02).

### Q21. Do the remaining native ports wait for the unit store?
**Raised:** 2026-10-14, the design session on spec 34 item 4 · **Raise when:** An implementation session chooses between spec 34's "Next, in order" list (body parting, wounds' port, blows, deaths, recovery, stamina, steering, the walk) and spec 35's steps

Each port made before spec 35 step 4 adds the gather, sync and write-back that the store then removes. The design session recommends spec 35 steps 0 to 4 first and new ports after, but the user didn't settle this.

**Possibly settled by:** the user starting spec 35 step 0 next (2026-10-14), ahead of spec 34's list.

### Q22. When is a GDScript reference retired, and what checks go with it?
**Raised:** 2026-10-15, while parallelising CI (the user: "realistically we are gearing towards rust only") · **Raise when:** A design session on the unit store's later steps or on CI, or any PR that would drop a GDScript pass, the GDScript suite or the digests' GDScript half

Decision 129 makes GDScript the reference: the digests compare both engines, and the parity tests (`test_unit_store.gd`'s column comparison, the field tests such as `test_walk_field.gd`) run each ported pass on both. The user asked whether a parity check can go once it is green. The answer given: not while both implementations exist and change, since parity at one commit doesn't hold for the next; a pass's parity tests go when its GDScript twin is deleted, and its digests are then recorded against Rust. Also to settle: moving the GDScript full suite and scaling check from every PR to pushes to `main` only, keeping the digests and `bench` on both engines. Each of these loosens Decision 129, so it needs a new Decision that the user authorises.

### Q23. Did the unit store pay for itself? Review spec 35's bench series as a whole
**Raised:** 2026-10-15, after spec 35 step 4a (the user: "something to keep an eye on and review in totality once spec 35 is complete") · **Raise when:** Spec 35's last step (7) is built, or any session proposes reverting or reshaping a spec 35 step because of its bench

Each spec 35 step starts a `bench` run (`tick_phases engine=rust side=4096 squads=4 fight=3`) against its base and records it in spec 34's "Measured" section. Steps 1 to 4a add boundary crossings, and 4a adds a store read wherever a rule reads one unit's place. Step 4 onwards should remove the glue. A single step's numbers are noisy, and runners differ, so judge the series as a whole: the tick at the end against the tick before step 0 (Q20's 380 ms), phase by phase. Name any phase that got slower and stayed slower, and whether a pass should fetch columns once where it now reads per unit.

### Q24. Which per-tick passes should become work lists fed by events?
**Raised:** 2026-10-15, after spec 35 step 4 (the user: "are there any elements that could be handled via event based arch?") · **Raise when:** A design session on spec 35 steps 5 to 7, or on what follows spec 35 for performance; or any change to deaths, wounds, recovery, taking, carry or `living()`

`principles/bounded-work.md` Principle 2 ("work follows change") already asks for this, and spec 34 item 5 applies it to structural supports only. Several unit passes still scan every unit every tick to find the few that changed (seen at `tick_phases engine=rust side=4096 squads=4 fight=3`, ms a tick):
- **Deaths (about 22 ms):** `FormationDeaths.bury` checks every living unit's hp each tick. A blow that takes a unit past its threshold could queue it.
- **Wounds (about 20 ms), taking (about 1 ms), recovery (about 21 ms), carry (about 7 ms):** each walks every unit for the downed, or builds a map of all units, every tick. A list of the downed, kept by the events that down, wake, carry or bury a unit, would bound them by the number of downed.
- **`living()`:** filters `squad.units` on `is_alive()` into a new array on each of its about 100 calls. A living list kept by deaths, downings and joins would be O(1), and settles step 7's plan for the alive flags (Decision 144).
- **`fighters()` and ranged combat's rank groups:** rebuilt every tick; they change only on a step-up, a death, a swap or a join.
- **Squad edges and spans (`SquadEdges`, `SquadMemo`):** worked out per phase; they change only when a squad moves, turns, or its units die or shift.
- **The re-form's occupancy:** rebuilt every tick while a squad re-forms; it changes only as swaps end and units fall.

Each would need the events to queue every change that matters (Principle 2's correctness rule) and a deterministic order for the list (Decision 97). It is a design subject: likely a spec of its own after spec 35, or folded into steps 5 to 7 where the field moves anyway (deaths and wounds with hp in step 5, `living()` with the alive flags in step 7).

**Partly settled by Decision 145 (2026-10-15):** deaths drains a fell queue fed by hp's one writer in spec 35 step 5b; that store queue (one writer, drained once a tick in a fixed order) is the pattern for the downed list and the living list in step 7. Still open: `fighters()` and combat's rank groups, squad edges and spans, and the re-form's occupancy.

### Q25. Game-like maps on the real sims, in place of the old test maps
**Raised:** 2026-10-15, retiring the lane skirmish (spec 35 step 5a) · **Raise when:** Spec 35 is complete, or any session adds, keeps or rebuilds a test or demo map or scene

The user: "any old maps have purely been for either visualisation or test purposes, really we need to get game like-for-like maps even if only considering a 3x1 set of tiles with no unique logic, only using our game sims and renderers as and when created (of course using temp renderers until the full set is available)". So the next playable scenes should be small maps built from the game's own content and run by the real simulation (the formation battle, the lane and tile model as they become real), rendered by the game's renderers where they exist and temporary ones until then, rather than one-off sims or scenes kept for a test. To settle in a design session: the first such map (a 3x1 set of tiles?), which sims it runs, which renderers are temporary, and which existing test or demo scenes (`formation_skirmish.tscn`, the 2D feel-test field, the map viewer) it replaces or keeps.

### Q26. Simulation threading and slot locality
**Raised:** 2026-10-15, a question session on performance while spec 35 was being built (the user: "All of the potential gains you listed, I would like to explore those") · **Raise when:** Spec 35's last step (7) is built (take it with Q23 and Q24 in the same design session), or any session proposes threads, a slot order or a change to how the store's columns are laid out

Threads today: only body parting uses Rayon (`native/rust/src/parting.rs`), and only when `BREACH_NATIVE_THREADS=1`; it is off by default. The other Rust kernels (seek, walk, facing, blows) run on one thread. Nothing in `sim/` or `presentation/` uses a Godot thread, so the tick runs on the main thread with rendering. The gains to explore, each needing a Decision the user authorises and the identity gate (Decision 129) on both engines:
- **The tick off the main thread.** Tick N+1 runs on a worker while presentation draws tick N from its per-frame column snapshot (spec 35 step 1a). The tick costs the same; frames stop waiting on it.
- **Parallel kernels.** Seek, walk and facing read shared state and write only their own unit's columns. Body parting's pattern fits them: compute in parallel into a buffer, apply in squad and member order. Float reductions keep a fixed summation order (map in parallel, reduce serially). A unit-count threshold keeps Rayon's overhead off small battles.
- **Threads on by default,** once the digests run with them on in CI.
- **Slots packed by squad.** Passes run in squad and member order, but slots are given lowest free first, so a squad's members scatter over the columns as units die and join. Packing a squad's slots together (at enrol, or a periodic compaction that updates the unit-to-slot map) makes those passes read memory in order. A Morton (spatial) order is the alternative for perception and grid queries. Slots are storage only (Decision 97), so neither may change an outcome.
- **No hashing in hot loops.** `by_unit` is a HashMap; hot loops should index by slot and keep the map for enrolling.
- **The grid as one sorted array.** Slots counting-sorted by cell each tick, in place of a list per cell.
- **Snapshots of changed columns only,** for presentation.
- **The GDScript passes still looping per unit.** Spec 35 moves state, not loops: after it, shuffle (about 37 ms), perception (about 17 ms), wounds, deaths, recovery, stamina, carry and steering still loop over units in GDScript. Spec 34's "Next, in order" list ports wounds, deaths, recovery, stamina and steering, and Q24 turns several into event-fed work lists; **shuffle and perception are on neither list.** Settle their ports, and the order of all of them, here.
- **Out of scope unless the user reopens them:** `f32` columns (they break identity with the GDScript reference) and double-buffered columns read from the last tick (they change outcomes).

Measure each with a `bench` run against its base, and the user's PC (a profiler such as VTune or uProf) for cache misses, before any is built. Threads speed up only the Rust part of a pass, so a pass with GDScript glue left gains little until its glue goes.

## Closed

- Q20, the native field owning unit state so the glue goes. **Resolved by Decision 142 (2026-10-14):** a unit store owns every unit's per-tick state as columns, with GDScript and Rust backends, built in steps as spec 35.
- Whether an authored `MapEdgeDef`/link is a hard constraint (the only route that exists) or a soft default a future pathfinding/movement-cost system could override — raised alongside the terrain-resistance idea above. Not designed; depends on that idea's outcome. **Resolved by Decision 26 (2026-09-27):** both. An authored link is a hard constraint on *which* nodes connect, and its route *geometry* is pathfound over terrain cost.
- `NodeDef.structure_slots` semantics (raised while iterating on the tile designer prototype): per Decision 4's original framing this is a *count* of discrete buildable foundations at a location ("room for 2 towers here"), not a physical size/area value — but nothing consumes it yet, so this is genuinely underspecified. A real gap raised directly: if `structure_slots: 0` means "nothing permanent can be built here" (used in the prototype as a pure routing waypoint), can a barricade or other lightweight stationary defense still go there? A barricade may need to be a different, non-slot-consuming category entirely rather than something gated by `structure_slots`. Not designed — belongs to the still-unbuilt spatial-placement pass (Decision 4's Option 2). See "Future direction: layered tile model" below, which treats a barricade as a 0-floor structure and asks whether *footprint* should replace or redefine `structure_slots`. **Resolved by Decision 25 (2026-09-27):** superseded by the tile capacity + segment-profile model, and the field has been removed (`specs/14-structure-slots-removal.md`).

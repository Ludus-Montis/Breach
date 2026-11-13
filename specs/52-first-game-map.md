---
spec_type: code
status: active
parent_spec: ../Breach — Reverse Tower Defense Design Spec.md
---

# Spec 52: The first game-like map, on the real battle

## Purpose

Every map and scene so far has been for looking or for testing. The owner: "any old maps have
purely been for either visualisation or test purposes, really we need to get game like-for-like
maps even if only considering a 3x1 set of tiles with no unique logic, only using our game sims
and renderers as and when created (of course using temp renderers until the full set is
available)" (Q25, #280).

Today the two halves don't meet. A designer map (`content/maps/<name>.json`) is a node graph
and a grid of tiles, each tile 64 battle cells. Only the map viewer and the lane skirmish read
one. The formation battle, the real simulation, plays on grounds hard-coded in
`native/rust/src/ground/map.rs` (`field`, `mixed`), and the field's routes and the kingdom's line
are hard-coded in `field_play.rs`.

This round joins them. The crate sets up the formation battle from a map: its ground, its route,
its garrison. A node is taken by standing on it, and the map is won or lost by its loss groups.
One map scene plays it, draws thousands of bodies in one batch, and takes over from the lane
skirmish and the map viewer.

**Settled in the design session of 2026-11-12 (the owner):** Decisions 182 to 185.
- **The first map:** a fresh 3x1 of fields, `road_3x1`. simple_map (uncommitted in the owner's
  checkout) is the second map, later.
- **The sims:** the formation battle plus taking nodes and a win or loss. The lane sim, the
  kingdom's response and the economy aren't in this round.
- **The ground:** flat, move cost only. Sub-tile and agent-assisted design, and a designer
  version of the ford and flank field, are Q50 (#1001).
- **Taking a node:** stand on it unopposed (Decision 183). A garrison of n is n kingdom lines.
- **The scenes:** replace all three. The skirmish goes now, the viewer once the map scene opens
  any map, and the field after Q50 (Decision 184).
- **Renderers:** all temporary; bodies in one batched draw.
- **Debug seeding:** `--debug`, with unlimited reserves and a wave-size picker.
- **The pipeline:** maps reach Godot as the designer's JSON through the crate, with no `.tscn`
  (Q18, #274). The designer is v1's developer tool, and a player's map editor is post-launch
  (Q8, #264). Decision 185.

## Status

Agreed in the design session of 2026-11-12. Ten steps, each its own PR, built with
`/run-phase` as "spec 52 step N".
- Steps 1 to 4 are the crate and its binding, in order.
- Step 5a needs only step 4's snapshot, and step 9 needs nothing: both can run beside the rest.
- Steps 6, 7 and 8 all change the map scene, so they run one at a time after 5b.

No step touches the regroup, rout or pursuit code (specs 46 and 50), or CI and the release
workflows (spec 49). No step changes what the 2D field plays: its digests and named scenarios
stay identical.

## Pre-mortem

- **Regret in a month:** a "map scene" that is the field with a new ground and two nodes
  painted on it, with no taking and no end: another demo under a new name. Or the other way, the
  lane capture, the kingdom's response and the economy wired in at once, so the round balloons
  and runs into specs 46 and 50 in `native/`. This round takes the battle and the taking of
  nodes only.
- **Riskiest assumption:** that the crate can make a battle's ground, route and kingdom's line
  from a designer map without touching how the field plays. The first attempt at a test
  (author the field's own ground in the designer and prove it identical) failed on inspection:
  the designer paints one terrain per tile, while the field's ford, wood and hill are smaller
  than a tile. So the ground here is move cost only, and the rest waits for Q50.
- **Cheapest test:** steps 1 and 2. The 3x1's ground has the cells the library says, and a
  wave sent headless walks the map's road and meets the garrison, before anything is drawn.

## Steps

### Step 1, a map's ground, and `road_3x1`

**Needs:** none
**Touches:** `native/rust/src/ground/map.rs`, `native/rust/src/ground/tests.rs`,
`content/maps/road_3x1.json`, `content/maps_src/road_3x1.designer.json`, `native/README.md`

- **The map,** authored in the designer (its Save writes both files): 3 columns by 1 row, all
  FIELDS, a road along the row from the first tile's centre to the last's, the player's node `P`
  (ORIGIN) on the first tile, the kingdom's node `c` on the last with a garrison of 1 (a FORT if
  the map rules don't let an ORIGIN hold a garrison), a route from `P` to `c`, and the loss
  groups `P` (player, ANY) and `c` (the kingdom, ANY). No new field in the schema.
- **A map's ground** (`ground::map`, beside `field` and `mixed`): the map's size in cells
  (columns and rows times `CELLS_PER_TILE`), each cell at its tile's terrain's `move_cost`, and
  each cell within 4 cells of a road segment's line, between its two tiles' centres, at that
  times the library's `road_move_multiplier`. Flat, with no liquid, open to sight. Every other
  property of a tile is ignored, and the doc comment says so and names Q50.
- **The field's and the mixed ground don't change.**

Scenarios:
- Given `road_3x1` and the terrain library, when its ground is made, then it is 192 by 64
  cells, a cell off the road costs FIELDS' move cost, a cell on the road costs that times the
  road multiplier, and every cell is level and dry.
- Given a map with a tile of another terrain, then that tile's cells take that terrain's cost.

### Step 2, a battle set up from a map

**Needs:** 1
**Touches:** `native/rust/src/map_play.rs` (new), `native/rust/src/map_play/tests.rs` (new),
`native/rust/src/lib.rs`, `native/README.md`

- **`MapPlay`,** a sibling of `FieldPlay` that reuses its parts (`Play`, `Route`, `Wave`,
  `Builders`) without changing them: the battle on step 1's ground, with the map's tuning tick.
- **The player's route** is the layout's route from the player's node to the kingdom's, its
  tiles' centres as points in cells, with the field's corridor of 4 cells either side. A map
  with no such route can't be played (`None`, nothing written).
- **The player's waves and builders** are the field's: waves of 8 grems, sent from the start of
  the route, and four grem builders.
- **The garrison:** a node with garrison n gets n of the field's kingdom lines, stood one behind
  another across the route at the node, the first line's front at the node's tile edge facing
  the player, each next line behind it at the field's reserve spacing. A garrison of 0 has no
  line.

Scenarios:
- Given `road_3x1` set up, then the kingdom has one line across the route on `c`'s tile, facing
  `P`.
- Given a wave sent, when the battle is stepped, then its front walks the road towards `c` and
  engages the line.
- Given a map with no route from the player's node to the kingdom's, then it can't be set up.

### Step 3, taking nodes and the end

**Needs:** 2
**Touches:** `native/rust/src/map_play.rs`, `native/rust/src/map_play/taking.rs` (new),
`native/rust/src/map_play/tests.rs`, `native/README.md`

- **Holders at the start:** each loss group's nodes are held by its faction; a node in none is
  held by no one.
- **Taking** (Decision 183), at the end of each step, after the whole tick, for every node
  against the same snapshot: a side takes a node when it has a living, unrouted unit on the
  node's tile and the holder has none there. Found through the body grid over the tile, stopping
  at the first unit of each side. Reading whether a unit is routed is all this needs from the
  rout code, which it doesn't change.
- **The end:** a faction whose loss group's rule (`ANY`, `ALL`) is met has lost; both on the
  same tick is a draw. `MapPlay` reports the holders, each node taken (with its tick) and the
  end.

Scenarios:
- Given `road_3x1` with its garrison routed or fallen, when a grem stands on `c`'s tile, then
  the player takes `c` and wins.
- Given units of both sides on a node's tile, then its holder keeps it.
- Given a routed grem alone on `c`'s tile, then `c` isn't taken.
- Given the same battle stepped twice, the second time with every squad and unit list reversed,
  then `c` is taken on the same tick (Decision 97).

### Step 4, the map battle for GDScript

**Needs:** 3
**Touches:** `native/rust/src/map_play/binding.rs` (new), `native/rust/src/map_play.rs`,
`content/definitions/native_library.gd`, `tests/presentation/map/test_map_state.gd` (new),
`native/README.md`

- **`MapState`,** as `FieldState` is the field's (`field_play/binding.rs`): opened on a content
  map by name, played on a worker, owed the clock's ticks each frame and never waited on. It
  gives the snapshot to draw (every body where it stands, living or fallen, and its side), the
  holders, the taken nodes and the end. Sending a wave is an order, recorded with its tick
  (FormationRecord, Decision 115).

Scenarios:
- Given `MapState` opened on `road_3x1`, when a wave is sent and ticks are played, then the
  snapshot holds its 8 grems and the kingdom's line, and the holders say `P` is the player's and
  `c` the kingdom's.

### Step 5a, the bodies in one batch

**Needs:** 4
**Touches:** `presentation/map/body_layer.gd` (new), `tests/presentation/map/test_body_layer.gd`
(new)

- **One `MultiMesh`** for every body, living and fallen, where it stands (Decision 106),
  coloured by side, the fallen darker, eased between ticks as the field eases them. An instance
  per body, set from the snapshot each frame, and no per-unit `_draw` call. A temporary renderer
  (Decision 184).

Scenarios:
- Given a snapshot of n living and m fallen bodies, then the layer holds n + m instances at
  their places, in their sides' colours.
- Given 8,192 bodies, then they are one draw (checked by the instance count; the frame rate is
  checked by hand).

### Step 5b, the map scene

**Needs:** 5a
**Touches:** `presentation/map/map_scene.tscn` (new), `presentation/map/map_scene.gd` (new),
`presentation/map/map_scene_hud.gd` (new), `tests/presentation/map/test_map_scene.gd` (new),
`README.md` (how to run it)

- **The scene** opens `--map=<name>` (else `road_3x1`) through `MapState`. It draws the terrain
  with `MapLayoutPainter` and the nodes with `MapView`'s markers, in the holder's colours, and
  the bodies with 5a's layer, under a camera that pans and zooms.
- **The HUD:** send a wave, pause, speed, each node's holder, and the end as a banner (won,
  lost, a draw) that stops play. All temporary renderers.

Scenarios:
- Given `--map=road_3x1`, then the scene opens it, and sending a wave puts 8 grems on the road.
- Given the end reported, then the banner names it and play stops.

### Step 6, debug seeding

**Needs:** 5b
**Touches:** `native/rust/src/map_play.rs`, `native/rust/src/map_play/binding.rs`,
`native/rust/src/map_play/tests.rs`, `presentation/map/map_scene.gd`,
`presentation/map/map_scene_hud.gd`, `README.md`

- **`--debug`:** unlimited reserves (a wave is full the moment it is asked for) and a wave-size
  picker from 8 to 4,096 in powers of two. A wave of n is set out as the bench's clash sets out
  a squad of n, centred on the route at its start. The orders are recorded like any other.
  Without the flag, nothing changes.

Scenarios:
- Given debug seeding and a wave size of 1,024, when two waves are sent one after the other,
  then 2,048 grems are on the route, the second behind the first.
- Given no debug seeding, then a wave of more than 8 can't be sent.

### Step 7, replaying a record on the map

**Needs:** 6
**Touches:** `presentation/map/map_scene.gd`, `presentation/map/map_replay.gd` (new),
`tests/presentation/map/test_map_replay.gd` (new), `tests/fixtures/road_3x1_waves.record` (new),
`README.md`

- **`--replay=<path>`** plays a FormationRecord on the scene's map, hands-free, ending at the
  map's end or 90 seconds (Decision 169's limit). The record carries its map's name and seed. A
  record for another map is refused, with the reason.
- **The shared scenario list doesn't change** (Decision 184), and nothing in `ci/` changes.

Scenarios:
- Given the fixture (two debug waves of 1,024 on `road_3x1`), when it is replayed twice, then
  both replays take `c` on the same tick.

### Step 8, any map, and the map viewer retires

**Needs:** 7
**Touches:** `presentation/map/map_scene.gd`, `presentation/map/map_scene_hud.gd`,
`tests/presentation/map/test_map_scene.gd`, `presentation/map_viewer.tscn` (deleted),
`presentation/map_viewer.gd` (deleted), `tests/presentation/test_map_viewer.gd` (deleted),
`native/rust/src/designer/repo.rs`, `native/rust/src/designer/server_tests.rs`,
`tools/designer/repo.js` (the button's tooltip), `tools/designer/README.md`, `README.md`,
`docs/` (wherever the viewer is named)

- **The viewer's parts move over:** the map picker over `content/maps`, "view as", and a
  clicked node's details (`MapDetails`). A map the scene can't play opens for viewing only, and
  says why.
- **"View in Godot"** in the designer opens the map scene on the saved map.
- **The viewer is deleted**, with whatever only it uses. `MapView`, `MapLayoutView`,
  `MapLayoutPainter` and `MapDetails` stay, used by the map scene.

Scenarios:
- Given `--map=skirmish_p_c`, then the scene opens it for viewing, with the picker and "view
  as".
- Given the designer's View in Godot, then its command opens `map_scene.tscn` with the map.

### Step 9, the skirmish retires

**Needs:** none
**Touches:** `presentation/skirmish/formation/` (deleted), `presentation/skirmish/` (whatever
only the skirmish uses: `skirmish_route.gd`, `skirmish_camera.gd`, `skirmish_camera_rig.gd`),
`tests/presentation/skirmish/test_formation_skirmish_scene.gd` (deleted, with any test of only
those), `native/rust/src/battle_play.rs` and `native/rust/src/battle_play/` (deleted),
`native/rust/src/lib.rs`, `content/definitions/native_library.gd`, `README.md`, `docs/`

- **Deleted:** `formation_skirmish.tscn`, its scripts, and `BattleState` (`battle_play`), which
  only it uses. Before deleting each file, check that nothing else uses it (`grep`). `lane.rs`,
  `kingdom.rs`, `economy.rs` and `waves.rs` stay. `skirmish_two_lanes` stays a content map.
- Its README and `docs/` mentions go, with a pointer to the map scene.

Scenarios:
- Given the project, then nothing loads or names `formation_skirmish.tscn` or `BattleState`,
  and the crate builds without `battle_play`.

## Not in this spec

- **Sub-tile ground, heights, water and sight from a designer map,** and the ford and flank
  field as a designer map: Q50 (#1001). The 2D field retires only after that (Decision 184).
- **Map scenarios on the shared list, and the capture playing them on the map scene:** a step
  that changes `ci/showcase/capture.sh`, after spec 49.
- **The lane sim, the kingdom's response and the economy on a map:** a later round.
- **simple_map** as the second map.
- **Real art** for terrain, nodes and units.

## Test-first order

Each step's scenarios are its test, red first, in the one file named in its Touches:
`ground/tests.rs` (1), `map_play/tests.rs` (2, 3, 6), `test_map_state.gd` (4),
`test_body_layer.gd` (5a), `test_map_scene.gd` (5b, 8), `test_map_replay.gd` (7). Step 9 deletes
code; its check is the build and the suite in CI.

## Rubric answers

- **single-noun-phrase:** a map's ground; a battle from a map; taking nodes; the map battle for
  GDScript; the bodies' batch; the map scene; debug seeding; replays; any map; the skirmish's
  retirement.
- **ocp-extension-point:** a new map is a designer save; a renderer is replaced in the map scene
  without touching `MapState`.
- **lsp-contract-scope:** not applicable; no subtypes.
- **isp-fit:** the scene reads only `MapState`'s snapshot, holders and end.
- **dip-direction:** the presentation depends on the crate's binding, never the reverse.
- **general-over-special:** general. One rule makes any map's ground; one taking rule holds for
  every node and side; one scene plays any map (Decisions 182 to 184).
- **pre-mortem:** above. Steps 1 and 2 test the riskiest assumption before anything is drawn.
- **order-independent:** taking is decided from one snapshot for every node; a same-tick end is
  a draw; step 3 tests reversed lists. The field's digests stay identical.
- **bounded-work:** the ground is made once; taking reads each node's tile through the body grid
  and stops at the first unit of each side; the bodies are one batched draw.

## Built

(Each step's PR adds a line here.)

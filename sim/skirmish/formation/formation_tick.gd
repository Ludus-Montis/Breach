class_name FormationTick
extends RefCounted
## The whole tick as one crate call (`tick_run`, spec 39 step 8b5-xlii): the opening
## (FormationOpening), the scrum (FormationScrum), the fight and its aftermath
## (FormationAftermath), the strays through the groups (FormationMustering) and the end
## (FormationEnding), in that order over one roster of every unit the squads hold, every
## pass's events in the crate's log in that order. The crate builds each pass's rules from the
## battle's tuning it loaded (CrateContent.store, spec 39 step 8c-i), as each pass's own `args`
## builds them for its binding. Then each pass's result is mirrored into GDScript's copies in
## the same order, while CrateMirroring is on: a mirror runs after every later pass in the
## call, so the crate's values are the tick's, and none of its writes may land over them.
## Then the events are decoded once.

## The passes' parts in what the crate returns, in order.
enum Part { OPENING, SCRUM, AFTERMATH, MUSTERING, ENDING }

const FormationAftermath = preload("res://sim/skirmish/formation/formation_aftermath.gd")
const FormationEnding = preload("res://sim/skirmish/formation/formation_ending.gd")
const FormationMustering = preload("res://sim/skirmish/formation/formation_mustering.gd")
const FormationOpening = preload("res://sim/skirmish/formation/formation_opening.gd")
const FormationTerrain = preload("res://sim/skirmish/formation/formation_terrain.gd")
const ScrumSeek = preload("res://sim/skirmish/formation/scrum_seek.gd")
const UnitMover = preload("res://sim/skirmish/formation/unit_mover.gd")


## One tick over `squads` on `terrain` (null: open ground). `at` = [the tick, the seed, the
## ticks a second, the next free squad id]; `paces` = [the travel scale (tiles a second at
## speed 1), the tick's seconds, the lane's length, cells a second at speed 1]; `rolls`
## whether blows roll. New squads are added to `squads`, those joined or taken in leave it;
## adds the tick's events and returns its perception (FormationOpening's).
static func step(
	squads: Array, terrain: FormationTerrain, at: Array, paces: Array, rolls: bool, events: Array
) -> Dictionary:
	if squads.is_empty():
		return {}
	var store: UnitStore = squads[0].store
	var field: Object = ColumnsLent.field(store.columns)
	var next_id: int = at[3]
	var ticks := PackedInt64Array([at[0], at[1], at[2], int(rolls), int(NativeKernels.threaded)])
	var given := [CrateContent.store(), CrateRoster.of(squads, terrain, true)]
	given.append_array([UnitMover.ground(terrain), ticks, PackedFloat64Array(paces)])
	var flat: PackedInt64Array = ColumnsLent.all(
		store.columns, func(): return field.callv(&"tick_run", given)
	)
	var parts := _parts(flat)
	var perception: Dictionary = CrateMirroring.run(
		func():
			var seen := FormationOpening.mirror(squads, parts[Part.OPENING])
			ScrumSeek.clock_usec += parts[Part.SCRUM][0]  # the seek's own time, for the benches
			var strays := FormationAftermath.mirror(squads, parts[Part.AFTERMATH])
			FormationMustering.mirror(squads, strays, parts[Part.MUSTERING], next_id)
			FormationEnding.mirror(store, parts[Part.ENDING])
			return seen
	)
	events.append_array(CrateEvents.taken(field, store))  # every pass's, in order
	return perception


## The crate's `flat` result cut into the passes' parts, each its length first.
static func _parts(flat: PackedInt64Array) -> Array[PackedInt64Array]:
	var parts: Array[PackedInt64Array] = []
	var at := 0
	while at < flat.size():
		parts.append(flat.slice(at + 1, at + 1 + flat[at]))
		at += 1 + flat[at]
	return parts

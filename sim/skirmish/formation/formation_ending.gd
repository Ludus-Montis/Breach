class_name FormationEnding
extends RefCounted
## The tick's end as one crate call (`ending_step`, spec 39 step 8b5-xl): stamina
## (FormationStamina), the bodies parted (UnitBodies, BodyParting), the shuffle
## (FormationShuffle) and the march's sync (FormationMarch.sync_units), in that order over
## the squads, as FormationSimulation made them one call at a time. Then stamina's result is
## mirrored into GDScript's copies and the shuffle's events taken.

const BattleTuning = preload("res://content/definitions/battle_tuning.gd")
const FormationLocks = preload("res://sim/skirmish/formation/formation_locks.gd")
const FormationStamina = preload("res://sim/skirmish/formation/formation_stamina.gd")
const FormationTerrain = preload("res://sim/skirmish/formation/formation_terrain.gd")
const NativeKernels = preload("res://sim/skirmish/formation/native_kernels.gd")
const SkirmishSquad = preload("res://sim/skirmish/formation/skirmish_squad.gd")
const UnitBodies = preload("res://sim/skirmish/formation/unit_bodies.gd")
const UnitMover = preload("res://sim/skirmish/formation/unit_mover.gd")

## Numbers before the made units' ids in what the crate returns: the parting's [usec,
## passes, pairs].
const PARTED := 3


## The tick's end over `squads` on `terrain`. `at` = [the tick, the seed]; `paces` = [the
## tick's seconds, cells a second at speed 1 for the shuffle (the travel scale), cells a
## second at speed 1 for the march]. Returns the shuffle's events.
static func step(squads: Array, at: Array, paces: Array, terrain: FormationTerrain) -> Array:
	if squads.is_empty():
		return []
	for squad: SkirmishSquad in squads:
		squad.living()  # its members written to the squad store, if its lists changed
	var tuning := BattleTuning.current()
	var listed := FormationLocks.slots(squads)
	var ints := PackedInt64Array(at + [int(NativeKernels.threaded)])
	var floats := PackedFloat64Array([paces[0], paces[1], tuning.scrum_crowding, paces[2]])
	var stamina := FormationStamina.tuning()
	var bodies := PackedFloat64Array(
		[tuning.bodies_passes, tuning.bodies_resist, tuning.bodies_brush]
	)
	var walk := CrateMarch.walk_tuning()
	walk.append(tuning.wounds_reach)
	var ground: Object = UnitMover.ground(terrain)
	var store: UnitStore = squads[0].store
	var field: Object = ColumnsLent.field(store.columns)
	var flat: PackedInt64Array = ColumnsLent.all(
		store.columns,
		func(): return field.ending_step(listed, ints, floats, stamina, bodies, walk, ground)
	)
	UnitBodies.clock_usec += flat[0]  # the parting's own time, for the benches
	BodyParting.spent[2] += flat[0]
	FormationStamina.mirror(store, flat.slice(PARTED))
	return CrateEvents.taken(field)  # the shuffle's "swapped", "stepped_up"

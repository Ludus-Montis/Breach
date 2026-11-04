class_name FormationStrays
extends RefCounted
## A unit on its own makes for home (Decision 126): one that came to after being downed,
## its formation gone or out of sight. It becomes a lone router of its own (FormationRout):
## it flees home along its formation's route - caught by a friendly formation it runs into,
## it joins it; struck, it may be downed again or surrender; home, it returns to the
## reserve ("fled_home"). Pure. The crate makes the strays' squads (`strays::adopt`, spec 39
## step 8b5-xxvi): it takes each unit out of its squad, opens its own squad's slot, numbers
## it and sets it off; GDScript's SkirmishSquad over that slot is mirrored here.

const SkirmishSquad = preload("res://sim/skirmish/formation/skirmish_squad.gd")
const SkirmishUnit = preload("res://sim/skirmish/skirmish_unit.gd")
const SquadColumns = preload("res://sim/skirmish/formation/squad_columns.gd")
## What the crate set on a stray's squad (routing, at no morale, facing as its squad, set off
## from where it stands), read before GDScript's squad is made over the slot and kept after.
const HELD := [
	SquadColumns.HEADING,
	SquadColumns.STATE,
	SquadColumns.MORALE,
	SquadColumns.FRONT_DISTANCE,
	SquadColumns.POSITION
]
const HELD_NAMES: Array[StringName] = [
	&"heading", &"state", &"morale", &"front_distance", &"position"
]


## Sets each of `strays` ([[unit, its squad], ...]) off home alone, as squads numbered from
## `next_id` added to `squads`; returns the next free squad id. Each is a routing squad of
## the unit alone, taken out of its squad, fleeing home along its squad's route from where it
## stands. Strays setting off together are numbered in their units' seeded draws' order,
## never the list's (Decision 97): a squad's id salts its own draws.
static func adopt(strays: Array, squads: Array, next_id: int, fight_seed: int) -> int:
	if strays.is_empty():
		return next_id
	var store: UnitStore = strays[0][1].store
	var flat := PackedInt32Array()
	for stray in strays:
		stray[1].living()  # its members written to the squad store, if its lists changed
		flat.append_array([stray[1].slot, stray[0].slot])
	var field: Object = store.field()
	field.put_squads_issued(next_id - 1)
	var opened: PackedInt32Array = ColumnsLent.all(
		store.columns, func(): return field.strays_adopt(flat, fight_seed)
	)
	return mirror(strays, squads, PackedInt64Array(Array(opened)), next_id)


## GDScript's squads over the slots the crate opened for `strays` (`opened`: two numbers a
## stray, [its index in `strays`, the slot], in the order numbered from `next_id`), added
## to `squads`; returns the next free squad id. Later passes of the same crate call may have
## changed a slot since (FormationMustering): what the crate holds there is kept.
static func mirror(strays: Array, squads: Array, opened: PackedInt64Array, next_id: int) -> int:
	for n in range(0, opened.size(), 2):
		var stray: Array = strays[opened[n]]
		squads.append(_mirrored(stray[0], stray[1], opened[n + 1], next_id))
		next_id += 1
	return next_id


## GDScript's squad `id` over the slot the crate opened for `unit`, which left `squad`: the
## same values assigned through its setters.
static func _mirrored(
	unit: SkirmishUnit, squad: SkirmishSquad, slot: int, id: int
) -> SkirmishSquad:
	squad.units.erase(unit)
	UnitState.regather(squad)
	var held := HELD.map(func(column): return squad.store.squads.value(slot, column))
	var members: Array[SkirmishUnit] = [unit]
	squad.store.squads.opened = slot
	var stray := SkirmishSquad.new(
		id,
		squad.faction_id,
		squad.direction,
		squad.home_distance,
		1,
		members,
		squad.command,
		squad.store
	)
	stray.route = squad.route
	for n in HELD.size():  # the crate's, as its setters write them
		stray.set(HELD_NAMES[n], held[n])
	return stray

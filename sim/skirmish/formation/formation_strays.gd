class_name FormationStrays
extends RefCounted
## A unit on its own makes for home (Decision 126): one that came to after being downed,
## its formation gone or out of sight. It becomes a lone router of its own (FormationRout):
## it flees home along its formation's route - caught by a friendly formation it runs into,
## it joins it; struck, it may be downed again or surrender; home, it returns to the
## reserve ("fled_home"). Pure. Its setting off and the strays' numbering are the crate's
## (`strays.rs`, spec 39 step 5b); the stray's squad, its command and orders, are made here.

const SkirmishSquad = preload("res://sim/skirmish/formation/skirmish_squad.gd")
const SkirmishUnit = preload("res://sim/skirmish/skirmish_unit.gd")
const SquadEntries = preload("res://sim/skirmish/formation/squad_entries.gd")


## A new routing squad (id 0, for the simulation to number) of `unit` alone, taken out of
## `squad`, fleeing home along `squad`'s route from where it stands.
static func strand(unit: SkirmishUnit, squad: SkirmishSquad) -> SkirmishSquad:
	squad.units.erase(unit)
	UnitState.regather(squad)
	SquadEntries.unloose(squad, unit.id)
	SquadEntries.unflee(squad, unit.id)
	squad.unchase(unit.id)
	var members: Array[SkirmishUnit] = [unit]
	var stray := SkirmishSquad.new(
		0,
		squad.faction_id,
		squad.direction,
		squad.home_distance,
		1,
		members,
		squad.command,
		squad.store
	)
	stray.route = squad.route
	stray.heading = squad.heading
	stray.state = SkirmishSquad.State.ROUTING
	stray.morale = 0
	var columns: UnitColumns = squad.store.columns
	var field: Object = ColumnsLent.field(columns)
	var points: PackedVector2Array = squad.route.points()
	stray.front_distance = ColumnsLent.all(
		columns, func(): return field.stray_set_off(stray.slot, unit.id, unit.slot, points)
	)
	return stray


## Sets each of `strays` ([[unit, its squad], ...]) off home alone, as squads numbered from
## `next_id` added to `squads`; returns the next free squad id. Strays setting off together
## are numbered in their units' seeded draws' order, never the list's (Decision 97): a
## squad's id salts its own draws.
static func adopt(strays: Array, squads: Array, next_id: int, fight_seed: int) -> int:
	if strays.is_empty():
		return next_id
	var ids := PackedInt64Array()
	for stray in strays:
		ids.append(stray[0].id)
	var order: PackedInt32Array = strays[0][1].store.field().strays_numbered(ids, fight_seed)
	for index in order:
		var lone := strand(strays[index][0], strays[index][1])
		lone.id = next_id
		next_id += 1
		squads.append(lone)
	return next_id

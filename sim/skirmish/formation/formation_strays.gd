class_name FormationStrays
extends RefCounted
## A unit on its own makes for home (Decision 126): one that came to after being downed,
## its formation gone or out of sight. It becomes a lone router of its own (FormationRout):
## it flees home along its formation's route - caught by a friendly formation it runs into,
## it joins it; struck, it may be downed again or surrender; home, it returns to the
## reserve ("fled_home"). Pure.

const SkirmishSquad = preload("res://sim/skirmish/formation/skirmish_squad.gd")
const SkirmishUnit = preload("res://sim/skirmish/skirmish_unit.gd")
const MapLayoutDef = preload("res://content/definitions/map_layout_def.gd")
const ScrumContest = preload("res://sim/skirmish/formation/scrum_contest.gd")
const UnitColumns = preload("res://sim/skirmish/formation/unit_columns.gd")
const SquadEntries = preload("res://sim/skirmish/formation/squad_entries.gd")
const EntryKeys = preload("res://sim/skirmish/formation/entry_keys.gd")


## A new routing squad (id 0, for the simulation to number) of `unit` alone, taken out of
## `squad`, fleeing home along `squad`'s route from where it stands.
static func strand(unit: SkirmishUnit, squad: SkirmishSquad) -> SkirmishSquad:
	squad.units.erase(unit)
	SquadEntries.unloose(squad, unit.id)
	SquadEntries.unflee(squad, unit.id)
	squad.chasers.erase(unit.id)
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
	var at: Vector2 = squad.store.columns.value(unit.slot, UnitColumns.POSITION)
	var along: float = squad.route.distance_of(at)
	stray.front_distance = along / MapLayoutDef.CELLS_PER_TILE
	stray.heading = squad.heading
	stray.state = SkirmishSquad.State.ROUTING
	stray.morale = 0
	var offset: Vector2 = at - squad.route.point_at(along)
	# it walks home straight, not fanning out as a panicked rout does (RoutFlight)
	SquadEntries.flee(stray, unit, along, offset)
	EntryKeys.keep(stray.store.columns, unit.slot, UnitColumns.FANNED, INF)
	UnitPlace.put(stray, unit, Vector2i.ZERO)
	return stray


## Sets each of `strays` ([[unit, its squad], ...]) off home alone, as squads numbered from
## `next_id` added to `squads`; returns the next free squad id. Strays setting off together
## are numbered in their units' seeded draws' order, never the list's (Decision 97): a
## squad's id salts its own draws.
static func adopt(strays: Array, squads: Array, next_id: int, fight_seed: int) -> int:
	var ordered := strays.duplicate()
	ordered.sort_custom(
		func(a, b): return ScrumContest.draw(a[0], fight_seed) < ScrumContest.draw(b[0], fight_seed)
	)
	for stray in ordered:
		var lone := strand(stray[0], stray[1])
		lone.id = next_id
		next_id += 1
		squads.append(lone)
	return next_id

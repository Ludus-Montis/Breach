extends GdUnitTestSuite
## The strays through the groups as one crate call (FormationMustering.step, `mustering_step`,
## spec 39 step 8b5-xxxix): a stray set off near home leaves GDScript's copies and the events
## as the strays, morale, the rout, carrying, the strays again and the groups called one
## after another did; its squad, home and scattered within the same call, stays destroyed in
## GDScript's copy.

const FormationSimulation = preload("res://sim/skirmish/formation/formation_simulation.gd")
const FormationMustering = preload("res://sim/skirmish/formation/formation_mustering.gd")
const FormationStrays = preload("res://sim/skirmish/formation/formation_strays.gd")
const FormationMorale = preload("res://sim/skirmish/formation/formation_morale.gd")
const FormationRout = preload("res://sim/skirmish/formation/formation_rout.gd")
const FormationCarry = preload("res://sim/skirmish/formation/formation_carry.gd")
const FormationGroups = preload("res://sim/skirmish/formation/formation_groups.gd")
const FormationRoute = preload("res://sim/skirmish/formation/formation_route.gd")
const SkirmishSquad = preload("res://sim/skirmish/formation/skirmish_squad.gd")
const SkirmishUnit = preload("res://sim/skirmish/skirmish_unit.gd")
const UnitDef = preload("res://content/definitions/unit_def.gd")

const TICK := 0.1
const PACE := 8.0  # cells a second at speed 1 (FormationSimulation.TRAVEL_SCALE)


## A player's squad of two holding at home, east along y = 10; its first unit a third of a
## cell along. Returns the sim and that unit as a stray, [[unit, its squad]].
func _straying() -> Array:
	var sim := FormationSimulation.new(1.0, TICK)
	sim.fight_seed = 5
	var east := FormationRoute.new(PackedVector2Array([Vector2(0, 10), Vector2(64, 10)]))
	var unit_def := UnitDef.new()
	unit_def.speed = 1.0
	var placements := [[unit_def, Vector2i(0, 0)], [unit_def, Vector2i(1, 0)]]
	var squad := sim.spawn_squad(2, placements, "player", true, 0, east)
	sim.order(squad.id, SkirmishUnit.Order.HOLD)
	sim._tick = 9  # off the second: morale doesn't step
	var stray: SkirmishUnit = squad.living()[0]
	sim.store.columns.put(stray.slot, UnitColumns.POSITION, Vector2(0.3, 10))
	return [sim, [[stray, squad]]]


## The six passes one call each, as FormationSimulation made them.
func _phased(sim: FormationSimulation, strays: Array, events: Array) -> void:
	var squads := sim._squads
	var seed := sim.fight_seed
	var tick := sim._tick
	var next_id := FormationStrays.adopt(strays, squads, sim._next_squad_id, seed)
	FormationMorale.step(squads, tick, 10, events)
	events.append_array(FormationRout.step(squads, tick, PACE, TICK, null, seed))
	var bearers := FormationCarry.step(squads, tick, seed, events)
	next_id = FormationStrays.adopt(bearers, squads, next_id, seed)
	sim._next_squad_id = FormationGroups.step(squads, tick, seed, events, next_id)


func _whole(sim: FormationSimulation, strays: Array, events: Array) -> void:
	var at := [sim._tick, 10, sim.fight_seed, sim._next_squad_id]
	var paces := [PACE, TICK]
	sim._next_squad_id = FormationMustering.step(sim._squads, strays, at, paces, null, events)


## Each squad's id, state, morale and units by id, in GDScript's copy and in the crate.
func _squads(sim: FormationSimulation) -> Array:
	var out := []
	for squad: SkirmishSquad in sim._squads:
		var crate := [
			sim.store.squads.value(squad.slot, SquadColumns.STATE),
			sim.store.squads.value(squad.slot, SquadColumns.MORALE)
		]
		var ids := squad.units.map(func(u): return u.id)
		out.append([squad.id, squad.state, squad.morale, crate, ids])
	return out


func test_the_one_call_leaves_the_copies_and_events_as_the_passes_one_by_one() -> void:
	var phased: Array = _straying()
	var whole: Array = _straying()
	var events := [[], []]

	_phased(phased[0], phased[1], events[0])
	_whole(whole[0], whole[1], events[1])

	assert_array(events[1]).is_equal(events[0])
	assert_array(_squads(whole[0])).is_equal(_squads(phased[0]))
	assert_int(whole[0]._next_squad_id).is_equal(phased[0]._next_squad_id)
	var stray: SkirmishSquad = whole[0]._squads[1]
	assert_int(stray.state).is_equal(SkirmishSquad.State.DESTROYED)  # home: scattered

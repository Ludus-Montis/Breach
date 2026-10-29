extends GdUnitTestSuite
## The tick's end as one crate call (FormationEnding.step, `ending_step`, spec 39 step
## 8b5-xl): on two squads of three locked in a fight, it leaves the units' columns,
## GDScript's copies and the events as stamina, the bodies' parting, the shuffle and the
## march's sync called one after another did.

const FormationSimulation = preload("res://sim/skirmish/formation/formation_simulation.gd")
const FormationEnding = preload("res://sim/skirmish/formation/formation_ending.gd")
const FormationStamina = preload("res://sim/skirmish/formation/formation_stamina.gd")
const FormationShuffle = preload("res://sim/skirmish/formation/formation_shuffle.gd")
const FormationMarch = preload("res://sim/skirmish/formation/formation_march.gd")
const FormationRoute = preload("res://sim/skirmish/formation/formation_route.gd")
const UnitBodies = preload("res://sim/skirmish/formation/unit_bodies.gd")
const SkirmishSquad = preload("res://sim/skirmish/formation/skirmish_squad.gd")
const SkirmishUnit = preload("res://sim/skirmish/skirmish_unit.gd")
const UnitDef = preload("res://content/definitions/unit_def.gd")
const WeaponDef = preload("res://content/definitions/weapon_def.gd")
const MapLayoutDef = preload("res://content/definitions/map_layout_def.gd")

const TICK := 0.1
const SCALE := FormationSimulation.TRAVEL_SCALE
const PACE := SCALE * MapLayoutDef.CELLS_PER_TILE  # cells a second at speed 1
const COLUMNS := [UnitColumns.POSITION, UnitColumns.STAMINA, UnitColumns.RANK]


func _def() -> UnitDef:
	var unit_def := UnitDef.new()
	unit_def.hp = 100000
	unit_def.speed = 1.0
	unit_def.items = [WeaponDef.innate_weapon(1)]
	return unit_def


func _placements() -> Array:
	return [[_def(), Vector2i(0, 0)], [_def(), Vector2i(0, 1)], [_def(), Vector2i(0, 2)]]


## A player squad of three in a column marching east into a kingdom one holding, stepped
## until both fight and a few ticks more; then each unit pushed a cell and a half back off
## its place with half its stamina, the first squad's first two on one point; the log emptied.
func _fight() -> FormationSimulation:
	var sim := FormationSimulation.new(1.0, TICK)
	sim.fight_seed = 5
	var east := FormationRoute.new(PackedVector2Array([Vector2(0, 10), Vector2(64, 10)]))
	var west := FormationRoute.new(PackedVector2Array([Vector2(40, 10), Vector2(0, 10)]))
	sim.spawn_squad(1, _placements(), "player", true, 0, east)
	var line := sim.spawn_squad(1, _placements(), "the_kingdom", true, 0, west)
	sim.order(line.id, SkirmishUnit.Order.HOLD)
	for _i in range(600):
		sim.step()
		if sim.squads().all(func(s): return s.state == SkirmishSquad.State.FIGHTING):
			break
	for _i in range(5):
		sim.step()
	var columns: UnitColumns = sim.store.columns
	var first: Array = sim.squads()[0].living()
	for squad in sim.squads():
		for unit: SkirmishUnit in squad.living():
			var at: Vector2 = columns.value(unit.slot, UnitColumns.POSITION)
			columns.put(unit.slot, UnitColumns.POSITION, at + Vector2(-1.5, 0.5))
			columns.put(unit.slot, UnitColumns.STAMINA, 50.0)
	var under: Vector2 = columns.value(first[0].slot, UnitColumns.POSITION)
	columns.put(first[1].slot, UnitColumns.POSITION, under)  # lying on each other
	CrateEvents.taken(sim._field)
	return sim


## The four passes one call each, as FormationSimulation made them.
func _phased(sim: FormationSimulation) -> Array:
	var squads := sim.squads()
	FormationStamina.step(squads, TICK)
	UnitBodies.step(squads, sim.fight_seed, sim._field)
	var events := FormationShuffle.step(squads, TICK, SCALE, sim._tick)
	FormationMarch.sync_units(squads, [TICK, PACE], sim.terrain, sim.fight_seed)
	return events


func _whole(sim: FormationSimulation) -> Array:
	var at := [sim._tick, sim.fight_seed]
	return FormationEnding.step(sim.squads(), at, [TICK, SCALE, PACE], sim.terrain)


## Each unit's place, stamina and rank in the crate, and its fresh speed in GDScript's copy.
func _units(sim: FormationSimulation) -> Array:
	var out := []
	for squad in sim.squads():
		for unit: SkirmishUnit in squad.units:
			var row: Array = COLUMNS.map(func(c): return sim.store.columns.value(unit.slot, c))
			out.append(row + [unit.fresh_speed])
	return out


func test_the_one_call_leaves_the_columns_copies_and_events_as_the_passes_one_by_one() -> void:
	var phased := _fight()
	var whole := _fight()
	var events := [[], []]

	for _i in range(5):
		events[0].append_array(_phased(phased))
		events[1].append_array(_whole(whole))

	assert_array(_units(phased)).is_not_equal(_units(_fight()))
	assert_array(_units(whole)).is_equal(_units(phased))
	assert_array(events[1]).is_equal(events[0])

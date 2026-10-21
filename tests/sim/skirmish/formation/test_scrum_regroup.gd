extends GdUnitTestSuite
## Regroup bounded (spec 38 step 1b, Decision 150): the hostile index a drilled retreat's
## units look through is built once a tick for each faction, and the walk back holds and
## reads the store's columns once for every regrouping squad, not once per squad.

const FormationSimulation = preload("res://sim/skirmish/formation/formation_simulation.gd")
const FormationRoute = preload("res://sim/skirmish/formation/formation_route.gd")
const ScrumRegroup = preload("res://sim/skirmish/formation/scrum_regroup.gd")
const SquadEntries = preload("res://sim/skirmish/formation/squad_entries.gd")
const SkirmishSquad = preload("res://sim/skirmish/formation/skirmish_squad.gd")
const SkirmishUnit = preload("res://sim/skirmish/skirmish_unit.gd")
const UnitDef = preload("res://content/definitions/unit_def.gd")
const UnitPlaces = preload("res://tests/sim/skirmish/formation/unit_places.gd")
const UnitColumns = preload("res://sim/skirmish/formation/unit_columns.gd")
const CountedColumns = preload("res://tests/sim/skirmish/formation/counted_columns.gd")

const TICK := 0.1
const SEED := 7


func _def() -> UnitDef:
	var unit_def := UnitDef.new()
	unit_def.hp = 400
	unit_def.items = [WeaponDef.innate_weapon(1)]
	unit_def.speed = 1.0
	unit_def.discipline = 80  # it meets threats: a retreat backs away facing its foes
	return unit_def


func _row(columns: int) -> Array:
	var placements := []
	for column in range(columns):
		placements.append([_def(), Vector2i(0, column)])
	return placements


## A drilled player squad of 4 retreating at `y`, every unit 2 cells out of its place.
func _retreating(sim: FormationSimulation, y: float) -> SkirmishSquad:
	var lane := FormationRoute.new(PackedVector2Array([Vector2(20, y), Vector2(60, y)]))
	var squad := sim.spawn_squad(4, _row(4), "player", true, 0, lane)
	squad.order = SkirmishUnit.Order.RETREAT
	for unit in squad.living():
		SquadEntries.loosen(squad, unit, UnitPlaces.of(squad, unit) + Vector2(2, 0))
	return squad


## [the battle's squads, the player squads retreating]: `count` of them, and a kingdom
## squad close by.
func _battle(count: int) -> Array:
	var sim := FormationSimulation.new(1.0, TICK)
	var retreating := []
	for index in range(count):
		retreating.append(_retreating(sim, 20.0 + 6.0 * index))
	var lane := FormationRoute.new(PackedVector2Array([Vector2(24, 23), Vector2(0, 23)]))
	sim.spawn_squad(4, _row(4), "the_kingdom", true, 0, lane)
	return [sim.squads(), retreating]


## Each column's whole-column reads in one regroup of a battle with `count` retreating
## squads, and where their units stand after it.
func _regrouped(count: int) -> Array:
	var battle := _battle(count)
	var squads: Array = battle[0]
	var store: UnitStore = squads[0].store
	var counted := CountedColumns.new(store.columns)
	store.columns = counted
	ScrumRegroup.step(squads, 1.0, TICK, SEED)
	store.columns = counted.inner
	var at := []
	for squad in battle[1]:
		for unit in squad.living():
			at.append(SquadEntries.loose_at(squad, unit) if squad.loose.has(unit.id) else null)
	return [counted.reads, at]


func test_two_regrouping_squads_of_one_side_read_the_columns_as_one_does() -> void:
	var one: Dictionary = _regrouped(1)[0]
	var two: Dictionary = _regrouped(2)[0]

	for name in [
		UnitColumns.POSITION,
		UnitColumns.LOOSE_AT,
		UnitColumns.NEXT,
		UnitColumns.BEARING,
		UnitColumns.RANK,
		UnitColumns.COLUMN,
		UnitColumns.SPEED,
	]:
		(
			assert_int(two.get(name, 0))
			. override_failure_message(
				"%s: %d, one: %d" % [name, two.get(name, 0), one.get(name, 0)]
			)
			. is_equal(one.get(name, 0))
		)


func test_each_unit_walks_back_as_its_squad_alone_would() -> void:
	var walked: Array = _regrouped(2)[1]

	assert_int(walked.size()).is_equal(8)
	for at in walked:
		assert_object(at).is_not_null()  # still loose, walking back
	assert_array(walked.slice(0, 4)).is_equal(_regrouped(1)[1])

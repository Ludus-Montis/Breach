extends GdUnitTestSuite
## Body parting through Godot (BodyParting, Decision 129): the field brought to the squads
## as they stand - framed, loose and fleeing bodies, friends and foes, bodies lying on each
## other - parts them the same on a field kept across ticks while units die and change
## squad as on a new field each tick, and nothing turns on list order. The pass's own rules
## are the crate's (native/rust/src/parting/tests.rs, spec 39 step 6a).

const NativeKernels = preload("res://sim/skirmish/formation/native_kernels.gd")
const UnitBodies = preload("res://sim/skirmish/formation/unit_bodies.gd")
const SkirmishSquad = preload("res://sim/skirmish/formation/skirmish_squad.gd")
const SkirmishUnit = preload("res://sim/skirmish/skirmish_unit.gd")

const UnitPlaces = preload("res://tests/sim/skirmish/formation/unit_places.gd")
const SquadEntries = preload("res://sim/skirmish/formation/squad_entries.gd")
const EntryKeys = preload("res://sim/skirmish/formation/entry_keys.gd")

var _places := UnitPlaces.new()


func before_test() -> void:
	_places = UnitPlaces.new()


func _unit(unit_id: int, size: int = 1) -> SkirmishUnit:
	var unit := SkirmishUnit.new()
	unit.id = unit_id
	_places.health(unit, 10)
	unit.footprint_width = size
	unit.footprint_depth = size
	return unit


## Squads crowding one spot: a framed line with a loose friend and a router among them, a
## brute and grems lying exactly on each other, and a foe pressed in.
func _crowd(reversed: bool) -> Array:
	var units := [_unit(1), _unit(2, 2), _unit(3), _unit(4), _unit(5), _unit(6)]
	var points := [
		Vector2(4, 4),
		Vector2(4, 4),
		Vector2(4.2, 4),
		Vector2(4.6, 4.3),
		Vector2(5, 5),
		Vector2(3, 4)
	]
	var line := SkirmishSquad.new(
		1, "player", 1, 0.0, 4, [] as Array[SkirmishUnit], null, _places.store
	)
	for index in range(4):
		line.units.append(units[index])
		UnitState.regather(line)
		_places.stand(units[index], points[index])
	for index in [1, 2]:
		var at: Vector2 = points[index]
		SquadEntries.loosen(line, units[index], at)
	SquadEntries.flee(line, units[3], 4.0, Vector2(0.6, 4.3))
	var other := SkirmishSquad.new(
		2, "player", 1, 0.0, 1, [units[4]] as Array[SkirmishUnit], null, _places.store
	)
	_places.stand(units[4], points[4])
	var foes := SkirmishSquad.new(
		3, "the_kingdom", 1, 0.0, 1, [units[5]] as Array[SkirmishUnit], null, _places.store
	)
	SquadEntries.loosen(foes, units[5], points[5])
	var squads := [line, other, foes]
	if reversed:
		squads.reverse()
		line.units.reverse()
		UnitState.regather(line)
	return squads


## Every squad's bodies, loose and fleeing entries, as bytes (exact, not approximate).
func _bytes(squads: Array) -> PackedByteArray:
	var state := []
	for squad in squads:
		for unit in squad.units:
			var columns: UnitColumns = squad.store.columns
			var along = columns.value(unit.slot, UnitColumns.ALONG)
			var held := [unit.id, UnitPlaces.of(squad, unit)]
			held.append(along if squad.fleeing.has(unit.id) else null)
			var loose := []
			if squad.loose.has(unit.id):
				loose = [
					SquadEntries.loose_at(squad, unit), columns.value(unit.slot, UnitColumns.NEXT)
				]
				loose.append(EntryKeys.goal(columns, unit.slot))
			held.append(loose)
			state.append(held)
	state.sort_custom(func(a, b): return a[0] < b[0])
	return var_to_bytes(state)


## The crowd parted for four ticks while a unit dies and another changes squad (after the
## second): on one field kept across them (`kept`), or on a new one each tick.
func _crowd_parted(reversed: bool, kept: bool) -> PackedByteArray:
	_places = UnitPlaces.new()  # each run its own store: no bit of another run's state
	var squads := _crowd(reversed)
	var field := NativeKernels.body_field()
	UnitPlaces.onto(field, squads)
	for tick in range(4):
		if tick == 2:
			_churn(squads)
		if not kept:
			field = NativeKernels.body_field()
			UnitPlaces.onto(field, squads)
		UnitBodies.step(squads, 9 + tick, field)
	return _bytes(squads)


## Unit 3 dies; unit 5 leaves its squad for the line (squad 1), its entries in its old
## squad taken away first, as every move between squads does (FormationRecovery, FormationGroups,
## FormationStrays): left behind, the store's flags would still call it loose while the
## line's `loose` holds no entry for it, and the field would part a unit in a state the
## battle never makes.
func _churn(squads: Array) -> void:
	var by_id := {}
	for squad in squads:
		by_id[squad.id] = squad
		for unit in squad.living():
			if unit.id == 3:
				UnitState.put(squad, unit, SkirmishUnit.State.DEAD)
	var mover: SkirmishUnit = by_id[2].units[0]
	SquadEntries.unloose(by_id[2], mover.id)
	SquadEntries.unflee(by_id[2], mover.id)
	by_id[2].units.erase(mover)
	UnitState.regather(by_id[2])
	by_id[1].units.append(mover)
	UnitState.regather(by_id[1])


func test_a_field_kept_across_ticks_parts_as_a_new_one_does_as_units_die_and_move() -> void:
	var fresh := _crowd_parted(false, false)
	assert_array(Array(_crowd_parted(false, true))).is_equal(Array(fresh))


func test_a_battle_has_a_field() -> void:
	var sim := FormationSimulation.new(9.0, 0.1)
	assert_object(sim.store.field()).is_not_null()


func test_bodies_part_the_same_whatever_the_list_order() -> void:
	for kept in [false, true]:
		var forward := _crowd_parted(false, kept)
		assert_array(Array(_crowd_parted(true, kept))).is_equal(Array(forward))

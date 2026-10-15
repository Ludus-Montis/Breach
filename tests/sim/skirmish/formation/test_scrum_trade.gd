extends GdUnitTestSuite
## Trading places while regrouping, per Decision 110: a unit held off its place for
## scrum_trade_seconds trades places with the interchangeable friend whose place is nearest it;
## with none, it takes its own place there and then.

const BattleTuning = preload("res://content/definitions/battle_tuning.gd")
const ScrumTrade = preload("res://sim/skirmish/formation/scrum_trade.gd")
const ScrumStance = preload("res://sim/skirmish/formation/scrum_stance.gd")
const SkirmishSquad = preload("res://sim/skirmish/formation/skirmish_squad.gd")
const SkirmishUnit = preload("res://sim/skirmish/skirmish_unit.gd")
const UnitDef = preload("res://content/definitions/unit_def.gd")
const UnitPlaces = preload("res://tests/sim/skirmish/formation/unit_places.gd")
const SquadEntries = preload("res://sim/skirmish/formation/squad_entries.gd")
const EntryKeys = preload("res://sim/skirmish/formation/entry_keys.gd")

var _places := UnitPlaces.new()


func before_test() -> void:
	_places = UnitPlaces.new()


func _unit(unit_id: int, rank: int, kind: UnitDef) -> SkirmishUnit:
	var unit := SkirmishUnit.new()
	unit.id = unit_id
	_places.health(unit, 10)
	_places.set_place(unit, Vector2i(rank, 0))
	unit.definition = kind
	return unit


## A file of two, front and rear; the rear unit stands loose a cell in front of the front
## one, held there - its own front rank in the way.
func _held_in_front(front_kind: UnitDef, rear_kind: UnitDef) -> Array:
	var front := _unit(1, 0, front_kind)
	var rear := _unit(2, 1, rear_kind)
	var squad := SkirmishSquad.new(
		1, "player", 1, 0.0, 1, [front, rear] as Array[SkirmishUnit], null, _places.store
	)
	_places.stand(front, ScrumStance.anchor(squad, front))
	var at := _places.at(front) + (_places.at(front) - ScrumStance.anchor(squad, rear))
	SquadEntries.loosen(squad, rear, at)
	EntryKeys.keep(
		squad.store.columns,
		rear.slot,
		UnitColumns.STUCK,
		BattleTuning.current().scrum_trade_seconds
	)
	return [squad, front, rear]


func test_a_unit_held_off_by_its_own_ranks_trades_places_with_a_like_friend() -> void:
	var kind := UnitDef.new()
	var setup := _held_in_front(kind, kind)

	ScrumTrade.trade(setup[0], 7)

	# the held unit takes the front place, nearer it; its friend steps back into the one it left
	assert_int(UnitPlace.rank(setup[0], setup[2])).is_equal(0)
	assert_int(UnitPlace.rank(setup[0], setup[1])).is_equal(1)
	assert_bool(setup[0].loose.has(setup[1].id)).is_true()


func test_a_unit_with_no_like_friend_takes_its_own_place() -> void:
	var setup := _held_in_front(UnitDef.new(), UnitDef.new())  # two kinds: no trade

	ScrumTrade.trade(setup[0], 7)

	assert_int(UnitPlace.rank(setup[0], setup[2])).is_equal(1)
	assert_bool(setup[0].loose.has(setup[2].id)).is_false()
	assert_vector(_places.at(setup[2])).is_equal(ScrumStance.anchor(setup[0], setup[2]))


func test_a_unit_not_yet_held_long_enough_does_not_trade() -> void:
	var kind := UnitDef.new()
	var setup := _held_in_front(kind, kind)
	EntryKeys.keep(
		setup[0].store.columns,
		setup[2].slot,
		UnitColumns.STUCK,
		BattleTuning.current().scrum_trade_seconds / 2.0
	)

	ScrumTrade.trade(setup[0], 7)

	assert_int(UnitPlace.rank(setup[0], setup[2])).is_equal(1)
	assert_bool(setup[0].loose.has(setup[2].id)).is_true()

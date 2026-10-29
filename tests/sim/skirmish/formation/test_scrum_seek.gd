extends GdUnitTestSuite
## The scrum's seek, walk and facing, one call into the crate over the scrum's roster
## (ScrumSeek.step, spec 39 step 8b5-xiii): units within reach of a foe touch it, and their
## squads count as active for the stall.

const FormationSimulation = preload("res://sim/skirmish/formation/formation_simulation.gd")
const FormationScrum = preload("res://sim/skirmish/formation/formation_scrum.gd")
const ScrumEngage = preload("res://sim/skirmish/formation/scrum_engage.gd")
const ScrumSeek = preload("res://sim/skirmish/formation/scrum_seek.gd")
const EntryKeys = preload("res://sim/skirmish/formation/entry_keys.gd")
const SkirmishSquad = preload("res://sim/skirmish/formation/skirmish_squad.gd")
const SkirmishUnit = preload("res://sim/skirmish/skirmish_unit.gd")
const UnitDef = preload("res://content/definitions/unit_def.gd")
const UnitPlaces = preload("res://tests/sim/skirmish/formation/unit_places.gd")

const TICK := 0.1


func _one() -> Array:
	var unit_def := UnitDef.new()
	unit_def.hp = 400
	unit_def.items = [WeaponDef.innate_weapon(1)]
	unit_def.speed = 1.0
	return [[unit_def, Vector2i(0, 0)]]


func test_units_within_reach_touch_and_keep_their_fight_active() -> void:
	var sim := FormationSimulation.new(9.0, TICK)
	var mine := sim.spawn_squad(1, _one(), "player", true)
	var theirs := sim.spawn_squad(1, _one(), "the_kingdom", false)
	var near := UnitPlaces.of(mine, mine.units[0]) + Vector2(0.0, 1.2)
	UnitPlaces.put(theirs, theirs.units[0], near)  # beside it: within reach, not front to front
	var roster := CrateRoster.of(sim.squads())
	ScrumEngage.step(sim._field, roster, 4, sim.fight_seed)
	FormationScrum._prepare(sim._field, roster, 4)
	var ctx := {"tick": 4, "seed": sim.fight_seed, "seconds": TICK, "pace": 0.8}
	ctx.merge({"terrain": null, "active": {}})

	ScrumSeek.step(sim._field, roster, ctx)

	var active: Array = ctx["active"].keys()
	active.sort()
	assert_array(active).is_equal([mine.id, theirs.id])
	var touching := []
	for squad: SkirmishSquad in [mine, theirs]:
		var unit: SkirmishUnit = squad.units[0]
		var entry := EntryKeys.loose_entry(squad.store.columns, squad.loose[unit.id], Vector2.ZERO)
		touching.append(entry.get("touch"))
	assert_array(touching).is_equal([true, true])

extends GdUnitTestSuite
## FormationLane, per specs/22-formation-feel-test.md and Decisions 42-45: one lane's
## simulation, the player's wave and painted template, and the kingdom's wave. The lane
## doesn't build - the domains fill its waves (FormationBattle) - but every reshape goes
## through it and hands back the units it displaces.

const FormationLane = preload("res://sim/skirmish/formation/formation_lane.gd")
const WaveTemplate = preload("res://sim/skirmish/formation/wave_template.gd")
const WavePresets = preload("res://sim/skirmish/formation/wave_presets.gd")

const GREM := "grem"
const BRUTE := "grem_brute"
const MILITIA := "kingdom_militia"


func _lane() -> FormationLane:
	var lane := FormationLane.new("c", 5, 9.0, 0.1, CrateContent.unit(MILITIA), 3)
	lane.apply(WavePresets.line(CrateContent.unit(GREM), 5, 5))
	return lane


func _fill(lane: FormationLane, count: int) -> void:
	for i in range(count):
		lane.production.fill(CrateContent.unit(GREM))


func test_painting_a_brute_over_filled_grems_hands_back_the_grems_it_displaces() -> void:
	var lane := _lane()
	_fill(lane, 5)
	lane.brush = CrateContent.unit(BRUTE)

	var leftovers := lane.paint(Vector2i(0, 1), 8)

	var grem := CrateContent.unit(GREM)
	assert_array(leftovers).is_equal([grem, grem])  # it replaces columns 1 and 2


func test_painting_off_the_lane_changes_nothing() -> void:
	var lane := _lane()

	assert_array(lane.paint(Vector2i(0, 7), 8)).is_empty()
	assert_int(lane.template.ordered().size()).is_equal(5)


func test_erasing_with_no_brush_clears_the_cell() -> void:
	var lane := _lane()
	lane.brush = null

	lane.paint(Vector2i(0, 0), 8)

	assert_int(lane.template.ordered().size()).is_equal(4)


func test_painting_is_limited_by_the_allowance() -> void:
	var lane := _lane()
	lane.brush = CrateContent.unit(GREM)

	lane.paint(Vector2i(1, 0), 5)  # 5 cells used, none spare
	assert_int(lane.cells_used()).is_equal(5)
	lane.paint(Vector2i(1, 0), 6)
	assert_int(lane.cells_used()).is_equal(6)


func test_the_kingdom_wave_steps_only_when_asked() -> void:
	var lane := _lane()
	for i in range(3):
		lane.kingdom.fill(CrateContent.unit(MILITIA))

	lane.step(false)
	var before := lane.sim.squads().size()
	lane.step(true)  # the kingdom's wave departs automatically once full

	assert_int(before).is_equal(0)
	assert_bool(lane.sim.squads().any(func(s): return s.faction_id == "the_kingdom")).is_true()


func test_auto_departure_sends_the_players_wave_once_full() -> void:
	var lane := _lane()
	lane.set_auto_departure(true)
	_fill(lane, 5)

	var events := lane.step(false)

	assert_bool(events.any(func(e): return e["type"] == "departed")).is_true()

extends GdUnitTestSuite
## The fight's melee blows on the native core (ScrumBlowsField, Decision 129): at moments
## through a 40-a-side clash, who strikes whom (FormationMelee.blows) and how each blow
## lands, rolled (BlowLanding.land), come out on the field exactly as in GDScript - the
## same blows in the same order, landed the same, and every unit left with the same
## target and cooldown - with bearings stirred, a squad wavering and one retreating, one
## only striking the retreat, and whatever the list order. Skipped where the library
## isn't built (native/build.sh): the switch falls back to GDScript.

const NativeKernels = preload("res://sim/skirmish/formation/native_kernels.gd")
const FormationBench = preload("res://sim/skirmish/formation/formation_bench.gd")
const FormationMelee = preload("res://sim/skirmish/formation/formation_melee.gd")
const BlowLanding = preload("res://sim/skirmish/formation/blow_landing.gd")
const ScrumBlowsField = preload("res://sim/skirmish/formation/scrum_blows_field.gd")
const SkirmishSquad = preload("res://sim/skirmish/formation/skirmish_squad.gd")
const SkirmishUnit = preload("res://sim/skirmish/skirmish_unit.gd")
const UnitPlaces = preload("res://tests/sim/skirmish/formation/unit_places.gd")

const SEED := 2
const TICK := 7


func after_test() -> void:
	NativeKernels.use(NativeKernels.GDSCRIPT)


func _field() -> Object:
	NativeKernels.use(NativeKernels.RUST)
	var field := NativeKernels.body_field()
	NativeKernels.use(NativeKernels.GDSCRIPT)
	if field == null:
		print("test_blows_field: the Rust core isn't built - skipped")
	return field


## A 40-a-side clash's squads after `ticks` ticks under GDScript, the lists reversed if
## asked.
func _squads(ticks: int, reversed: bool) -> Array:
	var sim := FormationBench.clash(40, 10, SEED)
	sim.blow_rolls = true
	for _tick in range(ticks):
		sim.step()
	var squads: Array = sim.squads().duplicate()
	if reversed:
		squads.reverse()
		for squad in squads:
			squad.units.reverse()
	return squads


## One fight's melee on `field` (null: GDScript), from the state as it stands: [the blows
## as [striker, target, damage, flank, landed] by ids, every unit's [id, target,
## cooldown, last wounding, regeneration halt]].
func _struck(squads: Array, field: Object) -> Array:
	if field != null:
		UnitPlaces.onto(field, squads)  # its passes read the store's columns on the field
	var blows := FormationMelee.blows(squads, 10, SEED, field)
	BlowLanding.land(blows, [], squads, null, [SEED, TICK], field)
	var units := []
	for squad in squads:
		for unit in squad.units:
			var cooldown: int = squad.store.columns.value(unit.slot, UnitColumns.ATTACK_COOLDOWN)
			var left := [unit.target_id, cooldown, unit.last_wounding]
			units.append([unit.id] + left + [unit.regeneration_halt])
	return [blows.map(func(b): return [b[0].id, b[1].id, b[2], b[3], b[4]]), units]


func _saved(squads: Array) -> Array:
	var out := []
	for squad in squads:
		for unit in squad.units:
			var cooldown: int = squad.store.columns.value(unit.slot, UnitColumns.ATTACK_COOLDOWN)
			out.append([unit.target_id, cooldown, unit.last_wounding])
	return out


func _restore(squads: Array, saved: Array) -> void:
	var index := 0
	for squad in squads:
		for unit in squad.units:
			unit.target_id = saved[index][0]
			squad.store.columns.put(unit.slot, UnitColumns.ATTACK_COOLDOWN, saved[index][1])
			unit.last_wounding = saved[index][2]
			unit.regeneration_halt = 0.0
			index += 1


## Both engines from the same state; returns [GDScript's, the field's].
func _both(squads: Array, field: Object) -> Array:
	var saved := _saved(squads)
	var reference := _struck(squads, null)
	_restore(squads, saved)
	return [reference, _struck(squads, field)]


## Every unit due to strike now or next tick, and turned to a bearing of its own.
func _stirred(squads: Array) -> Array:
	for squad in squads:
		for unit in squad.units:
			squad.store.columns.put(unit.slot, UnitColumns.ATTACK_COOLDOWN, 1 + unit.id % 2)
			UnitPlaces.turn(squad, unit, fposmod(unit.id * 47.3, 360.0))
	return squads


func test_the_blows_on_the_field_are_gdscript_s() -> void:
	var field := _field()
	if field == null:
		return
	var struck := 0
	var surrounded := 0
	for ticks in [95, 104, 120, 140]:  # moments while the scrum is under way
		var squads := _squads(ticks, false)
		var both := _both(squads, field)
		assert_array(Array(var_to_bytes(both[1]))).is_equal(Array(var_to_bytes(both[0])))
		struck += both[0][0].size()
		surrounded = maxi(surrounded, _assert_pressed(squads, field))
	assert_int(struck).is_greater(0)
	assert_int(surrounded).is_greater(1)  # some units are fought by several foes


## The field counts the foes pressing each unit as BlowLanding._pressed does (by the
## targets the units have now), and knows every unit's squad; returns the most pressing
## any one unit.
func _assert_pressed(squads: Array, field: Object) -> int:
	var squad_of := {}
	var everyone := []
	for squad in squads:
		for unit in squad.living():
			squad_of[unit.id] = [unit, squad]
			everyone.append([unit, unit])
	var reference := BlowLanding._pressed(squad_of)
	UnitPlaces.onto(field, squads)
	FormationMelee.blows(squads, 10, SEED, field)  # syncs the field
	var whose := ScrumBlowsField.pressed(field, squads, everyone)
	assert_bool(whose[0] == squad_of).override_failure_message("squads differ").is_true()
	assert_bool(whose[1] == reference).override_failure_message("counts differ").is_true()
	return reference.values().max() if not reference.is_empty() else 0


func test_the_blows_on_the_field_are_gdscript_s_from_every_side() -> void:
	var field := _field()
	if field == null:
		return
	for ticks in [104, 130]:
		var squads := _squads(ticks, false)
		_stirred(squads)
		var both := _both(squads, field)
		var flanks: Array = both[0][0].filter(func(b): return b[3])
		assert_int(flanks.size()).is_greater(0)
		assert_int(both[0][0].size()).is_greater(flanks.size())
		assert_array(Array(var_to_bytes(both[1]))).is_equal(Array(var_to_bytes(both[0])))


func test_the_blows_on_the_field_are_gdscript_s_in_a_retreat_and_a_wavering() -> void:
	var field := _field()
	if field == null:
		return
	var squads := _squads(120, false)
	_stirred(squads)
	var order: int = squads[0].order
	squads[0].order = SkirmishUnit.Order.RETREAT  # strikes only foes still in its front
	squads[1].morale = 1  # wavering: a longer interval, a lower margin
	var both := _both(squads, field)
	assert_int(both[0][0].size()).is_greater(0)
	assert_array(Array(var_to_bytes(both[1]))).is_equal(Array(var_to_bytes(both[0])))
	squads[1].state = SkirmishSquad.State.HOLDING  # strikes only the retreat
	both = _both(squads, field)
	assert_int(both[0][0].size()).is_greater(0)
	assert_array(Array(var_to_bytes(both[1]))).is_equal(Array(var_to_bytes(both[0])))
	squads[0].order = order  # neither fights nor retreats: no blows
	squads[0].state = SkirmishSquad.State.HOLDING
	both = _both(squads, field)
	assert_array(both[0][0]).is_empty()
	assert_array(Array(var_to_bytes(both[1]))).is_equal(Array(var_to_bytes(both[0])))


func test_the_blows_on_the_field_are_the_same_whatever_the_list_order() -> void:
	var field := _field()
	if field == null:
		return
	var reference := _by_id(_struck(_stirred(_squads(120, false)), null))
	var forward := _by_id(_struck(_stirred(_squads(120, false)), field))
	var backward := _by_id(_struck(_stirred(_squads(120, true)), _field()))
	assert_array(reference[0]).is_not_empty()
	assert_array(Array(var_to_bytes(forward))).is_equal(Array(var_to_bytes(reference)))
	assert_array(Array(var_to_bytes(backward))).is_equal(Array(var_to_bytes(reference)))


## The outcome with the blows and units by id: what list order may not change.
func _by_id(struck: Array) -> Array:
	for list in struck:
		list.sort_custom(func(a, b): return a[0] < b[0])
	return struck

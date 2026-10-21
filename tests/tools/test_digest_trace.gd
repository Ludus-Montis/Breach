extends GdUnitTestSuite
## The outcome trace (specs/37-outcome-trace.md, step 1): the digest names every field it
## hashes, and with `only`, `trace`, `from` and `to` writes a run's ticks in that window as
## JSON lines that hold exactly what it hashed.

const DigestState = preload("res://tools/digest_state.gd")
const BattleTrials = preload("res://sim/skirmish/formation/battle_trials.gd")
const FormationSimulation = preload("res://sim/skirmish/formation/formation_simulation.gd")

const RUN := "mirror_headon s1 rtrue"

var _trace := []
var _out := {}
## Each number in the trace as written: Godot's own text-to-float reading is off by an ulp
## on some (it reads "1.8500000000000005" as 1.8500000000000003), Python's and JS's aren't.
var _numbers := RegEx.create_from_string(
	"(?<=[:\\[,])(-?[0-9][0-9.eE+-]*|NaN|-?Infinity)(?=[,\\]}])"
)


## The digest run once for the suite: one mirror run, traced from tick 10 to 15.
func before() -> void:
	var trace_path := OS.get_user_data_dir().path_join("digest_trace_test.jsonl")
	var out_path := OS.get_user_data_dir().path_join("digest_trace_test.txt")
	var args := [
		"--headless",
		"--path",
		ProjectSettings.globalize_path("res://"),
		"--script",
		"res://tools/formation_digest.gd",
		"--",
		"only=" + RUN,
		"trace=" + trace_path,
		"from=10",
		"to=15",
		"out=" + out_path,
	]
	OS.execute(OS.get_executable_path(), args)
	for line in _lines(trace_path):
		_trace.append(JSON.parse_string(_numbers.sub(line, '"#$1"', true)))
	for line in _lines(out_path):
		var parts: PackedStringArray = line.rsplit(" ", true, 2)
		_out["%s %s" % [parts[0], parts[1]]] = parts[2]


func _lines(path: String) -> PackedStringArray:
	var text := FileAccess.get_file_as_string(path)
	return text.strip_edges().split("\n", false)


## The mirror run's sim after `ticks` ticks, and that last tick's events.
func _mirror(ticks: int) -> Array:
	var sim := FormationSimulation.new(2.0, BattleTrials.TICK)
	sim.fight_seed = 1
	sim.blow_rolls = true
	for spawn in BattleTrials._mirror_spawns("mirror_headon", sim):
		spawn.call()
	var events := []
	for _tick in range(ticks):
		events = sim.step()
	return [sim, events]


func test_every_field_has_a_name() -> void:
	var sim: FormationSimulation = _mirror(1)[0]
	var state := DigestState.state(sim.squads(), sim.store)
	assert_bool(state.is_empty()).is_false()
	for squad in state:
		_assert_named(squad, DigestState.SQUAD_FIELDS)
		var units: Array = squad[2][2]
		assert_bool(units.is_empty()).is_false()
		for unit in units:
			_assert_named(unit, DigestState.UNIT_FIELDS)


func _assert_named(values: Array, names: Array) -> void:
	assert_int(values.size()).is_equal(names.size())
	for index in range(names.size()):
		if names[index] is Array:
			_assert_named(values[index], names[index])


func test_the_trace_holds_the_window_and_nothing_else() -> void:
	assert_int(_trace.size()).is_equal(6)
	for index in range(_trace.size()):
		assert_str(_trace[index]["tick"]).is_equal("#%d" % (10 + index))
		assert_str(_trace[index]["run"]).is_equal(RUN)
		assert_bool(_trace[index]["squads"].is_empty()).is_false()


## Each squad's side, for the page to colour by (not hashed: the digest's state leaves it out).
func test_each_squad_says_its_faction() -> void:
	var sim: FormationSimulation = _mirror(10)[0]
	var factions := sim.squads().map(func(s): return s.faction_id)
	var traced: Array = _trace[0]["squads"].map(func(s): return s.get("faction"))
	assert_array(traced).is_equal(factions)
	assert_int(factions.filter(func(f): return f != factions[0]).size()).is_greater(0)


func test_the_trace_says_what_the_digest_hashes() -> void:
	var tick := 12
	var line: Dictionary = _trace[tick - 10]
	var played := _mirror(tick)
	var sim: FormationSimulation = played[0]
	var hashed := [DigestState.plain(played[1]), DigestState.state(sim.squads(), sim.store)]
	var squads := []
	for index in range(line["squads"].size()):
		var squad: Dictionary = line["squads"][index].duplicate()
		squad["units"] = squad["units"].map(func(u): return _unnamed(u, DigestState.UNIT_FIELDS))
		squads.append(_unnamed(squad, DigestState.SQUAD_FIELDS))
	var restored: Variant = _typed([line["events"], squads], hashed)
	var one := HashingContext.new()
	one.start(HashingContext.HASH_SHA256)
	one.update(var_to_bytes(restored))
	var expected: String = _out["%s %d" % [RUN, tick]]
	assert_str(one.finish().hex_encode().substr(0, 16)).is_equal(expected)


## The named fields put back in the digest's order.
func _unnamed(named: Dictionary, names: Array) -> Array:
	var out := []
	for name in names:
		out.append(_unnamed(named, name) if name is Array else named.get(name))
	return out


## The trace's values given the digest's own types (a vector is written as an array, a
## number as its text): a float is the digest's own only when its text reads back to it bit
## for bit (var_to_str's shortest such text), else a value that hashes differently.
func _typed(value: Variant, like: Variant) -> Variant:
	if (like is Vector2 or like is Vector2i) and value is Array and value.size() == 2:
		var x: Variant = _typed(value[0], like.x)
		var y: Variant = _typed(value[1], like.y)
		return Vector2(x, y) if like is Vector2 else Vector2i(x, y)
	if value is String and value.begins_with("#"):
		return _number(value.substr(1), like)
	if like is StringName and value is String:
		return StringName(value)
	if like is Array and value is Array and like.size() == value.size():
		var out := []
		for index in range(value.size()):
			out.append(_typed(value[index], like[index]))
		return out
	return value


func _number(text: String, like: Variant) -> Variant:
	if like is int:
		return int(text)
	return like if like is float and text == var_to_str(like) else text

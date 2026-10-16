class_name FatigueTrials
extends RefCounted
## Does fighting tire a line so it pursues less (spec 28 part 7, Decision 125)? A line of 8
## grems holds its ground; a wave of 8 grems engages it, fights it for a set time, then
## retreats at a run. Both sides are too stout to fall, so the contact lasts as long as it
## is told to. Measured once per battle seed: how far (cells from its post) and how long
## (ticks) the line pursued, its stamina (share of its most) when the wave turned, and
## whether the chase ended with most of it tired. Compare a short contact (fresh) with
## long ones (tired).

const FormationSimulation = preload("res://sim/skirmish/formation/formation_simulation.gd")
const FormationStamina = preload("res://sim/skirmish/formation/formation_stamina.gd")
const SkirmishUnit = preload("res://sim/skirmish/skirmish_unit.gd")
const UnitDef = preload("res://content/definitions/unit_def.gd")

const TICK := 0.1
const LIMIT_TICKS := 3000
const STOUT := 100000


## {"pursued": mean cells, "ticks": mean ticks pursued, "stamina": mean share at the turn,
## "tired": runs whose chase ended with most of the line tired, "runs"}.
static func run(contact_seconds: float, runs: int, first_seed: int = 1) -> Dictionary:
	var out := {"pursued": 0.0, "ticks": 0.0, "stamina": 0.0, "tired": 0, "runs": runs}
	for index in range(runs):
		var one := _battle(contact_seconds, first_seed + index)
		out["pursued"] += one["pursued"] / runs
		out["ticks"] += float(one["ticks"]) / runs
		out["stamina"] += one["stamina"] / runs
		out["tired"] += 1 if one["tired"] else 0
	return out


static func _battle(contact_seconds: float, battle_seed: int) -> Dictionary:
	var sim := FormationSimulation.new(2.0, TICK)
	sim.fight_seed = battle_seed
	sim.blow_rolls = true
	var grem: UnitDef = load("res://content/units/grem.tres").duplicate()
	grem.hp = STOUT
	var wave := sim.spawn_squad(8, _row(grem), "player", true)
	var line := sim.spawn_squad(8, _row(grem), "the_kingdom", false)
	sim.order(line.id, SkirmishUnit.Order.HOLD)
	var result := {"pursued": 0.0, "ticks": 0, "stamina": 1.0, "tired": false}
	var contact := -1
	var turned := -1
	for tick in range(LIMIT_TICKS):
		var events := sim.step()
		if contact < 0 and events.any(func(e): return e["type"] == "engaged"):
			contact = tick
		if turned < 0 and contact >= 0 and tick - contact >= roundi(contact_seconds / TICK):
			turned = tick
			result["stamina"] = _stamina(line)
			wave.hurry = true
			sim.order(wave.id, SkirmishUnit.Order.RETREAT)
		if not line.pursuit.is_empty() and not line.pursuit["returning"]:
			result["ticks"] += 1
			var gone: float = line.position.distance_to(line.pursuit["post_at"])
			result["pursued"] = maxf(result["pursued"], gone)
		if events.any(func(e): return e["type"] == "pursuit_ended" and e["squad"] == line.id):
			result["tired"] = FormationStamina.squad_gives_up(line)
			break
	return result


static func _stamina(squad) -> float:
	var living: Array = squad.living()
	var stamina: PackedFloat64Array = UnitView.read(squad, [UnitColumns.STAMINA]).stamina
	var total: float = living.reduce(
		func(sum, u): return sum + stamina[u.slot] / u.max_stamina, 0.0
	)
	return total / maxi(1, living.size())


static func _row(unit_def: UnitDef) -> Array:
	var placements := []
	for column in range(8):
		placements.append([unit_def, Vector2i(0, column)])
	return placements

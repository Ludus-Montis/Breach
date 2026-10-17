class_name FormationRegeneration
extends RefCounted
## Regeneration (Decision 121), for FormationRecovery: a unit mends HP up to its limit per
## rest, times its condition, through damage; a blow of a type it fears stops it a while
## (`scorch`); downed, it mends only with "regenerates_downed". The part of a point regained
## so far and the seconds it stays stopped are the unit store's (REGENERATION_CARRY,
## REGENERATION_HALT, spec 35 step 7b), as are its hp, the hp it may still regain and its
## condition: the pass holds them and hands them on.

const BattleTuning = preload("res://content/definitions/battle_tuning.gd")
const SkirmishUnit = preload("res://sim/skirmish/skirmish_unit.gd")


## A blow that landed of a type that stops the target's regeneration stops it a while, in
## `halts` (REGENERATION_HALT, held by the blows' view).
static func scorch(
	halts: PackedFloat64Array, target: SkirmishUnit, parts: Array, dealt: int
) -> void:
	if dealt <= 0 or target.regeneration_stops.is_empty():
		return
	for part in parts:
		if target.regeneration_stops.has(part[1]):
			halts[target.slot] = BattleTuning.current().wounds_regeneration_halt
			return


## Mends what the unit may this tick (downed: only with "regenerates_downed"). `pools` is
## [the pass's view, its regeneration-left, condition, regeneration-carry and
## regeneration-halt columns], held for the pass.
static func mend(unit: SkirmishUnit, downed: bool, seconds: float, pools: Array) -> void:
	if unit.regeneration <= 0.0:
		return
	if downed and unit.traits.get("regenerates_downed", 0) <= 0:
		return
	var slot := unit.slot
	var halts: PackedFloat64Array = pools[4]
	if halts[slot] > 0.0:
		halts[slot] = maxf(0.0, halts[slot] - seconds)
		return
	var health: UnitView = pools[0]
	var left: PackedFloat64Array = pools[1]
	var condition: PackedFloat64Array = pools[2]
	var carries: PackedFloat64Array = pools[3]
	var gain := minf(unit.regeneration * condition[slot] * seconds, left[slot])
	gain = minf(gain, float(unit.max_hp - UnitHealth.of(health, unit)) - carries[slot])
	if gain <= 0.0:
		return
	left[slot] -= gain
	carries[slot] += gain
	var whole := floori(carries[slot] + 0.000001)  # float sums fall short
	UnitHealth.heal(health, unit, whole)
	carries[slot] -= whole

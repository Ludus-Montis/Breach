class_name RoutFlight
extends RefCounted
## How a disorderly flight spreads (Decision 99, spec 27 round 10): each unit's own flight
## angle off its route's line - seeded by the battle and the unit - up to rout_fan_degrees
## either side (BattleTuning). The rout's flight is the crate's (`rout/flight.rs`, spec 39
## step 7b); a ragged retreat fans out by its disorder, handed these (FormationWithdraw).
## Pure.

const BattleTuning = preload("res://content/definitions/battle_tuning.gd")
const SkirmishUnit = preload("res://sim/skirmish/skirmish_unit.gd")
const BattleRolls = preload("res://sim/skirmish/formation/battle_rolls.gd")


## The unit's own flight angle (radians) off its route's line, at full disorder.
static func fan(unit: SkirmishUnit, fight_seed: int) -> float:
	var roll := BattleRolls.uniform(fight_seed, [unit.id, "fan"]) * 2.0 - 1.0
	return deg_to_rad(BattleTuning.current().rout_fan_degrees) * roll

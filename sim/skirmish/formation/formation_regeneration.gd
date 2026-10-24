class_name FormationRegeneration
extends RefCounted
## Regeneration's stop (Decision 121): a blow of a type a unit fears stops its regeneration
## a while (`scorch`), in the unit store's REGENERATION_HALT, which the blows' landing holds.
## The mending itself is the crate's (`regeneration.rs`, spec 39 step 6b), run by
## FormationRecovery.

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

class_name ScrumBlows
extends RefCounted
## Melee blows in the scrum (Decisions 88 and 95, spec 27 rounds 5 and 8): every unit of a
## fighting squad - and of any squad, at a retreating enemy - strikes one enemy it touches
## (ScrumReach), on a face or a corner: one in its front first, then the nearest, then by
## the battle's seeded draw (ScrumContest; never by id, Decision 97). A blow from outside
## the target's front, or on a turning squad, is a flank blow. Blows land together. Who
## strikes whom is the crate's, on the battle's native bodies (ScrumBlowsField, `blows.rs`,
## spec 39 step 4a), and so are who touches a foe and the nearest it touches (`near.rs`,
## step 4c); what is here is a unit's ticks between blows and clearing the targets as a
## fight begins. Routing squads are left to FormationRout. Pure over the squads it is given.

const SkirmishUnit = preload("res://sim/skirmish/skirmish_unit.gd")


## Clears every living unit's target, as a fight begins (FormationMelee): TARGET_ID 0 in
## the battle's store, which the squads share.
static func clear_targets(squads: Array) -> void:
	if squads.is_empty():
		return
	var columns: UnitColumns = squads[0].store.columns
	var targets: PackedInt64Array = columns.column(UnitColumns.TARGET_ID)
	for squad in squads:
		for unit: SkirmishUnit in squad.living():
			targets[unit.slot] = 0
	columns.commit(UnitColumns.TARGET_ID, targets)


## Ticks between the unit's melee blows, `second` ticks making a second (Decision 120).
static func ticks(second: int, unit: SkirmishUnit) -> int:
	return maxi(1, roundi(second * unit.melee_seconds))

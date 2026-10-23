class_name SquadRanks
extends RefCounted
## Closing ranks after a fight (spec 27 round 6): the dead leave holes in a squad's places,
## so when its fight ends its living units are laid out again as a solid block - front
## band first, then by their old rank and column - in rows as wide as the squad, the last
## row centred. The layout is the crate's (`ranks.rs`, spec 39 step 4b). Pure over the
## squad it is given.

const SkirmishSquad = preload("res://sim/skirmish/formation/skirmish_squad.gd")


static func close(squad: SkirmishSquad) -> void:
	var slots := PackedInt32Array()
	var preferred := PackedInt64Array()
	var feet := PackedInt32Array()
	for unit in squad.living():
		slots.append(unit.slot)
		preferred.append(unit.preferred_position)
		feet.append_array([unit.footprint_width, unit.footprint_depth])
	var columns: UnitColumns = squad.store.columns
	var field: Object = ColumnsLent.field(columns)
	ColumnsLent.all(columns, func(): field.close_ranks(squad.width, slots, preferred, feet))
	squad.swaps.clear()

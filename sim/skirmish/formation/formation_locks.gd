class_name FormationLocks
extends RefCounted
## Which squads are locked in a fight (Decisions 40 and 47): a lock makes a squad fight its
## foe from this tick; a release frees it and everyone fighting it. A turning squad keeps
## turning through either and takes up the fight when the turn ends (Decision 74). The
## rules are the crate's (`locks.rs`, spec 39 step 4b), on the squads' own store.

const SkirmishSquad = preload("res://sim/skirmish/formation/skirmish_squad.gd")


static func lock(squad: SkirmishSquad, foe: SkirmishSquad) -> void:
	var living := PackedInt32Array(squad.living().map(func(unit): return unit.slot))
	var joined: bool = not squad.joined.is_empty()
	var columns: UnitColumns = squad.store.columns
	var field: Object = ColumnsLent.field(columns)
	ColumnsLent.all(columns, func(): field.squad_lock(squad.slot, foe.id, living, joined))


## Ends a squad's fight and frees every squad that was fighting it, front or flank.
static func release(released: SkirmishSquad, squads: Array) -> void:
	released.store.field().squad_release(released.slot, slots(squads))


## The squads' slots in their store, in the lists' order.
static func slots(squads: Array) -> PackedInt32Array:
	return PackedInt32Array(squads.map(func(squad): return squad.slot))


## The squads' factions as codes, the same for one faction (by its first in the list).
static func factions(squads: Array) -> PackedInt32Array:
	var codes := {}
	var out := PackedInt32Array()
	for squad in squads:
		out.append(codes.get_or_add(squad.faction_id, codes.size()))
	return out

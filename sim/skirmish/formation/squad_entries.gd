class_name SquadEntries
extends RefCounted
## A squad's loose and fleeing entries (FormationScrum, FormationRout) kept with their
## places in the unit store (specs/35-unit-store.md, step 1b): where a loose unit stands is
## the store's LOOSE_AT, a fleeing unit's place off its route its FLEE_OFFSET, and its
## FLAGS bits say whether it has each entry. Entries are given and taken away only here, so
## the bits always match them. The entries' other keys are the store's too (step 2, EntryKeys):
## `loose` and `fleeing` map each unit's id to the unit, in the order its entry was given.

const SkirmishSquad = preload("res://sim/skirmish/formation/skirmish_squad.gd")
const SkirmishUnit = preload("res://sim/skirmish/skirmish_unit.gd")
const UnitColumns = preload("res://sim/skirmish/formation/unit_columns.gd")
const EntryKeys = preload("res://sim/skirmish/formation/entry_keys.gd")


## Gives the unit a new loose entry in `squad`, standing loose at `at`, its next step there
## and seeking no foe.
static func loosen(squad: SkirmishSquad, unit: SkirmishUnit, at: Vector2) -> void:
	squad.loose[unit.id] = unit
	var columns: UnitColumns = squad.store.columns
	# placement: where it stands as it goes loose (Decision 139)
	columns.put(unit.slot, UnitColumns.LOOSE_AT, at)
	columns.put(unit.slot, UnitColumns.NEXT, at)
	EntryKeys.set_goal(columns, unit.slot, null)
	EntryKeys.forget(columns, unit.slot, EntryKeys.LOOSE_KEYS)
	_mark(squad, unit.slot, UnitColumns.LOOSE)


## Where the unit stands loose (it has a loose entry): for scattered use; a pass over many
## units reads the column once.
static func loose_at(squad: SkirmishSquad, unit: SkirmishUnit) -> Vector2:
	return squad.store.columns.value(unit.slot, UnitColumns.LOOSE_AT)


## Takes the unit's loose entry away (nothing if it has none).
static func unloose(squad: SkirmishSquad, unit_id: int) -> void:
	if squad.loose.has(unit_id):
		_clear(squad, squad.loose[unit_id].slot, UnitColumns.LOOSE)
		squad.loose.erase(unit_id)


## Takes every loose entry of the squad away.
static func unloose_all(squad: SkirmishSquad) -> void:
	for unit_id in squad.loose:
		_clear(squad, squad.loose[unit_id].slot, UnitColumns.LOOSE)
	squad.loose.clear()


## Gives the unit a new fleeing entry in `squad`, `along` its route and `offset` off it.
static func flee(squad: SkirmishSquad, unit: SkirmishUnit, along: float, offset: Vector2) -> void:
	squad.fleeing[unit.id] = unit
	var columns: UnitColumns = squad.store.columns
	# placement: its place as it starts to flee (Decision 139)
	columns.put(unit.slot, UnitColumns.FLEE_OFFSET, offset)
	columns.put(unit.slot, UnitColumns.ALONG, along)  # placement: as above
	EntryKeys.forget(columns, unit.slot, EntryKeys.FLEEING_KEYS)
	_mark(squad, unit.slot, UnitColumns.FLEEING)


## Takes the unit's fleeing entry away (nothing if it has none).
static func unflee(squad: SkirmishSquad, unit_id: int) -> void:
	if squad.fleeing.has(unit_id):
		_clear(squad, squad.fleeing[unit_id].slot, UnitColumns.FLEEING)
		squad.fleeing.erase(unit_id)


## Takes every fleeing entry of the squad away.
static func unflee_all(squad: SkirmishSquad) -> void:
	for unit_id in squad.fleeing:
		_clear(squad, squad.fleeing[unit_id].slot, UnitColumns.FLEEING)
	squad.fleeing.clear()


static func _mark(squad: SkirmishSquad, slot: int, bit: int) -> void:
	var columns: UnitColumns = squad.store.columns
	columns.put(slot, UnitColumns.FLAGS, columns.value(slot, UnitColumns.FLAGS) | bit)


static func _clear(squad: SkirmishSquad, slot: int, bit: int) -> void:
	var columns: UnitColumns = squad.store.columns
	columns.put(slot, UnitColumns.FLAGS, columns.value(slot, UnitColumns.FLAGS) & ~bit)

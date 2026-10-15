class_name SquadEntries
extends RefCounted
## A squad's loose and fleeing entries (FormationScrum, FormationRout) kept with their
## places in the unit store (specs/35-unit-store.md, step 1b): where a loose unit stands is
## the store's LOOSE_AT, a fleeing unit's place off its route its FLEE_OFFSET, and its
## FLAGS bits say whether it has each entry. Entries are given and taken away only here, so
## the bits always match them.

const SkirmishSquad = preload("res://sim/skirmish/formation/skirmish_squad.gd")
const SkirmishUnit = preload("res://sim/skirmish/skirmish_unit.gd")
const UnitColumns = preload("res://sim/skirmish/formation/unit_columns.gd")


## Gives the unit a loose entry in `squad`, standing loose at `at`.
static func loosen(squad: SkirmishSquad, unit: SkirmishUnit, entry: Dictionary, at: Vector2):
	squad.loose[unit.id] = entry
	# placement: where it stands as it goes loose (Decision 139)
	squad.store.columns.put(unit.slot, UnitColumns.LOOSE_AT, at)
	_mark(squad, unit.slot, UnitColumns.LOOSE)


## Where the unit stands loose (it has a loose entry): for scattered use; a pass over many
## units reads the column once.
static func loose_at(squad: SkirmishSquad, unit: SkirmishUnit) -> Vector2:
	return squad.store.columns.value(unit.slot, UnitColumns.LOOSE_AT)


## Takes the unit's loose entry away (nothing if it has none).
static func unloose(squad: SkirmishSquad, unit_id: int) -> void:
	if squad.loose.has(unit_id):
		_clear(squad, squad.loose[unit_id]["unit"].slot, UnitColumns.LOOSE)
		squad.loose.erase(unit_id)


## Takes every loose entry of the squad away.
static func unloose_all(squad: SkirmishSquad) -> void:
	for unit_id in squad.loose:
		_clear(squad, squad.loose[unit_id]["unit"].slot, UnitColumns.LOOSE)
	squad.loose.clear()


## Gives the unit a fleeing entry in `squad`, `offset` off its route.
static func flee(squad: SkirmishSquad, unit: SkirmishUnit, entry: Dictionary, offset: Vector2):
	squad.fleeing[unit.id] = entry
	# placement: its place as it starts to flee (Decision 139)
	squad.store.columns.put(unit.slot, UnitColumns.FLEE_OFFSET, offset)
	_mark(squad, unit.slot, UnitColumns.FLEEING)


## Takes the unit's fleeing entry away (nothing if it has none).
static func unflee(squad: SkirmishSquad, unit_id: int) -> void:
	if squad.fleeing.has(unit_id):
		_clear(squad, squad.store.slot_of(unit_id), UnitColumns.FLEEING)
		squad.fleeing.erase(unit_id)


## Takes every fleeing entry of the squad away.
static func unflee_all(squad: SkirmishSquad) -> void:
	for unit_id in squad.fleeing:
		_clear(squad, squad.store.slot_of(unit_id), UnitColumns.FLEEING)
	squad.fleeing.clear()


static func _mark(squad: SkirmishSquad, slot: int, bit: int) -> void:
	var columns: UnitColumns = squad.store.columns
	columns.put(slot, UnitColumns.FLAGS, columns.value(slot, UnitColumns.FLAGS) | bit)


static func _clear(squad: SkirmishSquad, slot: int, bit: int) -> void:
	var columns: UnitColumns = squad.store.columns
	columns.put(slot, UnitColumns.FLAGS, columns.value(slot, UnitColumns.FLAGS) & ~bit)

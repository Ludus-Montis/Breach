class_name SquadEntries
extends RefCounted
## A squad's loose and fleeing entries (FormationScrum, FormationRout), kept with their
## places in the unit store and given and taken away in the crate (`entries.rs`, spec 39
## step 4b): where a loose unit stands is the store's LOOSE_AT, a fleeing unit's place off
## its route its FLEE_OFFSET, and its FLAGS bits say whether it has each entry. The squad
## store lists who has one, which SkirmishSquad's `loose` and `fleeing` (unit id -> the
## unit, in the order each entry was given) are views of (SquadRecords). The entries' other
## keys are the store's too (EntryKeys). A pass holding the columns lends them to each call
## (ColumnsLent).

const SkirmishSquad = preload("res://sim/skirmish/formation/skirmish_squad.gd")
const SkirmishUnit = preload("res://sim/skirmish/skirmish_unit.gd")
const UnitColumns = preload("res://sim/skirmish/formation/unit_columns.gd")
const EntryKeys = preload("res://sim/skirmish/formation/entry_keys.gd")

## The column taking an entry away writes.
const FLAGGED := [UnitColumns.FLAGS]


## Gives the unit a new loose entry in `squad`, standing loose at `at`, its next step there
## and seeking no foe.
static func loosen(squad: SkirmishSquad, unit: SkirmishUnit, at: Vector2) -> void:
	var columns: UnitColumns = squad.store.columns
	var field: Object = ColumnsLent.field(columns)
	ColumnsLent.slot(
		columns,
		unit.slot,
		EntryKeys.LOOSENED,
		func(): field.loosen(squad.slot, unit.id, unit.slot, at)
	)


## Where the unit stands loose (it has a loose entry): for scattered use; a pass over many
## units reads the column once.
static func loose_at(squad: SkirmishSquad, unit: SkirmishUnit) -> Vector2:
	return squad.store.columns.value(unit.slot, UnitColumns.LOOSE_AT)


## Takes the unit's loose entry away (nothing if it has none).
static func unloose(squad: SkirmishSquad, unit_id: int) -> void:
	var columns: UnitColumns = squad.store.columns
	var field: Object = ColumnsLent.field(columns)
	ColumnsLent.slot(
		columns, squad.store.slot_of(unit_id), FLAGGED, func(): field.unloose(squad.slot, unit_id)
	)


## Takes every loose entry of the squad away.
static func unloose_all(squad: SkirmishSquad) -> void:
	var columns: UnitColumns = squad.store.columns
	var field: Object = ColumnsLent.field(columns)
	ColumnsLent.all(columns, func(): field.unloose_all(squad.slot))


## Gives the unit a new fleeing entry in `squad`, `along` its route and `offset` off it.
static func flee(squad: SkirmishSquad, unit: SkirmishUnit, along: float, offset: Vector2) -> void:
	var columns: UnitColumns = squad.store.columns
	var field: Object = ColumnsLent.field(columns)
	ColumnsLent.slot(
		columns,
		unit.slot,
		EntryKeys.FLED,
		func(): field.flee(squad.slot, unit.id, unit.slot, along, offset)
	)


## Takes the unit's fleeing entry away (nothing if it has none).
static func unflee(squad: SkirmishSquad, unit_id: int) -> void:
	var columns: UnitColumns = squad.store.columns
	var field: Object = ColumnsLent.field(columns)
	ColumnsLent.slot(
		columns, squad.store.slot_of(unit_id), FLAGGED, func(): field.unflee(squad.slot, unit_id)
	)


## Takes every fleeing entry of the squad away.
static func unflee_all(squad: SkirmishSquad) -> void:
	var columns: UnitColumns = squad.store.columns
	var field: Object = ColumnsLent.field(columns)
	ColumnsLent.all(columns, func(): field.unflee_all(squad.slot))

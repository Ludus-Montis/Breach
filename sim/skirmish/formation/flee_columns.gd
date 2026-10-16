class_name FleeColumns
extends RefCounted
## The fleeing entries' columns, typed, for the rout's loops over routers (FormationRout,
## RoutFlight): taken from a store the rout holds (StoreHolds), so each is the hold's own
## copy, read and written in place - an index, not a call - and every other reader in the
## pass sees the same values. `keep` is EntryKeys.keep on these.

const UnitColumns = preload("res://sim/skirmish/formation/unit_columns.gd")
const EntryKeys = preload("res://sim/skirmish/formation/entry_keys.gd")
const ColumnsHeld = preload("res://sim/skirmish/formation/columns_held.gd")

var along: PackedFloat64Array  # how far along its route each router has fled
var offset: PackedVector2Array  # its place off its route
var fanned: PackedFloat64Array  # how far it has fanned out
var caught: PackedInt64Array  # the ticks a friend has held it
var held_by: PackedInt64Array  # which friend holds it
var keys: PackedInt32Array  # which keys its entry holds (EntryKeys)


## The columns of `squad`'s store, which the rout must hold.
static func of(squad) -> FleeColumns:
	var columns: UnitColumns = squad.store.columns
	assert(columns is ColumnsHeld, "the rout holds its store (StoreHolds) before taking these")
	var out := FleeColumns.new()
	out.along = columns.column(UnitColumns.ALONG)
	out.offset = columns.column(UnitColumns.FLEE_OFFSET)
	out.fanned = columns.column(UnitColumns.FANNED)
	out.caught = columns.column(UnitColumns.CAUGHT)
	out.held_by = columns.column(UnitColumns.HELD_BY)
	out.keys = columns.column(UnitColumns.KEYS)
	return out


## EntryKeys.keep for FANNED, CAUGHT or HELD_BY: the slot's entry holds `name`, at `data`
## (null: HELD_BY holds no friend).
func keep(slot: int, name: StringName, data: Variant) -> void:
	var bits: int = keys[slot] | EntryKeys.BITS[name]
	if data == null:
		bits &= ~EntryKeys.VALUED[name]
		data = UnitColumns.BLANKS[name]
	else:
		bits |= EntryKeys.VALUED.get(name, 0)
	keys[slot] = bits
	match name:
		UnitColumns.FANNED:
			fanned[slot] = data
		UnitColumns.CAUGHT:
			caught[slot] = data
		UnitColumns.HELD_BY:
			held_by[slot] = data

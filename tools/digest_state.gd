extends RefCounted
## What the formation digest hashes each tick (tools/formation_digest.gd): every squad's and
## unit's state as plain values, and the names of its fields (spec 37), nested as the arrays
## nest. A field added to `state` or `unit_state` gets its name here, beside it: the trace
## writes each value by its name, and test_digest_trace.gd fails on an unnamed one.

const SkirmishSquad = preload("res://sim/skirmish/formation/skirmish_squad.gd")
const SkirmishUnit = preload("res://sim/skirmish/skirmish_unit.gd")
const EntryKeys = preload("res://sim/skirmish/formation/entry_keys.gd")

const SQUAD_FIELDS := [
	["id", "state", "order", "front_distance", "heading"],
	["position", "morale", "engaged_with", "stall_ticks"],
	["fight_since", ["flank_contacts", "stance", "pursuit", "withdraw"], "units"],
]
const UNIT_FIELDS := [
	["id", "hp", "position", "bearing", "attack_cooldown"],
	["target_id", "rank", "column", ["loose", "fleeing", "chaser"]],
]


## Every squad's and unit's state, as plain values (objects by their ids). Each unit's id is
## read through the battle's unit store (spec 35), the same way on both engines.
static func state(squads: Array, store: UnitStore) -> Array:
	var out := []
	var at: PackedVector2Array = store.columns.column(UnitColumns.POSITION)
	var loose_at: PackedVector2Array = store.columns.column(UnitColumns.LOOSE_AT)
	var offsets: PackedVector2Array = store.columns.column(UnitColumns.FLEE_OFFSET)
	for squad in squads:
		var units := []
		for unit in squad.units:
			var slot := store.slot_of(unit.id)
			var read := [store.id_at(slot), at[slot], loose_at[slot], offsets[slot]]
			units.append(unit_state(squad, unit, read))
		var held := [squad.flank_contacts, squad.stance, squad.pursuit, squad.withdraw]
		(
			out
			. append(
				[
					[squad.id, squad.state, squad.order, squad.front_distance, squad.heading],
					[squad.position, squad.morale, squad.engaged_with, squad.stall_ticks],
					[squad.fight_since, plain(held), units],
				]
			)
		)
	return out


## `read` = [its id, where it stands, its loose and fleeing places], read through the
## store. Its loose and fleeing entries are hashed as they were before they moved to the
## store (spec 35 steps 1b and 2): as dictionaries with the keys they held (EntryKeys).
static func unit_state(squad: SkirmishSquad, unit: SkirmishUnit, read: Array) -> Array:
	var columns := squad.store.columns
	var loose = null
	if squad.loose.has(unit.id):
		loose = EntryKeys.loose_entry(columns, unit, read[2])
	var fleeing = null
	if squad.fleeing.has(unit.id):
		fleeing = EntryKeys.fleeing_entry(columns, unit.slot, read[3])
	var held := [loose, fleeing, squad.chasers.get(unit.id)]
	return [
		[
			read[0],
			columns.value(unit.slot, UnitColumns.HP),
			read[1],
			columns.value(unit.slot, UnitColumns.BEARING),
			columns.value(unit.slot, UnitColumns.ATTACK_COOLDOWN)
		],
		[
			unit.target_id,
			columns.value(unit.slot, UnitColumns.RANK),
			columns.value(unit.slot, UnitColumns.COLUMN),
			plain(held)
		],
	]


## The value with every object replaced by its id (or its class), dictionaries by their
## entries in key order: plain data whose bytes don't hang on where objects live.
static func plain(value: Variant) -> Variant:
	if value is Object:
		return ["obj", value.get("id") if "id" in value else value.get_class()]
	if value is Array:
		return value.map(func(v): return plain(v))
	if value is Dictionary:
		var keys: Array = value.keys().map(func(k): return var_to_str(plain(k)))
		var pairs := []
		for key in value:
			pairs.append([var_to_str(plain(key)), plain(value[key])])
		pairs.sort_custom(func(a, b): return a[0] < b[0])
		return [keys.size(), pairs]
	return value

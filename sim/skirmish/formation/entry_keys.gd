class_name EntryKeys
extends RefCounted
## The keys of a unit's loose and fleeing entries, held in the unit store's columns (spec 35
## step 2). A key the entry doesn't hold reads as its column's blank, the value its readers
## took for a missing key. The KEYS column says which keys the entry holds, and for `toward`
## and `held_by`, which may be null, whether each holds a value: the digest hashes the
## entries with exactly the keys they hold (EntryKeys.loose_entry).

const UnitColumns = preload("res://sim/skirmish/formation/unit_columns.gd")

## KEYS' bits: the entry holds the key.
const BITS := {
	UnitColumns.FOE_AT: 1,
	UnitColumns.TOUCH: 2,
	UnitColumns.TOWARD: 4,
	UnitColumns.STUCK: 16,
	UnitColumns.CLOSEST: 32,
	UnitColumns.CAUGHT: 64,
	UnitColumns.HELD_BY: 128,
	UnitColumns.FANNED: 512,
}
## KEYS' bits: the key holds a value, not null.
const VALUED := {UnitColumns.TOWARD: 8, UnitColumns.HELD_BY: 256}
## The keys a new entry doesn't hold yet.
const LOOSE_KEYS := [
	UnitColumns.FOE_AT,
	UnitColumns.TOUCH,
	UnitColumns.TOWARD,
	UnitColumns.STUCK,
	UnitColumns.CLOSEST,
]
const FLEEING_KEYS := [UnitColumns.CAUGHT, UnitColumns.HELD_BY, UnitColumns.FANNED]


## Sets the key (`data` null only for `toward` and `held_by`).
static func keep(columns: UnitColumns, slot: int, name: StringName, data: Variant) -> void:
	var keys: int = columns.value(slot, UnitColumns.KEYS) | BITS[name]
	if data == null:
		keys &= ~VALUED[name]
		columns.put(slot, name, UnitColumns.BLANKS[name])
	else:
		keys |= VALUED.get(name, 0)
		columns.put(slot, name, data)
	columns.put(slot, UnitColumns.KEYS, keys)


## The value of `toward` or `held_by`: null if the entry doesn't hold one.
static func held(columns: UnitColumns, slot: int, name: StringName) -> Variant:
	if columns.value(slot, UnitColumns.KEYS) & VALUED[name]:
		return columns.value(slot, name)
	return null


## Takes the keys out of the entry: each reads blank again.
static func forget(columns: UnitColumns, slot: int, names: Array) -> void:
	var keys: int = columns.value(slot, UnitColumns.KEYS)
	for name in names:
		keys &= ~(BITS[name] | VALUED.get(name, 0))
		columns.put(slot, name, UnitColumns.BLANKS[name])
	columns.put(slot, UnitColumns.KEYS, keys)


## The foe and slot the unit seeks, [foe id, slot index], or null.
static func goal(columns: UnitColumns, slot: int) -> Variant:
	var sought: int = columns.value(slot, UnitColumns.GOAL_SLOT)
	return null if sought < 0 else [columns.value(slot, UnitColumns.GOAL_FOE), sought]


## Sets the goal ([foe id, slot index], or null for none).
static func set_goal(columns: UnitColumns, slot: int, sought: Variant) -> void:
	columns.put(slot, UnitColumns.GOAL_FOE, 0 if sought == null else sought[0])
	columns.put(slot, UnitColumns.GOAL_SLOT, -1 if sought == null else sought[1])


## The unit's loose entry as a dictionary, as squads held it before spec 35: `unit`, `goal`,
## `next` and `at`, and the other keys it holds. For the digest and tests only.
static func loose_entry(columns: UnitColumns, unit: Object, at: Vector2) -> Dictionary:
	var slot: int = unit.slot
	var out := {
		"unit": unit, "goal": goal(columns, slot), "next": columns.value(slot, UnitColumns.NEXT)
	}
	out["at"] = at
	_held_keys(columns, slot, LOOSE_KEYS, out)
	if out.has("touch"):
		out["touch"] = out["touch"] == 1
	return out


## The unit's fleeing entry as a dictionary, as squads held it before spec 35: `along`,
## `offset` and the other keys it holds. For the digest and tests only.
static func fleeing_entry(columns: UnitColumns, slot: int, offset: Vector2) -> Dictionary:
	var out := {"along": columns.value(slot, UnitColumns.ALONG), "offset": offset}
	_held_keys(columns, slot, FLEEING_KEYS, out)
	return out


static func _held_keys(columns: UnitColumns, slot: int, names: Array, out: Dictionary) -> void:
	var keys: int = columns.value(slot, UnitColumns.KEYS)
	for name in names:
		if keys & BITS[name]:
			out[String(name)] = (
				held(columns, slot, name) if VALUED.has(name) else columns.value(slot, name)
			)

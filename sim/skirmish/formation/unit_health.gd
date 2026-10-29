class_name UnitHealth
extends RefCounted
## A unit's hp in the unit store's HP column (spec 35 step 5b, Decision 145), and its one
## writer: every change to a unit's hp goes through `hurt` or `put` or their scattered
## forms, but the fight's hurts, which its crate pass lands (`fight::step`, `hurts.rs`;
## check_one_health.py holds that). A write that
## takes a unit from above 0 to 0 or below puts it on the store's fell queue, which
## FormationDeaths.bury drains; whether it died or is downed is decided there, from its hp at
## the end of the tick's blows.
##
## `hurt`, `put` and `of` take a pass's view (UnitView, Decision 146) holding HP and
## write `view.hp` in place: an index, not a call. Their scattered forms, `*_one`, take
## anything holding the store (a squad, the simulation), for an event's few units: under Rust
## each is a call into the field (check_store_reads.py flags them in a loop).

const UnitColumns = preload("res://sim/skirmish/formation/unit_columns.gd")


## The unit's hp, from the view.
static func of(view: UnitView, unit) -> int:
	return view.hp[unit.slot]


## Takes `amount` from the unit's hp, in the view.
static func hurt(view: UnitView, unit, amount: int) -> void:
	put(view, unit, of(view, unit) - amount)


## Sets the unit's hp to `value` in the view, which holds HP (UnitView.hold); a unit taken
## from above 0 to 0 or below falls.
static func put(view: UnitView, unit, value: int) -> void:
	assert(view.is_held(), "hp is written through a held view (UnitView.hold), not a read one")
	var hp: PackedInt32Array = view.hp
	var was := hp[unit.slot]
	hp[unit.slot] = value
	if was > 0 and value <= 0:
		view.store.fallen.push(unit.id)


## The unit's hp, read from `source`'s store: one unit, scattered.
static func of_one(source, unit) -> int:
	return source.store.columns.value(unit.slot, UnitColumns.HP)


## `hurt` for one unit of `source`'s store, scattered.
static func hurt_one(source, unit, amount: int) -> void:
	put_one(source, unit, of_one(source, unit) - amount)


## `put` for one unit of `source`'s store, scattered.
static func put_one(source, unit, value: int) -> void:
	var was := of_one(source, unit)
	source.store.columns.put(unit.slot, UnitColumns.HP, value)
	if was > 0 and value <= 0:
		source.store.fallen.push(unit.id)

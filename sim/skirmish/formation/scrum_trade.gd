class_name ScrumTrade
extends RefCounted
## Trading places while regrouping (Decision 110): a unit walking back to its place that has
## come no nearer than its closest for scrum_trade_seconds - its own ranks in the way - trades
## places with the friend whose place is nearest it, if that place is nearer it than its
## own and the two are interchangeable (one kind, one band); the friend walks to the place
## it left. With no such friend, its own place is the nearest: it takes it. Places stay one
## unit each. Stalled units trade in the order of their seeded draws, never the list's
## (Decision 97). The rules are the crate's (`trade.rs`, spec 39 step 4b). Pure over the
## squad it is given.

const BattleTuning = preload("res://content/definitions/battle_tuning.gd")
const SkirmishSquad = preload("res://sim/skirmish/formation/skirmish_squad.gd")
const SkirmishUnit = preload("res://sim/skirmish/skirmish_unit.gd")
const UnitColumns = preload("res://sim/skirmish/formation/unit_columns.gd")

## The columns `track` writes.
const TRACKED := [UnitColumns.CLOSEST, UnitColumns.STUCK, UnitColumns.KEYS]


## Notes how near the loose unit in `slot` stands to its place, `gap` cells off.
static func track(columns: UnitColumns, slot: int, gap: float, seconds: float) -> void:
	var field: Object = ColumnsLent.field(columns)
	ColumnsLent.slot(columns, slot, TRACKED, func(): field.trade_track(slot, gap, seconds))


## Each stalled unit of the squad trades places, if a friend's place is nearer it; one that
## can't is as near its place as the press allows, and takes it there and then. This hands
## the crate the squad's units: the living, then any loose ones that aren't, each one's
## kind a code for its definition.
static func trade(squad: SkirmishSquad, fight_seed: int) -> void:
	if squad.loose.is_empty():
		return  # nobody to trade: STUCK isn't read (Decision 150)
	var after: float = BattleTuning.current().scrum_trade_seconds - 0.000001
	var stuck: PackedFloat64Array = squad.store.columns.column(UnitColumns.STUCK)
	if not squad.loose.values().any(func(unit): return stuck[unit.slot] >= after):
		return  # nobody stalled: the crate would trade none, and a hold needn't be lent
	var units: Array = squad.living().duplicate()
	var living := units.size()
	var standing := {}
	for unit in units:
		standing[unit] = true
	for unit_id in squad.loose:
		var loose_unit: SkirmishUnit = squad.loose[unit_id]
		if not standing.has(loose_unit):
			units.append(loose_unit)
	var kinds := {}  # definition -> its code
	var ids := PackedInt64Array()
	var slots := PackedInt32Array()
	var codes := PackedInt32Array()
	var preferred := PackedInt64Array()
	var feet := PackedInt32Array()
	var alive := PackedByteArray()
	for index in units.size():
		var unit: SkirmishUnit = units[index]
		ids.append(unit.id)
		slots.append(unit.slot)
		codes.append(kinds.get_or_add(unit.definition, kinds.size()))
		preferred.append(unit.preferred_position)
		feet.append_array([unit.footprint_width, unit.footprint_depth])
		alive.append(int(index < living))
	var columns: UnitColumns = squad.store.columns
	var field: Object = ColumnsLent.field(columns)
	ColumnsLent.all(
		columns,
		func():
			field.trade(squad.slot, ids, slots, codes, preferred, feet, alive, fight_seed, after)
	)

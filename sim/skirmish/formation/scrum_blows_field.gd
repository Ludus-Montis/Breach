class_name ScrumBlowsField
extends RefCounted
## The fight's melee counts on the native core (NativeKernels, Decision 129; the crate's
## `blows.rs`): for rolled blows, BlowLanding's counts of the foes pressing each target, on
## the sync of the scrum the crate made as it chose the tick's blows (FormationMelee,
## `fight::choose`, spec 39 step 8b5-xvii), or one made here if it had no need to.

const StoreHolds = preload("res://sim/skirmish/formation/store_holds.gd")
const BattleTuning = preload("res://content/definitions/battle_tuning.gd")
const SkirmishUnit = preload("res://sim/skirmish/skirmish_unit.gd")
const BodyFieldSync = preload("res://sim/skirmish/formation/body_field_sync.gd")

## The field the last FormationMelee.blows synced (its instance id; 0: that call had no
## need to).
static var _synced := 0


## Marks whether this tick's blows synced `field` (FormationMelee.blows).
static func mark(field: Object, synced_now: bool) -> void:
	_synced = field.get_instance_id() if synced_now else 0


## True if this tick's blows synced `field`, so pressed() may count on it.
static func synced(field: Object) -> bool:
	return _synced == field.get_instance_id()


## Syncs `field` as the blows do, for pressed() to count on when they had no need to.
static func sync(field: Object, squads: Array, fight_seed: int) -> void:
	_sync(field, squads, fight_seed)
	_synced = field.get_instance_id()


## BlowLanding's [squad_of, pressed] for these blows ([striker, target, ...]), on the sync
## the blows made: each striker's [unit, squad], and how many touching foes fight each
## target this tick (by the units' targets now).
static func pressed(field: Object, squads: Array, landing: Array) -> Array:
	assert(StoreHolds.none_held(squads), StoreHolds.SPANS)
	var targets := PackedInt64Array()
	var target_ids: PackedInt64Array = squads[0].store.columns.column(UnitColumns.TARGET_ID)
	for squad in squads:
		for unit: SkirmishUnit in squad.living():
			targets.append(target_ids[unit.slot])
	var asked := PackedInt64Array()
	for blow in landing:
		asked.append(blow[0].id)
		asked.append(blow[1].id)
	var reach := BattleTuning.current().reach_contact
	var counts: PackedInt32Array = field.pressed(reach, targets, asked)
	var squad_of := {}
	var pressing := {}
	for index in range(landing.size()):
		var striker: SkirmishUnit = landing[index][0]
		if counts[2 * index] >= 0:
			squad_of[striker.id] = [striker, squads[counts[2 * index]]]
		if counts[2 * index + 1] > 0:
			pressing[landing[index][1].id] = counts[2 * index + 1]
	return [squad_of, pressing]


## Brings the field to the scrum as BodyFieldSync.scrum does (where ScrumReach puts each
## unit, and its bearing); returns [units, their squads] by flat index. The arrays are sized
## once and filled in place.
static func _sync(field: Object, squads: Array, fight_seed: int) -> Array:
	var batch := _sized(squads)
	var units := batch.units
	var owners := batch.owners
	var ids := batch.ids
	var flat := 0
	for squad in squads:
		var before := flat
		for unit: SkirmishUnit in squad.living():
			units[flat] = unit
			owners[flat] = squad
			ids[flat] = unit.id
			flat += 1
		BodyFieldSync._squad(batch, squad, flat - before)
	return BodyFieldSync._send(field, _trimmed(batch, flat), fight_seed)


## A sync's arrays, sized for every unit of the squads.
static func _sized(squads: Array) -> BodyFieldSync.Batch:
	var size := 0
	for squad in squads:
		size += squad.units.size()
	var batch := BodyFieldSync.Batch.new()
	batch.units.resize(size)
	batch.owners.resize(size)
	batch.ids.resize(size)
	return batch


static func _trimmed(batch: BodyFieldSync.Batch, size: int) -> BodyFieldSync.Batch:
	batch.units.resize(size)
	batch.owners.resize(size)
	batch.ids.resize(size)
	return batch

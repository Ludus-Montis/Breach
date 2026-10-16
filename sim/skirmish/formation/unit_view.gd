class_name UnitView
extends RefCounted
## One typed view onto the unit store for a pass (spec 35 step 6a, Decision 146): a typed
## property for every column in UnitColumns, each the packed array that column is. A pass
## names the columns it reads; the others stay empty, and reading one asserts. Its loops take
## the arrays into typed locals before the loop (`var hp: PackedInt32Array = view.hp`) and
## index them by slot: an index is about 0.04 us, where a `value` or `put` on the native store
## is a call, about 1 us.
## - `hold` (a pass that writes): holds the names on every store the squads use (StoreHolds,
##   ColumnsHeld), so each property is the hold's own copy, written in place, and every
##   reader in the pass sees the writes; `release` commits them. A view spans one store: a
##   squad's slots index its own store, so each store held gets its own view, and the pass
##   takes each squad's with `of` (once per squad, not per unit).
## - `read` (a pass that only reads): copies of the names, no hold and nothing to release. On a
##   store a pass holds, a held name is the hold's own copy, so the reader sees its writes.
##   A write to a read view isn't committed.
## No hold spans a native call (StoreHolds.unheld).

const UnitColumns = preload("res://sim/skirmish/formation/unit_columns.gd")
const StoreHolds = preload("res://sim/skirmish/formation/store_holds.gd")
const ColumnsHeld = preload("res://sim/skirmish/formation/columns_held.gd")
const EntryKeys = preload("res://sim/skirmish/formation/entry_keys.gd")
const NOT_NAMED := "the view doesn't hold %s: name it when taking the view"
## A unit's place in its squad's frame (UnitPlace): its rank and column.
const PLACES := [UnitColumns.RANK, UnitColumns.COLUMN]

var store: UnitStore  # the store its slots index
var names := {}  # the columns it holds or read -> true
var position: PackedVector2Array:
	get:
		assert(names.has(UnitColumns.POSITION), NOT_NAMED % UnitColumns.POSITION)
		return position
var at: PackedVector2Array:
	get:
		assert(names.has(UnitColumns.LOOSE_AT), NOT_NAMED % UnitColumns.LOOSE_AT)
		return at
var offset: PackedVector2Array:
	get:
		assert(names.has(UnitColumns.FLEE_OFFSET), NOT_NAMED % UnitColumns.FLEE_OFFSET)
		return offset
var flags: PackedInt32Array:
	get:
		assert(names.has(UnitColumns.FLAGS), NOT_NAMED % UnitColumns.FLAGS)
		return flags
var next: PackedVector2Array:
	get:
		assert(names.has(UnitColumns.NEXT), NOT_NAMED % UnitColumns.NEXT)
		return next
var goal_foe: PackedInt64Array:
	get:
		assert(names.has(UnitColumns.GOAL_FOE), NOT_NAMED % UnitColumns.GOAL_FOE)
		return goal_foe
var goal_slot: PackedInt64Array:
	get:
		assert(names.has(UnitColumns.GOAL_SLOT), NOT_NAMED % UnitColumns.GOAL_SLOT)
		return goal_slot
var foe_at: PackedVector2Array:
	get:
		assert(names.has(UnitColumns.FOE_AT), NOT_NAMED % UnitColumns.FOE_AT)
		return foe_at
var touch: PackedInt32Array:
	get:
		assert(names.has(UnitColumns.TOUCH), NOT_NAMED % UnitColumns.TOUCH)
		return touch
var toward: PackedVector2Array:
	get:
		assert(names.has(UnitColumns.TOWARD), NOT_NAMED % UnitColumns.TOWARD)
		return toward
var stuck: PackedFloat64Array:
	get:
		assert(names.has(UnitColumns.STUCK), NOT_NAMED % UnitColumns.STUCK)
		return stuck
var closest: PackedFloat64Array:
	get:
		assert(names.has(UnitColumns.CLOSEST), NOT_NAMED % UnitColumns.CLOSEST)
		return closest
var along: PackedFloat64Array:
	get:
		assert(names.has(UnitColumns.ALONG), NOT_NAMED % UnitColumns.ALONG)
		return along
var caught: PackedInt64Array:
	get:
		assert(names.has(UnitColumns.CAUGHT), NOT_NAMED % UnitColumns.CAUGHT)
		return caught
var held_by: PackedInt64Array:
	get:
		assert(names.has(UnitColumns.HELD_BY), NOT_NAMED % UnitColumns.HELD_BY)
		return held_by
var fanned: PackedFloat64Array:
	get:
		assert(names.has(UnitColumns.FANNED), NOT_NAMED % UnitColumns.FANNED)
		return fanned
var keys: PackedInt32Array:
	get:
		assert(names.has(UnitColumns.KEYS), NOT_NAMED % UnitColumns.KEYS)
		return keys
var bearing: PackedFloat64Array:
	get:
		assert(names.has(UnitColumns.BEARING), NOT_NAMED % UnitColumns.BEARING)
		return bearing
var rank: PackedInt32Array:
	get:
		assert(names.has(UnitColumns.RANK), NOT_NAMED % UnitColumns.RANK)
		return rank
var column: PackedInt32Array:
	get:
		assert(names.has(UnitColumns.COLUMN), NOT_NAMED % UnitColumns.COLUMN)
		return column
var hp: PackedInt32Array:
	get:
		assert(names.has(UnitColumns.HP), NOT_NAMED % UnitColumns.HP)
		return hp
var attack_cooldown: PackedInt32Array:
	get:
		assert(names.has(UnitColumns.ATTACK_COOLDOWN), NOT_NAMED % UnitColumns.ATTACK_COOLDOWN)
		return attack_cooldown
var regeneration_left: PackedFloat64Array:
	get:
		assert(names.has(UnitColumns.REGENERATION_LEFT), NOT_NAMED % UnitColumns.REGENERATION_LEFT)
		return regeneration_left

var _held_with := []  # the stores held with it (hold), for release


## Whether the view is a hold's (`hold`), whose writes are committed on `release`.
func is_held() -> bool:
	return not _held_with.is_empty()


## EntryKeys.keep on the view, which holds KEYS and `name` (FANNED, CAUGHT or HELD_BY), in
## place: the slot's entry holds `name`, at `data` (null only for HELD_BY: no friend).
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


## Holds `names` on every store `squads` use until `release`; the view of the first squad's
## store (`of` gives each squad's).
static func hold(squads: Array, names: Array) -> UnitView:
	var stores := StoreHolds.hold(squads, names)
	var first: UnitView = null
	for held_store: UnitStore in stores:
		var view := _filled(held_store, names)
		view._held_with = stores
		var held: ColumnsHeld = held_store.columns
		held.view = view
		if first == null:
			first = view
	return first if first != null else UnitView.new()


## The view `squad`'s store is held under (`hold`). A call: take it once per squad.
static func of(squad) -> UnitView:
	assert(squad.store.columns is ColumnsHeld, "the squad's store is held (UnitView.hold)")
	var held: ColumnsHeld = squad.store.columns
	return held.view


## Commits every store held with `view` and gives each its own columns back.
static func release(view: UnitView) -> void:
	StoreHolds.release(view._held_with)


## Copies of `names` of `squad`'s store, for a pass that only reads them: no hold.
static func read(squad, names: Array) -> UnitView:
	return _filled(squad.store, names)


## `read`, taken once a store into `views` (store -> view: one set of `names` a dictionary),
## for a pass that moves nobody while it reads.
static func read_once(views: Dictionary, squad, names: Array) -> UnitView:
	if not views.has(squad.store):
		views[squad.store] = read(squad, names)
	return views[squad.store]


static func _filled(from: UnitStore, names: Array) -> UnitView:
	var view := UnitView.new()
	view.store = from
	for name: StringName in names:
		view.names[name] = true
		view.set(String(name), from.columns.column(name))
	return view

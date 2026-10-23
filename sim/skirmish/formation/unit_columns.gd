class_name UnitColumns
extends RefCounted
## The unit store's columns (specs/35-unit-store.md): each unit's per-tick state as packed
## arrays indexed by its slot. A pass asks once for each column it reads and indexes it by
## `unit.slot`; it writes one value with put, or a whole column with commit. A column
## fetched before a write doesn't see it: fetch again, or write to the fetched array and
## commit it.

const POSITION := &"position"  # where the unit stands, in cells (spec 27)
const LOOSE_AT := &"at"  # where it stands loose, while it has a loose entry (FormationScrum)
const FLEE_OFFSET := &"offset"  # its place off its route, while fleeing (FormationRout)
const FLAGS := &"flags"  # its bits, below

## FLAGS' bits: whether the unit has an entry in its squad's `loose`, and in its `fleeing`.
const LOOSE := 1
const FLEEING := 2

## The loose entry's other keys (spec 35 step 2): where it steps to next, the foe and slot it
## seeks (GOAL_SLOT -1: none), where that foe stands, whether it touches a foe, where it
## walks toward, and how long it has made no way toward its place since it was closest.
const NEXT := &"next"
const GOAL_FOE := &"goal_foe"
const GOAL_SLOT := &"goal_slot"
const FOE_AT := &"foe_at"
const TOUCH := &"touch"  # 1: it touches a foe
const TOWARD := &"toward"
const STUCK := &"stuck"
const CLOSEST := &"closest"
## The fleeing entry's other keys: how far along its route it is, the ticks a friend has
## held it and that friend's id, and how far it has fanned out off its route.
const ALONG := &"along"
const CAUGHT := &"caught"
const HELD_BY := &"held_by"
const FANNED := &"fanned"
## The way the unit faces (spec 35 step 3; UnitMotion): degrees clockwise from north. A
## unit not yet turned faces east.
const BEARING := &"bearing"
## The unit's place in its squad's frame (spec 35 step 4a; SquadFrame): its rank back from
## the front and its column from the squad's left, in cells. Both change as the squad
## shuffles, turns and trades places, so they are per-tick state.
const RANK := &"rank"
const COLUMN := &"column"
## The unit's hp, the ticks till its next blow lands, and the hp it may still regenerate
## before it rests (spec 35 step 5b). Every hp write goes through UnitHealth.
const HP := &"hp"
const ATTACK_COOLDOWN := &"attack_cooldown"
const REGENERATION_LEFT := &"regeneration_left"
## The unit's stamina, its pace this tick (cells a second on open ground, from its pace fresh,
## its gait and how tired it is), whether it spent stamina this tick (1), seconds since it last
## did, seconds into its present run (below 0: still reacting), whether it ran last tick (1),
## and its condition: the one number its statuses make of its state, 1 as things stand (spec
## 35 step 6c; FormationStamina, FormationRecovery).
const STAMINA := &"stamina"
const SPEED := &"speed"
const EXERTED := &"exerted"
const BREATHER := &"breather"
const RUN_BUILD := &"run_build"
const WAS_RUNNING := &"was_running"
const CONDITION := &"condition"
## The downed's and recovery's (spec 35 step 7b; FormationRecovery, BlowLanding): the part of
## a point regeneration has regained so far, the seconds a blow it fears stops it for, the
## seconds a downed unit has yet to lie before it comes to (below 0: not yet reckoned), the
## part of a point its poor condition has drained so far, and the wounds the last blow that
## struck it would add (its striker's "wounding N").
const REGENERATION_CARRY := &"regeneration_carry"
const REGENERATION_HALT := &"regeneration_halt"
const WAKE_LEFT := &"wake_left"
const DRAIN_CARRY := &"drain_carry"
const LAST_WOUNDING := &"last_wounding"
## The rest (spec 35 step 7c; the blows, FormationMarch and its walk modes): the id of the
## unit it fights (0: none), how far along its route it stands in cells from the player's
## end, the last place it could stand, whether it is on a climb face or in deep water (1: no
## breather there), and the cells of a climb still above it.
const TARGET_ID := &"target_id"
const DISTANCE := &"distance"
const FOOTHOLD := &"foothold"
const ON_FACE := &"on_face"
const CLIMB_LEFT := &"climb_left"
## Which of the keys above an entry holds (EntryKeys), as the digest hashes them.
const KEYS := &"keys"

## Each column's value for a slot no unit has used yet, which is also what a reader takes
## for a key the entry doesn't hold.
const BLANKS := {
	POSITION: Vector2.ZERO,
	LOOSE_AT: Vector2.ZERO,
	FLEE_OFFSET: Vector2.ZERO,
	FLAGS: 0,
	NEXT: Vector2.ZERO,
	GOAL_FOE: 0,
	GOAL_SLOT: -1,
	FOE_AT: Vector2.ZERO,
	TOUCH: 0,
	TOWARD: Vector2.ZERO,
	STUCK: 0.0,
	CLOSEST: INF,
	ALONG: 0.0,
	CAUGHT: 0,
	HELD_BY: 0,
	FANNED: 0.0,
	KEYS: 0,
	BEARING: 90.0,
	RANK: 0,
	COLUMN: 0,
	HP: 0,
	ATTACK_COOLDOWN: 0,
	REGENERATION_LEFT: 0.0,
	STAMINA: 100.0,
	SPEED: 0.0,
	EXERTED: 0,
	BREATHER: 0.0,
	RUN_BUILD: 0.0,
	WAS_RUNNING: 0,
	CONDITION: 1.0,
	REGENERATION_CARRY: 0.0,
	REGENERATION_HALT: 0.0,
	WAKE_LEFT: -1.0,
	DRAIN_CARRY: 0.0,
	LAST_WOUNDING: 0,
	TARGET_ID: 0,
	DISTANCE: 0.0,
	FOOTHOLD: Vector2.ZERO,
	ON_FACE: 0,
	CLIMB_LEFT: 0.0,
}

var _columns := {
	POSITION: PackedVector2Array(),
	LOOSE_AT: PackedVector2Array(),
	FLEE_OFFSET: PackedVector2Array(),
	FLAGS: PackedInt32Array(),
	NEXT: PackedVector2Array(),
	GOAL_FOE: PackedInt64Array(),
	GOAL_SLOT: PackedInt64Array(),
	FOE_AT: PackedVector2Array(),
	TOUCH: PackedInt32Array(),
	TOWARD: PackedVector2Array(),
	STUCK: PackedFloat64Array(),
	CLOSEST: PackedFloat64Array(),
	ALONG: PackedFloat64Array(),
	CAUGHT: PackedInt64Array(),
	HELD_BY: PackedInt64Array(),
	FANNED: PackedFloat64Array(),
	KEYS: PackedInt32Array(),
	BEARING: PackedFloat64Array(),
	RANK: PackedInt32Array(),
	COLUMN: PackedInt32Array(),
	HP: PackedInt32Array(),
	ATTACK_COOLDOWN: PackedInt32Array(),
	REGENERATION_LEFT: PackedFloat64Array(),
	STAMINA: PackedFloat64Array(),
	SPEED: PackedFloat64Array(),
	EXERTED: PackedInt32Array(),
	BREATHER: PackedFloat64Array(),
	RUN_BUILD: PackedFloat64Array(),
	WAS_RUNNING: PackedInt32Array(),
	CONDITION: PackedFloat64Array(),
	REGENERATION_CARRY: PackedFloat64Array(),
	REGENERATION_HALT: PackedFloat64Array(),
	WAKE_LEFT: PackedFloat64Array(),
	DRAIN_CARRY: PackedFloat64Array(),
	LAST_WOUNDING: PackedInt32Array(),
	TARGET_ID: PackedInt64Array(),
	DISTANCE: PackedFloat64Array(),
	FOOTHOLD: PackedVector2Array(),
	ON_FACE: PackedInt32Array(),
	CLIMB_LEFT: PackedFloat64Array(),
}


## The native BodyField the columns live on (UnitColumnsNative); none for these.
func field() -> Object:
	return null


## Runs `call`, a native call on the field that reads or writes `slot`'s columns `names`
## (an entry given or taken away, a key kept), and returns what it returns. A pass's hold
## (ColumnsHeld) lends it that slot's values of those it holds around it.
func lent(_slot: int, _names: Array, call: Callable) -> Variant:
	return call.call()


## Runs `call`, a native call on the field that reads or writes many slots' columns (a
## squad's trade, a lock), and returns what it returns. A hold lends it every held column.
func lent_all(call: Callable) -> Variant:
	return call.call()


## The column, by slot.
func column(name: StringName) -> Variant:
	return _columns[name]


## Replaces the column with `values` (by slot, as long as the column).
func commit(name: StringName, values: Variant) -> void:
	assert(values.size() == _columns[name].size(), "a column keeps its length")
	_columns[name] = values


## The value of one unit's slot.
func value(slot: int, name: StringName) -> Variant:
	return _columns[name][slot]


## Sets the value of one unit's slot.
func put(slot: int, name: StringName, data: Variant) -> void:
	_columns[name][slot] = data


## Readies a slot for a unit just given it: every column blank there.
func open(slot: int) -> void:
	reach(slot)
	for name in _columns:
		_columns[name][slot] = BLANKS[name]


## Grows every column to hold `slot`, each new slot blank; keeps the values held.
func reach(slot: int) -> void:
	for name in _columns:
		var size: int = _columns[name].size()
		if size <= slot:
			_columns[name].resize(slot + 1)
			for added in range(size, slot + 1):
				_columns[name][added] = BLANKS[name]

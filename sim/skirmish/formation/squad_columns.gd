class_name SquadColumns
extends RefCounted
## The squad store (specs/39-simulation-in-rust.md step 1): each squad's state that a pass
## reads or writes, as packed arrays by the squad's slot, the layout of the unit store's
## (UnitColumns, Decision 146). A squad takes a slot as it is made and keeps it for the battle:
## squads are few, and one that has left the lists may still be read (an event, the
## presentation). SkirmishSquad's fields of the same names read and write them here, so a
## pass reads a squad as before; its members are its units' unit-store slots, in its list's
## order. Two backends keep one contract (test_squad_store.gd): this, the reference, and
## SquadColumnsNative on the battle's BodyField (the crate's `Squads`, which serialises).
## Columns are named by index, as the crate's NAMES has them: the ints, then the longs
## (from LONGS), the reals (from REALS) and the point (POSITION).

const DIRECTION := 0
const WIDTH := 1
const ORDER := 2
const STATE := 3
const FOUGHT := 4
const PATCHED_COUNT := 5
const STALL_TICKS := 6
const MANOEUVRE := 7
const MORALE := 8
const RALLY_TICKS := 9
const BLOCKED := 10
const PAINTED_WIDTH := 11
const NARROW_TICKS := 12
const WAIT_TICKS := 13
const REFORMING := 14
const COMBAT_WIDTH := 15
const ENGAGED_WITH := 16
const FIGHT_SINCE := 17
const TAKING_SINCE := 18
const TOOK_UNTIL := 19
const HOME_DISTANCE := 20
const FRONT_DISTANCE := 21
const HEADING := 22
const LOOKED_AT := 23
const PAINTED_SHIFT := 24
const CENTRE_SHIFT := 25
const POSITION := 26
const LONGS := 16
const REALS := 20
const POINTS := 26
## Each column's value in a slot just given: SkirmishSquad's starting values (manoeuvre 4,
## the player's order; morale -1, unread; no tick yet; facing east; never looked ahead).
const BLANKS := [
	0,
	0,
	0,
	0,
	0,
	0,
	0,
	4,
	-1,
	0,
	0,
	0,
	0,
	0,
	0,
	0,
	0,
	-1,
	-1,
	-1,
	0.0,
	0.0,
	90.0,
	-INF,
	0.0,
	0.0,
	Vector2.ZERO,
]

## A slot the crate opened already (FormationStrays.adopt): the next `open` takes it instead
## of opening one, and clears it. -1: none.
var opened := -1
var _columns: Array = []  # by column index, a packed array by slot
var _ids := PackedInt64Array()  # slot -> squad id
var _members: Array[PackedInt32Array] = []  # slot -> its units' slots


func _init() -> void:
	for column in BLANKS.size():
		if column < LONGS:
			_columns.append(PackedInt32Array())
		elif column < REALS:
			_columns.append(PackedInt64Array())
		elif column < POINTS:
			_columns.append(PackedFloat64Array())
		else:
			_columns.append(PackedVector2Array())


## A new squad's slot, every column blank there; or the slot the crate opened, `opened`.
func open(squad_id: int) -> int:
	if opened >= 0:
		return _take_opened()
	_ids.append(squad_id)
	for column in BLANKS.size():
		_columns[column].append(BLANKS[column])
	_members.append(PackedInt32Array())
	return _ids.size() - 1


## Squads with a slot.
func count() -> int:
	return _ids.size()


## A column's value in a slot.
func value(slot: int, column: int) -> Variant:
	return _columns[column][slot]


## Sets a column's value in a slot.
func put(slot: int, column: int, data: Variant) -> void:
	_columns[column][slot] = data


## A squad's members: its units' unit-store slots, in its list's order.
func members(slot: int) -> PackedInt32Array:
	return _members[slot]


func put_members(slot: int, slots: PackedInt32Array) -> void:
	_members[slot] = slots


## The slot the crate opened, taken: `opened` is cleared.
func _take_opened() -> int:
	var slot := opened
	opened = -1
	return slot

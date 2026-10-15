class_name FellQueue
extends RefCounted
## The unit store's fell queue (spec 35 step 5b, Decision 145): the ids of units an hp write
## took from above 0 to 0 or below, in the order they fell, till FormationDeaths.bury takes
## them. Only UnitHealth pushes. It says "I fell", not "I died": bury decides which from
## the unit's hp at the end of the tick's blows. FellQueueNative keeps it on the battle's
## BodyField, so native passes that land damage can push to it too.

var _ids := PackedInt64Array()


## A unit fell.
func push(unit_id: int) -> void:
	_ids.append(unit_id)


## The units fallen since this was last called, in the order they fell; the queue is left
## empty.
func take() -> PackedInt64Array:
	var out := _ids
	_ids = PackedInt64Array()
	return out

class_name FellQueueNative
extends FellQueue
## The fell queue on the battle's native BodyField (spec 35 step 5b): the same contract as
## FellQueue (test_unit_health.gd).

var _field: Object


func _init(field: Object) -> void:
	_field = field


func push(unit_id: int) -> void:
	_field.fell(unit_id)


func take() -> PackedInt64Array:
	return _field.take_fallen()

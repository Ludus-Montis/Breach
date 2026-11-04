class_name SuspicionSystem
extends RefCounted
## Core Suspicion meter and tiered state machine, per
## specs/04-suspicion-and-response.md and the parent spec's Enemy Threat Model
## section. Generalized engine, minimal content per Decision 5 - thresholds are
## data (MapDef.suspicion_tier_thresholds), so a tier with no assigned response unit
## (see TaskForceDispatch) simply has nothing dispatch when entered.
##
## Does not self-subscribe to SimEvents - on_tick_advanced() is a plain method, same
## reasoning as CommandQueue (specs/01).

enum Tier { CALM, WARY, ALARMED, MOBILIZED, FULL_ALERT }

var _thresholds: Array
var _decay_per_tick: int
var _meter: float = 0.0
var _current_tier: Tier = Tier.CALM


func _init(thresholds: Array, decay_per_tick: int) -> void:
	_thresholds = thresholds
	_decay_per_tick = decay_per_tick


func meter() -> int:
	return int(_meter)


func current_tier() -> Tier:
	return _current_tier


func add_suspicion(amount: float) -> void:
	_meter = clamp(_meter + amount, 0.0, 100.0)
	_check_tier_change()


func on_tick_advanced(_tick_number: int) -> void:
	_meter = max(0.0, _meter - _decay_per_tick)
	_check_tier_change()


func _check_tier_change() -> void:
	var new_tier := _tier_for_meter(_meter)
	if new_tier != _current_tier:
		_current_tier = new_tier
		SimEvents.suspicion_tier_changed.emit(new_tier)


func _tier_for_meter(meter_value: float) -> Tier:
	if meter_value >= _thresholds[3]:
		return Tier.FULL_ALERT
	if meter_value >= _thresholds[2]:
		return Tier.MOBILIZED
	if meter_value >= _thresholds[1]:
		return Tier.ALARMED
	if meter_value >= _thresholds[0]:
		return Tier.WARY
	return Tier.CALM

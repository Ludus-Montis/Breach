class_name FormationFieldActions
extends RefCounted
## The feel test's orders on its field, so a session can be recorded and replayed tick for
## tick (Decision 93: the same seed and orders replay a battle). The scene's buttons give
## them through here as commands of the record of play (FormationRecord, Decision 115),
## logged one a line; a record replays the same battle (tools/formation_replay.gd, or the
## scene's Replay button), and so does a version 0 log of the feel test's words. Pure over
## the field it is given.

const FormationField = preload("res://sim/skirmish/formation/formation_field.gd")
const SkirmishUnit = preload("res://sim/skirmish/skirmish_unit.gd")
const UnitDef = preload("res://content/definitions/unit_def.gd")
const FormationRecord = preload("res://sim/skirmish/formation/formation_record.gd")

const BUILDERS := 4
const TICK_SECONDS := 0.1


## The feel test's field under `battle_seed`, its line led by a captain or not.
static func field(battle_seed: int, captained: bool) -> FormationField:
	var captain: UnitDef = CrateContent.unit("kingdom_captain") if captained else null
	return FormationField.new(
		TICK_SECONDS,
		CrateContent.unit("grem"),
		BUILDERS,
		CrateContent.unit("kingdom_militia"),
		CrateContent.unit("grem_chieftain"),
		captain,
		battle_seed
	)


## Gives one command (FormationRecord) to the field; false if it isn't one the field
## takes from that side.
static func apply(field: FormationField, given: Dictionary) -> bool:
	if given.is_empty():
		return false
	var on: bool = given["value"] == "on"
	var wave: String = given["target"]
	if given["who"] == FormationRecord.KINGDOM:
		if given["order"] != "pursue":
			return false
		field.kingdom_line.pursues = on
		field.kingdom_reserve.pursues = on
		return true
	if given["who"] != FormationRecord.PLAYER:
		return false
	match given["order"]:
		"send":
			if wave == "A+B":
				field.send_together(["A", "B"])
			elif field.waves.has(wave):
				field.send(wave)
			else:
				return false
		"retreat":
			if not field.waves.has(wave):
				return false
			for squad in field.sim.squads():
				if squad.faction_id == "player" and squad.route == field.waves[wave].route:
					field.sim.order(squad.id, SkirmishUnit.Order.RETREAT)
		"auto":
			if not field.waves.has(wave):
				return false
			field.set_auto(wave, on)
		"wait":
			field.set_wait(on)
		"tend":
			if not field.waves.has(wave) or given["value"] not in ["leave", "recover", "carry"]:
				return false
			field.tending[wave] = "" if given["value"] == "leave" else given["value"]
		"hurry":
			if not field.waves.has(wave):
				return false
			if on:
				field.hurried[wave] = true
			else:
				field.hurried.erase(wave)
		"route":
			if not field.waves.has(wave) or not field.routes.has(given["value"]):
				return false
			field.waves[wave].route = field.routes[given["value"]]
		_:
			return false
	return true


## A record read back (FormationRecord.parse): its seed, set-up and commands; {} if it
## isn't one.
static func parse(log: String) -> Dictionary:
	return FormationRecord.parse(log)


## The field a log leaves after its last action and `extra` more ticks; after every tick
## `on_events` (the field, the tick's events) is called, if given.
static func replay(log: String, extra: int, on_events: Callable = Callable()) -> FormationField:
	var read := parse(log)
	var played := field(read["seed"], read["captained"])
	var tick := 0
	for queued in read["commands"]:
		while tick < queued["tick"]:
			_step(played, on_events)
			tick += 1
		apply(played, queued)
	for _i in range(extra):
		_step(played, on_events)
	return played


static func _step(played: FormationField, on_events: Callable) -> void:
	var events := played.step()
	if on_events.is_valid():
		on_events.call(played, events)

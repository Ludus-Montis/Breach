class_name ScrumBlowsField
extends RefCounted
## The fight's melee search on the native core (NativeKernels, Decision 129): who strikes
## whom in the scrum (ScrumBlows.blows: each unit of a squad that strikes picks the foe it
## touches, ScrumBlows._pick) worked out on the battle's field, synced to the scrum as it
## stands when the fight begins; and, for rolled blows, BlowLanding's counts of the foes
## pressing each target (BlowLanding._pressed) on that same sync. Only the search crosses:
## cooldowns, the morale's interval, the blows themselves and how they land stay
## GDScript's, taken unit by unit in ScrumBlows' order, so the results are the reference's
## bit for bit (native/README.md).

const BattleTuning = preload("res://content/definitions/battle_tuning.gd")
const SkirmishSquad = preload("res://sim/skirmish/formation/skirmish_squad.gd")
const SkirmishUnit = preload("res://sim/skirmish/skirmish_unit.gd")
const ScrumBlows = preload("res://sim/skirmish/formation/scrum_blows.gd")
const FormationCombat = preload("res://sim/skirmish/formation/formation_combat.gd")
const FormationMorale = preload("res://sim/skirmish/formation/formation_morale.gd")
const BodyFieldSync = preload("res://sim/skirmish/formation/body_field_sync.gd")
const DetMath = preload("res://sim/skirmish/formation/det_math.gd")

## A squad's rules, as the core reads them: it is fighting; it is ordered to retreat.
const FIGHTING := 1
const RETREAT := 2
## A pick's flags: it retreats and has turned away from its foe; the blow is a flank blow.
const AWAY := 1
const FLANK := 2

## Microseconds since last zeroed: [gathering and the sync, the call, the core's own
## work, writing back]. The bench reads them; no outcome does.
static var spent := PackedInt64Array([0, 0, 0, 0])
## The field the last blows() synced (its instance id; 0: that call had no need to).
static var _synced := 0


## ScrumBlows.blows on the field: [[attacker, target, damage, flank], ...]; updates each
## striker's target and cooldown. Clears every living unit's target first, as the fight
## begins (FormationMelee).
static func blows(field: Object, squads: Array, interval: int, fight_seed: int) -> Array:
	var began := Time.get_ticks_usec()
	_synced = 0
	if not _any_strike(squads):
		for squad in squads:
			for unit in squad.living():
				unit.target_id = 0
		return []
	var synced := _sync(field, squads, fight_seed)
	_synced = field.get_instance_id()
	var rules := PackedByteArray()
	for squad in squads:
		var fighting := FIGHTING if squad.state == SkirmishSquad.State.FIGHTING else 0
		rules.append(fighting | (RETREAT if squad.order == SkirmishUnit.Order.RETREAT else 0))
	var front := DetMath.cos(deg_to_rad(BattleTuning.current().reach_front_arc_degrees))
	var floats := PackedFloat64Array([BattleTuning.current().reach_contact, front - 0.000001])
	var called := Time.get_ticks_usec()
	var picks: PackedInt32Array = field.blows(floats, rules)
	var returned := Time.get_ticks_usec()
	var out := _struck(picks, synced[0], synced[1], interval)
	spent[0] += called - began
	spent[1] += returned - called
	spent[2] += field.usec()
	spent[3] += Time.get_ticks_usec() - returned
	return out


## True if this tick's blows() synced `field`, so pressed() may count on it.
static func synced(field: Object) -> bool:
	return _synced == field.get_instance_id()


## BlowLanding's [squad_of, pressed] for these blows ([striker, target, ...]), on the sync
## blows() made: each striker's [unit, squad], and how many touching foes fight each
## target this tick (by the units' targets now).
static func pressed(field: Object, squads: Array, landing: Array) -> Array:
	var targets := PackedInt64Array()
	for squad in squads:
		for unit: SkirmishUnit in squad.units:
			if unit.is_alive():
				targets.append(unit.target_id)
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


## True if some squad may strike in the scrum (ScrumBlows._struck_by would give it foes),
## by the squads' states, orders and factions alone: a squad standing (not routing nor
## destroyed) and fighting or retreating strikes any other faction's standing squad,
## another only one retreating.
static func _any_strike(squads: Array) -> bool:
	var standing := {}  # faction -> true
	var retreating := {}
	for squad in squads:
		if squad.state in [SkirmishSquad.State.ROUTING, SkirmishSquad.State.DESTROYED]:
			continue
		standing[squad.faction_id] = true
		if squad.order == SkirmishUnit.Order.RETREAT:
			retreating[squad.faction_id] = true
	for squad in squads:
		if squad.state in [SkirmishSquad.State.ROUTING, SkirmishSquad.State.DESTROYED]:
			continue
		var fights: bool = squad.state == SkirmishSquad.State.FIGHTING
		var foes := standing if fights or squad.order == SkirmishUnit.Order.RETREAT else retreating
		if foes.size() > (1 if foes.has(squad.faction_id) else 0):
			return true
	return false


## Brings the field to the scrum as BodyFieldSync.scrum does (where ScrumReach puts each
## unit, and its bearing), clearing each living unit's target as it goes; returns [units,
## their squads] by flat index. The arrays are sized once and filled in place.
static func _sync(field: Object, squads: Array, fight_seed: int) -> Array:
	var batch := _sized(squads)
	var units := batch.units
	var owners := batch.owners
	var ids := batch.ids
	var flags := batch.flags
	var bearings := batch.bearings
	var flat := 0
	for squad in squads:
		var before := flat
		var loose: Dictionary = squad.loose
		for unit: SkirmishUnit in squad.units:
			if not unit.is_alive():
				continue
			unit.target_id = 0
			units[flat] = unit
			owners[flat] = squad
			ids[flat] = unit.id
			flags[flat] = BodyFieldSync.LOOSE if loose.has(unit.id) else 0  # the field reads where
			bearings[flat] = unit.bearing
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
	batch.flags.resize(size)
	batch.bearings.resize(size)
	return batch


static func _trimmed(batch: BodyFieldSync.Batch, size: int) -> BodyFieldSync.Batch:
	batch.units.resize(size)
	batch.owners.resize(size)
	batch.ids.resize(size)
	batch.flags.resize(size)
	batch.bearings.resize(size)
	return batch


## Each pick taken as ScrumBlows.blows takes it, in its order: the unit's target, its
## cooldown, and its blow when the cooldown runs out (unless it has turned away).
static func _struck(picks: PackedInt32Array, units: Array, owners: Array, interval: int) -> Array:
	var out := []
	for index in range(0, picks.size(), 3):
		var unit: SkirmishUnit = units[picks[index]]
		var target: SkirmishUnit = units[picks[index + 1]]
		unit.target_id = target.id
		unit.attack_cooldown -= 1
		if unit.attack_cooldown > 0:
			continue
		var usual := ScrumBlows.ticks(interval, unit)
		unit.attack_cooldown = FormationMorale.interval(owners[picks[index]], usual)
		if picks[index + 2] & AWAY == 0:
			var flank: bool = picks[index + 2] & FLANK != 0
			out.append([unit, target, FormationCombat.damage(unit), flank])
	return out

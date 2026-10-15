class_name FormationScrum
extends RefCounted
## Both sides seek contact (Decision 88, spec 27 round 5). While its squad fights, each
## unit leaves its place for the scrum:
## - **Seeking:** a unit touching no foe walks to the nearest open slot beside one's body
##   (ScrumSlots), anywhere within the route's leash of its place (Decision 75) - the far end
##   of a wide line comes too. Only front-band units
##   seek, at any morale short of a rout (Decision 101: a shaken squad strikes softer, not
##   less); the rest keep to their places. Contested slots go by ScrumContest's key, so the
##   earliest arrival takes a slot and the rest look further; one keeps its slot while it
##   is still open.
## - **No delay:** a squad's units seek as soon as its fight begins (Decision 92).
## - **Cohesion:** a disciplined squad whose front is free re-forms its line to meet an
##   enemy closing in at another face, before contact (ScrumStance); a less disciplined one
##   meets it unit by unit, leaving gaps.
## - **Engaging:** squads whose units come within reach fight, whatever their faces.
## - **Regrouping:** when the fight ends the squad closes ranks over its dead (SquadRanks)
##   and its units walk to their places at its re-form pace; it moves on once all are back.
## - **Manoeuvres** go by priority (Decision 94); a fight stalled scrum_stall_seconds is released.
## Each phase decides from where units stood before it began, never letting a squad listed
## earlier move first and change what a later one sees (Decision 97).
## Squads keep `loose`, `stance`, `fight_since` and `stall_ticks`. Pure over the squads.

const BattleTuning = preload("res://content/definitions/battle_tuning.gd")
const SkirmishSquad = preload("res://sim/skirmish/formation/skirmish_squad.gd")
const SkirmishUnit = preload("res://sim/skirmish/skirmish_unit.gd")
const ScrumBlows = preload("res://sim/skirmish/formation/scrum_blows.gd")
const ScrumSeek = preload("res://sim/skirmish/formation/scrum_seek.gd")
const ScrumEngage = preload("res://sim/skirmish/formation/scrum_engage.gd")
const ScrumRegroup = preload("res://sim/skirmish/formation/scrum_regroup.gd")
const UnitMotion = preload("res://sim/skirmish/formation/unit_motion.gd")
const UnitShuffle = preload("res://sim/skirmish/formation/unit_shuffle.gd")
const GroundBodies = preload("res://sim/skirmish/formation/ground_bodies.gd")
const UnitSteer = preload("res://sim/skirmish/formation/unit_steer.gd")
const FormationManoeuvre = preload("res://sim/skirmish/formation/formation_manoeuvre.gd")
const SquadRanks = preload("res://sim/skirmish/formation/squad_ranks.gd")
const FormationWithdraw = preload("res://sim/skirmish/formation/formation_withdraw.gd")
const FormationPursuit = preload("res://sim/skirmish/formation/formation_pursuit.gd")
const ScrumPursuit = preload("res://sim/skirmish/formation/scrum_pursuit.gd")
const ScrumStance = preload("res://sim/skirmish/formation/scrum_stance.gd")
const FormationLocks = preload("res://sim/skirmish/formation/formation_locks.gd")
const FormationMorale = preload("res://sim/skirmish/formation/formation_morale.gd")
const FormationEvents = preload("res://sim/skirmish/formation/formation_events.gd")
const FormationTerrain = preload("res://sim/skirmish/formation/formation_terrain.gd")
const BodyGrid = preload("res://sim/skirmish/formation/body_grid.gd")
const ScrumNear = preload("res://sim/skirmish/formation/scrum_near.gd")
const FoeIndex = preload("res://sim/skirmish/formation/foe_index.gd")
const ScrumReach = preload("res://sim/skirmish/formation/scrum_reach.gd")
const ScrumFaceField = preload("res://sim/skirmish/formation/scrum_face_field.gd")
const ScrumWalkField = preload("res://sim/skirmish/formation/scrum_walk_field.gd")
const UnitMover = preload("res://sim/skirmish/formation/unit_mover.gd")
const UnitColumns = preload("res://sim/skirmish/formation/unit_columns.gd")
const SquadEntries = preload("res://sim/skirmish/formation/squad_entries.gd")
const EntryKeys = preload("res://sim/skirmish/formation/entry_keys.gd")


## One tick of the scrum. `cells_per_second` is the march pace at speed 1; given the
## battle's native bodies (`field`, NativeKernels), the slot search runs on them. Returns
## "faced" and "disengaged" events.
static func step(
	squads: Array,
	tick: int,
	cells_per_second: float,
	tick_seconds: float,
	fight_seed: int,
	terrain: FormationTerrain = null,
	field: Object = null
) -> Array:
	var events := ScrumEngage.step(squads, tick, fight_seed)
	for squad in squads:
		_prepare(squad, tick)
	ScrumStance.anticipate_all(squads, tick, events, terrain, fight_seed)
	FormationManoeuvre.step(squads)
	var ctx := {
		"squads": squads,
		"tick": tick,
		"seconds": tick_seconds,
		"pace": cells_per_second * tick_seconds,
		"seed": fight_seed,
		"terrain": terrain,
		"bodies": ScrumSeek.bodies(squads),
		"lying": GroundBodies.ground(squads),
		"active": {},
		"field": field,  # the bodies, native (Decision 129), or null
	}
	ctx["crowd"] = BodyGrid.of_bodies(ctx["bodies"])  # the bodies, found by where they stand
	_seek(ctx)
	ScrumRegroup.step(squads, ctx["pace"], tick_seconds, fight_seed, terrain)
	ScrumPursuit.step(squads, tick, cells_per_second, tick_seconds, fight_seed, terrain)
	FormationWithdraw.step(squads, tick, ctx["pace"], tick_seconds, fight_seed, terrain, events)
	FormationPursuit.step(squads, tick, cells_per_second, tick_seconds, events)
	_stall(squads, ctx["active"], tick, tick_seconds, events)
	return events


## True while the squad's units are out of their places: it doesn't march. Units out chasing
## or straggling back on their own don't hold it (Decision 112).
static func regrouping(squad: SkirmishSquad) -> bool:
	return squad.loose.keys().any(func(unit_id): return not squad.chasers.has(unit_id))


static func _prepare(squad: SkirmishSquad, tick: int) -> void:
	if squad.state in [SkirmishSquad.State.ROUTING, SkirmishSquad.State.DESTROYED]:
		SquadEntries.unloose_all(squad)
		squad.stance = {}
		squad.fight_since = -1
		return
	for unit_id in squad.loose.keys():
		if not squad.loose[unit_id].is_alive():
			SquadEntries.unloose(squad, unit_id)
	var fighting := squad.state == SkirmishSquad.State.FIGHTING
	if fighting and squad.fight_since < 0:
		squad.fight_since = tick
		squad.fought = true
		squad.stall_ticks = 0
	elif not fighting:
		if squad.fight_since >= 0 and squad.painted.is_empty():
			SquadRanks.close(squad)  # the dead leave holes: the living close up
		squad.fight_since = -1
	if not fighting and squad.stance.is_empty():
		return
	var at: PackedVector2Array = squad.store.columns.column(UnitColumns.POSITION)
	for unit in squad.living():
		if not squad.loose.has(unit.id):
			var stood := at[unit.slot]
			SquadEntries.loosen(squad, unit, stood)


## Plans every fighting squad's seekers (ScrumSeek), then walks and turns its units.
static func _seek(ctx: Dictionary) -> void:
	ScrumSeek.plan(ctx)
	var fighting: Array = ctx["squads"].filter(
		func(s): return s.state == SkirmishSquad.State.FIGHTING
	)
	walk(fighting, ctx)
	face(fighting, ctx)  # after all have moved (a unit stepped up to is seen)


## Walks the fighting squads' units a step each (_walk), each from the bodies as they
## stood before any stepped: in GDScript (the reference), or on the battle's native
## bodies, ctx["field"] (ScrumWalkField, Decision 129) - the same steps, each paying the
## ground it crosses (UnitMover.pay) in GDScript in either engine (_pay_walked).
static func walk(fighting: Array, ctx: Dictionary) -> void:
	if ctx.get("field") != null:
		var seek = ctx["seek_batch"]  # ScrumSeekField.Batch: the units the walk steps
		var from := []
		for member in seek.units.size():
			from.append(SquadEntries.loose_at(seek.owners[member], seek.units[member]))
		ScrumWalkField.walk(ctx)
		_pay_walked(seek, from, ctx)
		return
	for squad in fighting:  # in the crush, but at the march pace in a pursuit
		_walk(
			squad,
			(
				ctx["pace"]
				* (BattleTuning.current().scrum_crowding if squad.pursuit.is_empty() else 1.0)
			),
			ctx
		)


## The native walk's steps (from where each unit stood, `from`) paid for the ground they
## cross, as _walk pays its own: where each now stands, in its entry and for the facing.
static func _pay_walked(seek, from: Array, ctx: Dictionary) -> void:
	if ctx["terrain"] == null:
		return
	var codes: PackedByteArray = seek.plan[0]
	var ats: PackedVector2Array = ctx["face_batch"].ats
	for member in from.size():
		if codes[member] == ScrumWalkField.TOUCH:
			continue  # it didn't step
		var unit: SkirmishUnit = seek.units[member]
		var columns: UnitColumns = seek.owners[member].store.columns
		var walked: Vector2 = columns.value(unit.slot, UnitColumns.LOOSE_AT)
		var paid := UnitMover.pay(unit, from[member], walked, ctx["terrain"])
		columns.put(unit.slot, UnitColumns.LOOSE_AT, paid)
		ats[member] = paid
		if codes[member] != ScrumWalkField.AIM and codes[member] != ScrumWalkField.PRESS:
			columns.put(unit.slot, UnitColumns.NEXT, paid)  # back to its place: it stands where it got
	ctx["face_batch"].ats = ats


## Turns the fighting squads' units once each (_faces), all decided before any turns: in
## GDScript (the reference), or on the battle's native bodies, ctx["field"]
## (ScrumFaceField, Decision 129) - the same bearings.
static func face(fighting: Array, ctx: Dictionary) -> void:
	if ctx.get("field") != null:
		ScrumFaceField.face(ctx)
		return
	var turns := []
	for squad in fighting:
		turns.append_array(_faces(squad, ctx))
	for turn in turns:
		UnitMotion.turn(turn[0], turn[1], ctx["seconds"])


## Fighting units walk a step to their goal, or back towards their place if they have none
## and touch no one, stepping round bodies in their way (UnitSteer) and paying the ground
## they cross (UnitMover, Decision 139); units out chasing on their own are left to ScrumPursuit.
static func _walk(squad: SkirmishSquad, pace: float, ctx: Dictionary) -> void:
	var seconds: float = ctx["seconds"]
	var columns := squad.store.columns
	for unit in squad.living():
		if squad.chasers.has(unit.id):
			continue  # out chasing on its own (ScrumPursuit)
		var full: float = unit.speed * pace
		var at: Vector2 = SquadEntries.loose_at(squad, unit)
		var toward = null
		if columns.value(unit.slot, UnitColumns.GOAL_SLOT) >= 0:
			var next: Vector2 = columns.value(unit.slot, UnitColumns.NEXT)
			toward = next
			var to := UnitSteer.toward(unit, at, next, ctx["bodies"], ctx["seed"], ctx["crowd"])
			var paced := full * _footing(unit, at, to, ctx)
			var slid := UnitMover.slide(unit, at, to, paced, ctx["terrain"])
			columns.put(unit.slot, UnitColumns.LOOSE_AT, slid)
		elif columns.value(unit.slot, UnitColumns.TOUCH) == 0:
			var place := ScrumStance.anchor(squad, unit)
			var heading: float = squad.stance.get("heading", squad.heading)
			var speed := full / seconds
			toward = UnitShuffle.look(unit, at, place, heading, speed)
			var to := UnitSteer.toward(unit, at, place, ctx["bodies"], ctx["seed"], ctx["crowd"])
			var paced := full * _footing(unit, at, to, ctx)
			var slid := UnitMover.slide(unit, at, to, paced, ctx["terrain"])
			columns.put(unit.slot, UnitColumns.LOOSE_AT, slid)
			columns.put(unit.slot, UnitColumns.NEXT, slid)
		EntryKeys.keep(columns, unit.slot, UnitColumns.TOWARD, toward)


## The share of its pace a loose unit keeps stepping towards `to` over the bodies on the
## ground (GroundBodies, spec 30 round 3).
static func _footing(unit: SkirmishUnit, at: Vector2, to: Vector2, ctx: Dictionary) -> float:
	var way := to - at
	if way.length() < 0.000001:
		return 1.0
	return GroundBodies.underfoot(unit, at + way.normalized() * 0.5, ctx["lying"])


## [[unit, the bearing it turns towards], ...]: each unit turns once a tick towards the
## nearest foe it touches, or else so as to arrive facing what it will do - its foe at the
## slot it seeks, or its squad's way at its place - the quicker way (UnitShuffle).
static func _faces(squad: SkirmishSquad, ctx: Dictionary) -> Array:
	if not ctx.has("facing"):  # where they now stand, after the walk: one index a faction
		ctx["facing"] = FoeIndex.of(ctx["squads"])
	var near := FoeIndex.fought(ctx["facing"], squad)
	var reach := BattleTuning.current().reach_contact
	var out := []
	var columns := squad.store.columns
	var ats: PackedVector2Array = columns.column(UnitColumns.LOOSE_AT)
	var goal_slots: PackedInt64Array = columns.column(UnitColumns.GOAL_SLOT)
	for unit in squad.living():
		if squad.chasers.has(unit.id):
			continue
		var at := ats[unit.slot]
		var foes := ScrumNear.around(near, at, ScrumReach.radius(unit) + reach)
		var look = ScrumBlows.nearest_touching(squad, unit, foes, ctx["seed"])
		if look == null and goal_slots[unit.slot] >= 0:
			look = _seeking_look(unit, columns, at, ctx)
		if look == null:
			look = EntryKeys.held(columns, unit.slot, UnitColumns.TOWARD)
		if look != null:
			out.append([unit, UnitMotion.bearing_to(at, look, unit.bearing)])
	return out


## Where a unit making for the slot it seeks looks: so as to arrive facing the foe it will
## touch there, the quicker way (UnitShuffle); `at` is where it stands.
static func _seeking_look(unit: SkirmishUnit, columns: UnitColumns, at: Vector2, ctx: Dictionary):
	var speed: float = (
		unit.speed * ctx["pace"] * BattleTuning.current().scrum_crowding / ctx["seconds"]
	)
	var next: Vector2 = columns.value(unit.slot, UnitColumns.NEXT)
	var foe_at: Vector2 = columns.value(unit.slot, UnitColumns.FOE_AT)
	var bearing := UnitMotion.bearing_to(next, foe_at, unit.bearing)
	return UnitShuffle.look(unit, at, next, bearing, speed)


## Releases fights where nobody on either side has touched or sought a foe for a while.
static func _stall(
	squads: Array, active: Dictionary, tick: int, tick_seconds: float, events: Array
) -> void:
	var stalled := []
	for squad in squads:
		if squad.state != SkirmishSquad.State.FIGHTING:
			continue
		var quiet := not active.has(squad.id)
		for foe_id in ScrumSeek.foe_ids(squad, squads):
			quiet = quiet and not active.has(foe_id)
		squad.stall_ticks = squad.stall_ticks + 1 if quiet else 0
		if squad.stall_ticks >= roundi(BattleTuning.current().scrum_stall_seconds / tick_seconds):
			stalled.append(squad)
	for squad in stalled:
		squad.stall_ticks = 0
		FormationLocks.release(squad, squads)
		events.append(FormationEvents.squad_event("disengaged", tick, squad))

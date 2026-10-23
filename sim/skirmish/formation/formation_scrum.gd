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

const ScrumSeek = preload("res://sim/skirmish/formation/scrum_seek.gd")
const ScrumEngage = preload("res://sim/skirmish/formation/scrum_engage.gd")
const ScrumRegroup = preload("res://sim/skirmish/formation/scrum_regroup.gd")

const GroundBodies = preload("res://sim/skirmish/formation/ground_bodies.gd")

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

const ScrumFaceField = preload("res://sim/skirmish/formation/scrum_face_field.gd")
const ScrumWalkField = preload("res://sim/skirmish/formation/scrum_walk_field.gd")
const UnitColumns = preload("res://sim/skirmish/formation/unit_columns.gd")
const SquadEntries = preload("res://sim/skirmish/formation/squad_entries.gd")


## One tick of the scrum. `cells_per_second` is the march pace at speed 1; the seek, the
## walk and the facing run on the battle's native bodies (`field`, NativeKernels; without
## one, its squads' store's). Returns "faced" and "disengaged" events.
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
		"lying": GroundBodies.ground(squads),
		"active": {},
		"field": field if field != null or squads.is_empty() else squads[0].store.field(),
	}
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


## Plans every fighting squad's seekers (ScrumSeek), then walks and turns its units, on the
## battle's native bodies, ctx["field"] (Decision 129). The walk (ScrumWalkField) steps each
## from the bodies as they stood before any stepped: one making for a slot towards its next
## point, one with no slot that touches no foe back towards its place, round the bodies in
## its way (UnitSteer), over those lying underfoot (GroundBodies), paying the ground it
## crosses (ScrumWalkField.pay); units out chasing on their own are left to ScrumPursuit.
## The facing (ScrumFaceField), after all have moved so a unit stepped up to is seen, turns
## each towards the nearest foe it touches, or else so as to arrive facing what it will do
## (UnitShuffle).
static func _seek(ctx: Dictionary) -> void:
	if ctx["field"] == null:
		return  # no squads: no field to run on
	ScrumSeek.plan(ctx)
	ScrumWalkField.walk(ctx)
	ScrumFaceField.face(ctx)


## Releases fights where nobody on either side has touched or sought a foe for a while, in
## the crate (`manoeuvre::stall`, spec 39 step 4b).
static func _stall(
	squads: Array, active: Dictionary, tick: int, tick_seconds: float, events: Array
) -> void:
	if squads.is_empty():
		return
	var ticks := roundi(BattleTuning.current().scrum_stall_seconds / tick_seconds)
	var stalled: PackedInt32Array = squads[0].store.field().scrum_stall(
		FormationLocks.slots(squads),
		FormationLocks.factions(squads),
		PackedInt64Array(active.keys()),
		ticks
	)
	var by_slot := {}
	for squad in squads:
		by_slot[squad.slot] = squad
	for slot in stalled:
		events.append(FormationEvents.squad_event("disengaged", tick, by_slot[slot]))

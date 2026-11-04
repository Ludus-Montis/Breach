class_name FormationScrum
extends RefCounted
## Both sides seek contact (Decision 88, spec 27 round 5). While its squad fights, each
## unit leaves its place for the scrum:
## - **Seeking:** a unit touching no foe walks to the nearest open slot beside one's body
##   (`scrum.rs`), anywhere within the route's leash of its place (Decision 75) - the far end
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

const FormationManoeuvre = preload("res://sim/skirmish/formation/formation_manoeuvre.gd")
const FormationWithdraw = preload("res://sim/skirmish/formation/formation_withdraw.gd")
const FormationPursuit = preload("res://sim/skirmish/formation/formation_pursuit.gd")
const ScrumPursuit = preload("res://sim/skirmish/formation/scrum_pursuit.gd")
const ScrumStance = preload("res://sim/skirmish/formation/scrum_stance.gd")
const FormationMorale = preload("res://sim/skirmish/formation/formation_morale.gd")
const FormationTerrain = preload("res://sim/skirmish/formation/formation_terrain.gd")


## One tick of the scrum. `cells_per_second` is the march pace at speed 1; the seek, the
## walk and the facing run in the crate on the battle's native bodies (`field`,
## NativeKernels; without one, its squads' store's). The squads' roster (CrateRoster) is
## made once: nobody falls in the scrum. Returns "engaged", "faced" and "disengaged" events.
static func step(
	squads: Array,
	tick: int,
	cells_per_second: float,
	tick_seconds: float,
	fight_seed: int,
	terrain: FormationTerrain = null,
	field: Object = null
) -> Array:
	if field == null and not squads.is_empty():
		field = squads[0].store.field()
	var roster := CrateRoster.of(squads, terrain)  # who is living, through the scrum
	var events := ScrumEngage.step(field, roster, tick, fight_seed)
	_prepare(field, roster, tick)
	ScrumStance.anticipate_all(field, roster, [tick, fight_seed], terrain, events)
	FormationManoeuvre.step(field, roster)
	var ctx := {
		"tick": tick,
		"seconds": tick_seconds,
		"pace": cells_per_second * tick_seconds,
		"seed": fight_seed,
		"terrain": terrain,
		"active": {},
	}
	_seek(field, roster, ctx)
	ScrumRegroup.step(squads, ctx["pace"], tick_seconds, fight_seed, terrain, roster)
	ScrumPursuit.step(squads, tick, cells_per_second, tick_seconds, fight_seed, terrain, roster)
	FormationWithdraw.step(
		squads, tick, ctx["pace"], tick_seconds, fight_seed, terrain, events, roster
	)
	FormationPursuit.step(squads, tick, cells_per_second, tick_seconds, events, roster)
	_stall(field, roster, ctx["active"], [tick, tick_seconds], events)
	return events


## True while the squad's units are out of their places: it doesn't march. Units out chasing
## or straggling back on their own don't hold it (Decision 112). The crate's rule
## (`moving::regrouping`, spec 39 step 8b1), as the move reads it.
static func regrouping(squad: SkirmishSquad) -> bool:
	return squad.store.field().squad_regrouping(squad.slot)


## Readies each squad for the scrum, in the crate (`scrum::prepare::prepare`, spec 39 step
## 8b5-xii): a routing or destroyed one lets its loose units, its stance and its fight go;
## the fallen leave their loose entries; a fight beginning is marked (`fight_since`,
## `fought`), and one ending closes its ranks over the dead (SquadRanks); a squad fighting,
## or holding a stance, sends each living unit loose where it stands.
static func _prepare(field: Object, roster: Object, tick: int) -> void:
	if field != null:
		field.scrum_prepare(roster, tick)


## Seeks, walks and turns every fighting squad's units, in the crate over the scrum's
## roster (ScrumSeek, spec 39 step 8b5-xiii): the squads active this tick go into
## ctx["active"], for the stall.
static func _seek(field: Object, roster: Object, ctx: Dictionary) -> void:
	if field == null:
		return  # no squads: no field to run on
	ScrumSeek.step(field, roster, ctx)


## Releases fights where nobody on either side has touched or sought a foe for a while, in
## the crate (`scrum::pass::stall`, spec 39 step 8b5-xi), each its "disengaged" event in the
## crate's log. `when`: the tick and the seconds a tick lasts.
static func _stall(
	field: Object, roster: Object, active: Dictionary, when: Array, events: Array
) -> void:
	if field == null:
		return  # no squads: nothing to release
	var ticks := roundi(BattleTuning.current().scrum_stall_seconds / when[1])
	field.scrum_stall(roster, PackedInt64Array(active.keys()), ticks, when[0])
	events.append_array(CrateEvents.taken(field))

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
const ScrumRegroup = preload("res://sim/skirmish/formation/scrum_regroup.gd")
const FormationWithdraw = preload("res://sim/skirmish/formation/formation_withdraw.gd")
const FormationPursuit = preload("res://sim/skirmish/formation/formation_pursuit.gd")
const ScrumStance = preload("res://sim/skirmish/formation/scrum_stance.gd")
const FormationTerrain = preload("res://sim/skirmish/formation/formation_terrain.gd")


## One tick of the scrum. `cells_per_second` is the march pace at speed 1. The scrum is one
## crate pass (`scrum::step`, spec 39 step 8b5-xxxiv) on the battle's native bodies
## (`field`, NativeKernels; without one, its squads' store's): engaging, readying, the
## stances, the manoeuvres, the seek, walk and facing, the regroup and chasers, the
## withdrawal, the pursuit and the stall, over one roster (CrateRoster): nobody falls in the
## scrum. Each phase's class hands it its rules. The pursuers back at their posts hold their
## own routes again in GDScript (FormationPursuit.mirror). Returns the scrum's events
## ("engaged", "faced", "disengaged" and the rest) in the order its phases told them.
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
	if field == null:
		return []  # no squads: no scrum
	var roster := CrateRoster.of(squads, terrain)  # who is living, through the scrum
	var ctx := {"seconds": tick_seconds, "pace": cells_per_second * tick_seconds}
	ctx["cells_per_second"] = cells_per_second
	var tuning := BattleTuning.current()
	var stall := roundi(tuning.scrum_stall_seconds / tick_seconds)
	var back: PackedInt32Array = field.scrum_step(
		roster,
		UnitMover.ground(terrain),
		PackedInt64Array([tick, fight_seed, tuning.combat_contest_die, stall]),
		tuning.reach_engage,
		ScrumStance.rules(),
		ScrumSeek.rules(ctx),
		ScrumRegroup.rules(ctx),
		FormationWithdraw.rules(ctx),
		FormationPursuit.rules(cells_per_second, tick_seconds)
	)
	ScrumSeek.clock_usec += field.usec()  # the seek's own time, for the benches
	var events := CrateEvents.taken(field)
	for index in back:
		FormationPursuit.mirror(squads[index], field)
	return events


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

class_name ScrumStance
extends RefCounted
## A disciplined squad meets a threat as a line (Decisions 88 and 92, spec 27 rounds 5
## and 6): marching or holding, with its front free, when it sees an enemy closing in on
## another face within scrum_anticipate cells and is disciplined enough (FormationDiscipline), it
## re-lays its places facing the threat at that face (its stance: its frame turned a
## quarter, half or three quarters about) before contact, and
## commits to it: it faces that enemy while it is still a threat, not swinging to another. Its
## units walk there at its re-form pace and keep to those places until the threat has
## gone; it doesn't march meanwhile. A less disciplined squad meets it unit by unit. Two
## threats equally near: the squads' seeded draw picks, not the list (Decision 97). Pure.

const BattleTuning = preload("res://content/definitions/battle_tuning.gd")
const SkirmishSquad = preload("res://sim/skirmish/formation/skirmish_squad.gd")
const SkirmishUnit = preload("res://sim/skirmish/skirmish_unit.gd")
const SquadFrame = preload("res://sim/skirmish/formation/squad_frame.gd")
const SquadEdges = preload("res://sim/skirmish/formation/squad_edges.gd")
const UnitMotion = preload("res://sim/skirmish/formation/unit_motion.gd")
const FormationDiscipline = preload("res://sim/skirmish/formation/formation_discipline.gd")
const FormationSight = preload("res://sim/skirmish/formation/formation_sight.gd")
const FormationEvents = preload("res://sim/skirmish/formation/formation_events.gd")
const FormationTerrain = preload("res://sim/skirmish/formation/formation_terrain.gd")
const ScrumContest = preload("res://sim/skirmish/formation/scrum_contest.gd")
const SquadMemo = preload("res://sim/skirmish/formation/squad_memo.gd")


## The place a unit keeps to: in its squad's stance if it has one. `places`: a view of the
## squad's store holding UnitView.PLACES.
static func anchor(squad: SkirmishSquad, unit: SkirmishUnit, places: UnitView) -> Vector2:
	return _anchor(squad, unit, UnitPlace.in_view(places, unit))


## `anchor` for one unit, its place read from the store: scattered, for an event's few.
static func anchor_one(squad: SkirmishSquad, unit: SkirmishUnit) -> Vector2:
	return _anchor(squad, unit, UnitPlace.of(squad, unit))


static func _anchor(squad: SkirmishSquad, unit: SkirmishUnit, at: Vector2i) -> Vector2:
	if squad.stance.is_empty():
		return SquadFrame.place(
			squad.position, squad.heading, squad.width, squad.centre_shift, unit, at
		)
	var stance := squad.stance
	return SquadFrame.place(stance["anchor"], stance["heading"], squad.width, 0.0, unit, at)


## anchor()'s frame for the squad, worked out once for all its units: SquadFrame.place_in
## gives each unit's anchor from it, bit for bit.
static func frame(squad: SkirmishSquad) -> Array:
	if squad.stance.is_empty():
		return SquadFrame.basis(squad.position, squad.heading, squad.width, squad.centre_shift)
	return SquadFrame.basis(squad.stance["anchor"], squad.stance["heading"], squad.width, 0.0)


## Every squad, in turn, meets the threats it sees (anticipate), sharing one SquadMemo: a
## stance moves no squad's frame or units.
static func anticipate_all(
	squads: Array, tick: int, events: Array, terrain: FormationTerrain, fight_seed: int = 0
) -> void:
	var memo := SquadMemo.new()
	for squad in squads:
		anticipate(squad, squads, tick, events, terrain, fight_seed, memo)


## A disciplined squad with a free front turns its line to meet a threat it sees closing
## in at another face; it lets the stance go once none is and it isn't fighting. `memo`:
## the phase's (SquadMemo).
static func anticipate(
	squad: SkirmishSquad,
	squads: Array,
	tick: int,
	events: Array,
	terrain: FormationTerrain,
	fight_seed: int = 0,
	memo: SquadMemo = null
) -> void:
	if memo == null:
		memo = SquadMemo.new()
	var able := (
		squad.state
		in [SkirmishSquad.State.HOLDING, SkirmishSquad.State.MOVING, SkirmishSquad.State.FIGHTING]
	)
	if not able or squad.engaged_with != 0:
		return
	if not FormationDiscipline.meets_threats(squad):
		if squad.state != SkirmishSquad.State.FIGHTING:
			squad.stance = {}  # it can't hold a re-formed line: back to its places
		return
	if _committed(squad, squads, memo):
		return  # it made its call: it faces that enemy while it is still a threat
	var seeing := [terrain, fight_seed, memo]
	var found := _threat(squad, squads, seeing, true)
	var threat: int = found[0]
	if threat < 0 or threat == SquadEdges.FRONT:
		var gone: bool = _threat(squad, squads, seeing, false)[0] < 0
		var holds := squad.order == SkirmishUnit.Order.HOLD and not gone
		if (
			squad.state != SkirmishSquad.State.FIGHTING
			and (not holds or threat == SquadEdges.FRONT)
		):
			squad.stance = {}  # a holding line keeps it while that enemy is near (Decision 94)
		return
	var heading := fposmod(squad.heading + threat * 90.0, 360.0)
	if not squad.stance.is_empty() and is_equal_approx(squad.stance["heading"], heading):
		return
	var centre := memo.bounds(squad).get_center()
	var outward := UnitMotion.vector(heading)
	var face := centre + outward * (memo.reach(squad, outward).y - centre.dot(outward))
	squad.stance = {"anchor": face, "heading": heading, "foe": found[1]}
	var faced := {"heading": heading, "facing": posmod(roundi(heading / 90.0), 4)}
	events.append(FormationEvents.squad_event("faced", tick, squad, faced))


## True if the squad holds a re-formed line towards an enemy that is still a threat - alive,
## not routing, within scrum_anticipate cells, and closing in (or near a line ordered to hold): it
## doesn't swing to another.
static func _committed(squad: SkirmishSquad, squads: Array, memo: SquadMemo) -> bool:
	if squad.stance.is_empty() or not squad.stance.has("foe"):
		return false
	for other in squads:
		if other.id != squad.stance["foe"]:
			continue
		if other.is_destroyed() or other.state == SkirmishSquad.State.ROUTING:
			return false
		var holding := squad.order == SkirmishUnit.Order.HOLD
		var pressing := holding or _closing(other, squad, memo)
		return pressing and _gap(squad, other, memo) <= BattleTuning.current().scrum_anticipate
	return false


## [edge, squad id]: the edge of the squad's frame (SquadEdges) facing the nearest seen
## hostile within scrum_anticipate cells - closing in, if `closing` - or [-1, 0]. `seeing` = [the
## terrain, the battle seed, the phase's SquadMemo].
static func _threat(squad: SkirmishSquad, squads: Array, seeing: Array, closing: bool) -> Array:
	var terrain: FormationTerrain = seeing[0]
	var memo: SquadMemo = seeing[2]
	var area := memo.bounds(squad)
	var best := [BattleTuning.current().scrum_anticipate + 0.000001, 0]
	var edge := -1
	var foe := 0
	for other in squads:
		if other.faction_id == squad.faction_id or memo.living(other).is_empty():
			continue
		if other.state == SkirmishSquad.State.ROUTING:
			continue
		if not FormationSight.detects(squad, other, terrain, memo):
			continue
		if closing and not _closing(other, squad, memo):
			continue
		var gap := [
			snappedf(_gap(squad, other, memo), 0.000001), ScrumContest.squad_draw(other, seeing[1])
		]
		if gap < best:
			best = gap
			var theirs := memo.bounds(other)
			var point := area.get_center().clamp(theirs.position, theirs.end)
			var bearing := UnitMotion.bearing_to(area.get_center(), point, squad.heading)
			edge = posmod(roundi(fposmod(bearing - squad.heading, 360.0) / 90.0), 4)
			foe = other.id
	return [edge, foe]


static func _gap(squad: SkirmishSquad, other: SkirmishSquad, memo: SquadMemo) -> float:
	var area := memo.bounds(squad)
	var theirs := memo.bounds(other)
	var nearest := theirs.get_center().clamp(area.position, area.end)
	return nearest.distance_to(nearest.clamp(theirs.position, theirs.end))


## True if `other` is closing in on `squad`: marching with its front towards it, or
## fighting it. A line holding its ground, or fighting someone else, is no reason to
## re-form.
static func _closing(other: SkirmishSquad, squad: SkirmishSquad, memo: SquadMemo) -> bool:
	var fights_it: bool = (
		other.engaged_with == squad.id
		or other.flank_contacts.values().any(func(c): return c["foe"] == squad.id)
	)
	if fights_it:
		return true
	if other.state != SkirmishSquad.State.MOVING:
		return false
	var towards := memo.bounds(squad).get_center() - memo.bounds(other).get_center()
	return towards.dot(UnitMotion.vector(other.heading)) > 0.0

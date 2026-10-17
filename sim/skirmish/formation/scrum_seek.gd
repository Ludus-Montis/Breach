class_name ScrumSeek
extends RefCounted
## Seeking in the scrum (Decision 88, spec 30 round 1): each tick a fighting squad's free
## front-band units make for slots beside their foes' bodies (ScrumSlots), in contest order
## (ScrumContest) - first those keeping a slot still open, then the rest picking the
## nearest open one - so the earliest arrival takes a slot and the rest look further; with
## none open, it waits just behind the nearest (Decision 108), in a pursuit too (Decision
## 113). A unit touching a foe stands; one not free to seek keeps to its place. Pure over
## the squads.

const SkirmishSquad = preload("res://sim/skirmish/formation/skirmish_squad.gd")
const SkirmishUnit = preload("res://sim/skirmish/skirmish_unit.gd")
const ScrumReach = preload("res://sim/skirmish/formation/scrum_reach.gd")
const ScrumContest = preload("res://sim/skirmish/formation/scrum_contest.gd")
const ScrumBlows = preload("res://sim/skirmish/formation/scrum_blows.gd")
const ScrumSlots = preload("res://sim/skirmish/formation/scrum_slots.gd")
const ScrumStance = preload("res://sim/skirmish/formation/scrum_stance.gd")
const ScrumNear = preload("res://sim/skirmish/formation/scrum_near.gd")
const SlotSearch = preload("res://sim/skirmish/formation/slot_search.gd")
const FoeIndex = preload("res://sim/skirmish/formation/foe_index.gd")
const BattleTuning = preload("res://content/definitions/battle_tuning.gd")
const SquadEntries = preload("res://sim/skirmish/formation/squad_entries.gd")
const EntryKeys = preload("res://sim/skirmish/formation/entry_keys.gd")

## Microseconds spent in plan() since it was last zeroed (the bench reads it; no outcome does).
static var clock_usec := 0


## [[where, radius, unit], ...]: every body standing, for which slots are open.
static func bodies(squads: Array) -> Array:
	return ScrumSlots.bodies(squads)


## Plans every fighting squad's units: in GDScript (the reference), or on the battle's
## native bodies, ctx["field"] (ScrumSeekField, Decision 129) - the same choices.
static func plan(ctx: Dictionary) -> void:
	var began := Time.get_ticks_usec()
	if ctx.get("field") != null:
		ScrumSeekField.plan(ctx)
	else:
		_plan(ctx)
	clock_usec += Time.get_ticks_usec() - began


## Plans and walks every fighting squad's units, seekers in contest order: first those
## keeping slots still open, then the rest picking the nearest open one. `ctx["crowd"]` is
## a BodyGrid over ctx["bodies"]; slots, foes and claims are found through grids too
## (SlotSearch, ScrumNear), the same choices as looking through every one.
static func _plan(ctx: Dictionary) -> void:
	ctx["rings"] = {}  # squad -> {seeker radius -> the slots round its foes (SlotSearch)}
	ctx["claims"] = {}  # cell -> points claimed (SlotSearch.claim)
	ctx["foes"] = FoeIndex.of(ctx["squads"])  # where they stand as the seeking begins
	var seekers := []
	for squad in ctx["squads"]:
		if squad.state == SkirmishSquad.State.FIGHTING:
			seekers.append_array(_seekers_of(squad, ctx))
	seekers.sort_custom(func(a, b): return ScrumContest.before(a[0], b[0]))
	var claimed := []
	var picking := []
	for seeker in seekers:
		var kept := _kept(seeker, ctx, claimed)
		if kept.is_empty():
			picking.append(seeker)
		else:
			_aim(seeker, kept, claimed, ctx)
	# store: scattered (reference: the GDScript seek; the Rust engine runs ScrumSeekField)
	for seeker in picking:
		var squad: SkirmishSquad = seeker[1]
		var unit: SkirmishUnit = seeker[2]
		var ground := _ground(squad, unit, claimed, ctx, ctx["seed"])
		var at: Vector2 = SquadEntries.loose_at(squad, unit)
		var facing: float = squad.store.columns.value(unit.slot, UnitColumns.BEARING)
		var slot := SlotSearch.pick(unit, at, facing, seeker[3], ground)
		if slot.is_empty():  # it waits behind the nearest, a pursuit too (Decision 113)
			_press(seeker, SlotSearch.pick(unit, at, facing, seeker[3], ground, true), ctx)
		else:
			_aim(seeker, slot, claimed, ctx)


## ScrumSlots' ground for the seeker, with SlotSearch's grids of bodies and claims.
static func _ground(
	squad: SkirmishSquad, unit: SkirmishUnit, claimed: Array, ctx: Dictionary, draw_seed: int
) -> Array:
	return [
		ScrumStance.anchor_one(squad, unit),  # the GDScript reference's seek, per unit
		ctx["bodies"],
		claimed,
		ctx["terrain"],
		draw_seed,
		ctx["crowd"],
		ctx["claims"],
	]


## The seeker's slot this tick if it still holds one that is open, or [].
static func _kept(seeker: Array, ctx: Dictionary, claimed: Array) -> Array:
	var squad: SkirmishSquad = seeker[1]
	var unit: SkirmishUnit = seeker[2]
	var goal = EntryKeys.goal(squad.store.columns, unit.slot)
	if goal == null:
		return []
	var slot := SlotSearch.find(seeker[3], goal)
	if slot.is_empty():
		return []
	return slot if SlotSearch.open(unit, slot[0], _ground(squad, unit, claimed, ctx, 0)) else []


## Sets the seeker making for `slot` (none if empty), and claims it.
static func _aim(seeker: Array, slot: Array, claimed: Array, ctx: Dictionary) -> void:
	var squad: SkirmishSquad = seeker[1]
	var columns := squad.store.columns
	var unit_slot: int = seeker[2].slot
	if slot.is_empty():
		EntryKeys.set_goal(columns, unit_slot, null)
		columns.put(unit_slot, UnitColumns.NEXT, SquadEntries.loose_at(squad, seeker[2]))
		return
	EntryKeys.set_goal(columns, unit_slot, [slot[1].id, slot[3]])
	columns.put(unit_slot, UnitColumns.NEXT, slot[0])
	EntryKeys.keep(columns, unit_slot, UnitColumns.FOE_AT, ScrumReach.at(slot[2], slot[1]))
	claimed.append(slot[0])
	SlotSearch.claim(ctx["claims"], slot[0])
	ctx["active"][squad.id] = true


## Sets the seeker making for a body's breadth short of `slot`, a taken one (none if
## empty), claiming nothing: it waits there for a slot to open.
static func _press(seeker: Array, slot: Array, ctx: Dictionary) -> void:
	if slot.is_empty():
		_aim(seeker, slot, [], ctx)
		return
	var squad: SkirmishSquad = seeker[1]
	var columns := squad.store.columns
	var unit_slot: int = seeker[2].slot
	var at: Vector2 = SquadEntries.loose_at(squad, seeker[2])
	var back: Vector2 = at - slot[0]
	var breadth := 2.0 * ScrumReach.radius(seeker[2])
	EntryKeys.set_goal(columns, unit_slot, [slot[1].id, slot[3]])
	var next: Vector2 = at if back.length() <= breadth else slot[0] + back.normalized() * breadth
	columns.put(unit_slot, UnitColumns.NEXT, next)
	EntryKeys.keep(columns, unit_slot, UnitColumns.FOE_AT, ScrumReach.at(slot[2], slot[1]))
	ctx["active"][squad.id] = true


## [[key, squad, unit, slots], ...] for the squad's units free to seek, the slots round
## its foes as SlotSearch.ring gives them; the rest stand (touching a foe) or keep to their
## places.
static func _seekers_of(squad: SkirmishSquad, ctx: Dictionary) -> Array:
	var near := FoeIndex.fought(ctx["foes"], squad)
	var reach := BattleTuning.current().reach_contact
	var front_left := squad.living().any(func(u): return u.preferred_position == 0)
	var out := []
	var columns := squad.store.columns
	var ats: PackedVector2Array = columns.column(UnitColumns.LOOSE_AT)
	var speeds: PackedFloat64Array = columns.column(UnitColumns.SPEED)
	# store: scattered (reference: the GDScript seek; the Rust engine runs ScrumSeekField)
	for unit in squad.living():
		if squad.chasers.has(unit.id):
			continue  # out chasing on its own (ScrumPursuit)
		var radius := ScrumReach.radius(unit)
		var at := ats[unit.slot]
		var close := ScrumNear.around(near, at, radius + reach)
		var touch := ScrumBlows.touches_any(squad, unit, close)
		EntryKeys.keep(columns, unit.slot, UnitColumns.TOUCH, 1 if touch else 0)
		if touch:
			EntryKeys.set_goal(columns, unit.slot, null)
			columns.put(unit.slot, UnitColumns.NEXT, at)
			ctx["active"][squad.id] = true
			continue
		if not _may_seek(squad, unit, front_left) or not near["any"]:
			EntryKeys.set_goal(columns, unit.slot, null)
			continue
		var speed := speeds[unit.slot]
		var arrival := ScrumNear.gap_to(near, at, radius) / maxf(speed, 0.01)
		var key := ScrumContest.key(unit, arrival, speed, ctx["seed"], ctx["tick"])
		out.append([key, squad, unit, _ring(squad, near, radius, ctx)])
	return out


## The slots round the squad's foes (`near`: FoeIndex.fought) for a seeker of `radius`, made
## once a tick.
static func _ring(
	squad: SkirmishSquad, near: Dictionary, radius: float, ctx: Dictionary
) -> Dictionary:
	if not ctx["rings"].has(squad):
		ctx["rings"][squad] = {}
	var rings: Dictionary = ctx["rings"][squad]
	if not rings.has(radius):
		if not rings.has("foes"):
			rings["foes"] = FoeIndex.listed(near)
		rings[radius] = SlotSearch.ring(rings["foes"], radius, ctx["bodies"], ctx["crowd"])
	return rings[radius]


## Front-band units seek; the rest too once no front-band unit is left (`front_left`: one
## is).
static func _may_seek(squad: SkirmishSquad, unit: SkirmishUnit, front_left: bool) -> bool:
	return (
		(unit.preferred_position == 0 or not front_left)
		and unit.footprint_width == 1
		and unit.footprint_depth == 1
		and squad.order != SkirmishUnit.Order.RETREAT
	)


## Ids of the squads it fights: on its front, its edges, or that fight it.
static func foe_ids(squad: SkirmishSquad, squads: Array) -> Array:
	var out := []
	for other in squads:
		if other.faction_id == squad.faction_id:
			continue
		var theirs: bool = other.flank_contacts.values().any(func(c): return c["foe"] == squad.id)
		var mine: bool = squad.flank_contacts.values().any(func(c): return c["foe"] == other.id)
		if other.id == squad.engaged_with or other.engaged_with == squad.id or theirs or mine:
			out.append(other.id)
	return out


## [[unit, squad], ...] for the living units of the squads it fights.
static func foe_units(squad: SkirmishSquad, squads: Array) -> Array:
	var ids := foe_ids(squad, squads)
	var out := []
	for other in squads:
		if ids.has(other.id) and other.state != SkirmishSquad.State.ROUTING:
			for unit in other.living():
				out.append([unit, other])
	return out

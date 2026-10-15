class_name ScrumPursuit
extends RefCounted
## Retreat and pursuit (Decisions 95 and 109, spec 27 round 8). When a formation is
## ordered to retreat out of a fight:
## - **The retreat:** its locks end at once and it withdraws (FormationWithdraw, Decision
##   99): its units flee from where they stand until it is safe. A ragged one also pays a
##   scaled rout: a morale shock in proportion to how far short of drilled it is.
## - **Its enemies:** any of their units still touching it strike it as it goes
##   (ScrumBlows). Each follows it as a whole, to the leash its discipline gives it
##   (FormationPursuit, Decision 107), unless ordered not to pursue. And each of their units
##   near it may break ranks to chase on its own - decided per unit, by its discipline and a
##   seeded roll - as far from where it broke away as its own discipline leashes it, while
##   it can see its quarry; then it goes back to its formation.
## Chasers all pick their quarry before any moves: the nearest unit, ties by its seeded
## draw (Decision 97).
## Squads keep `pursues` and `chasers` (unit id -> {"unit", "foe", "from", "leash"}). Pure
## over the squads it is given.

const BattleTuning = preload("res://content/definitions/battle_tuning.gd")
const SkirmishSquad = preload("res://sim/skirmish/formation/skirmish_squad.gd")
const SkirmishUnit = preload("res://sim/skirmish/skirmish_unit.gd")
const ScrumReach = preload("res://sim/skirmish/formation/scrum_reach.gd")
const UnitMotion = preload("res://sim/skirmish/formation/unit_motion.gd")
const BattleRolls = preload("res://sim/skirmish/formation/battle_rolls.gd")
const FormationDiscipline = preload("res://sim/skirmish/formation/formation_discipline.gd")
const FormationLocks = preload("res://sim/skirmish/formation/formation_locks.gd")
const FormationMorale = preload("res://sim/skirmish/formation/formation_morale.gd")
const FormationEvents = preload("res://sim/skirmish/formation/formation_events.gd")
const FormationWithdraw = preload("res://sim/skirmish/formation/formation_withdraw.gd")
const FormationPursuit = preload("res://sim/skirmish/formation/formation_pursuit.gd")
const FormationStamina = preload("res://sim/skirmish/formation/formation_stamina.gd")
const ScrumStance = preload("res://sim/skirmish/formation/scrum_stance.gd")
const ScrumSlots = preload("res://sim/skirmish/formation/scrum_slots.gd")
const UnitSteer = preload("res://sim/skirmish/formation/unit_steer.gd")
const ScrumNear = preload("res://sim/skirmish/formation/scrum_near.gd")
const BodyGrid = preload("res://sim/skirmish/formation/body_grid.gd")
const TerrainStep = preload("res://sim/skirmish/formation/terrain_step.gd")
const UnitMover = preload("res://sim/skirmish/formation/unit_mover.gd")
const FormationTerrain = preload("res://sim/skirmish/formation/formation_terrain.gd")
const SquadEntries = preload("res://sim/skirmish/formation/squad_entries.gd")

## A unit breaks ranks to chase with a chance of (discipline_meets_threats - its
## discipline) / 100.


## Applies a retreat order to `squad`: ends its fight, costs a ragged one its scaled rout,
## and sets its enemies pursuing or chasing. Returns events.
static func retreat(
	squad: SkirmishSquad, squads: Array, tick: int, battle_seed: int, _seconds: float
) -> Array:
	var events := []
	var enemies := squads.filter(func(s): return _fights(s, squad))
	FormationLocks.release(squad, squads)
	events.append(FormationEvents.squad_event("disengaged", tick, squad))
	var short := FormationWithdraw.disorder(squad)
	if short > 0.0:
		FormationMorale.shock(
			squad, roundi(BattleTuning.current().pursuit_ragged_shock * short), tick, events
		)
	FormationWithdraw.begin(squad, tick, events)
	for enemy in enemies:
		if enemy.pursues:
			FormationPursuit.begin(enemy, squad, tick, events)
		_tempt(enemy, squad, tick, battle_seed)
	return events


## One tick of chasing: each chaser walks at the march pace, paying the ground it crosses
## (UnitMover), after the nearest unit of the enemy it chases; once at its leash, out of
## sight of that enemy, or that enemy gone, it straggles back on its own to its place (or
## the nearest it can walk to it: TerrainStep.returns), wherever its formation now is, and
## rejoins it there - the formation doesn't wait for it (Decision 112).
static func step(
	squads: Array,
	_tick: int,
	cells_per_second: float,
	seconds: float,
	fight_seed: int = 0,
	terrain: FormationTerrain = null
) -> void:
	var by_id := {}
	for entry in squads:
		by_id[entry.id] = entry
	var walks := []  # [unit, its place in the scrum, where it heads]
	var quarries := {}  # foe squad -> its units' index (ScrumNear), where they stand now
	for squad in squads:
		var homes := (
			{} if squad.chasers.is_empty() else TerrainStep.returns(squad, terrain, fight_seed)
		)
		for unit_id in squad.chasers.keys():
			var chase: Dictionary = squad.chasers[unit_id]
			var foe: SkirmishSquad = by_id.get(chase["foe"])
			var unit: SkirmishUnit = chase["unit"]
			if not unit.is_alive() or not squad.loose.has(unit_id):
				squad.chasers.erase(unit_id)
				continue
			var heads: Vector2 = homes.get(unit_id, ScrumStance.anchor(squad, unit))  # back
			if not chase.get("returning", false):
				var at: Vector2 = SquadEntries.loose_at(squad, unit)
				var quarry = _quarry(at, unit, foe, quarries, fight_seed)
				if _done(chase, at, unit, quarry):
					chase["returning"] = true
				else:
					heads = quarry
			walks.append([squad, unit, heads, chase.get("returning", false)])
	var bodies := ScrumSlots.bodies(squads) if not walks.is_empty() else []
	var crowd := BodyGrid.of_bodies(bodies) if not walks.is_empty() else {}
	for walk in walks:
		var unit: SkirmishUnit = walk[1]
		var full: float = unit.speed * cells_per_second * seconds
		var at: Vector2 = SquadEntries.loose_at(walk[0], unit)
		var to := UnitSteer.toward(unit, at, walk[2], bodies, fight_seed, crowd)
		var stepped := UnitMover.step(unit, at, to, full, seconds, terrain)
		walk[0].store.columns.put(unit.slot, UnitColumns.LOOSE_AT, stepped)
		walk[0].store.columns.put(unit.slot, UnitColumns.NEXT, stepped)
		if walk[3] and stepped.distance_to(walk[2]) < 0.000001:
			walk[0].chasers.erase(unit.id)  # back in its place: it rejoins its formation
			SquadEntries.unloose(walk[0], unit.id)


## Where the nearest unit of the chaser's foe stands (_nearest_in), or null if the chase
## is over before it looks: its foe gone, or it too tired. `quarries` keeps each foe's index.
static func _quarry(
	at: Vector2, unit: SkirmishUnit, foe: SkirmishSquad, quarries: Dictionary, fight_seed: int
):
	if foe == null or foe.is_destroyed() or FormationStamina.gives_up(unit):
		return null
	if not quarries.has(foe):
		quarries[foe] = _index(foe)
	return _nearest_in(quarries[foe], at, fight_seed)


## True if a chase is over: its quarry (null) gone, the chaser at its leash or too tired,
## or the quarry out of its sight.
static func _done(chase: Dictionary, at: Vector2, unit: SkirmishUnit, quarry) -> bool:
	if quarry == null:
		return true
	return (
		at.distance_to(chase["from"]) >= chase["leash"] or quarry.distance_to(at) > unit.detection
	)


## True if the squad's unit is away from its place on its own - chasing, or its squad
## withdrawing - so its squad leaves it out of regrouping.
static func away(squad: SkirmishSquad, unit_id: int) -> bool:
	return squad.chasers.has(unit_id) or not squad.withdraw.is_empty()


static func _fights(other: SkirmishSquad, squad: SkirmishSquad) -> bool:
	if other.faction_id == squad.faction_id:
		return false
	var flanking: bool = other.flank_contacts.values().any(func(c): return c["foe"] == squad.id)
	return other.engaged_with == squad.id or squad.engaged_with == other.id or flanking


## Each of the enemy's units near the retreating squad may break ranks to chase it.
static func _tempt(enemy: SkirmishSquad, squad: SkirmishSquad, tick: int, battle_seed: int) -> void:
	var near := _index(squad)
	for unit in enemy.living():
		var at := ScrumReach.at(enemy, unit)
		if (
			_nearest_in(near, at, battle_seed).distance_to(at)
			> BattleTuning.current().pursuit_tempted_within
		):
			continue
		var steadied := FormationDiscipline.unit_discipline(enemy, unit)  # its leader's too
		var chance := float(BattleTuning.current().discipline_meets_threats - steadied) / 100.0
		if BattleRolls.uniform(battle_seed, [tick, unit.id, "chase"]) >= chance:
			continue
		if not enemy.loose.has(unit.id):
			SquadEntries.loosen(enemy, unit, at)
		var leash := FormationDiscipline.unit_leash(enemy, unit)
		enemy.chasers[unit.id] = {"unit": unit, "foe": squad.id, "from": at, "leash": leash}


## Where the squad's unit nearest `at` stands (ties by the units' draws); `at` if none.
## `near` is the squad's index (_index).
static func _nearest_in(near: Dictionary, at: Vector2, fight_seed: int) -> Vector2:
	var found := ScrumNear.nearest(near, at, fight_seed)
	return at if found < 0 else near["points"][found]


## The squad's living units, indexed where they stand (ScrumNear).
static func _index(squad: SkirmishSquad) -> Dictionary:
	return ScrumNear.index(squad.living().map(func(unit): return [unit, squad]))

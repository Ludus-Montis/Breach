extends SceneTree
## Where a mid-fight tick's time goes, phase by phase (Decision 129: what to port next): a
## FormationBench clash stepped through FormationSimulation.step's phases in its order, the
## scrum's (FormationScrum.step) split out, each timed; ms
## a tick, largest first. It
## mirrors step(), so it checks itself: the battle it plays must end tick for tick where a
## plain step() leaves it, or it says so.
##
##   godot --headless --path . --script res://tools/tick_phases.gd -- \
##       [side=1024] [width=32] [settle=10] [fight=5] [seed=1]
##       [stage=fight|march] [retreat=<squad id>] [squads=1] [archers=0] [copies]
## copies also counts each phase's column copies across the boundary
## (ColumnCopies); counting slows the phases it counts, so time without it.
## squads=N splits each side into N squads of side/N on parallel lanes (FormationBench.lanes);
## archers=K makes the grems' back K a squad spitters, which shoot.
## stage=march times `fight` ticks of the march before contact, `settle` ticks after the
## start, instead of the fight. retreat orders that squad to retreat once the fight has
## settled in (its withdrawal, and its enemies' chasers, timed).

const FormationBench = preload("res://sim/skirmish/formation/formation_bench.gd")
const Sim = preload("res://sim/skirmish/formation/formation_simulation.gd")
const Scrum = preload("res://sim/skirmish/formation/formation_scrum.gd")
const ScrumEngage = preload("res://sim/skirmish/formation/scrum_engage.gd")
const ScrumStance = preload("res://sim/skirmish/formation/scrum_stance.gd")
const ScrumSeek = preload("res://sim/skirmish/formation/scrum_seek.gd")
const ScrumRegroup = preload("res://sim/skirmish/formation/scrum_regroup.gd")
const FormationManoeuvre = preload("res://sim/skirmish/formation/formation_manoeuvre.gd")
const FormationWithdraw = preload("res://sim/skirmish/formation/formation_withdraw.gd")
const FormationPursuit = preload("res://sim/skirmish/formation/formation_pursuit.gd")
const BattleTuning = preload("res://content/definitions/battle_tuning.gd")
const FormationShuffle = preload("res://sim/skirmish/formation/formation_shuffle.gd")
const ColumnCopies = preload("res://tools/column_copies.gd")

var _spent := {}
var _last := 0
var _copies: ColumnCopies = null  # copies=true: the timed battle's columns, counted


func _init() -> void:
	var args := {}
	for arg in OS.get_cmdline_user_args():
		var parts: PackedStringArray = arg.split("=", true, 1)
		args[parts[0]] = parts[1] if parts.size() > 1 else "true"
	var side := int(args.get("side", 1024))
	var width := int(args.get("width", 32))
	var fight := int(args.get("fight", 5))
	var timed := _settled(side, width, args)
	var plain := _settled(side, width, args)
	if args.has("copies") and timed.store.field() != null:
		_copies = ColumnCopies.new(timed.store.field())
		timed.store.columns = _copies
	for _i in range(fight):
		_step(timed)
		plain.step()
	var total: float = _spent.values().reduce(func(a, b): return a + b, 0)
	print(
		(
			"machine: %s, %d threads, %s"
			% [OS.get_processor_name(), OS.get_processor_count(), OS.get_name()]
		)
	)
	var heading := "%d a side, %d wide: %.1f ms a tick"
	print(heading % [side, width, total / fight / 1000.0])
	var names := _spent.keys()
	names.sort_custom(func(a, b): return _spent[a] > _spent[b])
	for phase in names:
		print("  %-32s %8.2f" % [phase, _spent[phase] / float(fight) / 1000.0])
	if _copies != null:
		print("\n".join(_copies.report(fight)))
	if _state(timed) != _state(plain):
		print("WARNING: tick_phases no longer mirrors FormationSimulation.step - fix it")
	quit(0)


func _settled(side: int, width: int, args: Dictionary) -> Sim:
	var lanes := int(args.get("squads", 1))
	var archers := int(args.get("archers", 0))
	var battle_seed := int(args.get("seed", 1))
	var sim := (
		FormationBench.lanes(lanes, side / lanes, width, battle_seed, archers)
		if lanes > 1
		else FormationBench.clash(side, width, battle_seed, archers)
	)
	if args.get("stage", "fight") == "march":
		for _i in range(int(args.get("settle", 10))):
			sim.step()
		return sim
	var contact := false
	while not contact and sim.tick_number() < FormationBench.CONTACT_LIMIT:
		contact = sim.step().any(func(e): return e["type"] == "engaged")
	for _i in range(int(args.get("settle", 10))):
		sim.step()
	if args.has("retreat"):
		sim.order(int(args["retreat"]), Sim.SkirmishUnit.Order.RETREAT)
	return sim


func _state(sim: Sim) -> PackedByteArray:
	var out := []
	var at: PackedVector2Array = sim.store.columns.column(UnitColumns.POSITION)
	var bearings: PackedFloat64Array = sim.store.columns.column(UnitColumns.BEARING)
	var hp: PackedInt32Array = sim.store.columns.column(UnitColumns.HP)
	for squad in sim.squads():
		out.append([squad.id, squad.state, squad.loose.size(), squad.position])
		for unit in squad.units:
			out.append([unit.id, hp[unit.slot], at[unit.slot], bearings[unit.slot]])
	return var_to_bytes(out)


func _lap(phase: String) -> void:
	var now := Time.get_ticks_usec()
	_spent[phase] = _spent.get(phase, 0) + now - _last
	if _copies != null:
		_copies.file(phase)
	_last = now


## FormationSimulation.step, phase by phase.
func _step(sim: Sim) -> void:
	_last = Time.get_ticks_usec()
	sim._tick += 1
	var events := []
	var squads := sim.squads()
	var tick := sim.tick_number()
	sim._apply_orders(events)
	_lap("orders")
	var roster := CrateRoster.of(squads)  # sim._engage, its parts timed
	sim.perception = Sim.FormationPerception.see(squads, sim.terrain, roster)
	_lap("perception")
	Sim.FormationFronts.engage(squads, tick, sim.fight_seed, events, roster)
	_lap("engage: fronts")
	Sim.FormationEdges.engage(squads, tick, events, sim.fight_seed, roster)
	_lap("engage: edges")
	FormationPathing.step(squads, sim.terrain, tick, sim.fight_seed, events)
	_lap("pathing")
	sim._move(events, roster)
	_lap("march (squads move)")
	_scrum(sim, events)
	sim._fight(events)
	_lap("fight (blows)")
	var interval := sim._attack_interval_ticks()
	Sim.FormationDeaths.bury(squads, tick, events)
	_lap("deaths (bury)")
	Sim.FormationTaking.step(squads, tick, sim.tick_seconds, sim.fight_seed, events, sim.perception)
	_lap("taking the downed")
	Sim.FormationWounds.tend(squads, tick, interval, sim.fight_seed, events)
	_lap("wounds (tend)")
	var strays := Sim.FormationRecovery.step(squads, sim.tick_seconds, tick, sim.fight_seed, events)
	sim._next_squad_id = Sim.FormationStrays.adopt(
		strays, squads, sim._next_squad_id, sim.fight_seed
	)
	_lap("recovery, strays")
	Sim.FormationMorale.step(squads, tick, interval, events)
	_lap("morale")
	_after(sim, events)


func _after(sim: Sim, events: Array) -> void:
	var squads := sim.squads()
	var tick := sim.tick_number()
	var pace := Sim.TRAVEL_SCALE * Sim.MapLayoutDef.CELLS_PER_TILE
	events.append_array(
		Sim.FormationRout.step(squads, tick, pace, sim.tick_seconds, sim.terrain, sim.fight_seed)
	)
	_lap("rout")
	var strays := Sim.FormationCarry.step(squads, tick, sim.fight_seed, events)
	sim._next_squad_id = Sim.FormationStrays.adopt(
		strays, squads, sim._next_squad_id, sim.fight_seed
	)
	_lap("carry")
	sim._next_squad_id = Sim.FormationGroups.step(
		squads, tick, sim.fight_seed, events, sim._next_squad_id
	)
	_lap("groups (form up, split)")
	Sim.FormationStamina.step(squads, sim.tick_seconds)
	_lap("stamina")
	Sim.UnitBodies.step(squads, sim.fight_seed, sim._field)
	_lap("bodies (UnitBodies)")
	events.append_array(FormationShuffle.step(squads, sim.tick_seconds, Sim.TRAVEL_SCALE, tick))
	_lap("shuffle (re-form)")
	Sim.FormationMarch.sync_units(squads, [sim.tick_seconds, pace], sim.terrain, sim.fight_seed)
	_lap("units walk to places")
	sim.store.settle(squads)
	_lap("store settles")


## FormationScrum.step, phase by phase.
func _scrum(sim: Sim, events: Array) -> void:
	var squads := sim.squads()
	var tick := sim.tick_number()
	var roster := CrateRoster.of(squads, sim.terrain)  # who is living, through the scrum
	_lap("scrum: roster")
	events.append_array(_scrum_events(sim, roster))
	var pace := Sim.TRAVEL_SCALE * Sim.MapLayoutDef.CELLS_PER_TILE
	var ctx := {
		"tick": tick,
		"seconds": sim.tick_seconds,
		"pace": pace * sim.tick_seconds,
		"cells_per_second": pace,
		"seed": sim.fight_seed,
		"terrain": sim.terrain,
		"active": {},
	}
	ScrumSeek.step(sim._field, roster, ctx)
	_lap("scrum: seek, walk and facing")
	_after_seek(sim, [ctx, roster], events)


## FormationScrum.step after the seek: regroup, the chasers, withdrawal and pursuit, and
## the stall. `scrum`: the scrum's ctx and its roster.
func _after_seek(sim: Sim, scrum: Array, events: Array) -> void:
	var ctx: Dictionary = scrum[0]
	var roster: Object = scrum[1]
	var squads := sim.squads()
	var tick := sim.tick_number()
	var pace := Sim.TRAVEL_SCALE * Sim.MapLayoutDef.CELLS_PER_TILE
	var scrum_events := []
	ScrumRegroup.step(sim._field, roster, ctx)
	_lap("scrum: regroup and chasers")
	FormationWithdraw.step(sim._field, roster, ctx, scrum_events)
	_lap("scrum: withdraw")
	FormationPursuit.step(squads, tick, pace, sim.tick_seconds, scrum_events, roster)
	Scrum._stall(sim._field, roster, ctx["active"], [tick, sim.tick_seconds], scrum_events)
	events.append_array(scrum_events)
	_lap("scrum: pursuit, stall")


func _scrum_events(sim: Sim, roster: Object) -> Array:
	var events := ScrumEngage.step(sim._field, roster, sim.tick_number(), sim.fight_seed)
	Scrum._prepare(sim._field, roster, sim.tick_number())
	var when := [sim.tick_number(), sim.fight_seed]
	ScrumStance.anticipate_all(sim._field, roster, when, sim.terrain, events)
	FormationManoeuvre.step(sim._field, roster)
	_lap("scrum: engage, prepare, stance")
	return events

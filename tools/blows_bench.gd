extends SceneTree
## The fight's melee blows on their own (Decision 129: what porting them is worth): a
## FormationBench clash stepped `settle` ticks into the fight, then only the melee part of
## FormationSimulation._fight - who strikes whom (FormationMelee.blows: ScrumBlows, its
## per-unit target pick, and RoutBlows) and how each lands (BlowLanding.land) - timed
## under each engine on `ticks` ticks in a row (one attack interval: units strike in
## waves), `reps` times on each. Every repetition starts from the tick's state: the pass
## changes only the units' targets and cooldowns and, landing, a target's regeneration
## halt and last wounding, so those are saved before and put back after each one;
## UnitMotion's memo of bearing vectors is emptied before each, as a tick meets new
## bearings. Both engines must give the same blows, landed the same, and leave
## the same targets and cooldowns, or it says so. rolls=true rolls the blows (the bench's
## clash doesn't, as tick_phases), and the GDScript stage split is printed both ways, with
## the rest of _fight (the shots, hit points and events, surrender, stamina) for scale.
##
##   godot --headless --path . --script res://tools/blows_bench.gd -- \
##       [side=1024] [width=32] [settle=10] [reps=10] [ticks=10] [seed=1] [rolls=false]
##       [engines=gdscript,rust]

const FormationBench = preload("res://sim/skirmish/formation/formation_bench.gd")
const NativeKernels = preload("res://sim/skirmish/formation/native_kernels.gd")
const Sim = preload("res://sim/skirmish/formation/formation_simulation.gd")
const FormationMelee = preload("res://sim/skirmish/formation/formation_melee.gd")
const FormationCombat = preload("res://sim/skirmish/formation/formation_combat.gd")
const FormationEvents = preload("res://sim/skirmish/formation/formation_events.gd")
const FormationWounds = preload("res://sim/skirmish/formation/formation_wounds.gd")
const FormationStamina = preload("res://sim/skirmish/formation/formation_stamina.gd")
const FormationMorale = preload("res://sim/skirmish/formation/formation_morale.gd")
const BlowLanding = preload("res://sim/skirmish/formation/blow_landing.gd")
const RoutBlows = preload("res://sim/skirmish/formation/rout_blows.gd")
const ScrumBlows = preload("res://sim/skirmish/formation/scrum_blows.gd")
const ScrumBlowsField = preload("res://sim/skirmish/formation/scrum_blows_field.gd")
const ScrumNear = preload("res://sim/skirmish/formation/scrum_near.gd")
const ScrumReach = preload("res://sim/skirmish/formation/scrum_reach.gd")
const SkirmishUnit = preload("res://sim/skirmish/skirmish_unit.gd")
const UnitMotion = preload("res://sim/skirmish/formation/unit_motion.gd")
const BattleTuning = preload("res://content/definitions/battle_tuning.gd")

var _args := {}
var _times := {}  # engine -> each tick's median ms
var _blows := {}  # engine -> blows struck
var _split := [0, 0, 0, 0]  # the Rust pass's ScrumBlowsField.spent, summed


func _init() -> void:
	for arg in OS.get_cmdline_user_args():
		var parts: PackedStringArray = arg.split("=", true, 1)
		_args[parts[0]] = parts[1] if parts.size() > 1 else "true"
	var rust := NativeKernels.available(NativeKernels.RUST)
	NativeKernels.use(NativeKernels.RUST if rust else NativeKernels.GDSCRIPT)
	var sim := _settled(int(_args.get("side", 1024)), int(_args.get("width", 32)))
	var rolls: bool = _args.get("rolls", "false") == "true"
	var ticks := int(_args.get("ticks", 10))
	var units := _header(sim, ticks, rolls)
	var engines := Array(str(_args.get("engines", "gdscript,rust")).split(",")).filter(
		func(e): return e == NativeKernels.GDSCRIPT or (rust and e == NativeKernels.RUST)
	)
	var spent := {false: {}, true: {}}
	for _tick in range(ticks):
		sim.blow_rolls = rolls
		var saved := _saved(sim.squads())
		var after := engines.map(func(engine): return _engine(engine, sim, saved))
		if after.size() == 2 and after[0] != after[1]:
			print("WARNING: the engines struck differently on tick %d" % sim.tick_number())
		for rolled in [false, true]:
			sim.blow_rolls = rolled
			_stages(sim, saved, spent[rolled])
		sim.blow_rolls = rolls
		sim.step()
	_report(engines, units, spent, ticks)
	quit(0)


## Prints the machine and the battle; returns its living units.
func _header(sim: Sim, ticks: int, rolls: bool) -> int:
	print("machine: %s, %d threads" % [OS.get_processor_name(), OS.get_processor_count()])
	var units := _units(sim.squads()).size()
	var first := sim.tick_number() + 1
	var line := "%s a side, %s wide, ticks %d-%d: %d units, blows rolled: %s"
	var shape := [_args.get("side", 1024), _args.get("width", 32)]
	print(line % (shape + [first, first + ticks - 1, units, rolls]))
	return units


## ms a tick under each engine (each tick's median, averaged; the worst tick), then the
## GDScript stages.
func _report(engines: Array, units: int, spent: Dictionary, ticks: int) -> void:
	for engine in engines:
		var times: Array = _times[engine]
		var mean: float = times.reduce(func(a, t): return a + t, 0.0) / times.size()
		var line := "  %-8s pass %7.2f ms a tick (worst %6.2f), %5.2f us a unit, %d blows"
		print(line % [engine, mean, times.max(), mean * 1000.0 / units, _blows[engine]])
	var split := _split
	if engines.size() == 2:
		var reps := ticks * int(_args.get("reps", 10))
		var text := "           gather+sync %.2f, call %.2f (core %.2f), write back %.2f"
		print(text % Array(split).map(func(t): return t / 1000.0 / reps))
	for rolled in [false, true]:
		print("GDScript _fight by stage, blows rolled: %s (ms a tick)" % rolled)
		for stage in spent[rolled]:
			print("  %-34s %7.2f" % [stage, spent[rolled][stage] / 1000.0 / ticks])


func _settled(side: int, width: int) -> Sim:
	var sim := FormationBench.clash(side, width, int(_args.get("seed", 1)))
	var contact := false
	while not contact and sim.tick_number() < FormationBench.CONTACT_LIMIT:
		contact = sim.step().any(func(e): return e["type"] == "engaged")
	for _i in range(int(_args.get("settle", 10))):
		sim.step()
	return sim


## Times the pass under one engine; returns what it struck and left.
func _engine(engine: String, sim: Sim, saved: Array) -> Array:
	var field: Object = sim._field if engine == NativeKernels.RUST else null
	var times := []
	var struck := []
	var before := ScrumBlowsField.spent.duplicate()
	for _rep in range(int(_args.get("reps", 10))):
		_put(sim.squads(), saved)
		UnitMotion._vectors.clear()
		var began := Time.get_ticks_usec()
		struck = _pass(sim, field)
		times.append((Time.get_ticks_usec() - began) / 1000.0)
	var after := [_outcome(struck), _saved(sim.squads())]
	for stage in range(4):
		_split[stage] += ScrumBlowsField.spent[stage] - before[stage]
	_put(sim.squads(), saved)
	times.sort()
	_times[engine] = _times.get(engine, []) + [times[times.size() / 2]]
	_blows[engine] = _blows.get(engine, 0) + struck.size()
	return after


## The melee part of FormationSimulation._fight: the blows, then how they land.
func _pass(sim: Sim, field: Object) -> Array:
	var interval := sim._attack_interval_ticks()
	var blows: Array = _melee(sim.squads(), interval, sim.fight_seed, field)
	var rolls := [sim.fight_seed, sim.tick_number()] if sim.blow_rolls else []
	BlowLanding.land(blows, [], sim.squads(), sim.terrain, rolls, field)
	return blows


## FormationMelee.blows (on the field under Rust).
func _melee(squads: Array, interval: int, fight_seed: int, field: Object) -> Array:
	return FormationMelee.blows(squads, interval, fight_seed, field)


func _outcome(blows: Array) -> Array:
	return blows.map(func(b): return [b[0].id, b[1].id, b[2], b[3], b[4]])


func _units(squads: Array) -> Array:
	var out := []
	for squad in squads:
		out.append_array(squad.living())
	return out


## What the fight's melee changes, per unit (the dead too: a pass never revives one).
func _saved(squads: Array) -> Array:
	var out := []
	for squad in squads:
		for unit in squad.units:
			out.append(
				[
					squad.store.columns.value(unit.slot, UnitColumns.TARGET_ID),
					squad.store.columns.value(unit.slot, UnitColumns.ATTACK_COOLDOWN),
					squad.store.columns.value(unit.slot, UnitColumns.REGENERATION_HALT),
					squad.store.columns.value(unit.slot, UnitColumns.LAST_WOUNDING),
					UnitHealth.of_one(squad, unit),
					squad.store.columns.value(unit.slot, UnitColumns.STAMINA),
					unit.state
				]
			)
	return out


func _put(squads: Array, saved: Array) -> void:
	var index := 0
	for squad in squads:
		for unit in squad.units:
			var was: Array = saved[index]
			squad.store.columns.put(unit.slot, UnitColumns.TARGET_ID, was[0])
			squad.store.columns.put(unit.slot, UnitColumns.ATTACK_COOLDOWN, was[1])
			squad.store.columns.put(unit.slot, UnitColumns.REGENERATION_HALT, was[2])
			squad.store.columns.put(unit.slot, UnitColumns.LAST_WOUNDING, was[3])
			UnitHealth.put_one(squad, unit, was[4])
			squad.store.columns.put(unit.slot, UnitColumns.STAMINA, was[5])
			unit.state = was[6]
			index += 1


## The GDScript fight stage by stage, added into `spent` (usec), from the saved state.
func _stages(sim: Sim, saved: Array, spent: Dictionary) -> void:
	_put(sim.squads(), saved)
	UnitMotion._vectors.clear()
	_staged(sim, spent)
	_put(sim.squads(), saved)


## FormationSimulation._fight, with ScrumBlows.blows and BlowLanding.land opened up.
func _staged(sim: Sim, spent: Dictionary) -> void:
	var squads := sim.squads()
	var lap := [Time.get_ticks_usec()]
	ScrumBlows.clear_targets(squads)
	_lap(spent, lap, "zero targets")
	var interval := sim._attack_interval_ticks()
	var blows := _scrum_blows(squads, interval, sim.fight_seed, spent, lap)
	blows.append_array(RoutBlows.blows(squads, interval, sim.fight_seed))
	_lap(spent, lap, "rout blows (RoutBlows)")
	var shots := FormationCombat.ranged_blows(squads, interval, sim.fight_seed)
	_lap(spent, lap, "shots (ranged_blows)")
	_landing(sim, blows, shots, spent, lap)
	var events := []
	for blow in blows:
		UnitHealth.hurt_one(sim, blow[1], blow[2])
		events.append(FormationEvents.hit(sim.tick_number(), blow, UnitHealth.of_one(sim, blow[1])))
	_lap(spent, lap, "hit points, events")
	FormationWounds.surrender(blows, squads, sim.fight_seed, sim.tick_number(), events)
	_lap(spent, lap, "surrender")
	FormationStamina.strike(sim, blows + shots)
	_lap(spent, lap, "stamina (strike)")


func _landing(sim: Sim, blows: Array, shots: Array, spent: Dictionary, lap: Array) -> void:
	var rolls := [sim.fight_seed, sim.tick_number()] if sim.blow_rolls else []
	if rolls.is_empty():
		BlowLanding.land(blows, shots, sim.squads(), sim.terrain, rolls)
		_lap(spent, lap, "land (unrolled)")
		return
	var squad_of := {}
	for squad in sim.squads():
		for unit in squad.living():
			squad_of[unit.id] = [unit, squad]
	_lap(spent, lap, "land: who's whose")
	var pressed := BlowLanding._pressed(squad_of, sim.store.columns.column(UnitColumns.TARGET_ID))
	_lap(spent, lap, "land: pressed")
	for blow in blows:
		var known := [squad_of, pressed, sim.store.columns.column(UnitColumns.POSITION)]
		BlowLanding._land(blow, false, blow[3], known, sim.terrain, rolls)
	_lap(spent, lap, "land: each blow rolled")


## ScrumBlows.blows, stage by stage.
func _scrum_blows(squads: Array, interval: int, seed: int, spent: Dictionary, lap: Array) -> Array:
	var out := []
	var reach := BattleTuning.current().reach_contact
	var index := FoeIndex.of(squads)
	_lap(spent, lap, "scrum: foe index (FoeIndex.of)")
	for squad in squads:
		var near := FoeIndex.struck(index, squad)  # builds the faction's grid on first use
		_lap(spent, lap, "scrum: foes struck (FoeIndex.struck)")
		if near.is_empty():
			continue
		for unit in squad.living():
			var where := ScrumReach.at(squad, unit)
			var close := ScrumNear.around(near, where, ScrumReach.radius(unit) + reach)
			_lap(spent, lap, "scrum: near (ScrumNear.around)")
			var facing: float = squad.store.columns.value(unit.slot, UnitColumns.BEARING)
			var pick := ScrumBlows._pick(squad, unit, facing, close, seed)
			_lap(spent, lap, "scrum: pick (_pick)")
			if not pick.is_empty():
				_strike(squad, unit, pick, interval, out)
			_lap(spent, lap, "scrum: cooldown, flank, blow")
	return out


## ScrumBlows.blows' work once a unit has its pick.
func _strike(squad, unit: SkirmishUnit, pick: Array, interval: int, out: Array) -> void:
	var where := ScrumReach.at(squad, unit)
	var columns: UnitColumns = squad.store.columns
	columns.put(unit.slot, UnitColumns.TARGET_ID, pick[0].id)
	var cooldown: int = columns.value(unit.slot, UnitColumns.ATTACK_COOLDOWN) - 1
	columns.put(unit.slot, UnitColumns.ATTACK_COOLDOWN, cooldown)
	if cooldown > 0:
		return
	var due := FormationMorale.interval(squad, ScrumBlows.ticks(interval, unit))
	columns.put(unit.slot, UnitColumns.ATTACK_COOLDOWN, due)
	if (
		squad.order == SkirmishUnit.Order.RETREAT
		and not ScrumReach.in_front(columns.value(unit.slot, UnitColumns.BEARING), where, pick[2])
	):
		return
	var facing: float = columns.value(pick[0].slot, UnitColumns.BEARING)
	var flank := not ScrumReach.in_front(facing, pick[2], where)
	out.append([unit, pick[0], FormationCombat.damage(unit), flank])


func _lap(spent: Dictionary, lap: Array, stage: String) -> void:
	var now := Time.get_ticks_usec()
	spent[stage] = spent.get(stage, 0) + now - lap[0]
	lap[0] = now

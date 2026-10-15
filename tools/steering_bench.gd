extends SceneTree
## The scrum's walk on its own (Decision 129: what porting it is worth): a FormationBench
## clash stepped `settle` ticks into the fight, the tick's slot search run once, then only
## the walk - FormationScrum's walk, every fighting unit stepping to its slot or its place
## round the bodies in its way (UnitSteer), over the bodies on the ground (GroundBodies),
## looking where UnitShuffle says - timed `reps` times on that one snapshot under each
## engine. The walk changes nothing but the units' loose entries (where each stands, its
## next point and where it looks), so those are saved before and put back after each
## repetition, and UnitMotion's memo of bearing vectors is emptied, as a tick meets new
## bearings. Both engines must leave the same entries, or it says so. The GDScript walk is
## also split into stages.
##
##   godot --headless --path . --script res://tools/steering_bench.gd -- \
##       [side=1024] [width=32] [settle=10] [reps=20] [seed=1] [engines=gdscript,rust]

const FormationBench = preload("res://sim/skirmish/formation/formation_bench.gd")
const NativeKernels = preload("res://sim/skirmish/formation/native_kernels.gd")
const Sim = preload("res://sim/skirmish/formation/formation_simulation.gd")
const Scrum = preload("res://sim/skirmish/formation/formation_scrum.gd")
const ScrumSeek = preload("res://sim/skirmish/formation/scrum_seek.gd")
const ScrumStance = preload("res://sim/skirmish/formation/scrum_stance.gd")
const GroundBodies = preload("res://sim/skirmish/formation/ground_bodies.gd")
const BodyGrid = preload("res://sim/skirmish/formation/body_grid.gd")
const SkirmishSquad = preload("res://sim/skirmish/formation/skirmish_squad.gd")
const UnitMotion = preload("res://sim/skirmish/formation/unit_motion.gd")
const UnitShuffle = preload("res://sim/skirmish/formation/unit_shuffle.gd")
const UnitSteer = preload("res://sim/skirmish/formation/unit_steer.gd")
const BattleTuning = preload("res://content/definitions/battle_tuning.gd")
const SquadEntries = preload("res://sim/skirmish/formation/squad_entries.gd")

const FIELDS := ["next", "toward"]  # with where each stands loose (the store's LOOSE_AT)

var _args := {}


func _init() -> void:
	for arg in OS.get_cmdline_user_args():
		var parts: PackedStringArray = arg.split("=", true, 1)
		_args[parts[0]] = parts[1] if parts.size() > 1 else "true"
	var rust := NativeKernels.available(NativeKernels.RUST)
	NativeKernels.use(NativeKernels.RUST if rust else NativeKernels.GDSCRIPT)
	var side := int(_args.get("side", 1024))
	var width := int(_args.get("width", 32))
	var sim := _settled(side, width)
	var ctx := _ctx(sim)
	ScrumSeek.plan(_engine_ctx(ctx, sim, NativeKernels.GDSCRIPT))  # the entries a walk starts from
	var fighting: Array = ctx["squads"].filter(
		func(s): return s.state == SkirmishSquad.State.FIGHTING
	)
	var saved := _entries(fighting)
	print("machine: %s, %d threads" % [OS.get_processor_name(), OS.get_processor_count()])
	var walkers := _walkers(fighting)
	var line := "%d a side, %d wide, tick %d: %d units walk (%d to slots), %d bodies lie"
	print(
		(
			line
			% [side, width, sim.tick_number(), walkers[0], walkers[1], ctx["lying"]["lying"].size()]
		)
	)
	var after := {}
	for engine in str(_args.get("engines", "gdscript,rust")).split(","):
		if engine == NativeKernels.GDSCRIPT or rust:
			after[engine] = _engine(engine, sim, fighting, ctx, saved)
	if after.size() == 2 and after.values()[0] != after.values()[1]:
		print("WARNING: the engines walked the units differently")
	_stages(fighting, ctx, saved)
	quit(0)


## Times the walk under one engine and prints it; returns the entries it leaves.
func _engine(engine: String, sim: Sim, fighting: Array, ctx: Dictionary, saved: Array) -> Array:
	var engine_ctx := _engine_ctx(ctx, sim, engine)
	_zero_split()
	var times := []
	for _rep in range(int(_args.get("reps", 20))):
		_put(fighting, saved)
		UnitMotion._vectors.clear()
		var began := Time.get_ticks_usec()
		_pass(fighting, engine_ctx)
		times.append((Time.get_ticks_usec() - began) / 1000.0)
	var after := _entries(fighting)
	times.sort()
	var median: float = times[times.size() / 2]
	var line := "  %-8s walk %7.2f ms (min %6.2f), %5.2f us a unit"
	print(line % [engine, median, times[0], median * 1000.0 / _walkers(fighting)[0]])
	if engine == NativeKernels.RUST:
		_split(times.size())
	_put(fighting, saved)
	return after


## The walk, as FormationScrum._seek runs it (every fighting squad's units; the engine
## switch where the build has one).
func _pass(fighting: Array, ctx: Dictionary) -> void:
	var scrum: Script = Scrum
	if scrum.get_script_method_list().any(func(m): return m["name"] == "walk"):
		scrum.call("walk", fighting, ctx)
		return
	for squad in fighting:
		var crowding := BattleTuning.current().scrum_crowding if squad.pursuit.is_empty() else 1.0
		Scrum._walk(squad, ctx["pace"] * crowding, ctx)


func _zero_split() -> void:
	var walk_field = _walk_field()
	if walk_field != null:
		walk_field.spent = PackedInt64Array([0, 0, 0, 0])


## Where the Rust walk's time goes, ms a pass (ScrumWalkField.spent; the core inside the
## call).
func _split(reps: int) -> void:
	var walk_field = _walk_field()
	if walk_field == null:
		return
	var spent: PackedInt64Array = walk_field.spent
	var line := "           gather %.2f, call %.2f (core %.2f), write back %.2f"
	print(line % [spent[0], spent[1], spent[2], spent[3]].map(func(t): return t / 1000.0 / reps))


func _walk_field():
	var path := "res://sim/skirmish/formation/scrum_walk_field.gd"
	return load(path) if ResourceLoader.exists(path) else null


func _settled(side: int, width: int) -> Sim:
	var sim := FormationBench.clash(side, width, int(_args.get("seed", 1)))
	var contact := false
	while not contact and sim.tick_number() < FormationBench.CONTACT_LIMIT:
		contact = sim.step().any(func(e): return e["type"] == "engaged")
	for _i in range(int(_args.get("settle", 10))):
		sim.step()
	return sim


## The scrum's context as FormationScrum.step makes it (what the walk reads).
func _ctx(sim: Sim) -> Dictionary:
	var pace := Sim.TRAVEL_SCALE * Sim.MapLayoutDef.CELLS_PER_TILE
	var squads := sim.squads()
	for squad in squads:
		Scrum._prepare(squad, sim.tick_number())
	var ctx := {
		"squads": squads,
		"tick": sim.tick_number(),
		"seconds": sim.tick_seconds,
		"pace": pace * sim.tick_seconds,
		"seed": sim.fight_seed,
		"terrain": sim.terrain,
		"bodies": ScrumSeek.bodies(squads),
		"lying": GroundBodies.ground(squads),
		"active": {},
		"field": null,
	}
	ctx["crowd"] = BodyGrid.of_bodies(ctx["bodies"])
	return ctx


## The context for one engine: under Rust, the tick's slot search run on the battle's
## field, which syncs it to the snapshot and keeps its batch and answer for the walk
## (ScrumSeekField).
func _engine_ctx(ctx: Dictionary, sim: Sim, engine: String) -> Dictionary:
	var out := ctx.duplicate()
	if engine == NativeKernels.RUST:
		out["field"] = sim._field
		out["active"] = {}
		ScrumSeek.plan(out)  # the same entries, and the field synced and its batch kept
	return out


## [walkers, of them making for a slot].
func _walkers(fighting: Array) -> Array:
	var counts := [0, 0]
	for squad in fighting:
		for unit in squad.living():
			if squad.chasers.has(unit.id):
				continue
			var entry: Dictionary = squad.loose[unit.id]
			if entry["goal"] != null or not entry.get("touch", false):
				counts[0] += 1
				counts[1] += 0 if entry["goal"] == null else 1
	return counts


func _entries(fighting: Array) -> Array:
	var out := []
	for squad in fighting:
		for unit in squad.living():
			var entry: Dictionary = squad.loose[unit.id]
			out.append(
				FIELDS.map(func(key): return entry.get(key)) + [SquadEntries.loose_at(squad, unit)]
			)
	return out


func _put(fighting: Array, saved: Array) -> void:
	var index := 0
	for squad in fighting:
		for unit in squad.living():
			var entry: Dictionary = squad.loose[unit.id]
			for k in range(FIELDS.size()):
				entry[FIELDS[k]] = saved[index][k]
			squad.store.columns.put(unit.slot, UnitColumns.LOOSE_AT, saved[index][FIELDS.size()])
			index += 1


## The GDScript walk stage by stage (ms): the places (ScrumStance.anchor) and looks
## (UnitShuffle.look) of those keeping to their places, the steering (UnitSteer.toward),
## the footing (GroundBodies.underfoot), the steps (UnitMotion.move); each from the saved
## entries.
func _stages(fighting: Array, ctx: Dictionary, saved: Array) -> void:
	_put(fighting, saved)
	UnitMotion._vectors.clear()
	var walks := _walks(fighting, ctx)
	var spent := []
	var lap := Time.get_ticks_usec()
	for walk in walks:
		if walk[1]["goal"] == null:
			walk[2] = ScrumStance.anchor(walk[4], walk[0])
	spent.append(Time.get_ticks_usec() - lap)
	lap = Time.get_ticks_usec()
	for walk in walks:
		if walk[1]["goal"] == null:
			var squad: SkirmishSquad = walk[4]
			var heading: float = squad.stance.get("heading", squad.heading)
			UnitShuffle.look(walk[0], walk[5], walk[2], heading, walk[3] / ctx["seconds"])
	spent.append(Time.get_ticks_usec() - lap)
	lap = Time.get_ticks_usec()
	var tos := []
	for walk in walks:
		tos.append(
			UnitSteer.toward(walk[0], walk[5], walk[2], ctx["bodies"], ctx["seed"], ctx["crowd"])
		)
	spent.append(Time.get_ticks_usec() - lap)
	lap = Time.get_ticks_usec()
	var footings := []
	for index in range(walks.size()):
		footings.append(Scrum._footing(walks[index][0], walks[index][5], tos[index], ctx))
	spent.append(Time.get_ticks_usec() - lap)
	lap = Time.get_ticks_usec()
	for index in range(walks.size()):
		var walk: Array = walks[index]
		UnitMotion.move(walk[0], walk[5], tos[index], walk[3] * footings[index])
	spent.append(Time.get_ticks_usec() - lap)
	_print_stages(spent)


func _print_stages(spent: Array) -> void:
	print("GDScript stages (one walk, ms):")
	var names := ["places", "looks", "steering", "footing", "steps"]
	for index in range(names.size()):
		print("  %-16s %7.2f" % [names[index], spent[index] / 1000.0])


## [[unit, entry, goal (its place, once found), full step, squad, where it stands], ...] for
## the walkers.
func _walks(fighting: Array, ctx: Dictionary) -> Array:
	var walks := []
	for squad in fighting:
		var crowding := BattleTuning.current().scrum_crowding if squad.pursuit.is_empty() else 1.0
		for unit in squad.living():
			var entry: Dictionary = squad.loose[unit.id]
			if squad.chasers.has(unit.id) or (entry["goal"] == null and entry.get("touch", false)):
				continue
			var pace: float = unit.speed * ctx["pace"] * crowding
			var at := SquadEntries.loose_at(squad, unit)
			walks.append([unit, entry, entry["next"], pace, squad, at])
	return walks

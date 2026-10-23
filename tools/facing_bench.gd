extends SceneTree
## The scrum's facing pass on its own (Decision 129: what porting it is worth): a FormationBench
## clash stepped `settle` ticks into the fight, then only the facing pass - FormationScrum's _faces
## and the turns - timed `reps` times on that one snapshot under each engine. Every repetition
## starts from the same state: the pass changes nothing but the units' bearings (and, in Rust, the
## field's copy of where the facers stand, set to the same points each time), so the bearings are
## saved before and put back after each one. Saving and restoring times the whole pass as a tick
## runs it, turns and write-back included, which working out the turns without applying them would
## not. UnitMotion's memo of bearing vectors is emptied before each repetition, as a tick meets new
## bearings. Both engines must leave the same bearings, or it says so.
## The GDScript pass is also split into stages, with how many DetMath calls it makes and what they
## cost (DetMath timed on its own, in GDScript and in Rust where built).
##   godot --headless --path . --script res://tools/facing_bench.gd -- \
##       [side=1024] [width=32] [settle=10] [reps=20] [seed=1] [engines=gdscript,rust]

const FormationBench = preload("res://sim/skirmish/formation/formation_bench.gd")
const NativeKernels = preload("res://sim/skirmish/formation/native_kernels.gd")
const Sim = preload("res://sim/skirmish/formation/formation_simulation.gd")
const Scrum = preload("res://sim/skirmish/formation/formation_scrum.gd")
const ScrumSeek = preload("res://sim/skirmish/formation/scrum_seek.gd")
const ScrumNear = preload("res://sim/skirmish/formation/scrum_near.gd")
const ScrumReach = preload("res://sim/skirmish/formation/scrum_reach.gd")
const ScrumBlows = preload("res://sim/skirmish/formation/scrum_blows.gd")
const BodyFieldSync = preload("res://sim/skirmish/formation/body_field_sync.gd")
const ScrumSeekField = preload("res://sim/skirmish/formation/scrum_seek_field.gd")
const ScrumFaceField = preload("res://sim/skirmish/formation/scrum_face_field.gd")
const SkirmishSquad = preload("res://sim/skirmish/formation/skirmish_squad.gd")
const UnitMotion = preload("res://sim/skirmish/formation/unit_motion.gd")
const UnitShuffle = preload("res://sim/skirmish/formation/unit_shuffle.gd")
const DetMath = preload("res://sim/skirmish/formation/det_math.gd")
const BattleTuning = preload("res://content/definitions/battle_tuning.gd")
const SquadEntries = preload("res://sim/skirmish/formation/squad_entries.gd")

const EPSILON := 0.000001
const CALLS := 100000

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
	var fighting: Array = ctx["squads"].filter(
		func(s): return s.state == SkirmishSquad.State.FIGHTING
	)
	var saved := _bearings(ctx["squads"])
	print("machine: %s, %d threads" % [OS.get_processor_name(), OS.get_processor_count()])
	var facers := _facers(fighting)
	print("%d a side, %d wide, tick %d: %d units face" % [side, width, sim.tick_number(), facers])
	var after := {}
	for engine in str(_args.get("engines", "gdscript,rust")).split(","):
		if engine == NativeKernels.GDSCRIPT or rust:
			after[engine] = _engine(engine, sim, fighting, ctx, saved)
	if after.size() == 2 and after.values()[0] != after.values()[1]:
		print("WARNING: the engines turned the units differently")
	_stages(fighting, ctx, saved)
	_det_math(rust)
	quit(0)


## Times the pass under one engine and prints it; returns the bearings it leaves.
func _engine(
	engine: String, sim: Sim, fighting: Array, ctx: Dictionary, saved: PackedFloat64Array
) -> PackedFloat64Array:
	ScrumFaceField.spent = PackedInt64Array([0, 0, 0, 0])
	var times := _timed(fighting, _engine_ctx(ctx, sim, engine), saved)
	var after := _bearings(ctx["squads"])
	times.sort()
	var median: float = times[times.size() / 2]
	var line := "  %-8s pass %7.2f ms (min %6.2f), %5.2f us a unit"
	print(line % [engine, median, times[0], median * 1000.0 / _facers(fighting)])
	if engine == NativeKernels.RUST:
		_split(times.size())
	_put(ctx["squads"], saved)
	return after


func _settled(side: int, width: int) -> Sim:
	var sim := FormationBench.clash(side, width, int(_args.get("seed", 1)))
	var contact := false
	while not contact and sim.tick_number() < FormationBench.CONTACT_LIMIT:
		contact = sim.step().any(func(e): return e["type"] == "engaged")
	for _i in range(int(_args.get("settle", 10))):
		sim.step()
	return sim


## The scrum's context as FormationScrum.step makes it (what the facing pass reads).
func _ctx(sim: Sim) -> Dictionary:
	var pace := Sim.TRAVEL_SCALE * Sim.MapLayoutDef.CELLS_PER_TILE
	return {
		"squads": sim.squads(),
		"tick": sim.tick_number(),
		"seconds": sim.tick_seconds,
		"pace": pace * sim.tick_seconds,
		"seed": sim.fight_seed,
		"field": null,
	}


## The context for one engine: under Rust, the battle's field brought to the snapshot and
## the units sent, as the tick's slot search leaves them (BodyFieldSync.scrum,
## ScrumSeekField's batch).
func _engine_ctx(ctx: Dictionary, sim: Sim, engine: String) -> Dictionary:
	var out := ctx.duplicate()
	if engine == NativeKernels.RUST:
		out["field"] = sim._field
		var synced := BodyFieldSync.scrum(sim._field, ctx["squads"], ctx["seed"])
		out["seek_batch"] = ScrumSeekField.gathered(synced[0], synced[1], ctx["squads"])
	return out


## ms for each repetition of the pass, each from the saved bearings.
func _timed(fighting: Array, ctx: Dictionary, saved: PackedFloat64Array) -> Array:
	var out := []
	for _rep in range(int(_args.get("reps", 20))):
		_put(ctx["squads"], saved)
		var began := Time.get_ticks_usec()
		_pass(fighting, ctx)
		out.append((Time.get_ticks_usec() - began) / 1000.0)
	return out


## The facing pass, as FormationScrum._seek runs it after the walk.
func _pass(fighting: Array, ctx: Dictionary) -> void:
	Scrum.face(fighting, ctx)


## Where the Rust pass's time goes, ms a pass (ScrumFaceField.spent; the core inside the
## call).
func _split(reps: int) -> void:
	var spent := ScrumFaceField.spent
	var line := "           gather %.2f, call %.2f (core %.2f), write back %.2f"
	print(line % [spent[0], spent[1], spent[2], spent[3]].map(func(t): return t / 1000.0 / reps))


func _facers(fighting: Array) -> int:
	var count := 0
	for squad in fighting:
		for unit in squad.living():
			count += 0 if squad.chasers.has(unit.id) else 1
	return count


## The battle's bearings, by slot (the store's column).
func _bearings(squads: Array) -> PackedFloat64Array:
	return squads[0].store.columns.column(UnitColumns.BEARING)


func _put(squads: Array, saved: PackedFloat64Array) -> void:
	squads[0].store.columns.commit(UnitColumns.BEARING, saved)


## The GDScript pass stage by stage (ms): the foes' grids (ScrumNear.index), the touching
## foe (ScrumNear.around, ScrumBlows.nearest_touching), the looks and wanted bearings
## (UnitShuffle.look, UnitMotion.bearing_to), the turns; and DetMath's calls in it.
func _stages(fighting: Array, ctx: Dictionary, saved: PackedFloat64Array) -> void:
	_put(ctx["squads"], saved)
	var lap := Time.get_ticks_usec()
	var nears := fighting.map(
		func(s): return ScrumNear.index(ScrumSeek.foe_units(s, ctx["squads"]))
	)
	var spent := [Time.get_ticks_usec() - lap]
	lap = Time.get_ticks_usec()
	var touched := _touched(fighting, nears, ctx)
	spent.append(Time.get_ticks_usec() - lap)
	lap = Time.get_ticks_usec()
	var turns := _wanted(fighting, touched, ctx)
	spent.append(Time.get_ticks_usec() - lap)
	lap = Time.get_ticks_usec()
	var columns: UnitColumns = ctx["squads"][0].store.columns
	for turn in turns:
		UnitMotion.turn(turn[0], columns, turn[1], ctx["seconds"])
	spent.append(Time.get_ticks_usec() - lap)
	var counts := _counted(fighting, touched, ctx)
	_put(ctx["squads"], saved)
	print("GDScript stages (one pass, ms):")
	var names := ["foe grids", "touching foe", "looks, bearings", "turns"]
	for index in range(names.size()):
		print("  %-16s %7.2f" % [names[index], spent[index] / 1000.0])
	print("DetMath calls a pass: atan2 %d, acos %d" % [counts[0], counts[1]])
	_args["counts"] = [counts[0], counts[1]]


func _touched(fighting: Array, nears: Array, ctx: Dictionary) -> Array:
	var reach := BattleTuning.current().reach_contact
	var out := []
	for index in range(fighting.size()):
		var squad: SkirmishSquad = fighting[index]
		for unit in squad.living():
			if squad.chasers.has(unit.id):
				continue
			var at: Vector2 = SquadEntries.loose_at(squad, unit)
			var foes := ScrumNear.around(nears[index], at, ScrumReach.radius(unit) + reach)
			out.append(ScrumBlows.nearest_touching(squad, unit, foes, ctx["seed"]))
	return out


## [[unit, wanted bearing], ...] given each unit's touching foe (FormationScrum._faces).
func _wanted(fighting: Array, touched: Array, ctx: Dictionary) -> Array:
	var out := []
	var index := 0
	for squad in fighting:
		for unit in squad.living():
			if squad.chasers.has(unit.id):
				continue
			var columns: UnitColumns = squad.store.columns
			var at: Vector2 = SquadEntries.loose_at(squad, unit)
			var look = touched[index]
			index += 1
			if look == null and EntryKeys.goal(columns, unit.slot) != null:
				look = Scrum._seeking_look(unit, columns, at, ctx)
			look = look if look != null else EntryKeys.held(columns, unit.slot, UnitColumns.TOWARD)
			if look != null:
				var facing: float = columns.value(unit.slot, UnitColumns.BEARING)
				out.append([unit, UnitMotion.bearing_to(at, look, facing)])
	return out


## [atan2 calls, acos calls] the pass makes (UnitMotion.bearing_to, pace, UnitShuffle).
func _counted(fighting: Array, touched: Array, ctx: Dictionary) -> Array:
	var counts := [0, 0]
	var index := 0
	for squad in fighting:
		for unit in squad.living():
			if squad.chasers.has(unit.id):
				continue
			var columns: UnitColumns = squad.store.columns
			var at: Vector2 = SquadEntries.loose_at(squad, unit)
			var look = touched[index]
			index += 1
			if look == null and EntryKeys.goal(columns, unit.slot) != null:
				var next: Vector2 = columns.value(unit.slot, UnitColumns.NEXT)
				var foe_at: Vector2 = columns.value(unit.slot, UnitColumns.FOE_AT)
				counts[0] += 1 if (foe_at - next).length() >= EPSILON else 0
				var shuffle: bool = (next - at).length() >= EPSILON
				counts[0] += 1 if shuffle else 0
				counts[1] += 4 if shuffle else 0
				look = Scrum._seeking_look(unit, columns, at, ctx)
			look = look if look != null else EntryKeys.held(columns, unit.slot, UnitColumns.TOWARD)
			if look != null:
				counts[0] += 1 if (look - at).length() >= EPSILON else 0
	return counts


## DetMath's calls timed on their own: ns a call in GDScript (less the loop's own cost),
## and in Rust (BodyField.det_math, one call over all the inputs) where built.
func _det_math(rust: bool) -> void:
	var xs := PackedFloat64Array()
	var ys := PackedFloat64Array()  # in [-1, 1]
	for i in range(CALLS):
		xs.append(sin(i * 0.37) * 3.0)
		ys.append(cos(i * 0.91))
	var ops := {
		"atan2": func(y, x): return DetMath.atan2(y, x),
		"acos": func(y, _x): return DetMath.acos(y),
		"asin": func(y, _x): return DetMath.asin(y),
		"sin": func(_y, x): return DetMath.sin(x),
	}
	var empty := _loop(xs, ys, func(y, x): return y + x)
	var ns := {}
	for op in ops:
		ns[op] = [maxf(_loop(xs, ys, ops[op]) - empty, 0.0)]
		if rust and ClassDB.class_has_method(NativeKernels.FIELD_CLASS, "det_math"):
			var began := Time.get_ticks_usec()
			var first := xs if op == "sin" else ys
			ClassDB.class_call_static(NativeKernels.FIELD_CLASS, "det_math", op, first, xs)
			ns[op].append((Time.get_ticks_usec() - began) * 1000.0 / CALLS)
	print("DetMath alone (ns a call): GDScript / Rust")
	for op in ns:
		var rust_ns: String = "%.1f" % ns[op][1] if ns[op].size() > 1 else "-"
		print("  %-6s %7.1f  %7s" % [op, ns[op][0], rust_ns])
	var counts: Array = _args.get("counts", [0, 0])
	var gd: float = counts[0] * ns["atan2"][0] + counts[1] * ns["acos"][0]
	print("  so DetMath is about %.2f ms of the GDScript pass" % [gd / 1e6])


## ns a call of `op` (y, x) over the inputs.
func _loop(xs: PackedFloat64Array, ys: PackedFloat64Array, op: Callable) -> float:
	var sink := 0.0
	var began := Time.get_ticks_usec()
	for i in range(CALLS):
		sink += op.call(ys[i], xs[i])
	return (Time.get_ticks_usec() - began) * 1000.0 / CALLS + sink * 0.0

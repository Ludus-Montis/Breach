extends SceneTree
## The native core's benchmark (Decision 129, native/README.md): a mid-fight tick of
## FormationBench's clash on one thread and on the core's pool ("rust_threads",
## NativeKernels.threaded), and two passes the core runs - body parting (UnitBodies) and the
## scrum's seek, walk and facing (ScrumSeek.step, one call into the crate). Body parting is
## split into syncing the field (BodyFieldSync), the call (the boundary both ways plus the
## core), the core's own work and writing back. Each run times `fight` ticks after
## `settle` ticks of contact; a line per run, then min / median over the runs. Nothing
## runs without the core. ms a tick throughout. For where the rest of the tick goes,
## tools/tick_phases.gd.
##
##   godot --headless --path . --script res://tools/native_bench.gd -- \
##       [engines=rust,rust_threads] [sizes=160:10,512:32,1024:32] [runs=3]
##       [settle=10] [fight=10] [seed=1]

const FormationBench = preload("res://sim/skirmish/formation/formation_bench.gd")
const NativeKernels = preload("res://sim/skirmish/formation/native_kernels.gd")
const UnitBodies = preload("res://sim/skirmish/formation/unit_bodies.gd")
const BodyParting = preload("res://sim/skirmish/formation/body_parting.gd")
const ScrumSeek = preload("res://sim/skirmish/formation/scrum_seek.gd")

## "rest" is the tick less the two passes. The b.* columns are parts of "bodies" and the
## "seek" the scrum's whole seek, walk and facing, with b.core inside b.call: don't add
## them up.
const COLUMNS := [
	"tick",
	"rest",
	"bodies",
	"b.sync",
	"b.call",
	"b.core",
	"b.write",
	"seek",
]


func _init() -> void:
	var args := {}
	for arg in OS.get_cmdline_user_args():
		var parts: PackedStringArray = arg.split("=", true, 1)
		args[parts[0]] = parts[1] if parts.size() > 1 else "true"
	var engines := str(args.get("engines", "rust,rust_threads")).split(",")
	var sizes := str(args.get("sizes", "160:10,512:32,1024:32")).split(",")
	var runs := int(args.get("runs", 3))
	var options := {
		"settle": int(args.get("settle", 10)),
		"fight": int(args.get("fight", 10)),
		"seed": int(args.get("seed", 1))
	}
	print("%-17s %5s %3s  %s" % ["engine", "side", "w", " ".join(COLUMNS.map(_cell))])
	for size in sizes:
		var parts := size.split(":")
		for engine in engines:
			if not NativeKernels.available():
				continue
			NativeKernels.threaded = engine.ends_with("_threads")
			var results := []
			for run in range(runs):
				results.append(_measure(int(parts[0]), int(parts[1]), options))
				print(_line("%s #%d" % [engine, run + 1], parts, results.back()))
			print(_line(engine + " min", parts, _pick(results, 0.0)))
			print(_line(engine + " med", parts, _pick(results, 0.5)))
	quit(0)


## Average ms a tick of each column over `fight` ticks mid-fight.
func _measure(per_side: int, width: int, options: Dictionary) -> Array:
	var sim := FormationBench.clash(per_side, width, options["seed"])
	var contact := false
	while not contact and sim.tick_number() < FormationBench.CONTACT_LIMIT:
		contact = sim.step().any(func(e): return e["type"] == "engaged")
	for _i in range(options["settle"]):
		sim.step()
	UnitBodies.clock_usec = 0
	ScrumSeek.clock_usec = 0
	BodyParting.spent = PackedInt64Array([0, 0, 0, 0])
	var began := Time.get_ticks_usec()
	for _i in range(options["fight"]):
		sim.step()
	var ticks := float(options["fight"]) * 1000.0
	var tick := (Time.get_ticks_usec() - began) / ticks
	var rest := tick - (UnitBodies.clock_usec + ScrumSeek.clock_usec) / ticks
	var out := [tick, rest, UnitBodies.clock_usec / ticks]
	for spent in BodyParting.spent:
		out.append(spent / ticks)
	out.append(ScrumSeek.clock_usec / ticks)
	return out


## Each column's value at quantile `at` (0: min, 0.5: median) across the runs.
func _pick(results: Array, at: float) -> Array:
	var out := []
	for column in range(COLUMNS.size()):
		var values := results.map(func(r): return r[column])
		values.sort()
		out.append(values[int(at * (values.size() - 1) + 0.5)])
	return out


func _cell(text: String) -> String:
	return "%7s" % text


func _line(label: String, size: PackedStringArray, values: Array) -> String:
	var cells := values.map(func(v): return "%7.2f" % v)
	return "%-17s %5s %3s  %s" % [label, size[0], size[1], " ".join(cells)]

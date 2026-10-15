extends SceneTree
## Bounded work (AI_First_Development_Kit/principles/bounded-work.md): a tick's cost must
## grow no faster than its units, n log n at worst. The FormationBench clash at each size,
## doubling, `ranks` deep (so each size is the last one's battle twice over), marching and
## mid-fight; each size's tick is the least of `runs` timed spans
## of `ticks` ticks (the least is the least disturbed by a busy machine). Fails when a
## doubling costs more than `limit` times as much - a pass that has started comparing all
## pairs, or scanning everything for each thing. tools/tick_phases.gd then shows which.
## Then the same by stacks: `stacks` squads of `stack` a side on neighbouring lanes
## (FormationBench.lanes), doubling, `settle` ticks into their fights - a pass that grows
## with the squads on the field, not just their units, shows here. `stages` picks some of
## the three: each compares only its own sizes, all timed in one process, so CI runs each
## stage as its own job (the timings within a stage must come from one machine).
##
##   godot --headless --path . --script res://tools/scaling_check.gd -- \
##       [sizes=128,256,512] [ranks=8] [ticks=6] [runs=3] [limit=2.5] [engine=gdscript]
##       [stacks=1,2,4] [stack=128] [settle=20] [stages=march,fight,stacks]

const FormationBench = preload("res://sim/skirmish/formation/formation_bench.gd")
const NativeKernels = preload("res://sim/skirmish/formation/native_kernels.gd")
const Sim = preload("res://sim/skirmish/formation/formation_simulation.gd")

const STAGES := ["march", "fight", "stacks"]


func _init() -> void:
	var args := {}
	for arg in OS.get_cmdline_user_args():
		var parts: PackedStringArray = arg.split("=", true, 1)
		args[parts[0]] = parts[1] if parts.size() > 1 else "true"
	NativeKernels.use(str(args.get("engine", "gdscript")))
	var sizes: Array = Array(str(args.get("sizes", "128,256,512")).split(",")).map(
		func(s): return int(s)
	)
	var limit := float(args.get("limit", 2.5))
	var failures := 0
	print("scaling_check (%s): ms a tick, and its growth per doubling" % NativeKernels.engine())
	var stacks: Array = Array(str(args.get("stacks", "1,2,4")).split(",")).map(
		func(s): return int(s)
	)
	var stages: Array = Array(str(args.get("stages", ",".join(STAGES))).split(","))
	for stage in STAGES.filter(func(s): return s in stages):
		var last := 0.0
		for size in stacks if stage == "stacks" else sizes:
			var spent := _least(stage, size, args)
			var growth := spent / last if last > 0.0 else 0.0
			var verdict := "" if growth <= limit else "  TOO STEEP (limit %.1f)" % limit
			if verdict != "":
				failures += 1
			var shown := "" if last == 0.0 else "x%.2f" % growth
			var what := "stacks" if stage == "stacks" else "a side"
			print("  %-6s %5d %s %9.2f ms  %s%s" % [stage, size, what, spent, shown, verdict])
			last = spent
	if failures > 0:
		print("FAILED: run tools/tick_phases.gd at the sizes above to find the pass")
	quit(1 if failures > 0 else 0)


## The least ms a tick over `runs` spans of `ticks` ticks of `stage` at `size` a side.
func _least(stage: String, size: int, args: Dictionary) -> float:
	var ticks := int(args.get("ticks", 6))
	var least := INF
	for run in range(int(args.get("runs", 3))):
		var width := maxi(1, size / int(args.get("ranks", 8)))
		var sim := _ready_for(stage, size, width, run + 1, args)
		var began := Time.get_ticks_usec()
		for _i in range(ticks):
			sim.step()
		least = minf(least, (Time.get_ticks_usec() - began) / 1000.0 / ticks)
	return least


## A clash at `size` a side, a few ticks into its march or its fight; for "stacks", `size`
## lanes of `stack` a side each, `settle` ticks into their fights.
func _ready_for(stage: String, size: int, width: int, battle_seed: int, args := {}) -> Sim:
	var sim := FormationBench.clash(size, width, battle_seed)
	var settle := 5
	if stage == "stacks":
		var stack := int(args.get("stack", 128))
		var deep := maxi(1, stack / int(args.get("ranks", 8)))
		sim = FormationBench.lanes(size, stack, deep, battle_seed)
		settle = int(args.get("settle", 20))
	if stage == "march":
		for _i in range(3):
			sim.step()
		return sim
	var contact := false
	while not contact and sim.tick_number() < FormationBench.CONTACT_LIMIT:
		contact = sim.step().any(func(e): return e["type"] == "engaged")
	for _i in range(settle):
		sim.step()
	return sim

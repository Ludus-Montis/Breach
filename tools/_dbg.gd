extends SceneTree
const FormationBench = preload("res://sim/skirmish/formation/formation_bench.gd")
func _init():
	NativeKernels.use("rust")
	var sim := FormationBench.clash(40, 10, 1)
	print(sim.store.get_class(), " ", sim.store.count(), " ", sim.store.columns.column(UnitColumns.POSITION).size())
	var u = sim.squads()[0].units[0]
	print(u.slot, " ", sim.squads()[0].store == sim.store)
	for t in 100: sim.step()
	print("ok")
	quit(0)

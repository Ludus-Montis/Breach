extends SceneTree
## A fingerprint of formation battles, tick by tick (Decision 93: the same seed and orders
## replay a battle): each run's events and the state of every squad and unit after every
## tick, hashed bit for bit. A change meant to keep every outcome (a speed-up) must leave
## every line the same - diff the output before and after.
##
##   godot --headless --path . --script res://tools/formation_digest.gd -- \
##       [set=standard|clash160|wide] [out=<file of every tick's hash>] \
##       [only=<run label>] [trace=<file>] [from=<tick>] [to=<tick>]
##
## only: play that one run (its label as printed, e.g. "mirror_headon s1 rtrue"). trace:
## write each tick from `from` to `to` (default every tick) as a line of JSON naming every
## field (spec 37, DigestTrace); the hashes are the same with it or without.
##
## standard: the mirrors head-on and on a flank (BattleTrials), seeds 1-3; five feel-test
## field logs (FormationFieldActions), two tending their downed; a 40-a-side clash of grems
## and militia, seeds 1-2; each with blows rolled and not. clash160: 160 a side, 120 ticks
## past first contact. wide: 96 a side, 32 wide. Every run plays on TAIL ticks after a side
## breaks.

const BattleTrials = preload("res://sim/skirmish/formation/battle_trials.gd")
const FormationSimulation = preload("res://sim/skirmish/formation/formation_simulation.gd")
const FormationFieldActions = preload("res://sim/skirmish/formation/formation_field_actions.gd")
const FormationField = preload("res://sim/skirmish/formation/formation_field.gd")
const SkirmishSquad = preload("res://sim/skirmish/formation/skirmish_squad.gd")
const FormationBench = preload("res://sim/skirmish/formation/formation_bench.gd")
const DigestState = preload("res://tools/digest_state.gd")
const DigestTrace = preload("res://tools/digest_trace.gd")

const FIELD_LOGS := [
	"seed 492625 captain off\n96 via_c on\n158 send A+B",
	"seed 515557 captain off\n0 pursues off\n120 send B\n318 retreat B",
	"seed 109563 captain off\n0 via_c on\n87 send A\n229 retreat A",
	(
		"record 1 seed 492625 captain off\n0 player tend A carry\n0 player tend B recover"
		+ "\n96 player route A C\n158 player send A+B"
	),
	(
		"record 1 seed 515557 captain off\n0 kingdom pursue all off\n0 player tend B recover"
		+ "\n120 player send B\n318 player retreat B"
	),
]
const LIMIT := 1500
const FIELD_EXTRA := 700
## Ticks a run plays on once a side has broken: its rout, the pursuit, the downed taken.
const TAIL := 150

var _out: FileAccess = null
var _trace: FileAccess = null
var _only := ""
var _from := 1
var _to := -1


func _init() -> void:
	var args := {}
	for arg in OS.get_cmdline_user_args():
		var parts: PackedStringArray = arg.split("=", true, 1)
		args[parts[0]] = parts[1] if parts.size() > 1 else "true"
	if args.has("out"):
		_out = FileAccess.open(args["out"], FileAccess.WRITE)
	if args.has("trace"):
		_trace = FileAccess.open(args["trace"], FileAccess.WRITE)
	_only = args.get("only", "")
	_from = int(args.get("from", "1"))
	_to = int(args.get("to", "-1"))
	if args.get("set", "standard") == "clash160":
		_clash("clash160", 160, 10, 1, true, 120)
	elif args.get("set") == "wide":
		_clash("wide96", 96, 32, 1, true, LIMIT)
		_clash("wide96", 96, 32, 2, false, LIMIT)
	else:
		_standard()
	for file in [_out, _trace]:
		if file != null:
			file.close()
	quit(0)


func _standard() -> void:
	for rolls in [true, false]:
		for scenario in ["mirror_headon", "mirror_flank"]:
			for battle_seed in [1, 2, 3]:
				_mirror(scenario, battle_seed, rolls)
		for index in range(FIELD_LOGS.size()):
			_field(index, rolls)
		for battle_seed in [1, 2]:
			_clash("clash40", 40, 10, battle_seed, rolls, LIMIT)


func _mirror(scenario: String, battle_seed: int, rolls: bool) -> void:
	var label := "%s s%d r%s" % [scenario, battle_seed, rolls]
	if not _plays(label):
		return
	var sim := FormationSimulation.new(2.0, BattleTrials.TICK)
	sim.fight_seed = battle_seed
	sim.blow_rolls = rolls
	for spawn in BattleTrials._mirror_spawns(scenario, sim):
		spawn.call()
	_run(label, sim, LIMIT, -1)


## Grems against militia, `per_side` each, `width` wide; `after` ticks past first contact.
func _clash(label: String, per_side: int, width: int, battle_seed: int, rolls: bool, after: int):
	var run := "%s s%d r%s" % [label, battle_seed, rolls]
	if not _plays(run):
		return
	var sim := FormationBench.clash(per_side, width, battle_seed)
	sim.blow_rolls = rolls
	_run(run, sim, LIMIT, after)


## Steps the sim until TAIL ticks after one side no longer stands, `limit` ticks, or
## `after` ticks past the first engagement (-1: no such stop); prints the run's hash.
func _run(label: String, sim: FormationSimulation, limit: int, after: int) -> void:
	var chain := HashingContext.new()
	chain.start(HashingContext.HASH_SHA256)
	var contact := -1
	var broken := -1  # the tick a side stopped standing: the rout and pursuit play on
	var ticks := 0
	for tick in range(limit):
		var events := sim.step()
		ticks = tick + 1
		_record(chain, label, ticks, events, sim)
		if contact < 0 and events.any(func(e): return e["type"] == "engaged"):
			contact = ticks
		if after >= 0 and contact >= 0 and ticks - contact >= after:
			break
		if broken < 0 and not _both_stand(sim.squads()):
			broken = ticks
		if broken >= 0 and ticks - broken >= TAIL:
			break
	print("%-32s ticks %4d  %s" % [label, ticks, chain.finish().hex_encode()])


func _field(index: int, rolls: bool) -> void:
	var label := "field%d r%s" % [index, rolls]
	if not _plays(label):
		return
	var read := FormationFieldActions.parse(FIELD_LOGS[index])
	var field := FormationFieldActions.field(read["seed"], read["captained"])
	field.sim.blow_rolls = rolls
	var chain := HashingContext.new()
	chain.start(HashingContext.HASH_SHA256)
	var tick := 0
	var commands: Array = read["commands"]
	var last: int = commands.back()["tick"] if not commands.is_empty() else 0
	while tick < last + FIELD_EXTRA:
		for queued in commands:
			if queued["tick"] == tick:
				FormationFieldActions.apply(field, queued)
		var events := field.step()
		tick += 1
		_record(chain, label, tick, events, field.sim)
	print("%-32s ticks %4d  %s" % [label, tick, chain.finish().hex_encode()])


## Whether the run is played: every run, or with `only`, that one.
func _plays(label: String) -> bool:
	return _only.is_empty() or _only == label


func _both_stand(squads: Array) -> bool:
	for faction in BattleTrials.FACTIONS:
		if not squads.any(
			func(s):
				return (
					s.faction_id == faction
					and not s.is_destroyed()
					and s.state != SkirmishSquad.State.ROUTING
				)
		):
			return false
	return true


func _record(
	chain: HashingContext, label: String, tick: int, events: Array, sim: FormationSimulation
):
	var plain_events: Array = DigestState.plain(events)
	var state := DigestState.state(sim.squads(), sim.store)
	var bytes := var_to_bytes([plain_events, state])
	chain.update(bytes)
	if _trace != null and tick >= _from and (_to < 0 or tick <= _to):
		_trace.store_line(DigestTrace.line(label, tick, plain_events, state))
	if _out != null:
		var one := HashingContext.new()
		one.start(HashingContext.HASH_SHA256)
		one.update(bytes)
		_out.store_line("%s %d %s" % [label, tick, one.finish().hex_encode().substr(0, 16)])

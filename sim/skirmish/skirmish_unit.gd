class_name SkirmishUnit
extends RefCounted
## One unit in the real-time skirmish feel test (specs/21-realtime-skirmish-feel-test.md).
## Plain data; SkirmishSimulation owns every rule that changes it.

## The player's standing instruction. Applied by the simulation on the next tick.
enum Order { ADVANCE, HOLD, RETREAT }
## What the unit is doing right now (derived by the simulation each tick). Out of the
## fight (Decision 121): DEAD; DOWNED, on the ground at 0 HP or below it at death's door;
## TAKEN, captured or surrendered; RELEASED, let go to carry word home; CARRIED, downed and
## borne by a friend (Decision 126).
enum State { MOVING, HOLDING, FIGHTING, ARRIVED, DEAD, DOWNED, TAKEN, RELEASED, CARRIED }

var id: int = 0
var faction_id: String = ""
var hp: int = 0
var max_hp: int = 0
var dmg: int = 0
## Cells per second.
var speed: float = 0.0
## Cells along the route, from the player's end (0) to the kingdom's (route length).
var distance: float = 0.0
## Formation sim: its slot in the battle's unit store (spec 35), given when it enrols and
## fixed while it is held. Where it stands and the rest of its per-tick state are there.
var slot := -1
## Formation sim: how far it detects others, in cells (Decision 87).
var detection := 40.0
## Formation sim: its courage and leadership (Decisions 81 and 82).
var courage := 60
var leadership := 0
var tactics: Array[String] = []
var height := 1.0
## Formation sim: its sheet as it stands now (Decision 117) - archetype tags, attributes
## (UnitDef.ATTRIBUTES, name -> value) and rated traits (name -> level) - copied from its
## definition, so tech, items and ranks can change one unit and not its type.
var tags: Array[String] = []
var attributes := {}
var traits := {}
## Formation sim: how quickly it acts, and how drilled it is (Decisions 88 and 92).
var initiative := 10
var discipline := 30
## Formation sim: how it turns and backs away (UnitMotion). The bearing it faces is the unit
## store's (UnitColumns.BEARING).
var turn_rate := 450.0
var backward_pace := 0.4
## Formation sim: the definition it was made from (a router fleeing home rejoins the
## reserve as one of these).
var definition: Resource = null
## Where this unit started: the end it retreats to.
var home_distance: float = 0.0
## +1 advances toward the kingdom's end, -1 toward the player's.
var advance_direction: int = 1
var order: Order = Order.ADVANCE
var state: State = State.MOVING
## The unit being fought; 0 = none.
var target_id: int = 0
var attack_cooldown: int = 0
## Ticks to stand still after spawning, so a wave leaves as a staggered group.
var wait_ticks: int = 0

# Formation fields (specs/22-formation-feel-test.md); unused by the spec 21 SkirmishSimulation.
## The squad (wave) this unit marches in; 0 = none.
var squad_id: int = 0
## The command it follows, and the one it started under (FormationCommand, spec 30 round
## 3); typed loosely to keep the unit free of formation code.
var command: RefCounted = null
var origin: RefCounted = null
## How it crosses terrain (TerrainWalker: swimming, climbing), from its definition; the
## last place it could stand, whether it is on a climb face or in deep water now (no
## breather there), and the cells of a climb still above it (WalkModes).
var walker: RefCounted = null
var foothold := Vector2.ZERO
var on_face := false
var climb_left := 0.0
## Row in its squad's formation, 0 = the front rank.
var rank: int = 0
## Leftmost formation column it covers.
var column: int = 0
## Slots it occupies, depth (ranks) x width (columns) - Decision 40.
var footprint_depth: int = 1
var footprint_width: int = 1
## UnitDef.Position band and claim within it (Decision 46).
var preferred_position: int = 0
var position_priority: int = 0
## From its best ranged weapon (Decision 47): reach in ranks (0 = melee only), damage, type.
## `dmg` is the unit's melee strike - all its melee weapons together.
var attack_range: int = 0
var ranged_dmg: int = 0
var damage_type: String = ""
## Formation sim: its skills, defence and critical multiplier, and whether it can parry
## (Decision 118, UnitDef).
var melee_skill := 40
var ranged_skill := 40
var defence := 10
var critical := 1.5
var parries := false
## Formation sim: what its blows are made of and what stands against them (Decision 119,
## UnitArms): its melee and ranged weapons' parts [[damage, type, magical], ...], their
## floor shares, its weaknesses, resistances and immunities, and its armour and ward.
var melee_parts := []
var ranged_parts := []
var melee_floor := 1.0
var ranged_floor := 1.0
var weaknesses: Array[String] = []
var resistances: Array[String] = []
var immunities: Array[String] = []
var armour := 0
var ward := 0
## Formation sim: its stamina (spec 28 part 7) - now and at most - its pace fresh, how
## fast it tires and how well it dodges under its load (UnitArms.load_stage), and whether
## it exerted itself this tick.
var stamina := 100.0
var max_stamina := 100.0
var fresh_speed := 0.0
var tiring := 1.0
var dodging := 1.0
var exerted := false
## Formation sim: how much faster it runs than it marches, its stage of load, seconds since
## it last spent stamina (Decision 125), and its condition - the one number its statuses,
## traits and surroundings make of its state, 1 as things stand.
var run_pace := 1.5
var load_stage := 0
var breather := 0.0
var condition := 1.0
## Formation sim: seconds into its present run (below 0: still reacting), and whether it
## ran last tick (Decision 126).
var run_build := 0.0
var was_running := false
## Formation sim: its regeneration (Decision 121): HP a second, HP it has left to regain
## before it must rest, the damage types that stop it, seconds it stays stopped, and the
## part of a point regained so far.
var regeneration := 0.0
var regeneration_left := 0.0
var regeneration_stops: Array[String] = []
var regeneration_halt := 0.0
var regeneration_carry := 0.0
## Formation sim: its wounds (Decision 126) - a time downed each, lowering its condition -
## seconds it has yet to lie downed before it comes to (-1: not yet reckoned), and the
## part of a point its poor condition has drained so far.
var wounded := 0
var wake_left := -1.0
var drain_carry := 0.0
## Formation sim: the wounds its weapons add to a foe they down ("wounding N", the most of
## its weapons'), and those the last blow that struck it would add.
var wounding := 0
var last_wounding := 0
## Formation sim: lying still while foes are near, though it could get up (Decision 127).
var playing_dead := false
## Formation sim: the friend it bears, or the friend bearing it (unit ids; 0: none), and the
## weight of its own gear, for its load with a body on it (Decision 126).
var carrying := 0
var carried_by := 0
var gear_weight := 0.0
## Seconds between its melee blows and its ranged shots (Decision 120): its weapons'
## intervals, scaled by its attack speed.
var melee_seconds := 1.0
var ranged_seconds := 1.0


## Still in the fight: standing, not dead, downed, taken or released.
func is_alive() -> bool:
	return state < State.DEAD


func is_hostile_to(other: SkirmishUnit) -> bool:
	return other.faction_id != faction_id

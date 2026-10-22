class_name BattleTuning
extends Resource
## The battle's tuning numbers in one place (Decision 123, spec 28 round 1): every number
## the fights lean on that isn't a unit's or an item's. The values live in
## content/tuning/battle_tuning.json, in the crate's schema (Decision 153), which names every
## one; edit them there. This script only names and explains them, and is filled from what
## the crate loaded (CrateContent). All are placeholders until play settles them.

static var _current: BattleTuning = null

@export_group("Morale (Decision 82)")
## Morale lost when struck on a side or on the rear.
@export var morale_side_impact := 0
@export var morale_rear_impact := 0
## Morale lost for each unit of its own that falls, and per point of a fallen leader's
## leadership.
@export var morale_loss := 0
@export var morale_leader_loss := 0
## Morale regained each second out of contact, before its leadership is added.
@export var morale_recovery := 0
## Morale lost each second by how many sides it fights on (0, 1, 2, 3, 4 or more).
@export var morale_pressure: Array[int] = []
## Cells within which a friendly squad supports a side.
@export var morale_support_cells := 0.0
## The ceiling's rise per point of its best leader's leadership (Decision 81).
@export var morale_per_leadership := 0
## The bands' floors: steady at or above the first, shaken at or above the second.
@export var morale_steady_at := 0
@export var morale_shaken_at := 0

@export_group("Discipline (Decisions 92, 107 and 109)")
## What each point of a formation's best leader's leadership adds to its discipline.
@export var discipline_per_leadership := 0
## At or above this a formation re-forms as a whole to meet a flank; below it, unit by
## unit. A unit under it may break ranks to chase, the more likely the further under.
@export var discipline_meets_threats := 0
## The discipline at which it re-forms at the march pace; the pace scales with it, within
## the slowest and fastest shares of the march pace.
@export var discipline_march_pace_at := 0.0
@export var discipline_slowest := 0.0
@export var discipline_fastest := 0.0
## The discipline a formation is expected to have at most; more buys no more.
@export var discipline_expected_max := 0.0
## How far (cells) a pursuer may go from where it set out, step by step: the steadier, the
## shorter its leash; the last is no leash (inf), while it can see its enemy.
@export var discipline_leash_steps: Array[float] = []
## The share of the expected most at or above which a pursuer takes the second, third and
## fourth leash steps; under the last, the fifth. A leader's "pursues" tactic steps it one
## out, "cautious" one in.
@export var discipline_leash_shares: Array[float] = []

@export_group("Combat (Decisions 88 and 93)")
## The contest die: a roll from 0 to this, added to initiative, decides who takes a
## contested slot (0: stats alone).
@export var combat_contest_die := 0

@export_group("Blows (Decision 118)")
## A blow's roll spans this much, centred on the striker's skill: its margin is the skill
## plus a roll from minus half this to plus half.
@export var blow_die := 0.0
## The parry a unit holding a weapon or shield puts up: this share of its melee skill.
@export var blow_parry_share := 0.0
## The dodge a unit puts up per point of agility.
@export var blow_dodge_per_agility := 0.0
## How wide the hit band is above the target's defence; a margin past it is a critical.
@export var blow_hit_band := 0.0
## A graze's share of the blow's damage.
@export var blow_graze_share := 0.0
## Added to the margin of a striker on higher ground than its target.
@export var blow_high_ground := 0.0
## Added to the margin for each foe beyond the first striking the target this tick.
@export var blow_surrounded := 0.0
## A blow from within this many degrees of straight behind the target is from its rear,
## denying its dodge as well as its parry.
@export var blow_rear_arc_degrees := 0.0
## Added to a formation's units' margins by its morale band: steady, shaken, wavering,
## routing.
@export var blow_morale_shift: Array[float] = []

@export_group("Damage (Decision 119)")
## A blow's worth against a target weak to, resistant to or immune to its damage type.
@export var damage_weak := 0.0
@export var damage_resist := 0.0
@export var damage_immune := 0.0
## The skill at which a blow's damage floor has risen to its full; each point of skill
## past it adds damage_spill to the blow's margin (crits) instead.
@export var damage_floor_skill := 0.0
@export var damage_spill := 0.0

@export_group("Items (Decision 120)")
## Skill shifted wielding a weapon untrained or mastered in its tags.
@export var item_untrained_skill := 0.0
@export var item_mastered_skill := 0.0
## Skill lost per point of strength short of a weapon's requirement.
@export var item_weak_skill := 0.0

@export_group("Load and stamina (Decision 120, spec 28 part 7)")
## Load a unit carries without trouble, per point of strength: its carry limit.
@export var load_per_strength := 0.0
## Encumbrance by stage - none (to load_easy of its limit), mild (to its limit), steep (to
## load_most times its limit, a hauler's level adding load_hauler each) and immobile
## beyond: its pace, how fast it tires and its dodge, times these.
@export var load_easy := 0.0
@export var load_most := 0.0
@export var load_hauler := 0.0
@export var load_pace: Array[float] = []
@export var load_tiring: Array[float] = []
@export var load_dodge: Array[float] = []
## Stamina a second marching under each stage of load (marching light costs nothing).
@export var load_march_tiring: Array[float] = []
## Stamina per point of constitution; spent a second running and per blow struck (Decision
## 125). Each cost restarts a breather: once stamina_delay seconds have passed with none,
## it is regained at stamina_recovery a second - both for a unit of average constitution,
## the delay shorter and the rate faster the hardier it is, the rate times its condition.
@export var stamina_per_constitution := 0.0
@export var stamina_run := 0.0
## Stamina a second spent climbing and swimming (spec 30): at half the mode's pace a step
## takes twice as long, so costs twice as much (ModeStamina).
@export var stamina_climb := 0.0
@export var stamina_swim := 0.0
@export var stamina_blow := 0.0
@export var stamina_delay := 0.0
@export var stamina_recovery := 0.0
## Seconds a unit takes to build up from a march to its full run, and the seconds a
## pursuer of average wits takes to react before it starts to (Decision 126): a retreat's
## head start.
@export var run_build_seconds := 0.0
@export var run_reaction := 0.0
## The most a unit's condition (Decision 125) may lift what it multiplies.
@export var condition_cap := 0.0
## Below these shares of its stamina a unit is tired, then spent: its pace times
## stamina_pace and its skill shifted by stamina_skill, by stage (fresh, tired, spent). A
## pursuer (or chaser) whose stamina falls below stamina_give_up gives up the chase.
@export var stamina_tired := 0.0
@export var stamina_spent := 0.0
@export var stamina_pace: Array[float] = []
@export var stamina_skill: Array[float] = []
@export var stamina_give_up := 0.0

@export_group("Wounds (Decision 121)")
## Cells within which a standing unit finishes or takes a downed foe; a friend standing
## within wounds_guard_reach of the downed guards it, and a unit with a standing foe that
## near is still fighting, not finishing.
@export var wounds_reach := 0.0
@export var wounds_guard_reach := 0.0
## Morale lost by each standing formation of a side one of its own is sent home to.
@export var wounds_messenger_shock := 0
## Seconds a blow of a type that stops a unit's regeneration stops it for.
@export var wounds_regeneration_halt := 0.0
## Each wound - a time downed (Decision 126) - takes this off a unit's condition; a unit
## "hardened N" shrugs off its first N.
@export var wounds_condition := 0.0
## Playing dead (Decision 127): a downed unit due to come to with a standing foe within
## wounds_danger_reach cells lies still instead by a seeded chance - wounds_play_dead at
## average wits, more with wits, plus wounds_cunning a level of "cunning" (its own or its
## leader's) - and looks again every wounds_play_dead_check seconds. A foe about to take
## it is fooled when the body's wits beat its own, each with wounds_trait_wits a level of
## "cunning" or "thorough", in a roll of wounds_believe_die.
@export var wounds_danger_reach := 0.0
@export var wounds_play_dead := 0.0
@export var wounds_cunning := 0.0
@export var wounds_play_dead_check := 0.0
@export var wounds_trait_wits := 0.0
@export var wounds_believe_die := 0.0
## The weight of a body borne by a friend, per cell of its footprint (Decision 126).
@export var wounds_body_weight := 0.0
## Seconds a downed unit of average constitution lies before it comes to at 1 HP, times a
## seeded share between wounds_wake_spread's two ends; the hardier, the sooner.
@export var wounds_wake_seconds := 0.0
@export var wounds_wake_spread: Array[float] = []
## Below this condition a unit loses HP, standing or downed, the faster the further below:
## condition_drain a second at condition 0, scaling with the distance below the threshold -
## so one left in a bad enough state dies of it rather than lie there, the faster the worse.
@export var condition_drain_below := 0.0
@export var condition_drain := 0.0
## The chance a routing unit struck surrenders, times its want of courage (1 - courage
## / 100).
@export var wounds_surrender := 0.0

@export_group("Routs and flight (Decisions 82, 89, 98 and 99)")
## Crush damage per cell of a router's footprint, to friends it shoves past.
@export var rout_crush := 0
## Morale lost by a friend a router crushes, and by a friend that sees a formation rout.
@export var rout_panic := 0
@export var rout_seen := 0
## How near (cells, a router's centre to a friend's body) a router must come to a led
## formation to rally to it.
@export var rout_rally_reach := 0.0
## Seconds a caught router waits with its friend steady before it joins it.
@export var rout_steady_rally_seconds := 0.0
## Seconds with no enemy within rout_enemy_near cells before a led rout re-forms (and a
## withdrawal is safe).
@export var rout_rally_seconds := 0.0
@export var rout_enemy_near := 0.0
## The morale a re-formed rout starts at.
@export var rout_reformed_morale := 0
## How near (cells, centre to centre) a pursuer must be to strike a router.
@export var rout_strike_reach := 0.0
## How far a wholly disorderly flight fans out: degrees off its route's line, and cells out.
@export var rout_fan_degrees := 0.0
@export var rout_fan_cells := 0.0

@export_group("Pursuit (Decisions 103 and 109)")
## The rout a ragged retreat costs: up to this much shock, for a formation with no
## discipline at all.
@export var pursuit_ragged_shock := 0
## How near (cells) a unit must stand to a retreating enemy to be tempted to chase.
@export var pursuit_tempted_within := 0.0

@export_group("Ground (Decision 85)")
## A rise of more than this many quarter-cells is a cliff: climbed, a movement mode.
@export var ground_cliff_quarters := 0
## The pace each quarter-cell risen costs; downhill is no faster.
@export var ground_slope_cost := 0.0
## The pace in water a quarter to a half of a unit's height deep (wading), and from a half
## to its height (slow wading); deeper, it swims.
@export var ground_wading := 0.0
@export var ground_slow_wading := 0.0
## Movement modes (spec 30): deep water is swum and a cliff climbed at a base pace times
## the pair rule (Decision 64: swimmer/climber against flows/climb difficulty, default
## ground_climb_demand); past ground_mode_load_share of its first load band, neither.
@export var ground_swim_pace := 0.0
@export var ground_climb_pace := 0.0
@export var ground_climb_demand := 0
@export var ground_mode_load_share := 0.0
## Sight by obscurance (spec 30): a sight line sums each cell's obscurance by the distance
## run through it, seeing no further past sight_budget; woods and fog paint theirs.
@export var sight_budget := 0.0
@export var sight_wood_obscurance := 0.0
@export var sight_fog_obscurance := 0.0
## A unit spent on a climb face falls: this hurt a cell dropped, × its area^fall_mass_power.
@export var fall_damage_per_cell := 0.0
@export var fall_mass_power := 0.0

@export_group("Bodies (Decisions 106 and 114)")
## How many times over a unit in its formation's frame weighs, against being pushed.
@export var bodies_resist := 0.0
## How many times a tick overlapping bodies are pushed apart.
@export var bodies_passes := 0
## A loose friend of its own squad overlapping a unit in its frame by less than this
## (cells) only brushes it: it keeps its place.
@export var bodies_brush := 0.0
## How far ahead (cells) a unit looks for a body in its way, and how far clear (cells) of
## it it aims to pass.
@export var bodies_steer_look := 0.0
@export var bodies_steer_clear := 0.0
## Bodies on the ground (Decision 121): a marching front stepping over one slows to 1 / (1
## + its mass over the walker's times bodies_ground_drag); one at least bodies_ground_block
## times the walker's mass blocks it.
@export var bodies_ground_drag := 0.0
@export var bodies_ground_block := 0.0

@export_group("Scrum (Decisions 48, 75, 88, 92 and 110)")
## Cells a unit in a fight may go from its place to reach a foe.
@export var scrum_leash := 0.0
## How far off (cells) a disciplined squad re-forms to meet a threat closing in.
@export var scrum_anticipate := 0.0
## Seconds a fight may stand with nobody able to strike before it is released.
@export var scrum_stall_seconds := 0.0
## The share of its pace a unit makes walking within a fight: the crush.
@export var scrum_crowding := 0.0
## Seconds a unit walking back to its place may come no nearer before it trades places.
@export var scrum_trade_seconds := 0.0

@export_group("Reach and movement (Decisions 85, 88, 103 and 106)")
## How far apart (cells) two units' bodies may be and still touch: a unit stepping back
## still reaches the one it is leaving.
@export var reach_contact := 0.0
## A point lies in a unit's front when its direction is within this many degrees of its
## bearing; a blow from anywhere else is a flank blow.
@export var reach_front_arc_degrees := 0.0
## How far apart (cells, between their cells) two units may be for their squads to engage.
@export var reach_engage := 0.0
## How far (cells) a pursuing formation's foremost unit may lag behind its place before the
## frame waits for it.
@export var reach_pursuit_lag := 0.0

@export_group("Walking (spec 30 round 3)")
## Cells a unit may lag its place before its formation waits (at discipline 100, 0); past
## walk_lost it has fallen behind; past walk_face_travel it faces where it walks.
@export var walk_slack_drilled := 0.0
@export var walk_slack_loose := 0.0
@export var walk_lost := 0.0
@export var walk_face_travel := 0.0
## Cells to form up or split off together; the reach and hold (seconds) of taking the downed.
@export var group_join := 0.0
@export var take_reach := 0.0
@export var take_hold := 0.0
## Groups look path_look cells ahead every path_relook, going round if path_detour_gain quicker.
@export var path_look := 0.0
@export var path_relook := 0.0
@export var path_detour_gain := 0.0


## The tuning in play: content/tuning/battle_tuning.json, loaded once.
static func current() -> BattleTuning:
	if _current == null:
		_current = CrateContent.tuning()
	return _current

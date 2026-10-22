//! The battle's tuning numbers (`BattleTuning`, Decision 123): every number the fights lean
//! on that isn't a unit's or an item's, in `content/tuning/battle_tuning.json`. Every field
//! is required: the file names them all. A list of reals may hold an infinity (`reals`). Names are GDScript's, field for field.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::reals::{self, Reals};

/// The battle's tuning (`BattleTuning`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct BattleTuning {
    /// Morale lost when struck on a side or on the rear.
    pub morale_side_impact: i64,
    pub morale_rear_impact: i64,
    /// Morale lost for each unit of its own that falls, and per point of a fallen leader's
    /// leadership.
    pub morale_loss: i64,
    pub morale_leader_loss: i64,
    /// Morale regained each second out of contact, before its leadership is added.
    pub morale_recovery: i64,
    /// Morale lost each second by how many sides it fights on (0, 1, 2, 3, 4 or more).
    pub morale_pressure: Vec<i64>,
    /// Cells within which a friendly squad supports a side.
    pub morale_support_cells: f64,
    /// The ceiling's rise per point of its best leader's leadership (Decision 81).
    pub morale_per_leadership: i64,
    /// The bands' floors: steady at or above the first, shaken at or above the second.
    pub morale_steady_at: i64,
    pub morale_shaken_at: i64,
    /// What each point of a formation's best leader's leadership adds to its discipline.
    pub discipline_per_leadership: i64,
    /// At or above this a formation re-forms as a whole to meet a flank; below it, unit by
    /// unit. A unit under it may break ranks to chase, the more likely the further under.
    pub discipline_meets_threats: i64,
    /// The discipline at which it re-forms at the march pace; the pace scales with it, within
    /// the slowest and fastest shares of the march pace.
    pub discipline_march_pace_at: f64,
    pub discipline_slowest: f64,
    pub discipline_fastest: f64,
    /// The discipline a formation is expected to have at most; more buys no more.
    pub discipline_expected_max: f64,
    /// How far (cells) a pursuer may go from where it set out, step by step: the steadier, the
    /// shorter its leash; the last is no leash (inf), while it can see its enemy.
    #[serde(with = "reals")]
    #[schemars(with = "Reals")]
    pub discipline_leash_steps: Vec<f64>,
    /// The share of the expected most at or above which a pursuer takes the second, third and
    /// fourth leash steps; under the last, the fifth. A leader's "pursues" tactic steps it one
    /// out, "cautious" one in.
    #[serde(with = "reals")]
    #[schemars(with = "Reals")]
    pub discipline_leash_shares: Vec<f64>,
    /// The contest die: a roll from 0 to this, added to initiative, decides who takes a
    /// contested slot (0: stats alone).
    pub combat_contest_die: i64,
    /// A blow's roll spans this much, centred on the striker's skill: its margin is the skill
    /// plus a roll from minus half this to plus half.
    pub blow_die: f64,
    /// The parry a unit holding a weapon or shield puts up: this share of its melee skill.
    pub blow_parry_share: f64,
    /// The dodge a unit puts up per point of agility.
    pub blow_dodge_per_agility: f64,
    /// How wide the hit band is above the target's defence; a margin past it is a critical.
    pub blow_hit_band: f64,
    /// A graze's share of the blow's damage.
    pub blow_graze_share: f64,
    /// Added to the margin of a striker on higher ground than its target.
    pub blow_high_ground: f64,
    /// Added to the margin for each foe beyond the first striking the target this tick.
    pub blow_surrounded: f64,
    /// A blow from within this many degrees of straight behind the target is from its rear,
    /// denying its dodge as well as its parry.
    pub blow_rear_arc_degrees: f64,
    /// Added to a formation's units' margins by its morale band: steady, shaken, wavering,
    /// routing.
    #[serde(with = "reals")]
    #[schemars(with = "Reals")]
    pub blow_morale_shift: Vec<f64>,
    /// A blow's worth against a target weak to, resistant to or immune to its damage type.
    pub damage_weak: f64,
    pub damage_resist: f64,
    pub damage_immune: f64,
    /// The skill at which a blow's damage floor has risen to its full; each point of skill
    /// past it adds damage_spill to the blow's margin (crits) instead.
    pub damage_floor_skill: f64,
    pub damage_spill: f64,
    /// Skill shifted wielding a weapon untrained or mastered in its tags.
    pub item_untrained_skill: f64,
    pub item_mastered_skill: f64,
    /// Skill lost per point of strength short of a weapon's requirement.
    pub item_weak_skill: f64,
    /// Load a unit carries without trouble, per point of strength: its carry limit.
    pub load_per_strength: f64,
    /// Encumbrance by stage - none (to load_easy of its limit), mild (to its limit), steep (to
    /// load_most times its limit, a hauler's level adding load_hauler each) and immobile
    /// beyond: its pace, how fast it tires and its dodge, times these.
    pub load_easy: f64,
    pub load_most: f64,
    pub load_hauler: f64,
    #[serde(with = "reals")]
    #[schemars(with = "Reals")]
    pub load_pace: Vec<f64>,
    #[serde(with = "reals")]
    #[schemars(with = "Reals")]
    pub load_tiring: Vec<f64>,
    #[serde(with = "reals")]
    #[schemars(with = "Reals")]
    pub load_dodge: Vec<f64>,
    /// Stamina a second marching under each stage of load (marching light costs nothing).
    #[serde(with = "reals")]
    #[schemars(with = "Reals")]
    pub load_march_tiring: Vec<f64>,
    /// Stamina per point of constitution; spent a second running and per blow struck (Decision
    /// 125). Each cost restarts a breather: once stamina_delay seconds have passed with none,
    /// it is regained at stamina_recovery a second - both for a unit of average constitution,
    /// the delay shorter and the rate faster the hardier it is, the rate times its condition.
    pub stamina_per_constitution: f64,
    pub stamina_run: f64,
    /// Stamina a second spent climbing and swimming (spec 30): at half the mode's pace a step
    /// takes twice as long, so costs twice as much (ModeStamina).
    pub stamina_climb: f64,
    pub stamina_swim: f64,
    pub stamina_blow: f64,
    pub stamina_delay: f64,
    pub stamina_recovery: f64,
    /// Seconds a unit takes to build up from a march to its full run, and the seconds a
    /// pursuer of average wits takes to react before it starts to (Decision 126): a retreat's
    /// head start.
    pub run_build_seconds: f64,
    pub run_reaction: f64,
    /// The most a unit's condition (Decision 125) may lift what it multiplies.
    pub condition_cap: f64,
    /// Below these shares of its stamina a unit is tired, then spent: its pace times
    /// stamina_pace and its skill shifted by stamina_skill, by stage (fresh, tired, spent). A
    /// pursuer (or chaser) whose stamina falls below stamina_give_up gives up the chase.
    pub stamina_tired: f64,
    pub stamina_spent: f64,
    #[serde(with = "reals")]
    #[schemars(with = "Reals")]
    pub stamina_pace: Vec<f64>,
    #[serde(with = "reals")]
    #[schemars(with = "Reals")]
    pub stamina_skill: Vec<f64>,
    pub stamina_give_up: f64,
    /// Cells within which a standing unit finishes or takes a downed foe; a friend standing
    /// within wounds_guard_reach of the downed guards it, and a unit with a standing foe that
    /// near is still fighting, not finishing.
    pub wounds_reach: f64,
    pub wounds_guard_reach: f64,
    /// Morale lost by each standing formation of a side one of its own is sent home to.
    pub wounds_messenger_shock: i64,
    /// Seconds a blow of a type that stops a unit's regeneration stops it for.
    pub wounds_regeneration_halt: f64,
    /// Each wound - a time downed (Decision 126) - takes this off a unit's condition; a unit
    /// "hardened N" shrugs off its first N.
    pub wounds_condition: f64,
    /// Playing dead (Decision 127): a downed unit due to come to with a standing foe within
    /// wounds_danger_reach cells lies still instead by a seeded chance - wounds_play_dead at
    /// average wits, more with wits, plus wounds_cunning a level of "cunning" (its own or its
    /// leader's) - and looks again every wounds_play_dead_check seconds. A foe about to take
    /// it is fooled when the body's wits beat its own, each with wounds_trait_wits a level of
    /// "cunning" or "thorough", in a roll of wounds_believe_die.
    pub wounds_danger_reach: f64,
    pub wounds_play_dead: f64,
    pub wounds_cunning: f64,
    pub wounds_play_dead_check: f64,
    pub wounds_trait_wits: f64,
    pub wounds_believe_die: f64,
    /// The weight of a body borne by a friend, per cell of its footprint (Decision 126).
    pub wounds_body_weight: f64,
    /// Seconds a downed unit of average constitution lies before it comes to at 1 HP, times a
    /// seeded share between wounds_wake_spread's two ends; the hardier, the sooner.
    pub wounds_wake_seconds: f64,
    #[serde(with = "reals")]
    #[schemars(with = "Reals")]
    pub wounds_wake_spread: Vec<f64>,
    /// Below this condition a unit loses HP, standing or downed, the faster the further below:
    /// condition_drain a second at condition 0, scaling with the distance below the threshold -
    /// so one left in a bad enough state dies of it rather than lie there, the faster the worse.
    pub condition_drain_below: f64,
    pub condition_drain: f64,
    /// The chance a routing unit struck surrenders, times its want of courage (1 - courage
    /// / 100).
    pub wounds_surrender: f64,
    /// Crush damage per cell of a router's footprint, to friends it shoves past.
    pub rout_crush: i64,
    /// Morale lost by a friend a router crushes, and by a friend that sees a formation rout.
    pub rout_panic: i64,
    pub rout_seen: i64,
    /// How near (cells, a router's centre to a friend's body) a router must come to a led
    /// formation to rally to it.
    pub rout_rally_reach: f64,
    /// Seconds a caught router waits with its friend steady before it joins it.
    pub rout_steady_rally_seconds: f64,
    /// Seconds with no enemy within rout_enemy_near cells before a led rout re-forms (and a
    /// withdrawal is safe).
    pub rout_rally_seconds: f64,
    pub rout_enemy_near: f64,
    /// The morale a re-formed rout starts at.
    pub rout_reformed_morale: i64,
    /// How near (cells, centre to centre) a pursuer must be to strike a router.
    pub rout_strike_reach: f64,
    /// How far a wholly disorderly flight fans out: degrees off its route's line, and cells out.
    pub rout_fan_degrees: f64,
    pub rout_fan_cells: f64,
    /// The rout a ragged retreat costs: up to this much shock, for a formation with no
    /// discipline at all.
    pub pursuit_ragged_shock: i64,
    /// How near (cells) a unit must stand to a retreating enemy to be tempted to chase.
    pub pursuit_tempted_within: f64,
    /// A rise of more than this many quarter-cells is a cliff: climbed, a movement mode.
    pub ground_cliff_quarters: i64,
    /// The pace each quarter-cell risen costs; downhill is no faster.
    pub ground_slope_cost: f64,
    /// The pace in water a quarter to a half of a unit's height deep (wading), and from a half
    /// to its height (slow wading); deeper, it swims.
    pub ground_wading: f64,
    pub ground_slow_wading: f64,
    /// Movement modes (spec 30): deep water is swum and a cliff climbed at a base pace times
    /// the pair rule (Decision 64: swimmer/climber against flows/climb difficulty, default
    /// ground_climb_demand); past ground_mode_load_share of its first load band, neither.
    pub ground_swim_pace: f64,
    pub ground_climb_pace: f64,
    pub ground_climb_demand: i64,
    pub ground_mode_load_share: f64,
    /// Sight by obscurance (spec 30): a sight line sums each cell's obscurance by the distance
    /// run through it, seeing no further past sight_budget; woods and fog paint theirs.
    pub sight_budget: f64,
    pub sight_wood_obscurance: f64,
    pub sight_fog_obscurance: f64,
    /// A unit spent on a climb face falls: this hurt a cell dropped, × its area^fall_mass_power.
    pub fall_damage_per_cell: f64,
    pub fall_mass_power: f64,
    /// How many times over a unit in its formation's frame weighs, against being pushed.
    pub bodies_resist: f64,
    /// How many times a tick overlapping bodies are pushed apart.
    pub bodies_passes: i64,
    /// A loose friend of its own squad overlapping a unit in its frame by less than this
    /// (cells) only brushes it: it keeps its place.
    pub bodies_brush: f64,
    /// How far ahead (cells) a unit looks for a body in its way, and how far clear (cells) of
    /// it it aims to pass.
    pub bodies_steer_look: f64,
    pub bodies_steer_clear: f64,
    /// Bodies on the ground (Decision 121): a marching front stepping over one slows to
    /// 1 / (1 + its mass over the walker's times bodies_ground_drag); one at least
    /// bodies_ground_block times the walker's mass blocks it.
    pub bodies_ground_drag: f64,
    pub bodies_ground_block: f64,
    /// Cells a unit in a fight may go from its place to reach a foe.
    pub scrum_leash: f64,
    /// How far off (cells) a disciplined squad re-forms to meet a threat closing in.
    pub scrum_anticipate: f64,
    /// Seconds a fight may stand with nobody able to strike before it is released.
    pub scrum_stall_seconds: f64,
    /// The share of its pace a unit makes walking within a fight: the crush.
    pub scrum_crowding: f64,
    /// Seconds a unit walking back to its place may come no nearer before it trades places.
    pub scrum_trade_seconds: f64,
    /// How far apart (cells) two units' bodies may be and still touch: a unit stepping back
    /// still reaches the one it is leaving.
    pub reach_contact: f64,
    /// A point lies in a unit's front when its direction is within this many degrees of its
    /// bearing; a blow from anywhere else is a flank blow.
    pub reach_front_arc_degrees: f64,
    /// How far apart (cells, between their cells) two units may be for their squads to engage.
    pub reach_engage: f64,
    /// How far (cells) a pursuing formation's foremost unit may lag behind its place before the
    /// frame waits for it.
    pub reach_pursuit_lag: f64,
    /// Cells a unit may lag its place before its formation waits (at discipline 100, 0); past
    /// walk_lost it has fallen behind; past walk_face_travel it faces where it walks.
    pub walk_slack_drilled: f64,
    pub walk_slack_loose: f64,
    pub walk_lost: f64,
    pub walk_face_travel: f64,
    /// Cells to form up or split off together; the reach and hold (seconds) of taking the downed.
    pub group_join: f64,
    pub take_reach: f64,
    pub take_hold: f64,
    /// Groups look path_look cells ahead every path_relook, going round if path_detour_gain quicker.
    pub path_look: f64,
    pub path_relook: f64,
    pub path_detour_gain: f64,
}

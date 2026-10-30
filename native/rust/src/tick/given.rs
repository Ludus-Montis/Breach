//! The tick's rules read in the crate (spec 39 step 8c-i): every pass's arguments built from
//! the battle's tuning (`content/tuning/battle_tuning.json`) and the tick's own numbers, laid
//! out as each pass's GDScript `args` laid them out (FormationOpening, FormationScrum,
//! FormationAftermath, FormationMustering and FormationEnding), so the binding reads them as
//! it read GDScript's and nothing outside the crate hands the tick its rules.

#[cfg(test)]
mod tests;

use crate::content::{BattleTuning, CELLS_PER_TILE};
use crate::det_math;

/// One of a pass's arguments, as its binding takes it.
#[derive(Clone, Debug, PartialEq)]
pub enum Arg {
    Ints(Vec<i64>),
    Floats(Vec<f64>),
    Real(f64),
    /// An array of float arrays (the mustering's tunings).
    Lists(Vec<Vec<f64>>),
}

/// The tick's own numbers (FormationTick.step's `at` and `rolls`, and whether the bodies'
/// parting is threaded).
#[derive(Clone, Copy, Debug)]
pub struct At {
    pub tick: i64,
    pub seed: i64,
    pub ticks_a_second: i64,
    pub rolls: bool,
    pub threaded: bool,
}

/// FormationTick.step's `paces`: tiles a second at speed 1, the tick's seconds, the lane's
/// length, cells a second at speed 1.
#[derive(Clone, Copy, Debug)]
pub struct Paces {
    pub travel_scale: f64,
    pub seconds: f64,
    pub route_length: f64,
    pub cells_a_second: f64,
}

/// Each pass's arguments past the roster and the ground, in the passes' order.
#[derive(Clone, Debug, PartialEq)]
pub struct Given {
    pub opening: Vec<Arg>,
    pub scrum: Vec<Arg>,
    pub aftermath: Vec<Arg>,
    pub mustering: Vec<Arg>,
    pub ending: Vec<Arg>,
}

/// Every pass's arguments for the tick `at`, as FormationTick's `args` gave them.
pub fn of(t: &BattleTuning, at: &At, paces: &Paces) -> Given {
    Given {
        opening: opening(t, at, paces),
        scrum: scrum(t, at, paces),
        aftermath: aftermath(t, at, paces),
        mustering: mustering(t, at, paces),
        ending: ending(t, at, paces),
    }
}

/// FormationOpening.args: [tick, seed], the orders' rules, the bands, the side and rear
/// impacts, the looks and the move's rules.
fn opening(t: &BattleTuning, at: &At, p: &Paces) -> Vec<Arg> {
    let mut orders = vec![t.pursuit_tempted_within, t.pursuit_ragged_shock as f64];
    orders.extend(leash(t, [t.discipline_meets_threats as f64, t.discipline_expected_max]));
    let looks = vec![
        t.path_relook,
        t.path_look,
        t.path_detour_gain,
        t.sight_budget,
        t.ground_cliff_quarters as f64,
        t.stamina_climb,
        t.stamina_swim,
    ];
    let moving = vec![
        p.travel_scale,
        p.seconds,
        p.route_length,
        t.discipline_per_leadership as f64,
        t.walk_slack_loose,
        t.walk_slack_drilled,
        t.bodies_ground_drag,
        t.bodies_ground_block,
        t.wounds_reach,
        t.walk_lost,
    ];
    vec![
        Arg::Ints(vec![at.tick, at.seed]),
        Arg::Floats(orders),
        Arg::Ints(bands(t)),
        Arg::Ints(vec![t.morale_side_impact, t.morale_rear_impact]),
        Arg::Floats(looks),
        Arg::Floats(moving),
    ]
}

/// FormationScrum.args: [tick, seed, the contest's die, the stall's ticks], reach_engage,
/// then ScrumStance's, ScrumSeek's, ScrumRegroup's, FormationWithdraw's and
/// FormationPursuit's rules.
fn scrum(t: &BattleTuning, at: &At, p: &Paces) -> Vec<Arg> {
    let pace = p.cells_a_second * p.seconds;
    let stall = (t.scrum_stall_seconds / p.seconds).round() as i64;
    let leads = t.discipline_per_leadership as f64;
    let threats = t.discipline_meets_threats as f64;
    let stance = vec![t.scrum_anticipate, leads, threats];
    let seek = vec![
        t.reach_contact,
        t.scrum_leash,
        p.seconds,
        pace,
        t.scrum_crowding,
        t.bodies_steer_look,
        t.bodies_steer_clear,
        t.bodies_ground_drag,
        t.bodies_ground_block,
        front(t),
    ];
    let mut regroup = vec![pace, p.seconds, t.reach_contact, t.reach_front_arc_degrees];
    regroup.extend([t.bodies_steer_look, t.bodies_steer_clear, t.scrum_trade_seconds]);
    regroup.extend([p.cells_a_second, t.stamina_give_up, leads, threats]);
    regroup.extend(re_forming(t));
    regroup.push(t.wounds_reach);
    let mut withdraw = vec![pace, p.seconds, t.reach_contact, t.rout_enemy_near];
    withdraw.extend([t.rout_fan_cells, t.rout_rally_seconds, t.rout_fan_degrees, leads, threats]);
    withdraw.extend(re_forming(t));
    let mut pursue = vec![p.cells_a_second, p.seconds, t.stamina_give_up, t.reach_pursuit_lag];
    pursue.extend(leash(t, [t.discipline_expected_max]));
    vec![
        Arg::Ints(vec![at.tick, at.seed, t.combat_contest_die, stall]),
        Arg::Real(t.reach_engage),
        Arg::Floats(stance),
        Arg::Floats(seek),
        Arg::Floats(regroup),
        Arg::Floats(withdraw),
        Arg::Floats(pursue),
    ]
}

/// FormationAftermath.args: [ticks a second, seed, tick, 1 if blows roll, the messenger's
/// shock], FormationMelee's rules and [seconds, take_hold, take_reach], the bands,
/// BlowLanding's tuning and FormationWounds'.
fn aftermath(t: &BattleTuning, at: &At, p: &Paces) -> Vec<Arg> {
    let ints = vec![at.ticks_a_second, at.seed, at.tick, at.rolls as i64, t.wounds_messenger_shock];
    let mut floats = vec![t.reach_contact, front(t), t.rout_strike_reach, t.stamina_blow];
    floats.extend([t.wounds_surrender, p.seconds, t.take_hold, t.take_reach]);
    vec![
        Arg::Ints(ints),
        Arg::Floats(floats),
        Arg::Ints(bands(t)),
        Arg::Floats(landing(t)),
        Arg::Floats(wounds(t)),
    ]
}

/// FormationMustering.args: [tick, seed, 1 if a second's tick], the tunings (morale's,
/// FormationRout's, FormationCarry's and its loads, FormationGroups') and the bands.
fn mustering(t: &BattleTuning, at: &At, p: &Paces) -> Vec<Arg> {
    let second = at.tick % at.ticks_a_second.max(1) == 0;
    let mut morale = vec![t.morale_recovery as f64, t.morale_support_cells, t.condition_cap];
    morale.extend(t.morale_pressure.iter().map(|&n| n as f64));
    let mut rout = vec![p.cells_a_second * p.seconds, p.seconds, t.rout_fan_degrees];
    rout.extend([t.rout_fan_cells, t.rout_rally_reach, t.rout_steady_rally_seconds]);
    rout.extend([t.rout_rally_seconds, t.rout_enemy_near, t.rout_reformed_morale as f64]);
    rout.extend([t.morale_steady_at, t.rout_crush, t.rout_panic, t.rout_seen].map(|n| n as f64));
    let mut carry = vec![t.wounds_reach, t.wounds_guard_reach, t.wounds_body_weight];
    carry.extend([t.load_per_strength, t.load_easy, t.load_most, t.load_hauler]);
    let loads = [t.load_pace.as_slice(), &t.load_tiring, &t.load_dodge].concat();
    let groups = vec![t.walk_lost, t.group_join, CELLS_PER_TILE as f64];
    vec![
        Arg::Ints(vec![at.tick, at.seed, second as i64]),
        Arg::Lists(vec![morale, rout, carry, loads, groups]),
        Arg::Ints(bands(t)),
    ]
}

/// FormationEnding.args: [tick, seed, 1 if threaded], [tick seconds, travel scale,
/// scrum_crowding, cells a second], FormationStamina's tuning, the bodies' and the walk's
/// (CrateMarch.walk_tuning, then wounds_reach).
fn ending(t: &BattleTuning, at: &At, p: &Paces) -> Vec<Arg> {
    let floats = vec![p.seconds, p.travel_scale, t.scrum_crowding, p.cells_a_second];
    let mut stamina = vec![t.stamina_run, t.stamina_delay, t.stamina_recovery];
    stamina.extend([t.run_build_seconds, t.run_reaction, t.condition_cap]);
    stamina.extend([t.stamina_tired, t.stamina_spent]);
    stamina.extend(&t.stamina_pace);
    stamina.extend(&t.load_march_tiring);
    let bodies = vec![t.bodies_passes as f64, t.bodies_resist, t.bodies_brush];
    vec![
        Arg::Ints(vec![at.tick, at.seed, at.threaded as i64]),
        Arg::Floats(floats),
        Arg::Floats(stamina),
        Arg::Floats(bodies),
        Arg::Floats(walk(t)),
    ]
}

/// The walk's tuning as the march's sync takes it: CrateMarch.walk_tuning, then wounds_reach.
pub(crate) fn walk(t: &BattleTuning) -> Vec<f64> {
    let mut walk = vec![t.walk_face_travel, t.bodies_ground_drag, t.bodies_ground_block];
    walk.extend([t.stamina_climb, t.stamina_swim, t.stamina_spent]);
    walk.extend([t.ground_cliff_quarters as f64, t.fall_damage_per_cell, t.fall_mass_power]);
    walk.push(t.wounds_reach);
    walk
}

/// BandTuning.rules: the ceiling's rise per leadership, the bands' floors and the losses.
pub(crate) fn bands(t: &BattleTuning) -> Vec<i64> {
    vec![
        t.morale_per_leadership,
        t.morale_steady_at,
        t.morale_shaken_at,
        t.morale_loss,
        t.morale_leader_loss,
    ]
}

/// The discipline's leash as the orders and the pursuit hand it over: discipline_per_leadership,
/// then `mid`, then how many shares, the shares and the steps.
fn leash<const N: usize>(t: &BattleTuning, mid: [f64; N]) -> Vec<f64> {
    let mut out = vec![t.discipline_per_leadership as f64];
    out.extend(mid);
    out.push(t.discipline_leash_shares.len() as f64);
    out.extend(&t.discipline_leash_shares);
    out.extend(&t.discipline_leash_steps);
    out
}

/// The re-form pace's discipline: where it reaches the march pace, the slowest and fastest.
fn re_forming(t: &BattleTuning) -> [f64; 3] {
    [t.discipline_march_pace_at, t.discipline_slowest, t.discipline_fastest]
}

/// ScrumReach.in_front's least dot: the cosine of the front arc, less 0.000001.
fn front(t: &BattleTuning) -> f64 {
    det_math::cos(t.reach_front_arc_degrees * (std::f64::consts::PI / 180.0)) - 0.000001
}

/// BlowLanding.tuning: the blow's and the damage's numbers, the morale shifts (their count
/// first) and the stamina's skill.
fn landing(t: &BattleTuning) -> Vec<f64> {
    let mut out = vec![t.blow_die, t.blow_parry_share, t.blow_dodge_per_agility];
    out.extend([t.blow_hit_band, t.blow_graze_share, t.blow_high_ground, t.blow_surrounded]);
    out.extend([t.blow_rear_arc_degrees, t.damage_weak, t.damage_resist, t.damage_immune]);
    out.extend([t.damage_floor_skill, t.damage_spill, t.wounds_regeneration_halt]);
    out.extend([t.reach_front_arc_degrees, t.stamina_tired, t.stamina_spent]);
    out.push(t.blow_morale_shift.len() as f64);
    out.extend(&t.blow_morale_shift);
    out.extend(&t.stamina_skill);
    out
}

/// FormationWounds.tuning.
fn wounds(t: &BattleTuning) -> Vec<f64> {
    let mut out = vec![t.wounds_reach, t.wounds_guard_reach, t.wounds_condition];
    out.extend([t.wounds_danger_reach, t.wounds_play_dead, t.wounds_cunning]);
    out.extend([t.wounds_play_dead_check, t.wounds_trait_wits, t.wounds_believe_die]);
    out.extend([t.wounds_wake_seconds, t.wounds_wake_spread[0], t.wounds_wake_spread[1]]);
    out.extend([t.condition_cap, t.condition_drain_below, t.condition_drain]);
    out
}

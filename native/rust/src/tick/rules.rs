//! The tick's rules parsed in the crate (spec 39 step 8c-ii): each pass's arguments, laid out
//! as its binding takes them, read into the rules its pass takes. The bindings hand these
//! their Packed arrays as slices, and `of` reads `tick::given`'s, so a cargo binary gives
//! `tick::step` its rules with nothing from Godot.

#[cfg(test)]
mod tests;

use super::given::{Arg, Given};
use super::Rules;
use crate::parting::Tuning;
use crate::paths::PathTuning;
use crate::{
    aftermath, bands, carry, contact, discipline, ending, fight, landing, march, morale, moving,
    mustering, near, opening, pathing, pursuit, regroup, rout, scrum, split, stamina, taking,
    withdraw, wounds,
};

/// The climb the paths take and the rules `tick::step` takes, read from `given`; None if a
/// pass is short of an argument or one is of the wrong kind.
pub fn of(given: &Given) -> Option<(PathTuning, Rules<'static>)> {
    let o = &given.opening;
    let (climb, opening) = opening_rules(
        [ints(o, 0)?, ints(o, 2)?, ints(o, 3)?],
        [floats(o, 1)?, floats(o, 4)?, floats(o, 5)?],
    );
    let s = &given.scrum;
    let phases = [floats(s, 2)?, floats(s, 3)?, floats(s, 4)?, floats(s, 5)?, floats(s, 6)?];
    let scrum = scrum_rules(ints(s, 0)?, real(s, 1)?, phases);
    let a = &given.aftermath;
    let aftermath =
        aftermath_rules(ints(a, 0)?, floats(a, 1)?, ints(a, 2)?, floats(a, 3)?, floats(a, 4)?);
    let m = &given.mustering;
    let tunings = lists(m, 1)?;
    let tuning = |n: usize| tunings.get(n).map(Vec::as_slice);
    let tunings = [tuning(0)?, tuning(1)?, tuning(2)?, tuning(3)?, tuning(4)?];
    let mustering = mustering_rules(ints(m, 0)?, tunings, ints(m, 2)?)?;
    let e = &given.ending;
    let ending =
        ending_rules(ints(e, 0)?, [floats(e, 1)?, floats(e, 2)?, floats(e, 3)?, floats(e, 4)?]);
    Some((climb, Rules { opening, scrum, aftermath, mustering, ending }))
}

fn ints(args: &[Arg], n: usize) -> Option<&[i64]> {
    match args.get(n)? {
        Arg::Ints(values) => Some(values),
        _ => None,
    }
}

fn floats(args: &[Arg], n: usize) -> Option<&[f64]> {
    match args.get(n)? {
        Arg::Floats(values) => Some(values),
        _ => None,
    }
}

fn real(args: &[Arg], n: usize) -> Option<f64> {
    match args.get(n)? {
        Arg::Real(value) => Some(*value),
        _ => None,
    }
}

fn lists(args: &[Arg], n: usize) -> Option<&[Vec<f64>]> {
    match args.get(n)? {
        Arg::Lists(values) => Some(values),
        _ => None,
    }
}

/// `opening_step`'s rules ([ints, bands, impacts] and [orders, looks, moving]) as the climb
/// its paths take and the opening's rules.
pub(crate) fn opening_rules(
    ints: [&[i64]; 3],
    floats: [&[f64]; 3],
) -> (PathTuning, opening::Rules<'static>) {
    let [ints, bands, impacts] = ints;
    let [o, looks, moving] = floats;
    let (tick, seed) = (ints[0], ints[1]);
    let (climb, looks) = looks_of(looks);
    let rules = opening::Rules {
        tick,
        seed,
        within: o[0],
        ragged_shock: o[1] as i64,
        discipline: orders_discipline(o),
        bands: bands::BandTuning::of_slice(bands),
        impacts: contact::pass::Impacts { side: impacts[0], rear: impacts[1] },
        looks,
        moving: moving_given(&[], tick, seed, moving),
    };
    (climb, rules)
}

/// FormationOrders' `rules` past its first two as the discipline a retreat reads: [
/// discipline_per_leadership, discipline_meets_threats, discipline_expected_max, how many
/// leash shares follow, the shares, the leash steps].
pub(crate) fn orders_discipline(r: &[f64]) -> discipline::DisciplineTuning {
    let shares = r[5] as usize;
    discipline::DisciplineTuning {
        per_leadership: r[2] as i64,
        meets_threats: r[3] as i64,
        expected_max: r[4],
        leash_shares: r[6..6 + shares].to_vec(),
        leash_steps: r[6 + shares..].to_vec(),
    }
}

/// FormationPathing's seven: [path_relook, path_look, path_detour_gain, sight_budget,
/// ground_cliff_quarters, stamina_climb, stamina_swim] as the climb and the looks.
pub(crate) fn looks_of(t: &[f64]) -> (PathTuning, pathing::pass::Looks) {
    let climb = PathTuning { cliff_quarters: t[4] as i64, stamina_climb: t[5], stamina_swim: t[6] };
    let look = pathing::LookTuning { look: t[1], detour_gain: t[2], sight_budget: t[3] };
    (climb, pathing::pass::Looks { relook: t[0], look })
}

/// `move_step`'s ten as the move's tuning, over `routes`.
pub(crate) fn moving_given<'a>(
    routes: &'a [u32],
    tick: i64,
    seed: i64,
    t: &[f64],
) -> moving::pass::Given<'a> {
    moving::pass::Given {
        routes,
        tick,
        travel_scale: t[0],
        seconds: t[1],
        route_length: t[2],
        share: share_tuning(seed, &t[3..6], &t[8..10]),
        drag: [t[6], t[7]],
    }
}

/// The share's tuning: the fight's `seed`, [discipline_per_leadership, walk_slack_loose,
/// walk_slack_drilled] and [wounds_reach, walk_lost].
pub(crate) fn share_tuning(seed: i64, slack: &[f64], reach: &[f64]) -> moving::share::ShareTuning {
    moving::share::ShareTuning {
        seed,
        wounds_reach: reach[0],
        lost: reach[1],
        slack: [slack[0], slack[1], slack[2]],
    }
}

/// `scrum_step`'s `ints`, reach_engage and its phases' rules ([stance, seek, regroup,
/// withdraw, pursue]) as the scrum's rules.
pub(crate) fn scrum_rules(i: &[i64], reach_engage: f64, phases: [&[f64]; 5]) -> scrum::step::Rules {
    let (tick, seed) = (i[0], i[1]);
    let [stance, seek, regroup, withdraw, pursue] = phases;
    let (anticipate, stance) = stance_rules(stance);
    scrum::step::Rules {
        tick,
        seed,
        reach_engage,
        anticipate,
        stance,
        seek: seek_rules(&i[..3], seek),
        regroup: regroup_rules(regroup, seed),
        withdraw: withdraw_rules(withdraw, tick, seed),
        pursue: pursue_rules(pursue, tick),
        stall_ticks: i[3],
    }
}

/// ScrumStance.anticipate_all's `rules` ([scrum_anticipate, discipline_per_leadership,
/// discipline_meets_threats]) as the anticipation and the discipline it reads.
pub(crate) fn stance_rules(r: &[f64]) -> (f64, discipline::DisciplineTuning) {
    let discipline = discipline::DisciplineTuning {
        per_leadership: r[1] as i64,
        meets_threats: r[2] as i64,
        ..discipline::DisciplineTuning::default()
    };
    (r[0], discipline)
}

/// `scrum_seek`'s `ints` and `floats` as the seek's rules.
pub(crate) fn seek_rules(i: &[i64], f: &[f64]) -> scrum::seek::Rules {
    scrum::seek::Rules {
        tick: i[0],
        seed: i[1],
        die: i[2],
        reach: f[0],
        leash: f[1],
        seconds: f[2],
        pace: f[3],
        crowding: f[4],
        look: f[5],
        clear: f[6],
        drag: f[7],
        block: f[8],
        front: f[9],
    }
}

/// `scrum_regroup`'s `rules` as the regroup's.
pub(crate) fn regroup_rules(r: &[f64], seed: i64) -> scrum::regroup::Rules {
    scrum::regroup::Rules {
        regroup: regroup::Rules {
            pace: r[0],
            seconds: r[1],
            seed,
            reach: r[2],
            arc: r[3],
            look: r[4],
            clear: r[5],
            trade_seconds: r[6],
        },
        chase: pursuit::Rules {
            cells_per_second: r[7],
            seconds: r[1],
            give_up: r[8],
            steering: near::Steering { look: r[4], clear: r[5], seed },
        },
        discipline: discipline::DisciplineTuning {
            per_leadership: r[9] as i64,
            meets_threats: r[10] as i64,
            ..discipline::DisciplineTuning::default()
        },
        reform: [r[11], r[12], r[13]],
        seed,
        wounds_reach: r[14],
    }
}

/// `scrum_withdraw`'s `rules` as the withdrawal's.
pub(crate) fn withdraw_rules(r: &[f64], tick: i64, seed: i64) -> scrum::withdraw::Rules {
    scrum::withdraw::Rules {
        flight: withdraw::Rules {
            pace: r[0],
            seconds: r[1],
            reach: r[2],
            near: r[3],
            fan_cells: r[4],
        },
        rally_seconds: r[5],
        fan_degrees: r[6],
        discipline: discipline::DisciplineTuning {
            per_leadership: r[7] as i64,
            meets_threats: r[8] as i64,
            ..discipline::DisciplineTuning::default()
        },
        reform: [r[9], r[10], r[11]],
        seed,
        tick,
    }
}

/// `scrum_pursue`'s `rules` as the pursuit's.
pub(crate) fn pursue_rules(r: &[f64], tick: i64) -> scrum::pursue::Rules {
    let shares = r[6] as usize;
    scrum::pursue::Rules {
        whole: pursuit::whole::Rules {
            cells_per_second: r[0],
            seconds: r[1],
            give_up: r[2],
            lag: r[3],
        },
        discipline: discipline::DisciplineTuning {
            per_leadership: r[4] as i64,
            expected_max: r[5],
            leash_shares: r[7..7 + shares].to_vec(),
            leash_steps: r[7 + shares..].to_vec(),
            ..discipline::DisciplineTuning::default()
        },
        tick,
    }
}

/// `aftermath_step`'s rules as the aftermath's.
pub(crate) fn aftermath_rules(
    i: &[i64],
    f: &[f64],
    bands: &[i64],
    tuning: &[f64],
    wounds: &[f64],
) -> aftermath::Rules {
    aftermath::Rules {
        fight: fight_rules(i, f, bands, tuning),
        seconds: f[5],
        taking: taking::pass::TakingTuning { hold: f[6], reach: f[7] },
        wounds: wounds::WoundsTuning::of_slice(wounds),
        messenger_shock: i[4],
        bands: bands::BandTuning::of_slice(bands),
    }
}

/// The fight's rules as `fight_step` is given them: `i` = [the ticks a second's strike takes,
/// seed, tick, 1 if blows roll], `f` = [reach_contact, the front's least dot,
/// rout_strike_reach, stamina_blow, wounds_surrender].
pub(crate) fn fight_rules(
    i: &[i64],
    f: &[f64],
    bands: &[i64],
    tuning: &[f64],
) -> fight::step::Rules {
    let bands = bands::BandTuning::of_slice(bands);
    fight::step::Rules {
        choose: fight::Rules {
            interval: i[0],
            seed: i[1],
            reach: f[0],
            front: f[1],
            rout_reach: f[2],
            bands,
        },
        land: fight::land::Rules {
            tuning: landing::LandingTuning::of_slice(tuning),
            rolls: (i[3] != 0).then_some((i[1], i[2])),
            reach: f[0],
            bands,
        },
        tick: i[2],
        blow: f[3],
        surrender: f[4],
    }
}

/// `mustering_step`'s `ints`, `tunings` and `bands` as the mustering's rules; None if one is
/// short.
pub(crate) fn mustering_rules(
    ints: &[i64],
    tunings: [&[f64]; 5],
    bands: &[i64],
) -> Option<mustering::Rules> {
    let [morale, rout, carrying, loads, grouping] = tunings;
    let (&[tick, seed, second], (r, &[crush, panic, seen]), &[lost, join, cells_per_tile]) =
        (ints, rout.split_at_checked(10)?, grouping)
    else {
        return None;
    };
    Some(mustering::Rules {
        morale: (second != 0).then(|| morale::MoraleTuning::of_slice(morale)),
        rout: rout::RoutTuning::of_slice(r),
        shocks: rout::pass::RoutShocks {
            crush: crush as i64,
            panic: panic as i64,
            seen: seen as i64,
        },
        bands: bands::BandTuning::of_slice(bands),
        carry: carry::CarryTuning::of_slice(carrying),
        loads: carry::pass::LoadTuning::of_slice(loads),
        groups: split::Rules { lost, join, seed, cells_per_tile },
        tick,
        seed,
    })
}

/// FormationStamina.tuning as `stamina_step` takes it: stamina_run, stamina_delay,
/// stamina_recovery, run_build_seconds, run_reaction, condition_cap, stamina_tired,
/// stamina_spent, the three stamina_pace, then load_march_tiring.
pub(crate) fn stamina_tuning(t: &[f64]) -> stamina::StaminaTuning {
    stamina::StaminaTuning {
        run: t[0],
        delay: t[1],
        recovery: t[2],
        run_build_seconds: t[3],
        run_reaction: t[4],
        condition_cap: t[5],
        tired: t[6],
        spent: t[7],
        pace: t[8..11].to_vec(),
        march_tiring: t[11..].to_vec(),
    }
}

/// `ending_step`'s `ints` and [floats, stamina, bodies, walk] as the end's rules.
pub(crate) fn ending_rules(i: &[i64], floats: [&[f64]; 4]) -> ending::Rules {
    let [f, stamina, b, walk] = floats;
    ending::Rules {
        tick: i[0],
        seconds: f[0],
        seed: i[1],
        stamina: stamina_tuning(stamina),
        parting: Tuning { passes: b[0] as i64, resist: b[1], brush: b[2] },
        threaded: i[2] != 0,
        shuffle: [f[0], f[1], f[2]],
        timing: [f[0], f[3]],
        walk: walk_tuning_of(walk),
        wounds_reach: walk[9],
    }
}

/// [walk_face_travel, bodies_ground_drag, bodies_ground_block, stamina_climb, stamina_swim,
/// stamina_spent, ground_cliff_quarters, fall_damage_per_cell, fall_mass_power].
pub(crate) fn walk_tuning_of(t: &[f64]) -> march::walk::WalkTuning {
    march::walk::WalkTuning {
        face_travel: t[0],
        drag: t[1],
        block: t[2],
        modes: march::modes::ModeTuning {
            climb_rate: t[3],
            swim_rate: t[4],
            spent: t[5],
            cliff_quarters: t[6] as i64,
            fall_per_cell: t[7],
            fall_mass_power: t[8],
        },
    }
}

//! The scrum as one crate pass (FormationScrum.step, spec 39 step 8b5-xxxiv): every phase in
//! its order over one roster, as GDScript made them one call at a time. Engaging (`pass::
//! engage`), readying (`prepare::prepare`), the stances (`prepare::anticipate`), the
//! manoeuvres (`pass::manoeuvre`), the seek, walk and facing (`seek::step`), the regroup and
//! the chasers (`regroup::step`), the withdrawal (`withdraw::step`), the pursuit
//! (`pursue::step`) and the stall (`pass::stall`, quiet unless the seek found its squad or a
//! foe active). Every phase's events go to the log in that order.

use std::time::Instant;

use crate::discipline::DisciplineTuning;
use crate::events::Log;
use crate::field::Field;
use crate::ground::Ground;
use crate::roster::Roster;
use crate::scrum::{pass, prepare, pursue, regroup, seek, withdraw};
use crate::squads::Squads;

/// Every phase's tuning: the tick and seed, `reach_engage`, the stances' anticipation and
/// discipline, each later phase's own rules and how many quiet ticks a fight stalls after.
pub struct Rules {
    pub tick: i64,
    pub seed: i64,
    pub reach_engage: f64,
    pub anticipate: f64,
    pub stance: DisciplineTuning,
    pub seek: seek::Rules,
    pub regroup: regroup::Rules,
    pub withdraw: withdraw::Rules,
    pub pursue: pursue::Rules,
    pub stall_ticks: i64,
}

/// What the scrum hands back: the listed indices of the pursuers back at their posts, in
/// order (they hold their own routes again), and the seek's time in microseconds (for the
/// benches; it decides nothing).
pub struct Scrummed {
    pub back: Vec<usize>,
    pub seek_usec: i64,
}

/// FormationScrum.step over `roster`'s squads on `ground`, its events in `log`.
pub fn step(
    squads: &mut Squads,
    field: &mut Field,
    roster: &Roster,
    ground: Option<&Ground>,
    rules: &Rules,
    log: &mut Log,
) -> Scrummed {
    let (tick, seed) = (rules.tick, rules.seed);
    pass::engage(squads, &mut field.columns, roster, (tick, seed, rules.reach_engage), log);
    prepare::prepare(squads, field, roster, tick);
    let lines = prepare::Lines { anticipate: rules.anticipate, seed, discipline: &rules.stance };
    prepare::anticipate(squads, field, roster, ground, (&lines, tick), log);
    pass::manoeuvre(squads, &mut field.columns, roster);
    let began = Instant::now();
    let active = seek::step(squads, field, roster, ground, &rules.seek);
    let seek_usec = began.elapsed().as_micros() as i64;
    regroup::step(squads, field, roster, ground, &rules.regroup);
    withdraw::step(squads, field, roster, ground, &rules.withdraw, log);
    let back = pursue::step(squads, field, roster, &rules.pursue, log);
    pass::stall(squads, roster, (&active, rules.stall_ticks), tick, log);
    Scrummed { back, seek_usec }
}

#[cfg(test)]
pub(crate) mod tests;

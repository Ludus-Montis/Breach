//! The scrum's withdrawals over the roster (FormationWithdraw.step, spec 39 step 8b5-xv).
//! Each withdrawing squad's say is read where the crate holds it: its record and state (one
//! routing or destroyed withdraws no more, `withdraw::ended`), its disorder and re-form pace
//! from its living units' sheets (FormationWithdraw.disorder, FormationDiscipline
//! .reform_pace), its route and home, and its units' flight angles (`rout::flight::fan`).
//! Then the flight and the records (`withdraw::step`, `withdraw::records`), and each re-form
//! landed in the order it came: its turn (`turn::landed`), "regrouping", and "returned" for
//! one home with nowhere further to go, in the log.

use crate::discipline::{self, DisciplineTuning};
use crate::events::{self, Log};
use crate::field::Field;
use crate::geometry::CELLS;
use crate::ground::Ground;
use crate::roster::Roster;
use crate::rout::flight;
use crate::squads::Squads;
use crate::turn;
use crate::withdraw::{self, Withdrawing};

/// The tuning it reads: the flight's own, `rout_rally_seconds`, `rout_fan_degrees`, the
/// discipline's (`discipline_per_leadership`, `discipline_meets_threats`), the re-form
/// pace's (`discipline_march_pace_at`, `discipline_slowest`, `discipline_fastest`), the
/// fight's seed and the tick.
pub struct Rules {
    pub flight: withdraw::Rules,
    pub rally_seconds: f64,
    pub fan_degrees: f64,
    pub discipline: DisciplineTuning,
    pub reform: [f64; 3],
    pub seed: i64,
    pub tick: i64,
}

/// FormationWithdraw.step over `roster`'s squads on `ground` (none: open): units flee, and
/// squads safe long enough, or home, re-form; their events go to `log`.
pub fn step(
    squads: &mut Squads,
    field: &mut Field,
    roster: &Roster,
    ground: Option<&Ground>,
    rules: &Rules,
    log: &mut Log,
) {
    withdraw::ended(squads, roster);
    let withdrawing: Vec<Withdrawing> = (0..roster.listed.len())
        .filter(|&n| squads.withdraw[roster.listed[n].squad].is_some())
        .map(|n| withdrawer(squads, field, roster, n, rules))
        .collect();
    if withdrawing.is_empty() {
        return;
    }
    let columns = &mut field.columns;
    let out = withdraw::step(squads, columns, roster, &withdrawing, ground, &rules.flight);
    let timing = (rules.flight.seconds, rules.rally_seconds);
    for r in withdraw::records(squads, columns, roster, &withdrawing, &out, timing) {
        let s = roster.listed[r.listed].squad;
        turn::landed(squads, columns, &roster.listed[r.listed], r.homeward, rules.tick, log);
        log.squad(events::REGROUPING, rules.tick, squads, s, &[]);
        if r.arrived {
            log.squad(events::RETURNED, rules.tick, squads, s, &[]); // it holds, facing out
        }
    }
}

/// FormationWithdraw's say of the listed squad `n`, withdrawing.
fn withdrawer(
    squads: &Squads,
    field: &Field,
    roster: &Roster,
    n: usize,
    rules: &Rules,
) -> Withdrawing {
    let listed = &roster.listed[n];
    let s = listed.squad;
    let columns = &field.columns;
    let held = discipline::of(columns, listed, &rules.discipline);
    Withdrawing {
        listed: n,
        pursued: withdraw::pursued(squads, roster, n),
        disorder: discipline::disorder(columns, listed, &rules.discipline),
        reform: discipline::reform_pace(held, rules.reform),
        home: squads.home_distance[s] * CELLS,
        route: squads.route[s].clone().expect("a withdrawing squad has a route"),
        fans: listed
            .living
            .iter()
            .map(|m| flight::fan(m.id, rules.seed, rules.fan_degrees))
            .collect(),
    }
}

#[cfg(test)]
mod tests;

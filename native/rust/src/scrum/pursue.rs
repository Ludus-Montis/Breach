//! The scrum's pursuits as a whole over the roster (FormationPursuit.step, spec 39 step
//! 8b5-xvi). Each pursuer's say is read where the crate holds it: the route it is on, the
//! route it held its post on (`post_route`, kept as its pursuit began) and its leash, its
//! discipline's step (from its living units' sheets) moved a step out by a leader who
//! "pursues" and in by one who is "cautious" (FormationDiscipline.pursuit_leash). Then the
//! pursuit (`pursuit::whole::step`), and each end landed in the order it came: "pursuit_ended"
//! for one that gave up, and for one back at its post its own route again and its turn to
//! face the way it held it (`turn::landed`), in the log.

use crate::discipline::{self, DisciplineTuning};
use crate::events::{self, Log};
use crate::field::Field;
use crate::pursuit::whole::{self, Pursued, Pursuing};
use crate::roster::Roster;
use crate::squads::Squads;
use crate::turn;

/// The tuning it reads: the pursuit's own, the discipline's (`discipline_per_leadership`,
/// `discipline_expected_max`, the leash's shares and steps) and the tick.
pub struct Rules {
    pub whole: whole::Rules,
    pub discipline: DisciplineTuning,
    pub tick: i64,
}

/// FormationPursuit.step over `roster`'s squads: pursuers advance after their enemy, give up
/// or are back at their posts; their events go to `log`. Returns the listed indices of those
/// back at their posts, in order: they hold their own routes again.
pub fn step(
    squads: &mut Squads,
    field: &mut Field,
    roster: &Roster,
    rules: &Rules,
    log: &mut Log,
) -> Vec<usize> {
    let pursuing: Vec<Pursuing> = (0..roster.listed.len())
        .filter(|&n| {
            let listed = &roster.listed[n];
            squads.pursuit[listed.squad].is_some() && !listed.living.is_empty()
        })
        .map(|n| pursuer(squads, field, roster, n, &rules.discipline))
        .collect();
    if pursuing.is_empty() {
        return Vec::new();
    }
    let mut back = Vec::new();
    let columns = &mut field.columns;
    for done in whole::step(squads, columns, roster, &pursuing, &rules.whole) {
        match done {
            Pursued::Ended(n) => {
                let s = roster.listed[n].squad;
                log.squad(events::PURSUIT_ENDED, rules.tick, squads, s, &[]);
            }
            Pursued::Arrived(n, way) => {
                let s = roster.listed[n].squad;
                squads.route[s] = squads.post_route[s].take(); // at its post: its own again
                turn::landed(squads, columns, &roster.listed[n], way, rules.tick, log);
                back.push(n);
            }
        }
    }
    back
}

/// FormationPursuit's say of the listed squad `n`, pursuing.
fn pursuer(
    squads: &Squads,
    field: &Field,
    roster: &Roster,
    n: usize,
    t: &DisciplineTuning,
) -> Pursuing {
    let listed = &roster.listed[n];
    let s = listed.squad;
    let held = discipline::of(&field.columns, listed, t);
    Pursuing {
        listed: n,
        route: squads.route[s].clone(),
        held: squads.post_route[s].clone(),
        leash: discipline::leash(held as f64, shift(field, roster, n), t),
    }
}

/// FormationDiscipline.pursuit_leash's shift: a step out if a living unit "pursues", a step
/// in if one is "cautious".
fn shift(field: &Field, roster: &Roster, n: usize) -> i64 {
    let has = |tactic: &str| {
        field.lexicon.find(tactic).is_some_and(|code| {
            let living = &roster.listed[n].living;
            living.iter().any(|m| field.columns.words[m.slot].tactics.contains(&code))
        })
    };
    i64::from(has("pursues")) - i64::from(has("cautious"))
}

#[cfg(test)]
mod tests;

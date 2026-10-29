//! The surrender (FormationWounds.surrender, Decision 121; spec 39 step 8b5-xx): each melee
//! blow, in the order chosen, that dealt damage to a standing unit of a routing squad, one
//! without "never_surrenders", catches it; `wounds::surrenders` rolls each caught. One that
//! gives up and still stands (struck twice, it gives up once) is TAKEN, and "surrendered" is
//! logged; its squad, left with no one standing, is DESTROYED and released (`locks::release`)
//! and "destroyed" logged.

use super::{Blow, Who};
use crate::events::{Log, DESTROYED as DESTROYED_EVENT, SURRENDERED};
use crate::field::Field;
use crate::locks::{self, DESTROYED, ROUTING};
use crate::roster::Roster;
use crate::squads::{Squads, DEAD};
use crate::wounds::{self, Caught};

/// SkirmishUnit.State's TAKEN.
pub const TAKEN: i32 = 6;

/// The tick and the chance (wounds_surrender) a routing unit struck surrenders, times its
/// want of courage.
pub struct Rules {
    pub tick: i64,
    pub seed: i64,
    pub chance: f64,
}

/// The surrender over this tick's melee `blows` (at the damage each dealt); returns the units
/// taken, in the order they gave up.
pub fn surrender(
    squads: &mut Squads,
    field: &mut Field,
    roster: &Roster,
    blows: &[Blow],
    rules: &Rules,
    log: &mut Log,
) -> Vec<Who> {
    let stout = field.lexicon.find("never_surrenders");
    let caught: Vec<Who> = blows
        .iter()
        .filter(|blow| blow.damage > 0)
        .map(|blow| blow.target)
        .filter(|&(n, m)| {
            let slot = roster.listed[n].living[m].slot;
            let traits = &field.columns.words[slot].traits;
            squads.state[roster.listed[n].squad] == ROUTING
                && field.columns.state[slot] < DEAD
                && !stout.is_some_and(|code| traits.iter().any(|&(c, l)| c == code && l > 0.0))
        })
        .collect();
    if caught.is_empty() {
        return vec![];
    }
    let rolled: Vec<Caught> = caught
        .iter()
        .map(|&(n, m)| {
            let unit = &roster.listed[n].living[m];
            let courage = field.columns.sheet[unit.slot].courage as f64;
            Caught { id: unit.id, slot: unit.slot, courage }
        })
        .collect();
    let gives = wounds::surrenders(&field.columns, &rolled, rules.tick, rules.seed, rules.chance);
    let listed: Vec<usize> = roster.listed.iter().map(|l| l.squad).collect();
    let mut taken = vec![];
    for (&(n, m), gives) in caught.iter().zip(gives) {
        let unit = &roster.listed[n].living[m];
        if !gives || field.columns.state[unit.slot] >= DEAD {
            continue;
        }
        field.columns.state[unit.slot] = TAKEN;
        let squad = roster.listed[n].squad;
        log.squad(SURRENDERED, rules.tick, squads, squad, &[unit.id]);
        taken.push((n, m));
        let none = squads.living(squad, &field.columns.state).is_empty();
        if none && squads.state[squad] != DESTROYED {
            squads.state[squad] = DESTROYED;
            locks::release(squads, squad, &listed);
            log.squad(DESTROYED_EVENT, rules.tick, squads, squad, &[]);
        }
    }
    taken
}

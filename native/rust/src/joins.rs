//! The joins (FormationJoins and FormationContact.reinforce; Decisions 44, 51 and 97; spec
//! 39 step 8b5-x): once every squad has moved, each advancing wave that has reached a
//! friend's back rank joins it (`contact::Field::joins`, nearest first). The joining squad's
//! living units move in behind the leader's back rank and its units close up (`take_in`,
//! `compact::reinforce`); they follow the leader's command from then on, the leader's members
//! and joined list take them in, and the joining squad is left with none. The events
//! ("merged" or "reinforced", then "stepped_up" for each unit that stepped up, with its rank)
//! go to the log as FormationEvents made them. GDScript mirrors its own lists and units'
//! copies, and drops the joined squads.

use crate::compact::{self, Closer, Joining};
use crate::contact;
use crate::events::{self, Log};
use crate::field::Field;
use crate::locks::FIGHTING;
use crate::roster::Roster;
use crate::squads::Squads;
use crate::swaps;

/// What a squad's taking in left: the living units it closed up over, by store slot (the
/// leader's, then the joining's, each in its order), and the indices into those of the
/// units that stepped up, in the order each first did.
pub struct Took {
    pub closed: Vec<i32>,
    pub stepped: Vec<usize>,
}

/// FormationJoins.join over `roster`'s squads: each of the listed `movers` that has reached
/// a friend's back rank is taken in by it, by the fight's `seed`. Returns the joins as
/// (leader, joining) listed indices, in the order taken.
pub fn step(
    squads: &mut Squads,
    field: &mut Field,
    roster: &Roster,
    movers: &[usize],
    seed: i64,
    tick: i64,
    log: &mut Log,
) -> Vec<(usize, usize)> {
    if movers.is_empty() {
        return Vec::new();
    }
    let joins = contact::Field { squads, columns: &field.columns, roster }.joins(movers, seed);
    for &(leader, joining) in &joins {
        let (l, j) = (roster.listed[leader].squad, roster.listed[joining].squad);
        let kind = if squads.state[l] == FIGHTING { events::REINFORCED } else { events::MERGED };
        log.squad(kind, tick, squads, j, &[squads.id[l]]);
        let took = take_in(squads, field, l, j);
        for at in took.stepped {
            let slot = took.closed[at] as usize;
            let id = field.unit_at(slot as u32).unwrap_or(0);
            let rank = i64::from(field.columns.rank[slot]);
            log.squad(events::STEPPED_UP, tick, squads, l, &[id, rank]);
        }
        squads.reforming[l] = 1;
    }
    joins
}

/// FormationContact.reinforce: the squad in slot `joining` taken in behind the squad in
/// `leader` (`compact::reinforce`), each read where it lives: all the leader's members shift
/// with its frame, each living unit's footprint and band from its sheet, the leader's moves
/// under way from its swaps. The joining units are enlisted in the leader.
pub fn take_in(squads: &mut Squads, field: &mut Field, leader: usize, joining: usize) -> Took {
    let leading = squads.living(leader, &field.columns.state);
    let taken = squads.living(joining, &field.columns.state);
    let frame: Vec<usize> = squads.members[leader].iter().map(|&u| u as usize).collect();
    let movers = swaps::moving(squads, leader);
    let join = Joining { leader, joining, leader_units: &frame };
    let (ahead, behind) = (closers(field, &leading), closers(field, &taken));
    let stepped = compact::reinforce(squads, &mut field.columns, join, &ahead, &behind, &movers);
    enlist(squads, field, leader, &taken);
    squads.members[joining].clear();
    Took { closed: [leading, taken].concat(), stepped }
}

/// FormationCommand.enlist for each of `taken` (store slots), in order: it marches in the
/// squad in `leader` and follows its command, and starts under it if it started under none;
/// the leader's members and joined list take it in.
fn enlist(squads: &mut Squads, field: &mut Field, leader: usize, taken: &[i32]) {
    let command = squads.command[leader].map_or(-1, |c| c as i64);
    for &unit in taken {
        let u = unit as usize;
        field.columns.sheet[u].squad_id = squads.id[leader];
        let objects = &mut field.columns.objects[u];
        objects.command = command;
        if objects.origin == -1 {
            objects.origin = command;
        }
        squads.members[leader].push(unit);
        squads.joined[leader].push(field.unit_at(unit as u32).unwrap_or(0));
    }
}

/// The `units` (store slots) as a squad closes up over them (SquadCompact.closers).
fn closers(field: &Field, units: &[i32]) -> Vec<Closer> {
    units
        .iter()
        .map(|&unit| {
            let sheet = &field.columns.sheet[unit as usize];
            Closer {
                id: field.unit_at(unit as u32).unwrap_or(0),
                slot: unit as usize,
                foot: (sheet.footprint_width as i32, sheet.footprint_depth as i32),
                preferred: sheet.preferred_position,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests;

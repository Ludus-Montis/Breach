//! The groups as one crate pass (FormationGroups.step, spec 30 round 3 part 4; spec 39 step
//! 8b5-xxx) over the listed squads: the groups at ease on a route whose units fell behind
//! split off (`split::step`), each new group opened in the crate with the battle's next id,
//! as the strays' are, its units taken out of the group they left ("split": from); then the
//! groups at ease meet and form up (`form_up`), each meeting landed in turn: a group taken in
//! ("merged": into, then `joins::take_in`), or units going back to the command they started
//! under ("rejoined": unit). GDScript wraps each slot opened and mirrors its lists.

#[cfg(test)]
mod tests;

use super::{form_up, Follower, Group, Meeting};
use crate::content::CELLS_PER_TILE;
use crate::entries;
use crate::events::{self, Log};
use crate::field::Field;
use crate::joins;
use crate::locks::{ARRIVED, HOLDING, MOVING};
use crate::march::walk::Rule;
use crate::moving::share;
use crate::roster::{Listed, Roster};
use crate::split::{self, Candidate, Rules};
use crate::squads::Squads;
use crate::swaps;
use crate::turn;

/// What the pass did, in the order it did it, by the squads' indices in the lists (the
/// groups split off listed after them, in the order opened).
#[derive(Clone, Debug, PartialEq)]
pub enum Landed {
    /// A group split off the listed `from`, in the squad slot `slot`: its units, indices
    /// into `from`'s living as the roster gave them, in the order they line up.
    Split { from: usize, slot: usize, members: Vec<usize> },
    /// The listed `taken` taken in by the listed `taker`.
    Taken { taker: usize, taken: usize },
    /// The units (ids, in `from`'s order) of the listed `from` gone back to `home`.
    Returned { from: usize, home: usize, units: Vec<i64> },
}

/// FormationGroups.step over `roster`'s squads, each one's route named by its code
/// (`Squads::route_code`: the same code, the same route); `rules` the split's, with
/// group_join for forming up too.
pub fn step(
    squads: &mut Squads,
    field: &mut Field,
    roster: &Roster,
    rules: &Rules,
    tick: i64,
    log: &mut Log,
) -> Vec<Landed> {
    let mut landed = Vec::new();
    let mut listed = roster.listed.clone();
    split_off(squads, field, &mut listed, rules, tick, log, &mut landed);
    meet(squads, field, &listed, rules, tick, log, &mut landed);
    landed
}

/// True if the squad in `s` is at ease: marching, holding or arrived.
fn at_ease(squads: &Squads, s: usize) -> bool {
    [MOVING, HOLDING, ARRIVED].contains(&squads.state[s])
}

/// The ids of each split's units, in the order `landed` has the splits and each one's units,
/// as `roster` (the one the pass was given) lists them.
pub fn split_ids(landed: &[Landed], roster: &Roster) -> Vec<Vec<i64>> {
    let split = landed.iter().filter_map(|done| match done {
        Landed::Split { from, members, .. } => Some((from, members)),
        _ => None,
    });
    split
        .map(|(&from, members)| members.iter().map(|&m| roster.listed[from].living[m].id).collect())
        .collect()
}

/// GroupSplit.step: the groups split off, each opened and listed after the rest.
fn split_off(
    squads: &mut Squads,
    field: &mut Field,
    listed: &mut Vec<Listed>,
    rules: &Rules,
    tick: i64,
    log: &mut Log,
    landed: &mut Vec<Landed>,
) {
    let roster = Roster { listed: listed.clone() };
    let candidates: Vec<Candidate> = (0..listed.len())
        .filter(|&n| at_ease(squads, listed[n].squad) && squads.route[listed[n].squad].is_some())
        .map(|n| candidate(squads, field, &listed[n], n))
        .collect();
    if candidates.is_empty() {
        return;
    }
    for s in split::step(squads, &field.columns, &roster, &candidates, rules) {
        let from = candidates[s.candidate].listed;
        let source = &roster.listed[from];
        let units: Vec<(i64, usize)> =
            s.members.iter().map(|&m| (source.living[m].id, source.living[m].slot)).collect();
        let slot = open(squads, field, source.squad, &units, s.front);
        log.squad(events::SPLIT, tick, squads, slot, &[squads.id[source.squad]]);
        let gone: Vec<usize> = units.iter().map(|&(_, u)| u).collect();
        listed[from].living.retain(|m| !gone.contains(&m.slot));
        let living = s.members.iter().map(|&m| source.living[m].clone()).collect();
        let id = squads.id[slot];
        listed.push(Listed { squad: slot, id, joined: false, living, ..source.clone() });
        landed.push(Landed::Split { from, slot, members: s.members });
    }
}

/// The listed squad `n` as the split reads it: whether its frame waits for every unit, its
/// route, and each living unit's taking and how far through a move it is.
fn candidate(squads: &Squads, field: &Field, listed: &Listed, n: usize) -> Candidate {
    let s = listed.squad;
    let offsets = swaps::offsets(squads, s);
    let offset = |id: i64| {
        let held = offsets.iter().find(|&&(at, _, _)| at == id);
        held.map_or(crate::maths::V2::ZERO, |&(_, r, c)| crate::maths::V2::new(r, c))
    };
    Candidate {
        listed: n,
        waits: share::rule(field, &listed.slots()) == Rule::NoManLeft,
        route: squads.route[s].clone().expect("a candidate is on a route"),
        taking: listed
            .living
            .iter()
            .map(|m| squads.taking[s].iter().any(|t| t.0 == m.id))
            .collect(),
        offsets: listed.living.iter().map(|m| offset(m.id)).collect(),
    }
}

/// GroupSplit._group: the `units` ([id, store slot], in the order they line up) out of the
/// squad in `own` and into a group the crate opens with the battle's next id, of the same
/// faction, way, home, command, route, heading, order, state and morale, its front `front`
/// tiles along the route. Returns the slot opened.
fn open(
    squads: &mut Squads,
    field: &mut Field,
    own: usize,
    units: &[(i64, usize)],
    front: f64,
) -> usize {
    for (k, &unit) in units.iter().enumerate() {
        squads.members[own].retain(|&u| u as usize != unit.1);
        entries::unloose(squads, &mut field.columns, own, unit);
        entries::unflee(squads, &mut field.columns, own, unit);
        field.columns.rank[unit.1] = 0;
        field.columns.column[unit.1] = k as i32;
    }
    let id = squads.issue_squad();
    let group = squads.open(id) as usize;
    squads.faction[group] = squads.faction[own];
    squads.direction[group] = squads.direction[own];
    squads.home_distance[group] = squads.home_distance[own];
    squads.width[group] = (units.len() as i32).max(1);
    squads.command[group] = squads.command[own];
    for &(_, slot) in units {
        joins::enlist_one(squads, field, group, slot as i32);
    }
    let route = squads.route[own].clone();
    let home = squads.home_distance[group] * CELLS_PER_TILE as f64;
    squads.position[group] = turn::place(route.as_ref(), home);
    squads.route[group] = route;
    squads.route_code[group] = squads.route_code[own];
    squads.front_distance[group] = front;
    squads.heading[group] = squads.heading[own];
    squads.order[group] = squads.order[own];
    squads.state[group] = squads.state[own];
    squads.morale[group] = squads.morale[own];
    group
}

/// FormationGroups._form_up over the `listed` groups at ease with living units: each meeting
/// landed in the order given.
fn meet(
    squads: &mut Squads,
    field: &mut Field,
    listed: &[Listed],
    rules: &Rules,
    tick: i64,
    log: &mut Log,
    landed: &mut Vec<Landed>,
) {
    let at: Vec<usize> = (0..listed.len())
        .filter(|&n| at_ease(squads, listed[n].squad) && !listed[n].living.is_empty())
        .collect();
    if at.len() < 2 {
        return;
    }
    let roster = Roster { listed: at.iter().map(|&n| listed[n].clone()).collect() };
    let groups: Vec<Group> = at.iter().map(|&n| group(squads, field, &listed[n])).collect();
    let meetings = form_up(squads, &field.columns, &roster, &groups, rules.join, rules.seed);
    for meeting in meetings {
        landed.push(match meeting {
            Meeting::Taken { taker, taken } => {
                let (taker, taken) = (at[taker], at[taken]);
                take_in(squads, field, (listed[taker].squad, listed[taken].squad), tick, log);
                Landed::Taken { taker, taken }
            }
            Meeting::Returned { from, home, units } => {
                let (from, home) = (at[from], at[home]);
                let back = (listed[from].squad, listed[home].squad);
                let units = return_origins(squads, field, back, &units, tick, log);
                Landed::Returned { from, home, units }
            }
        });
    }
}

/// The listed group as forming up reads it: its command and route, and each living unit's
/// leading, its "joins" and "gathers" traits and the command it started under.
fn group(squads: &Squads, field: &Field, listed: &Listed) -> Group {
    let columns = &field.columns;
    let has =
        |slot: usize, name: &str| field.lexicon.level(&columns.words[slot].traits, name).is_some();
    let followers = listed
        .living
        .iter()
        .map(|m| Follower {
            leads: columns.sheet[m.slot].leadership > 0,
            joins: has(m.slot, "joins"),
            gathers: has(m.slot, "gathers"),
            origin: u32::try_from(columns.objects[m.slot].origin).ok(),
        })
        .collect();
    let command = squads.command[listed.squad].map_or(u32::MAX, |c| c as u32);
    Group { command, route: squads.route_code[listed.squad] as u32, followers }
}

/// FormationGroups._take_in: the squad in `taken` taken in behind the one in `taker`.
fn take_in(
    squads: &mut Squads,
    field: &mut Field,
    (taker, taken): (usize, usize),
    tick: i64,
    log: &mut Log,
) {
    log.squad(events::MERGED, tick, squads, taken, &[squads.id[taker]]);
    for unit in squads.living(taken, &field.columns.state) {
        let id = field.unit_at(unit as u32).unwrap_or(0);
        entries::unloose(squads, &mut field.columns, taken, (id, unit as usize));
    }
    joins::take_in(squads, field, taker, taken);
    squads.reforming[taker] = 1;
}

/// FormationGroups._return_origins: the living units of the squad in `from` with these `ids`
/// go back to the squad in `home`, each behind its back row. Returns their ids in `from`'s
/// order.
fn return_origins(
    squads: &mut Squads,
    field: &mut Field,
    (from, home): (usize, usize),
    ids: &[i64],
    tick: i64,
    log: &mut Log,
) -> Vec<i64> {
    let living = squads.living(from, &field.columns.state);
    let back: Vec<(i64, usize)> = living
        .iter()
        .map(|&u| (field.unit_at(u as u32).unwrap_or(0), u as usize))
        .filter(|(id, _)| ids.contains(id))
        .collect();
    for &unit in &back {
        squads.members[from].retain(|&u| u as usize != unit.1);
        entries::unloose(squads, &mut field.columns, from, unit);
        field.columns.rank[unit.1] = back_row(squads, field, home);
        joins::enlist_one(squads, field, home, unit.1 as i32);
        log.squad(events::REJOINED, tick, squads, home, &[unit.0]);
    }
    squads.reforming[home] = 1;
    back.into_iter().map(|(id, _)| id).collect()
}

/// FormationGroups._back_row: the row behind the squad in `s`'s living units.
fn back_row(squads: &Squads, field: &Field, s: usize) -> i32 {
    let columns = &field.columns;
    let living = squads.living(s, &columns.state);
    let rows = living
        .iter()
        .map(|&u| columns.rank[u as usize] as i64 + columns.sheet[u as usize].footprint_depth);
    rows.max().unwrap_or(0).max(0) as i32
}

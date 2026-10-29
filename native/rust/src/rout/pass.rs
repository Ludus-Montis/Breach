//! The rout as one crate pass (FormationRout.step, Decision 82, spec 39 step 8b5-xxviii):
//! the listed squads at 0 morale are found before any breaks; every router flees (`flee`) and
//! what the flight did lands in its order - a crush hurts both units by the router's size
//! ("crushed" on the friend: unit, target, dmg; on the fell queue if it fell) and the first
//! panics that friend; a router home leaves its squad's members ("fled_home": unit,
//! definition); a squad left with none "scattered". Then, if any squad was routing as the
//! tick began, the rallies are decided (`rallies`; a band first read sets that squad's morale
//! to its ceiling) and each router joins its friend, nearest first: out of its squad, alone in
//! a single squad the crate opens for it (with its own command, as FormationRout's was), taken
//! in behind the friend (`joins::take_in`), loose where it stood, the friend re-forming
//! ("rallied": unit, into); a squad left with no one standing is DESTROYED. Then the routs
//! with a leader and no enemy near re-form ("reformed"). Last the breaking squads rout
//! (`breaks`): each drops its staging and swaps ("routed"), and each friend that saw it break
//! takes the shock. The squads' living are read again after routers leave or join, as
//! GDScript read them (CrateRoster.of). GDScript mirrors its lists.

#[cfg(test)]
mod tests;

use std::collections::BTreeMap;

use super::{breaking, breaks, flee, rallies, regroup, Fled, RoutTuning, CELLS};
use crate::bands::{self, BandTuning};
use crate::entries;
use crate::events::{self, Log};
use crate::field::Field;
use crate::ground::Ground;
use crate::joins;
use crate::locks;
use crate::roster::{Member, Roster};
use crate::rout_blows::{where_on, Routing};
use crate::shocks;
use crate::squads::Squads;
use crate::turn;
use crate::wounds;

/// BattleTuning's rout shocks and hurt: rout_crush (a crush's damage a cell of the router's
/// footprint), rout_panic (a crushed friend's shock) and rout_seen (a break's, to each friend
/// that sees it).
#[derive(Clone, Copy, Debug, Default)]
pub struct RoutShocks {
    pub crush: i64,
    pub panic: i64,
    pub seen: i64,
}

/// A router that left its squad, as GDScript mirrors it: its squad's index in the lists, its
/// id, and the index of the friend it joined (None: it reached home).
#[derive(Debug, PartialEq)]
pub struct Moved {
    pub listed: usize,
    pub id: i64,
    pub into: Option<usize>,
}

/// One tick of routs over `roster`'s squads on `tick`, ties to the fight's `seed`. Returns
/// the routers that left their squads, in the order they did.
#[allow(clippy::too_many_arguments)]
pub fn step(
    squads: &mut Squads,
    field: &mut Field,
    roster: &Roster,
    (t, hurt, bands): (&RoutTuning, &RoutShocks, &BandTuning),
    (tick, seed): (i64, i64),
    ground: Option<&Ground>,
    log: &mut Log,
) -> Vec<Moved> {
    let routing: Vec<Routing> = roster
        .listed
        .iter()
        .enumerate()
        .filter(|(_, l)| squads.state[l.squad] == locks::ROUTING)
        .map(|(n, l)| Routing { listed: n, route: squads.route[l.squad].clone() })
        .collect();
    let leaders = leaders_of(field, roster);
    let broke = breaking(squads, roster);
    let fled = flee(squads, &mut field.columns, roster, &routing, &leaders, t, seed, ground);
    let mut moved = Vec::new();
    for f in fled {
        land_flight(squads, field, (roster, &routing), f, (hurt, bands, tick), log, &mut moved);
    }
    let mut roster = refreshed(roster, squads, field);
    if !routing.is_empty() {
        rally(squads, field, (&roster, &routing), (t, bands), (tick, seed), log, &mut moved);
        roster = refreshed(&roster, squads, field);
        let leaders = leaders_of(field, &roster);
        let slots: BTreeMap<i64, Option<usize>> = routing
            .iter()
            .flat_map(|r| squads.fleeing[roster.listed[r.listed].squad].clone())
            .map(|id| (id, field.slot_of(id).map(|s| s as usize)))
            .collect();
        let slot_of = |id: i64| slots.get(&id).copied().flatten();
        let columns = &mut field.columns;
        for r in regroup(squads, columns, &roster, &routing, &leaders, t, seed, slot_of) {
            log.squad(events::REFORMED, tick, squads, roster.listed[routing[r].listed].squad, &[]);
        }
    }
    if !broke.is_empty() {
        let breaking: Vec<Routing> = broke
            .iter()
            .map(|&n| Routing { listed: n, route: squads.route[roster.listed[n].squad].clone() })
            .collect();
        let seen = breaks(squads, &mut field.columns, &roster, &breaking, ground);
        for (formation, friends) in breaking.iter().zip(seen) {
            let s = roster.listed[formation.listed].squad;
            squads.staging[s] = None;
            squads.swaps[s].clear();
            log.squad(events::ROUTED, tick, squads, s, &[]);
            for friend in friends {
                shock(squads, field, roster.listed[friend].squad, hurt.seen, (bands, tick), log);
            }
        }
    }
    moved
}

/// Lands one thing the flight did (FormationRout._land_flight). `routing` indexes `roster`.
fn land_flight(
    squads: &mut Squads,
    field: &mut Field,
    (roster, routing): (&Roster, &[Routing]),
    fled: Fled,
    (hurt, bands, tick): (&RoutShocks, &BandTuning, i64),
    log: &mut Log,
    moved: &mut Vec<Moved>,
) {
    match fled {
        Fled::Crushed { routing: r, member, hit, panic } => {
            let router = &roster.listed[routing[r].listed].living[member];
            let friend = &roster.listed[hit.0];
            let target = &friend.living[hit.1];
            let damage = hurt.crush * router.foot.0 * router.foot.1;
            let mut fell = Vec::new();
            wounds::hurt(&mut field.columns, router.slot, router.id, damage, &mut fell);
            wounds::hurt(&mut field.columns, target.slot, target.id, damage, &mut fell);
            for id in fell {
                field.fell(id);
            }
            log.squad(events::CRUSHED, tick, squads, friend.squad, &[router.id, target.id, damage]);
            if panic {
                shock(squads, field, friend.squad, hurt.panic, (bands, tick), log);
            }
        }
        Fled::Home { routing: r, member } => {
            let listed = routing[r].listed;
            let (s, unit) = (roster.listed[listed].squad, &roster.listed[listed].living[member]);
            squads.members[s].retain(|&u| u as usize != unit.slot);
            let definition = field.columns.objects[unit.slot].definition;
            log.squad(events::FLED_HOME, tick, squads, s, &[unit.id, definition]);
            moved.push(Moved { listed, id: unit.id, into: None });
        }
        Fled::Scattered { routing: r } => {
            let s = roster.listed[routing[r].listed].squad;
            log.squad(events::SCATTERED, tick, squads, s, &[]);
        }
    }
}

/// The rallies decided (`rallies`) and made, nearest first (FormationRout._rally).
fn rally(
    squads: &mut Squads,
    field: &mut Field,
    (roster, routing): (&Roster, &[Routing]),
    (t, bands): (&RoutTuning, &BandTuning),
    (tick, seed): (i64, i64),
    log: &mut Log,
    moved: &mut Vec<Moved>,
) {
    let leaders = leaders_of(field, roster);
    let morale: Vec<(i64, i64)> = roster
        .listed
        .iter()
        .map(|l| {
            let now = i64::from(squads.morale[l.squad]);
            (now, if now < 0 { ceiling(field, &l.living, bands) } else { 0 })
        })
        .collect();
    let columns = &mut field.columns;
    let rallied = rallies(squads, columns, roster, routing, &leaders, &morale, t, seed);
    for n in rallied.first_read {
        squads.morale[roster.listed[n].squad] = morale[n].1 as i32;
    }
    for (r, member, friend) in rallied.joins {
        let listed = routing[r].listed;
        let router = &roster.listed[listed].living[member];
        let into = roster.listed[friend].squad;
        join(squads, field, (roster.listed[listed].squad, routing[r].route.as_ref()), router, into);
        let (s, id) = (roster.listed[listed].squad, router.id);
        log.squad(events::RALLIED, tick, squads, s, &[id, squads.id[into]]);
        if squads.living(s, &field.columns.state).is_empty() {
            squads.state[s] = locks::DESTROYED;
        }
        moved.push(Moved { listed, id, into: Some(friend) });
    }
}

/// FormationRout._join: the router leaves its squad in slot `s` (on `route`), alone in a
/// single squad of its faction with its own command, which the squad in slot `into` takes in
/// behind its back rank; it is loose where it stood, and that squad re-forms.
fn join(
    squads: &mut Squads,
    field: &mut Field,
    (s, route): (usize, Option<&crate::route::Route>),
    router: &Member,
    into: usize,
) {
    let (id, slot) = (router.id, router.slot);
    let columns = &mut field.columns;
    let settled = where_on(route, columns.along[slot], columns.flee_offset[slot]);
    entries::unflee(squads, columns, s, (id, slot));
    squads.members[s].retain(|&u| u as usize != slot);
    columns.rank[slot] = 0;
    columns.column[slot] = 0;
    let single = squads.open(-1) as usize;
    squads.faction[single] = squads.faction[s];
    squads.direction[single] = squads.direction[into];
    squads.width[single] = 1;
    squads.command[single] = Some(squads.commands.open(0.0));
    joins::enlist_one(squads, field, single, slot as i32);
    let front = squads.front_distance[into];
    joins::take_in(squads, field, into, single);
    if squads.front_distance[into] != front {
        let cells = squads.front_distance[into] * CELLS;
        squads.position[into] = turn::place(squads.route[into].as_ref(), cells);
    }
    squads.reforming[into] = 1;
    entries::loosen(squads, &mut field.columns, into, (id, slot), settled);
}

/// Drains the morale of the squad in slot `s` by `amount`, read over who stands in it now
/// (FormationMorale.shock).
fn shock(
    squads: &mut Squads,
    field: &Field,
    s: usize,
    amount: i64,
    (bands, tick): (&BandTuning, i64),
    log: &mut Log,
) {
    let living: Vec<usize> =
        squads.living(s, &field.columns.state).into_iter().map(|u| u as usize).collect();
    shocks::shock_living((squads, &field.columns), (s, &living), amount, (bands, tick), log);
}

/// The ceiling of a squad whose living are `living` (FormationMorale.ceiling).
fn ceiling(field: &Field, living: &[Member], bands: &BandTuning) -> i64 {
    let sheets = living.iter().map(|m| &field.columns.sheet[m.slot]);
    let courage: Vec<i64> = sheets.clone().map(|sheet| sheet.courage).collect();
    let leaderships: Vec<i64> = sheets.map(|sheet| sheet.leadership).collect();
    bands::ceiling(&courage, &leaderships, bands)
}

/// Every listed squad's living units' leadership, in the roster's order.
fn leaders_of(field: &Field, roster: &Roster) -> Vec<i64> {
    let living = roster.listed.iter().flat_map(|l| l.living.iter());
    living.map(|m| field.columns.sheet[m.slot].leadership).collect()
}

/// `roster` read again: each listed squad's living as its members stand now, each unit as
/// the roster knew it (a router that joined a friend is that friend's now), and whether units
/// have joined it.
fn refreshed(roster: &Roster, squads: &Squads, field: &Field) -> Roster {
    let known: BTreeMap<usize, &Member> =
        roster.listed.iter().flat_map(|l| l.living.iter().map(|m| (m.slot, m))).collect();
    let mut out = roster.clone();
    for listed in &mut out.listed {
        let living = squads.living(listed.squad, &field.columns.state);
        listed.living =
            living.iter().filter_map(|u| known.get(&(*u as usize))).map(|m| (*m).clone()).collect();
        listed.joined = !squads.joined[listed.squad].is_empty();
    }
    out
}

//! The burial (FormationDeaths.bury, Decisions 47, 82, 121 and 145; spec 39 step 8b5-xxii):
//! the fell queue is taken, and each unit on it still standing at 0 hp or below is buried
//! with the squad it is in now, the squads in the lists' order and each one's fallen in its
//! own order (never the order they fell; one that fell twice is taken once). Each dies or is
//! downed (`deaths::fate`), DEAD or DOWNED in the STATE column, a downed unit's wounds on
//! its sheet, "died" or "downed" logged. Then, a squad at a time: "leader_fell" for each of
//! its fallen leaders and the losses' shock to its morale (`bands::losses`,
//! `shocks::shock_living`); its ranks close up (`compact::compact`, its front's place moved
//! with it), "stepped_up" for each unit that stepped up, with its rank; it re-forms; and,
//! left with no one standing, it is DESTROYED, released (`locks::release`) and "destroyed"
//! logged. GDScript mirrors the units' states and wounds.

use std::collections::HashSet;

use crate::bands::{self, BandTuning};
use crate::compact;
use crate::events::{self, Log};
use crate::field::Field;
use crate::geometry::CELLS;
use crate::joins;
use crate::locks::{self, DESTROYED};
use crate::squads::{Squads, DEAD, DOWNED};
use crate::swaps;
use crate::turn;

/// A constitution not set (UnitDef.AVERAGE).
const AVERAGE: f64 = 10.0;

/// A fallen unit buried: the index in the lists of the squad it lay in, its id, and the
/// wounds it bears now if downed (`None`: it died).
pub struct Buried {
    pub listed: usize,
    pub id: i64,
    pub wounded: Option<i64>,
}

/// The burial over the squads in `listed` (squad slots, in the lists' order), on `tick`.
/// Returns the units buried, in the order buried.
pub fn bury(
    squads: &mut Squads,
    field: &mut Field,
    listed: &[usize],
    (t, tick): (&BandTuning, i64),
    log: &mut Log,
) -> Vec<Buried> {
    let found = fallen(squads, field, listed);
    let constitution = field.lexicon.find("constitution");
    let each: Vec<(usize, f64)> = found
        .iter()
        .flat_map(|(_, slots)| slots.iter())
        .map(|&slot| {
            let said = &field.columns.words[slot].attributes;
            let level = said.iter().find(|&&(word, _)| Some(word) == constitution);
            (slot, level.map_or(AVERAGE, |&(_, level)| level))
        })
        .collect();
    let mut fates = super::fate(&mut field.columns, &each).into_iter();
    let mut buried = vec![];
    for (n, slots) in &found {
        let s = listed[*n];
        for &slot in slots {
            let id = field.unit_at(slot as u32).unwrap_or(0);
            let wounded = fates.next().flatten().map(|wounds| {
                field.columns.sheet[slot].wounded += wounds; // lowering its condition
                field.columns.sheet[slot].wounded
            });
            field.columns.state[slot] = if wounded.is_some() { DOWNED } else { DEAD };
            let kind = if wounded.is_some() { events::DOWNED } else { events::DIED };
            log.squad(kind, tick, squads, s, &[id]);
            buried.push(Buried { listed: *n, id, wounded });
        }
        losses(squads, field, (s, slots), (t, tick), log);
        close_up(squads, field, s, tick, log);
        squads.reforming[s] = 1; // front units may close gaps (Decision 49)
        let none = squads.living(s, &field.columns.state).is_empty();
        if none && squads.state[s] != DESTROYED {
            squads.state[s] = DESTROYED;
            locks::release(squads, s, listed);
            log.squad(events::DESTROYED, tick, squads, s, &[]);
        }
    }
    buried
}

/// The fell queue's units still standing at 0 hp or below, by the index in `listed` of the
/// squad each is in now, ascending, each squad's store slots in its own order.
fn fallen(squads: &Squads, field: &mut Field, listed: &[usize]) -> Vec<(usize, Vec<usize>)> {
    let mut seen = HashSet::new();
    let mut found: Vec<(usize, usize, usize)> = vec![]; // (listed index, index in it, slot)
    for id in field.take_fallen() {
        if !seen.insert(id) {
            continue;
        }
        let Some(slot) = field.slot_of(id).map(|slot| slot as usize) else { continue };
        if field.columns.state[slot] >= DEAD || field.columns.hp[slot] > 0 {
            continue;
        }
        if let Some((n, at)) = place(squads, field, listed, slot) {
            found.push((n, at, slot));
        }
    }
    found.sort_unstable();
    let mut out: Vec<(usize, Vec<usize>)> = vec![];
    for (n, _, slot) in found {
        match out.last_mut() {
            Some((last, slots)) if *last == n => slots.push(slot),
            _ => out.push((n, vec![slot])),
        }
    }
    out
}

/// [the index in `listed` of the squad the unit in `slot` is in now, its index in that
/// squad's members]: its own squad's id is looked at first; a unit moved since (joined,
/// reinforced, split off) is found by looking through the rest.
fn place(squads: &Squads, field: &Field, listed: &[usize], slot: usize) -> Option<(usize, usize)> {
    let at = |n: usize| squads.members[listed[n]].iter().position(|&u| u as usize == slot);
    let own = field.columns.sheet[slot].squad_id;
    let first = listed.iter().rposition(|&s| squads.id[s] == own);
    first
        .and_then(|n| at(n).map(|m| (n, m)))
        .or_else(|| (0..listed.len()).find_map(|n| at(n).map(|m| (n, m))))
}

/// FormationMorale.losses: "leader_fell" for each of the squad's fallen (`slots`) that led,
/// then their shock to its morale.
fn losses(
    squads: &mut Squads,
    field: &Field,
    (s, slots): (usize, &[usize]),
    (t, tick): (&BandTuning, i64),
    log: &mut Log,
) {
    let leaderships: Vec<i64> =
        slots.iter().map(|&slot| field.columns.sheet[slot].leadership).collect();
    for (&slot, &leadership) in slots.iter().zip(&leaderships) {
        if leadership > 0 {
            let id = field.unit_at(slot as u32).unwrap_or(0);
            log.squad(events::LEADER_FELL, tick, squads, s, &[id, leadership]);
        }
    }
    let living: Vec<usize> =
        squads.living(s, &field.columns.state).iter().map(|&u| u as usize).collect();
    let amount = bands::losses(&leaderships, t);
    crate::shocks::shock_living((squads, &field.columns), (s, &living), amount, (t, tick), log);
}

/// SquadCompact.compact over the squad's living units, round its moves under way; its place
/// follows its front if that moved (SquadCompact.placed); "stepped_up" for each that did.
fn close_up(squads: &mut Squads, field: &mut Field, s: usize, tick: i64, log: &mut Log) {
    let living = joins::closers(field, &squads.living(s, &field.columns.state));
    let movers = swaps::moving(squads, s);
    let front = squads.front_distance[s];
    let stepped = compact::compact(squads, &mut field.columns, s, &living, &movers);
    if squads.front_distance[s] != front {
        squads.position[s] =
            turn::place(squads.route[s].as_ref(), squads.front_distance[s] * CELLS);
    }
    for at in stepped {
        let rank = i64::from(field.columns.rank[living[at].slot]);
        log.squad(events::STEPPED_UP, tick, squads, s, &[living[at].id, rank]);
    }
}

#[cfg(test)]
mod tests;

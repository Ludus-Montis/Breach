//! FormationShuffle.step as one pass (spec 39 step 8b5-xxxii): for each listed squad, in the
//! lists' order, its swaps under way end (`end_held`: cancelled, counted down or ended, the
//! squad widened and closed up, its place following its front if that moved), "swapped"
//! (unit, passed) and "stepped_up" (unit, rank) logged for each that ended; then a squad
//! re-forming starts every move it can (`march::shuffle::start_moves`, each unit's place,
//! footprint, band, priority, pace and whether it joined read here) and holds it (`begin`),
//! and stops re-forming once none starts and none is under way.

#[cfg(test)]
mod tests;

use std::collections::{HashMap, HashSet};

use crate::events::{self, Log};
use crate::field::Field;
use crate::geometry::CELLS;
use crate::joins;
use crate::locks::FIGHTING;
use crate::march::front::Cell;
use crate::march::shuffle::{self, Ranked};
use crate::squads::Squads;
use crate::turn;

/// One tick of the listed squads' re-forming (their slots, in the lists' order) on `tick`:
/// `pace` is [tick seconds, cells a second at speed 1, scrum_crowding].
pub fn step(
    squads: &mut Squads,
    field: &mut Field,
    listed: &[usize],
    (pace, tick): ([f64; 3], i64),
    log: &mut Log,
) {
    for &s in listed {
        if !squads.swaps[s].is_empty() {
            end(squads, field, s, tick, log);
        }
        if squads.reforming[s] != 0 && !start(squads, field, s, pace) {
            squads.reforming[s] = i32::from(!squads.swaps[s].is_empty());
        }
        if squads.reforming[s] == 0 {
            field.reforms.tried.remove(&s);
        }
    }
}

/// FormationShuffle._end_swaps: the squad's swaps under way end, its place following its
/// front if that moved (SquadCompact.placed); each that ended told, then its step-ups.
fn end(squads: &mut Squads, field: &mut Field, s: usize, tick: i64, log: &mut Log) {
    let living = joins::closers(field, &squads.living(s, &field.columns.state));
    let units: Vec<usize> = squads.members[s].iter().map(|&u| u as usize).collect();
    let front = squads.front_distance[s];
    let ended = super::end_held(squads, &mut field.columns, s, &living, &units);
    if squads.front_distance[s] != front {
        squads.position[s] =
            turn::place(squads.route[s].as_ref(), squads.front_distance[s] * CELLS);
    }
    for one in ended {
        log.squad_listed(events::SWAPPED, (tick, squads, s), &[one.mover], &one.passed);
        for (n, rank) in one.stepped {
            log.squad(events::STEPPED_UP, tick, squads, s, &[living[n].id, i64::from(rank)]);
        }
    }
}

/// FormationShuffle._start_moves: every move the squad can start round those under way,
/// held. True if any started.
fn start(squads: &mut Squads, field: &mut Field, s: usize, pace: [f64; 3]) -> bool {
    let slots: Vec<usize> =
        squads.living(s, &field.columns.state).into_iter().map(|u| u as usize).collect();
    let living: Vec<(i64, usize)> =
        slots.iter().map(|&slot| (field.unit_at(slot as u32).unwrap_or(0), slot)).collect();
    let joined: HashSet<i64> = squads.joined[s].iter().copied().collect(); // not searched a unit
    let ranked: Vec<Ranked> =
        living.iter().map(|&(id, slot)| ranked(field, &joined, (id, slot))).collect();
    let (claimed, busy) = super::claims(squads, s, &slots);
    let shape = shuffle::Squad {
        width: squads.width[s],
        combat_width: squads.combat_width[s],
        centre_shift: squads.centre_shift[s],
        fighting: squads.state[s] == FIGHTING,
    };
    let read = Read { shape, ranked, claimed, busy, pace };
    if field.reforms.tried.get(&s) == Some(&read) {
        return false; // it read the same and started nothing then
    }
    field.reforms.planned += 1;
    let Read { ranked, .. } = &read;
    let started = shuffle::start_moves(&shape, ranked, &read.claimed, &read.busy, pace);
    if started.is_empty() {
        field.reforms.tried.insert(s, read);
        return false;
    }
    field.reforms.tried.remove(&s);
    let ranked = read.ranked;
    let places: Vec<(i32, i32)> = ranked.iter().map(|r| r.place).collect();
    let feet: Vec<(i32, i32)> =
        ranked.iter().map(|r| (r.footprint_width, r.footprint_depth)).collect();
    super::begin(squads, s, &started, &living, &places, &feet);
    true
}

/// What a re-form's planning reads (`shuffle::start_moves`' inputs): the plan is a function
/// of these alone. Two are the same only if every number is the same to the bit.
struct Read {
    shape: shuffle::Squad,
    ranked: Vec<Ranked>,
    claimed: Vec<(Cell, i32, i32)>,
    busy: Vec<usize>,
    pace: [f64; 3],
}

/// The re-forms that planned and started nothing, by squad slot, with what they read (Q24's
/// re-form occupancy, spec 39 step 10b-ii). A squad that reads exactly the same again would
/// start nothing again, so it isn't planned: its occupancy and plan are worked out only when
/// a swap ends, a unit falls or anything else it reads changes. Not part of the state: a
/// field read back from a snapshot plans afresh, to the same end.
#[derive(Default)]
pub struct Reforms {
    tried: HashMap<usize, Read>,
    planned: usize,
}

impl PartialEq for Read {
    fn eq(&self, other: &Read) -> bool {
        let shape =
            |q: &shuffle::Squad| (q.width, q.combat_width, q.centre_shift.to_bits(), q.fighting);
        let unit = |r: &Ranked| {
            let band = (r.preferred_position, r.position_priority);
            (r.place, r.footprint_width, r.footprint_depth, band, r.speed.to_bits(), r.joined)
        };
        shape(&self.shape) == shape(&other.shape)
            && self.ranked.len() == other.ranked.len()
            && self.ranked.iter().zip(&other.ranked).all(|(a, b)| unit(a) == unit(b))
            && self.claimed == other.claimed
            && self.busy == other.busy
            && self.pace.map(f64::to_bits) == other.pace.map(f64::to_bits)
    }
}

impl Reforms {
    /// How many re-forms were planned (for the scenarios).
    pub fn planned(&self) -> usize {
        self.planned
    }
}

/// A living unit as `march::shuffle` takes it, read from its columns and sheet.
fn ranked(field: &Field, joined: &HashSet<i64>, (id, slot): (i64, usize)) -> Ranked {
    let (columns, sheet) = (&field.columns, &field.columns.sheet[slot]);
    Ranked {
        place: (columns.rank[slot], columns.column[slot]),
        footprint_width: sheet.footprint_width as i32,
        footprint_depth: sheet.footprint_depth as i32,
        preferred_position: sheet.preferred_position as i32,
        position_priority: sheet.position_priority as i32,
        speed: columns.speed[slot],
        joined: joined.contains(&id),
    }
}

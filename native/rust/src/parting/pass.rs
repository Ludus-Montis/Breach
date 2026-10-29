//! UnitBodies.step as one pass (spec 39 step 8b5-xxxi): the field brought to the listed
//! squads' living units as the bodies stand (BodyFieldSync.bodies: a router's base its point
//! on its squad's route), each unit new to it enrolled from its sheet, the bodies parted
//! (`field_parting`), those pushed out of their frames made loose where they stand, and the
//! moved bodies written into the columns.

#[cfg(test)]
mod tests;

use std::f64::consts::TAU;

use super::Tuning;
use crate::contest;
use crate::entries;
use crate::field::{self, Enrol, Field, SyncIn};
use crate::field_parting::{self, FieldOutcome};
use crate::ghash::{self, Key};
use crate::locks::{DESTROYED, ROUTING};
use crate::maths::{rotated, V2};
use crate::reach;
use crate::rout_blows;
use crate::squads::Squads;

/// A synced unit: its squad's slot, its id and its store slot, by the sync's flat index.
type Synced = (usize, i64, usize);

/// Parts the bodies of the squads in slots `listed`, in the lists' order, on the battle's
/// `seed`. Returns what the parting did.
pub fn step(
    squads: &mut Squads,
    field: &mut Field,
    listed: &[usize],
    seed: i64,
    (tuning, threaded): (&Tuning, bool),
) -> FieldOutcome {
    let synced = sync(squads, field, listed, seed);
    let mut way = |draw: i64, other: i64| way(seed, draw, other);
    let out = field_parting::part_field(field, tuning, threaded, &mut way);
    for &flat in &out.loosened {
        let (s, id, slot) = synced[flat as usize];
        let at = field.columns.position[slot];
        entries::loosen(squads, &mut field.columns, s, (id, slot), at);
    }
    field_parting::commit(field, &out.moved, &out.points, &out.nexts);
    out
}

/// UnitBodies.part_way: the way two bodies lying exactly on each other part, seeded by the
/// battle and their draws.
pub fn way(seed: i64, draw: i64, other: i64) -> V2 {
    let angle = TAU * ghash::uniform(seed, &[Key::Int(draw), Key::Int(other), Key::Str("part")]);
    rotated(V2::new(1.0, 0.0), angle)
}

/// BodyFieldSync.bodies: the field brought to the listed squads' living units.
fn sync(squads: &Squads, field: &mut Field, listed: &[usize], seed: i64) -> Vec<Synced> {
    let (mut ids, mut factions, mut flags, mut counts) = (vec![], vec![], vec![], vec![]);
    let (mut synced, mut bases) = (vec![], vec![]);
    for &s in listed {
        let living = squads.living(s, &field.columns.state);
        ids.push(squads.id[s]);
        factions.push(format!("#{}", squads.faction[s])); // the field tells factions by name
        flags.push(match squads.state[s] {
            ROUTING => field::ROUTING,
            DESTROYED => field::DESTROYED,
            _ => 0,
        });
        counts.push(living.len() as i32);
        for u in living {
            let slot = u as usize;
            let id = field.unit_at(u as u32).unwrap_or(0);
            let fled = squads.fleeing[s].contains(&id); // its base: its point on the route
            let along = field.columns.along[slot];
            let on = |route| rout_blows::where_on(route, along, V2::ZERO);
            bases.push(if fled { on(squads.route[s].as_ref()) } else { V2::ZERO });
            synced.push((s, id, slot));
        }
    }
    let unit_ids: Vec<i64> = synced.iter().map(|&(_, id, _)| id).collect();
    let input = SyncIn {
        squad_ids: &ids,
        squad_factions: &factions,
        squad_flags: &flags,
        squad_counts: &counts,
        unit_ids: &unit_ids,
        mode: field::BODIES,
        bases: &bases,
    };
    let unknown = field.sync(seed, &input);
    if !unknown.is_empty() {
        enrol(field, &unknown, &synced, seed);
    }
    synced
}

/// BodyFieldSync._enrol: what the field needs once about each new unit, from its sheet.
fn enrol(field: &mut Field, flat: &[i32], synced: &[Synced], seed: i64) {
    let new: Vec<(i64, usize)> =
        flat.iter().map(|&f| synced[f as usize]).map(|(_, id, slot)| (id, slot)).collect();
    let sheet = |slot: usize| &field.columns.sheet[slot];
    let foot = |slot: usize| (sheet(slot).footprint_width, sheet(slot).footprint_depth);
    let radii: Vec<f64> = new.iter().map(|&(_, u)| reach::radius(foot(u).0, foot(u).1)).collect();
    let areas: Vec<f64> = new.iter().map(|&(_, u)| (foot(u).0 * foot(u).1) as f64).collect();
    let draws: Vec<i64> = new.iter().map(|&(id, _)| contest::draw(seed, id)).collect();
    let initiatives: Vec<i64> = new.iter().map(|&(_, u)| sheet(u).initiative).collect();
    let records: Vec<i64> = new
        .iter()
        .flat_map(|&(_, u)| [sheet(u).preferred_position, foot(u).0, foot(u).1])
        .collect();
    let paces: Vec<f64> =
        new.iter().flat_map(|&(_, u)| [sheet(u).turn_rate, sheet(u).backward_pace]).collect();
    field.enrol(&Enrol {
        flat,
        radii: &radii,
        areas: &areas,
        draws: &draws,
        initiatives: &initiatives,
        records: &records,
        paces: &paces,
    });
}

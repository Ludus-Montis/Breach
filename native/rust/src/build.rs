//! Builders of the crate's state for its tests (spec 39, Decision 154): a test builds the
//! state it needs and runs only the phase under test, with no Godot. Step 8's benches
//! share them.

use crate::contest;
use crate::field::{Enrol, Field, SyncIn, BODIES, LOOSE, SCRUM};
use crate::maths::V2;
use crate::reach;
use crate::squads::Squads;

/// Two squads of `per_squad` bodies each, facing each other in one crowded column on the
/// field (each body a radius of 0.4 a cell apart, so they overlap and part), with the store
/// holding every unit and the squad store their squads.
pub fn crowd(per_squad: usize, seed: i64) -> (Field, Squads) {
    let mut field = Field::default();
    let mut squads = Squads::default();
    let mut unit_ids = Vec::new();
    for (k, squad_id) in [1i64, 2].iter().enumerate() {
        let slot = squads.open(*squad_id) as usize;
        squads.direction[slot] = if k == 0 { 1 } else { -1 };
        squads.width[slot] = 1;
        for n in 0..per_squad {
            let unit_id = squad_id * 100 + n as i64;
            let held = field.hold(unit_id);
            let x = n as f32 * 0.5 + k as f32 * 0.25;
            field.columns.position[held as usize] = V2 { x, y: 0.1 * k as f32 };
            squads.members[slot].push(held as i32);
            unit_ids.push(unit_id);
        }
    }
    let input = SyncIn {
        squad_ids: &[1, 2],
        squad_factions: &["player".to_string(), "kingdom".to_string()],
        squad_flags: &[0, 0],
        squad_counts: &[per_squad as i32, per_squad as i32],
        unit_ids: &unit_ids,
        mode: BODIES,
        bases: &[],
    };
    let new = field.sync(seed, &input);
    let count = new.len();
    let draws: Vec<i64> = (0..count as i64).map(|k| (k * 7919 + seed) % 1000).collect();
    field.enrol(&Enrol {
        flat: &new,
        radii: &vec![0.4; count],
        areas: &vec![0.5; count],
        draws: &draws,
        initiatives: &vec![0; count],
        records: &[],
        paces: &[],
    });
    (field, squads)
}

/// A unit of a scrum built for a test: its id, where it stands loose and the way it faces,
/// its footprint (square, cells a side), its preferred band (0 the front) and initiative,
/// and its place in its squad's frame (rank, column).
#[derive(Clone)]
pub struct Fighter {
    pub id: i64,
    pub at: V2,
    pub bearing: f64,
    pub foot: i64,
    pub preferred: i64,
    pub initiative: i64,
    pub place: (i64, i64),
}

impl Fighter {
    /// A grem (a footprint of one cell, in the front band) at `at` facing `bearing`.
    pub fn grem(id: i64, at: V2, bearing: f64) -> Fighter {
        Fighter { id, at, bearing, foot: 1, preferred: 0, initiative: 0, place: (0, 0) }
    }
}

/// A squad of a scrum built for a test: its id, faction, flags (`field::ROUTING`...) and
/// its units in its list's order.
#[derive(Clone)]
pub struct Side {
    pub id: i64,
    pub faction: &'static str,
    pub flags: u8,
    pub fighters: Vec<Fighter>,
}

/// Turn rate (degrees a second) and backward pace every built unit has.
pub const TURN_RATE: f64 = 90.0;
pub const BACKWARD_PACE: f64 = 0.5;

/// A field synced to the scrum's snapshot over `sides` in their order (spec 39 step 4a): the
/// store holds every unit, loose where it stands, facing its bearing, at its place; each is
/// enrolled with its body (`reach::radius`), its draw in the battle's `seed`
/// (`contest::draw`), its initiative and record, and the built turn rate and backward pace.
pub fn scrum(sides: &[Side], seed: i64) -> Field {
    let mut field = Field::default();
    let mut unit_ids = Vec::new();
    for side in sides {
        for f in &side.fighters {
            let slot = field.hold(f.id) as usize;
            let c = &mut field.columns;
            c.loose_at[slot] = f.at;
            c.flags[slot] = LOOSE as i32;
            c.bearing[slot] = f.bearing;
            c.rank[slot] = f.place.0 as i32;
            c.column[slot] = f.place.1 as i32;
            unit_ids.push(f.id);
        }
    }
    let factions: Vec<String> = sides.iter().map(|s| s.faction.to_string()).collect();
    let ids: Vec<i64> = sides.iter().map(|s| s.id).collect();
    let flags: Vec<u8> = sides.iter().map(|s| s.flags).collect();
    let counts: Vec<i32> = sides.iter().map(|s| s.fighters.len() as i32).collect();
    let input = SyncIn {
        squad_ids: &ids,
        squad_factions: &factions,
        squad_flags: &flags,
        squad_counts: &counts,
        unit_ids: &unit_ids,
        mode: SCRUM,
        bases: &[],
    };
    let new = field.sync(seed, &input);
    let all: Vec<&Fighter> = sides.iter().flat_map(|s| s.fighters.iter()).collect();
    let of = |k: usize| all[new[k] as usize];
    let count = new.len();
    let records: Vec<i64> =
        (0..count).flat_map(|k| [of(k).preferred, of(k).foot, of(k).foot]).collect();
    let paces: Vec<f64> = (0..count).flat_map(|_| [TURN_RATE, BACKWARD_PACE]).collect();
    field.enrol(&Enrol {
        flat: &new,
        radii: &(0..count).map(|k| reach::radius(of(k).foot, of(k).foot)).collect::<Vec<_>>(),
        areas: &(0..count).map(|k| (of(k).foot * of(k).foot) as f64).collect::<Vec<_>>(),
        draws: &(0..count).map(|k| contest::draw(seed, of(k).id)).collect::<Vec<_>>(),
        initiatives: &(0..count).map(|k| of(k).initiative).collect::<Vec<_>>(),
        records: &records,
        paces: &paces,
    });
    field
}

/// `sides` listed the other way round, each squad's units too: the reversed-lists cases
/// (Decision 97) build the same state from it.
pub fn reversed(sides: &[Side]) -> Vec<Side> {
    let mut out: Vec<Side> = sides.iter().rev().cloned().collect();
    for side in &mut out {
        side.fighters.reverse();
    }
    out
}

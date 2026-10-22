//! Builders of the crate's state for its tests (spec 39, Decision 154): a test builds the
//! state it needs and runs only the phase under test, with no Godot. Step 8's benches
//! share them.

use crate::field::{Enrol, Field, SyncIn, BODIES};
use crate::maths::V2;
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

//! The scrum's facing (spec 39 step 4a), from the gdUnit scenarios it replaces
//! (test_face_field): a phase run alone on a field the builders make.

use std::collections::BTreeMap;

use super::*;
use crate::build::{reversed, scrum, Fighter, Side, TURN_RATE};
use crate::face_store;

const SEED: i64 = 5;
const SECONDS: f64 = 0.1;

fn v(x: f32, y: f32) -> V2 {
    V2::new(x, y)
}

fn params() -> Params {
    Params { reach: 0.25, front: reach::front(60.0), seconds: SECONDS, pace: 0.3, crowding: 1.0 }
}

/// Unit id -> its bearing after one facing of the fighting squads (`fights`: [squad, its
/// foe]), every unit of theirs making for nothing.
fn faced(field: &mut Field, fights: &[(i64, i64)]) -> BTreeMap<i64, f64> {
    let mut squads = Vec::new();
    let mut members = Vec::new();
    for &(id, _) in fights {
        let sq = field.squad_index(id).unwrap() as usize;
        let bodies = field.squads[sq].members.clone();
        squads.extend_from_slice(&[id, 1, bodies.len() as i64]);
        members.extend(bodies.iter().map(|&b| field.flat_of[b as usize] as i32));
    }
    let foe_ids: Vec<i64> = fights.iter().map(|f| f.1).collect();
    let speeds = vec![1.0; members.len()];
    let out = face_store::face(field, &squads, &foe_ids, &members, &speeds, 4, &params());
    let ids = members.iter().map(|&flat| field.unit_id[field.flat[flat as usize] as usize]);
    ids.zip(out).collect()
}

fn duel(bearing: f64) -> Vec<Side> {
    let foes = vec![
        Fighter::grem(20, v(0.0, -1.0), 180.0), // north, in its front, touching
        Fighter::grem(21, v(0.95, 0.0), 270.0), // east, nearer, at its side
    ];
    vec![
        Side {
            id: 1,
            faction: "player",
            flags: 0,
            fighters: vec![Fighter::grem(10, v(0.0, 0.0), bearing)],
        },
        Side { id: 2, faction: "kingdom", flags: 0, fighters: foes },
    ]
}

#[test]
fn a_unit_turns_at_its_rate_to_the_foe_in_its_front_before_a_nearer_one_at_its_side() {
    let mut field = scrum(&duel(20.0), SEED);
    let got = faced(&mut field, &[(1, 2)]);
    // 20 degrees off the foe north of it, it turns its rate's worth towards it.
    assert!((got[&10] - (20.0 - TURN_RATE * SECONDS)).abs() < 1e-9);
    let b = field.slot_of(10).unwrap() as usize;
    assert_eq!(field.columns.bearing[b], got[&10]); // in the store
}

#[test]
fn a_unit_touching_no_foe_and_making_for_nothing_keeps_its_bearing() {
    let mut sides = duel(20.0);
    sides[0].fighters[0].at = v(-6.0, 6.0);
    let mut field = scrum(&sides, SEED);
    assert_eq!(faced(&mut field, &[(1, 2)])[&10], 20.0);
}

#[test]
fn a_unit_whose_walk_chose_a_look_turns_towards_it() {
    let mut sides = duel(0.0);
    sides[0].fighters[0].at = v(-6.0, 6.0);
    let mut field = scrum(&sides, SEED);
    let b = field.slot_of(10).unwrap() as usize;
    field.columns.toward[b] = v(-5.0, 6.0); // east of it
    field.columns.keys[b] |= 4; // the look is held
    assert!((faced(&mut field, &[(1, 2)])[&10] - TURN_RATE * SECONDS).abs() < 1e-9);
}

#[test]
fn the_facing_is_the_same_whatever_the_list_order() {
    let player: Vec<Fighter> =
        (0..5).map(|n| Fighter::grem(100 + n, v(1.0, n as f32 * 1.05), 60.0 * n as f64)).collect();
    let kingdom: Vec<Fighter> =
        (0..5).map(|n| Fighter::grem(200 + n, v(2.0, n as f32 * 1.05 + 0.4), 270.0)).collect();
    let sides = vec![
        Side { id: 1, faction: "player", flags: 0, fighters: player },
        Side { id: 2, faction: "kingdom", flags: 0, fighters: kingdom },
    ];
    let forward = faced(&mut scrum(&sides, SEED), &[(1, 2), (2, 1)]);
    let backward = faced(&mut scrum(&reversed(&sides), SEED), &[(2, 1), (1, 2)]);
    assert_eq!(forward, backward);
    assert!(
        forward.iter().filter(|(&id, &b)| id < 200 && b != 60.0 * (id - 100) as f64).count() >= 3
    );
}

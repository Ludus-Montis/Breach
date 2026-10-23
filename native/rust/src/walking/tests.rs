//! The scrum's walk (spec 39 step 4a), from the gdUnit scenarios it replaces
//! (test_walk_field): a phase run alone on a field the builders make.

use std::collections::BTreeMap;

use super::*;
use crate::build::{reversed, scrum, Fighter, Side};
use crate::scrum::IDLE;
use crate::walk_store::{self, WalkSay};

const SEED: i64 = 9;
/// Cells a tick at speed 1.
const PACE: f64 = 0.3;

fn v(x: f32, y: f32) -> V2 {
    V2::new(x, y)
}

fn params() -> Params {
    // BattleTuning's bodies_steer_look, bodies_steer_clear, bodies_ground_drag and _block.
    Params { seed: SEED, seconds: 0.1, look: 3.0, clear: 0.05, drag: 1.0, block: 8.0 }
}

/// What the walk is told of one unit: its id, what the seek made of it, its next point.
struct Told {
    id: i64,
    code: u8,
    next: V2,
}

/// Unit id -> where it stands after one walk of the units `told` of squad 1 (its frame
/// facing east from the origin), over the bodies `lying`.
fn walked(field: &mut Field, told: &[Told], lying: &[V2]) -> BTreeMap<i64, V2> {
    let members: Vec<i32> =
        told.iter().map(|t| field.flat_of[field.body_of(t.id).unwrap() as usize] as i32).collect();
    for t in told {
        let b = field.body_of(t.id).unwrap() as usize;
        field.columns.next[b] = t.next; // as the seek left it
    }
    let codes: Vec<u8> = told.iter().map(|t| t.code).collect();
    let nexts: Vec<V2> = told.iter().map(|t| t.next).collect();
    let say = WalkSay {
        squad_floats: &[PACE, 90.0, 1.0, 0.0],
        squad_anchors: &[V2::ZERO],
        lying_ats: lying,
        lying_radii: &vec![0.5; lying.len()],
        lying_areas: &vec![1.0; lying.len()],
    };
    let squads = [1, 1, told.len() as i64];
    let speeds = vec![1.0; told.len()];
    let p = params();
    let out = walk_store::walk(field, &squads, &say, &members, &codes, &nexts, &speeds, [1, 2], &p);
    told.iter().map(|t| t.id).zip(out.ats).collect()
}

fn alone(at: V2, bearing: f64) -> Vec<Side> {
    let foe = Fighter::grem(20, v(40.0, 40.0), 0.0);
    vec![
        Side { id: 1, faction: "player", flags: 0, fighters: vec![Fighter::grem(10, at, bearing)] },
        Side { id: 2, faction: "kingdom", flags: 0, fighters: vec![foe] },
    ]
}

#[test]
fn a_unit_making_for_a_slot_steps_its_pace_towards_it() {
    let mut field = scrum(&alone(v(0.0, 0.0), 90.0), SEED);
    let told = [Told { id: 10, code: AIM, next: v(5.0, 0.0) }];
    let got = walked(&mut field, &told, &[])[&10];
    assert!((got.x as f64 - PACE).abs() < 1e-6 && got.y == 0.0);
    let b = field.slot_of(10).unwrap() as usize;
    assert_eq!((field.columns.loose_at[b], field.columns.toward[b]), (got, v(5.0, 0.0)));
}

#[test]
fn a_unit_touching_a_foe_does_not_step() {
    let mut field = scrum(&alone(v(0.0, 0.0), 90.0), SEED);
    let told = [Told { id: 10, code: TOUCH, next: v(5.0, 0.0) }];
    assert_eq!(walked(&mut field, &told, &[])[&10], v(0.0, 0.0));
}

#[test]
fn a_unit_with_no_slot_walks_back_to_its_place() {
    // Its place: half a cell behind the anchor, facing east - at (-0.5, 0).
    let mut field = scrum(&alone(v(-3.0, 0.0), 90.0), SEED);
    let told = [Told { id: 10, code: IDLE, next: V2::ZERO }];
    let got = walked(&mut field, &told, &[])[&10];
    assert!((got.x as f64 - (-3.0 + PACE)).abs() < 1e-6 && got.y == 0.0);
}

#[test]
fn a_unit_steps_round_a_body_in_its_way() {
    let mut sides = alone(v(0.0, 0.0), 90.0);
    sides.push(Side {
        id: 3,
        faction: "player",
        flags: 0,
        fighters: vec![Fighter::grem(30, v(1.2, 0.0), 0.0)],
    });
    let mut field = scrum(&sides, SEED);
    let told = [Told { id: 10, code: AIM, next: v(5.0, 0.0) }];
    let got = walked(&mut field, &told, &[])[&10];
    assert!(got.y != 0.0); // off the straight way, past the body
    assert!(got.x > 0.0 && got.length() as f64 <= PACE + 1e-6);
}

#[test]
fn the_bodies_lying_underfoot_slow_it() {
    let mut field = scrum(&alone(v(0.0, 0.0), 90.0), SEED);
    let told = [Told { id: 10, code: AIM, next: v(5.0, 0.0) }];
    let clear = walked(&mut field, &told, &[])[&10];
    let mut field = scrum(&alone(v(0.0, 0.0), 90.0), SEED);
    let over = walked(&mut field, &told, &[v(0.5, 0.0)])[&10];
    assert!(over.x > 0.0 && over.x < clear.x); // half its pace over a body its weight
    assert!((over.x as f64 - PACE / 2.0).abs() < 1e-6);
}

#[test]
fn the_walk_is_the_same_whatever_the_list_order() {
    let player: Vec<Fighter> =
        (0..6).map(|n| Fighter::grem(100 + n, v(0.0, n as f32 * 0.9), 90.0)).collect();
    let mut sides = vec![
        Side { id: 1, faction: "player", flags: 0, fighters: player },
        Side {
            id: 2,
            faction: "kingdom",
            flags: 0,
            fighters: vec![Fighter::grem(20, v(1.5, 2.0), 270.0)],
        },
    ];
    let told: Vec<Told> = (0..6)
        .map(|n| Told { id: 100 + n, code: AIM, next: v(1.5, 2.25 - n as f32 * 0.1) })
        .collect();
    let forward = walked(&mut scrum(&sides, SEED), &told, &[v(0.7, 1.0)]);
    sides = reversed(&sides);
    let told_back: Vec<Told> =
        told.iter().rev().map(|t| Told { id: t.id, code: t.code, next: t.next }).collect();
    let backward = walked(&mut scrum(&sides, SEED), &told_back, &[v(0.7, 1.0)]);
    assert_eq!(forward, backward);
}

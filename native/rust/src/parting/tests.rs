//! UnitBodies' scenarios (Decision 106, spec 30 round 1), from test_unit_bodies.gd: bodies
//! built here, given in their draw order, parted on the battle tuning's passes, resistance
//! and brush. A grem's body is 0.5 cells across its radius and weighs 1, a brute's 1.0 and 4.

use super::*;
use crate::content::tests::repo_content;
use crate::content::Content;
use crate::field::{Enrol, Field, SyncIn, BODIES};
use crate::reach;

fn tuning() -> Tuning {
    let content = Content::load(&repo_content()).unwrap_or_else(|error| panic!("{error}"));
    let t = &content.tuning;
    Tuning { passes: t.bodies_passes, resist: t.bodies_resist, brush: t.bodies_brush }
}

/// A body: where it stands, its radius, its squad and faction, and whether it is loose.
#[derive(Clone, Copy)]
struct B(f32, f32, f64, i32, i32, bool);

fn grem(x: f32, y: f32, squad: i32, faction: i32, loose: bool) -> B {
    B(x, y, 0.5, squad, faction, loose)
}

fn brute(x: f32, y: f32, squad: i32, faction: i32, loose: bool) -> B {
    B(x, y, 1.0, squad, faction, loose)
}

/// The bodies after parting, and the flags each ends with.
fn parted(given: &[B]) -> (Vec<V2>, Vec<u8>, Outcome) {
    let radii: Vec<f64> = given.iter().map(|b| b.2).collect();
    let areas: Vec<f64> = given.iter().map(|b| (b.2 * 2.0) * (b.2 * 2.0)).collect();
    let squads: Vec<i32> = given.iter().map(|b| b.3).collect();
    let factions: Vec<i32> = given.iter().map(|b| b.4).collect();
    let points: Vec<V2> = given.iter().map(|b| V2::new(b.0, b.1)).collect();
    let bases = vec![V2::ZERO; given.len()];
    let mut bodies = Bodies {
        nexts: points.clone(),
        points,
        flags: given.iter().map(|b| if b.5 { LOOSE } else { 0 }).collect(),
        bases: &bases,
        radii: &radii,
        areas: &areas,
        squads: &squads,
        factions: &factions,
    };
    let mut way = |_: usize, _: usize| V2::new(1.0, 0.0);
    let outcome = part(&mut bodies, &tuning(), false, &mut way);
    let flags = bodies.flags.iter().map(|f| f & !MOVED).collect();
    (bodies.points, flags, outcome)
}

fn near(a: V2, x: f32, y: f32) -> bool {
    (a.x - x).abs() < 0.0001 && (a.y - y).abs() < 0.0001
}

#[test]
fn a_body_is_the_circle_in_its_footprint() {
    assert_eq!(reach::radius(1, 1), 0.5);
    assert_eq!(reach::radius(2, 2), 1.0);
}

#[test]
fn a_heavier_body_moves_a_lighter_one_more() {
    let (at, _, _) = parted(&[grem(0.0, 0.0, 1, 1, true), brute(1.2, 0.0, 1, 1, true)]);

    assert!((at[0].distance_to(at[1]) - 1.5).abs() < 0.0001);
    assert!((at[0].x.abs() - 0.24).abs() < 0.0001); // 0.3 deep, split 4 : 1
}

#[test]
fn overlapping_foes_each_step_back_half_whatever_their_mass() {
    let (at, _, _) = parted(&[grem(0.0, 0.0, 1, 1, true), brute(1.2, 0.0, 2, 2, true)]);

    assert!(near(at[0], -0.15, 0.0));
    assert!(near(at[1], 1.35, 0.0));
}

#[test]
fn a_unit_in_its_frame_resists_but_gives_way_to_its_own_squad() {
    let (at, flags, outcome) = parted(&[grem(5.0, 5.0, 1, 1, false), grem(5.5, 5.0, 1, 1, true)]);

    assert_eq!(outcome.loosened, vec![0]); // it left its place, to walk back
    assert_eq!(flags[0] & LOOSE, LOOSE);
    assert!((at[0].distance_to(V2::new(5.0, 5.0)) - 0.1).abs() < 0.0001); // 0.5 deep, 1 : 4
    assert!((at[1].distance_to(V2::new(5.5, 5.0)) - 0.4).abs() < 0.0001);
}

#[test]
fn a_unit_in_its_frame_is_not_moved_by_its_own_squad_only_brushing_it() {
    let brush = tuning().brush;
    let x = 5.0 + 1.0 - (brush / 2.0) as f32;
    let (at, flags, outcome) = parted(&[grem(5.0, 5.0, 1, 1, false), grem(x, 5.0, 1, 1, true)]);

    assert!(outcome.loosened.is_empty()); // it keeps its place
    assert_eq!(flags[0] & LOOSE, 0);
    assert!(near(at[0], 5.0, 5.0));
    assert!(near(at[1], 6.0, 5.0));
}

#[test]
fn a_unit_in_its_frame_holds_its_space_against_another_squads_loose_unit() {
    let (at, flags, _) = parted(&[grem(5.0, 5.0, 1, 1, false), grem(5.5, 5.0, 2, 1, true)]);

    assert_eq!(flags[0] & LOOSE, 0); // it keeps its place
    assert_eq!(at[0], V2::new(5.0, 5.0));
    assert!(near(at[1], 6.0, 5.0));
}

#[test]
fn two_squads_frames_keep_them_apart_and_one_squads_frames_part() {
    let (two, _, _) = parted(&[grem(5.0, 5.0, 1, 1, false), grem(5.5, 5.0, 2, 1, false)]);
    let (one, flags, _) = parted(&[grem(5.0, 5.0, 1, 1, false), grem(5.5, 5.0, 1, 1, false)]);

    assert_eq!(two, vec![V2::new(5.0, 5.0), V2::new(5.5, 5.0)]);
    assert!((one[0].distance_to(one[1]) - 1.0).abs() < 0.0001);
    assert_eq!(flags, vec![0, 0]); // each stays in its frame
}

#[test]
fn bodies_on_one_spot_part_along_the_way_they_are_given() {
    let (at, _, _) = parted(&[grem(4.0, 4.0, 1, 1, true), grem(4.0, 4.0, 1, 1, true)]);

    assert!(near(at[0], 3.5, 4.0));
    assert!(near(at[1], 4.5, 4.0));
}

/// A field of three grems of one squad, standing in their frames at `xs` (unit k, id k + 1,
/// drawn 30 - k), listed in the order of `order`; where each id stands after parting.
fn field_parted(xs: &[f32], order: &[usize]) -> Vec<(i64, V2)> {
    let mut field = Field::default();
    let ids: Vec<i64> = order.iter().map(|&k| k as i64 + 1).collect();
    for (&k, &id) in order.iter().zip(&ids) {
        let held = field.hold(id) as usize;
        field.columns.position[held] = V2::new(xs[k], 4.0);
    }
    let input = SyncIn {
        squad_ids: &[1],
        squad_factions: &["player".to_string()],
        squad_flags: &[0],
        squad_counts: &[ids.len() as i32],
        unit_ids: &ids,
        mode: BODIES,
        bases: &[],
    };
    let new = field.sync(7, &input);
    let draws: Vec<i64> = order.iter().map(|&k| 30 - k as i64).collect();
    field.enrol(&Enrol {
        flat: &new,
        radii: &vec![0.5; new.len()],
        areas: &vec![1.0; new.len()],
        draws: &draws,
        initiatives: &vec![0; new.len()],
        records: &[],
        paces: &[],
    });
    let mut way =
        |draw: i64, other: i64| if draw < other { V2::new(0.0, 1.0) } else { V2::new(1.0, 0.0) };
    let out = crate::field_parting::part_field(&mut field, &tuning(), false, &mut way);
    let mut moved: Vec<(i64, V2)> =
        out.moved.iter().zip(&out.points).map(|(&flat, &p)| (ids[flat as usize], p)).collect();
    moved.sort_by_key(|&(id, _)| id);
    moved
}

#[test]
fn bodies_on_one_spot_part_the_same_whatever_the_list_order() {
    let xs = [4.0, 4.0, 4.2];

    let forward = field_parted(&xs, &[0, 1, 2]);

    assert_eq!(forward.len(), 3);
    assert_eq!(field_parted(&xs, &[2, 1, 0]), forward);
    assert_eq!(field_parted(&xs, &[1, 2, 0]), forward);
}

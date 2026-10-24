//! The squads' geometry (Decisions 74, 75 and 78) on squads built here.

use super::*;
use crate::ground::Walker;
use crate::roster::Listed;

fn grem(id: i64, slot: usize) -> Member {
    Member {
        id,
        slot,
        foot: (1, 1),
        preferred: 0,
        attack_range: 0,
        kind: 0,
        turn_rate: 90.0,
        backward_pace: 0.5,
        detection: 8.0,
        max_stamina: 100.0,
        walker: Walker { height: 1.0, swimmer: 0, climber: 0, swims: false, climbs: false },
    }
}

/// Squads (one slot each) at `fronts` (position, heading), `width` columns wide, each of
/// grems at `places` (rank, column).
fn built(
    fronts: &[(V2, f64)],
    width: i32,
    places: &[(i32, i32)],
) -> (Squads, Columns, Vec<Listed>) {
    let mut squads = Squads::default();
    let mut columns = Columns::default();
    let mut listed = Vec::new();
    for (s, &(at, heading)) in fronts.iter().enumerate() {
        squads.open(s as i64 + 1);
        squads.position[s] = at;
        squads.heading[s] = heading;
        squads.width[s] = width;
        let mut living = Vec::new();
        for &(rank, column) in places {
            let slot = columns.position.len();
            columns.grow();
            columns.rank[slot] = rank;
            columns.column[slot] = column;
            living.push(grem(slot as i64 + 1, slot));
        }
        listed.push(Listed {
            squad: s,
            id: s as i64 + 1,
            faction: s as u32,
            living,
            ..Listed::default()
        });
    }
    (squads, columns, listed)
}

#[test]
fn a_footprint_lies_its_columns_from_the_left_and_its_ranks_back_from_the_front() {
    let frame = Frame { anchor: V2::ZERO, heading: 90.0, width: 2, shift: 0.0 };
    let corners = frame.corners((1, 1), (0, 0));
    let expect = [V2::new(0.0, -1.0), V2::new(0.0, 0.0), V2::new(-1.0, 0.0), V2::new(-1.0, -1.0)];
    for (got, want) in corners.iter().zip(expect) {
        assert!(got.distance_to(want) < 0.00001, "{got:?} {want:?}");
    }
    let rect = frame.unit_rect((1, 1), (1, 1));
    assert!(rect.position.distance_to(V2::new(-2.0, 0.0)) < 0.00001);
    assert!(rect.size.distance_to(V2::new(1.0, 1.0)) < 0.00001);
}

#[test]
fn two_lines_face_to_face_overlap_and_have_a_gap_in_tiles() {
    let fronts = [(V2::new(0.0, 0.0), 90.0), (V2::new(3.2, 0.0), 270.0)];
    let (squads, columns, listed) = built(&fronts, 2, &[(0, 0), (0, 1)]);
    let (a, b) =
        (Shape::of(&squads, &columns, &listed[0]), Shape::of(&squads, &columns, &listed[1]));
    assert!(facing_off(&a, &b));
    assert!(overlaps(&a, &b));
    assert!((gap(&a, &b) - 3.2 / 64.0).abs() < 0.000001);
    let fronts = [(V2::new(0.0, 0.0), 90.0), (V2::new(0.0, 10.0), 90.0)];
    let (squads, columns, listed) = built(&fronts, 2, &[(0, 0), (0, 1)]);
    let (a, b) =
        (Shape::of(&squads, &columns, &listed[0]), Shape::of(&squads, &columns, &listed[1]));
    assert!(!facing_off(&a, &b));
    assert!(!overlaps(&a, &b), "side by side, ten cells apart");
}

#[test]
fn a_face_gap_is_to_the_near_face_of_the_others_footprints() {
    let fronts = [(V2::new(0.0, 0.0), 90.0), (V2::new(5.0, 3.0), 0.0)];
    let (squads, columns, listed) = built(&fronts, 2, &[(0, 0), (0, 1)]);
    let (a, b) =
        (Shape::of(&squads, &columns, &listed[0]), Shape::of(&squads, &columns, &listed[1]));
    // facing north at (5, 3), two wide: its footprints span x 4 to 6
    assert!((face_gap(&a, &b) - 4.0 / 64.0).abs() < 0.000001);
    assert!(!overlap_across(&a, &b), "it lies off the attacker's line");
}

#[test]
fn an_attacker_meets_the_edge_it_comes_from() {
    assert_eq!(edge_hit(90.0, 270.0), 0); // head on: the front
    assert_eq!(edge_hit(90.0, 0.0), 1); // heading north into an east-facing line: its right
    assert_eq!(edge_hit(90.0, 90.0), 2); // from behind: the rear
    assert_eq!(edge_hit(90.0, 180.0), 3);
    assert_eq!(edge_hit(90.0, 315.0), 1); // between the front and the right: the right
}

#[test]
fn the_units_on_an_edge_are_the_outermost_that_way() {
    let fronts = [(V2::new(0.0, 0.0), 90.0)];
    let (squads, columns, listed) = built(&fronts, 2, &[(0, 0), (0, 1), (1, 0), (1, 1)]);
    let shape = Shape::of(&squads, &columns, &listed[0]);
    assert_eq!(shape.edge_units(0), vec![0, 1]);
    assert_eq!(shape.edge_units(2), vec![2, 3]);
    assert_eq!(shape.edge_units(1), vec![1, 3]);
    assert_eq!(shape.edge_units(3), vec![0, 2]);
    assert_eq!(shape.back_rows(), 2);
    let bounds = shape.bounds();
    assert!(bounds.position.distance_to(V2::new(-2.0, -1.0)) < 0.00001);
    assert!(bounds.size.distance_to(V2::new(2.0, 2.0)) < 0.00001);
}

// From test_squad_heading.gd: the geometry reckons from the heading, not the nearest facing.

/// Squads (ids from 1) at `fronts` (front, heading), each `width` by `ranks` grems.
fn turned(fronts: &[(V2, f64)], width: i32, ranks: i32) -> (Squads, Columns, Vec<Listed>) {
    let places: Vec<(i32, i32)> =
        (0..ranks).flat_map(|r| (0..width).map(move |c| (r, c))).collect();
    built(fronts, width, &places)
}

fn shapes<'a>(squads: &Squads, columns: &'a Columns, listed: &'a [Listed]) -> Vec<Shape<'a>> {
    listed.iter().map(|l| Shape::of(squads, columns, l)).collect()
}

#[test]
fn squads_face_off_by_how_far_apart_their_headings_are() {
    for (heading, faces) in [(200.0, true), (165.0, true), (150.0, false)] {
        let (squads, columns, listed) = turned(&[(V2::ZERO, 30.0), (V2::ZERO, heading)], 2, 1);
        let s = shapes(&squads, &columns, &listed);
        assert_eq!(facing_off(&s[0], &s[1]), faces, "{heading}");
    }
}

#[test]
fn the_gap_runs_along_a_slanting_heading() {
    let (squads, columns, listed) = turned(&[(V2::ZERO, 45.0), (V2::new(2.0, -2.0), 225.0)], 2, 1);
    let s = shapes(&squads, &columns, &listed);
    assert!((gap(&s[0], &s[1]) * CELLS - 2.0 * 2f64.sqrt()).abs() < 0.0001);
    assert!((gap(&s[1], &s[0]) * CELLS - 2.0 * 2f64.sqrt()).abs() < 0.0001);
}

#[test]
fn slanting_lines_overlap_across_their_heading_or_pass_by() {
    let across = V2::new(1.0, 1.0).normalized(); // the first's right hand
    let opposite = V2::new(3.0, -3.0);
    for (at, heading, over) in [
        (opposite, 225.0, true),
        (opposite.add(across.scale(2.5)), 225.0, false),
        (opposite, 135.0, false),
    ] {
        let (squads, columns, listed) = turned(&[(V2::ZERO, 45.0), (at, heading)], 2, 1);
        let s = shapes(&squads, &columns, &listed);
        assert_eq!(overlaps(&s[0], &s[1]), over, "{at:?} {heading}");
    }
}

#[test]
fn the_edge_met_is_the_quarter_of_the_victims_frame_the_attacker_comes_from() {
    for (attacker, edge) in [(210.0, 0), (30.0, 2), (120.0, 3), (300.0, 1), (50.0, 2)] {
        assert_eq!(edge_hit(30.0, attacker), edge, "{attacker}");
    }
}

#[test]
fn a_turned_squads_edges_are_its_outermost_units_that_way() {
    let (squads, columns, listed) = turned(&[(V2::new(5.0, 5.0), 45.0)], 2, 2);
    let shape = Shape::of(&squads, &columns, &listed[0]);
    assert_eq!(shape.edge_units(0), vec![0, 1]);
    assert_eq!(shape.edge_units(2), vec![2, 3]);
    assert_eq!(shape.edge_units(3), vec![0, 2]);
    assert_eq!(shape.edge_units(1), vec![1, 3]);
}

#[test]
fn the_face_gap_reaches_a_turned_squads_nearest_corner() {
    let (squads, columns, listed) = turned(&[(V2::ZERO, 90.0), (V2::new(6.0, 0.0), 315.0)], 2, 1);
    let s = shapes(&squads, &columns, &listed);
    assert!((face_gap(&s[0], &s[1]) * CELLS - (6.0 - 0.5f64.sqrt())).abs() < 0.0001);
    assert!(overlap_across(&s[0], &s[1]));
}

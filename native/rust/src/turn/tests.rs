//! Turning's scenarios (Decisions 74 and 92), on one squad built here, its units placed by
//! hand.

use super::*;
use crate::ground::Walker;
use crate::roster::Member;

fn unit(id: i64, slot: usize, foot: (i64, i64)) -> Member {
    Member {
        id,
        slot,
        foot,
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

/// A unit as placed: its id, (rank, column) and footprint (width, depth).
type Placed = (i64, (i32, i32), (i64, i64));

struct Built {
    squads: Squads,
    columns: Columns,
    listed: Listed,
}

/// One squad `width` columns wide facing east (direction +1) with its front `front` tiles
/// along a straight lane, its units (id, (rank, column), footprint) placed.
fn built(width: i32, front: f64, units: &[Placed]) -> Built {
    let mut squads = Squads::default();
    let mut columns = Columns::default();
    squads.open(1);
    squads.width[0] = width;
    squads.direction[0] = 1;
    squads.heading[0] = 90.0;
    squads.front_distance[0] = front;
    squads.position[0] = V2::new((front * CELLS) as f32, 0.0);
    let mut living = Vec::new();
    for &(id, (rank, column), foot) in units {
        let slot = columns.position.len();
        columns.grow();
        columns.rank[slot] = rank;
        columns.column[slot] = column;
        columns.position[slot] = V2::new(slot as f32, 3.0);
        living.push(unit(id, slot, foot));
    }
    let listed = Listed { squad: 0, id: 1, living, ..Listed::default() };
    Built { squads, columns, listed }
}

fn place(b: &Built, n: usize) -> (i32, i32) {
    let slot = b.listed.living[n].slot;
    (b.columns.rank[slot], b.columns.column[slot])
}

#[test]
fn an_about_face_puts_the_back_rank_in_front_and_turns_the_columns() {
    let mut b = built(3, 0.0, &[(1, (0, 0), (1, 1)), (2, (1, 2), (1, 1))]);
    b.squads.centre_shift[0] = 0.5;
    let depth = reverse_ranks(&mut b.squads, &mut b.columns, &b.listed);
    assert_eq!(depth, 2);
    assert_eq!(place(&b, 1), (0, 0), "the archer at the back now leads, at the old right");
    assert_eq!(place(&b, 0), (1, 2));
    assert_eq!(b.squads.centre_shift[0], -0.5);
    assert_eq!(b.squads.reforming[0], 1);
}

#[test]
fn a_wide_or_deep_unit_keeps_its_whole_footprint_inside_when_reversed() {
    let mut b = built(4, 0.0, &[(1, (0, 0), (2, 2)), (2, (2, 3), (1, 1))]);
    let depth = reverse_ranks(&mut b.squads, &mut b.columns, &b.listed);
    assert_eq!(depth, 3);
    assert_eq!(place(&b, 0), (1, 2), "a brute two deep and two wide");
    assert_eq!(place(&b, 1), (0, 0));
}

#[test]
fn going_back_the_way_it_faces_turns_it_about_its_front_moved_back_by_its_depth() {
    let mut b = built(2, 1.0, &[(1, (0, 0), (1, 1)), (2, (1, 1), (1, 1))]);
    let turned = begin(&mut b.squads, &mut b.columns, &b.listed, -1, None);
    assert_eq!(turned, ABOUT_FACED);
    assert_eq!(b.squads.direction[0], -1);
    assert_eq!(b.squads.heading[0], 270.0);
    assert_eq!(b.squads.front_distance[0], 1.0 - 2.0 * RANK_DEPTH);
    assert_eq!(b.squads.position[0], V2::new(62.0, 0.0), "placed on the lane");
    assert_eq!(place(&b, 0), (1, 1));
}

#[test]
fn a_turn_loosens_every_unit_in_its_frame_where_it_stands_in_the_lists_order() {
    let mut b = built(2, 1.0, &[(1, (0, 0), (1, 1)), (2, (0, 1), (1, 1)), (3, (1, 0), (1, 1))]);
    b.squads.loose[0].push(2);
    b.columns.loose_at[1] = V2::new(9.0, 9.0);
    begin(&mut b.squads, &mut b.columns, &b.listed, -1, None);
    assert_eq!(b.squads.loose[0], vec![2, 1, 3]);
    assert_eq!(b.columns.loose_at[0], V2::new(0.0, 3.0));
    assert_eq!(b.columns.loose_at[2], V2::new(2.0, 3.0));
    assert_eq!(b.columns.loose_at[1], V2::new(9.0, 9.0), "one already loose keeps its entry");
}

#[test]
fn on_a_bending_route_it_faces_the_route_where_it_stands() {
    let route = Route::new(vec![V2::new(0.0, 0.0), V2::new(64.0, 0.0), V2::new(64.0, 64.0)], 0.0);
    let mut b = built(1, 1.5, &[(1, (0, 0), (1, 1))]);
    let turned = begin(&mut b.squads, &mut b.columns, &b.listed, 1, Some(&route));
    assert_eq!(turned, TURNED, "past the bend, going on: it turns south, no about-face");
    assert_eq!(b.squads.heading[0], 180.0);
    assert_eq!(b.squads.direction[0], 1);
    assert_eq!(b.squads.loose[0], vec![1]);
}

#[test]
fn facing_its_way_already_it_keeps_its_heading_and_its_units_their_places() {
    let route = Route::new(vec![V2::new(0.0, 0.0), V2::new(640.0, 0.0)], 0.0);
    let mut b = built(1, 1.0, &[(1, (0, 0), (1, 1))]);
    assert_eq!(begin(&mut b.squads, &mut b.columns, &b.listed, 1, Some(&route)), KEPT);
    assert_eq!(begin(&mut b.squads, &mut b.columns, &b.listed, 1, None), KEPT);
    assert!(b.squads.loose[0].is_empty());
    assert_eq!(b.squads.reforming[0], 0);
}

#[test]
fn turning_about_on_a_route_faces_back_along_it() {
    let route = Route::new(vec![V2::new(0.0, 0.0), V2::new(640.0, 0.0)], 0.0);
    let mut b = built(1, 2.0, &[(1, (0, 0), (1, 1))]);
    assert_eq!(begin(&mut b.squads, &mut b.columns, &b.listed, -1, Some(&route)), ABOUT_FACED);
    assert_eq!(b.squads.heading[0], 270.0);
    assert_eq!(b.squads.position[0], V2::new(127.0, 0.0));
}

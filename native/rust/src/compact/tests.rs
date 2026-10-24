//! SquadCompact's scenarios (Decision 47; spec 35 step 4a) and FormationContact.reinforce's
//! (Decisions 44 and 51), on squads built here: units given by (rank, column, footprint width
//! and depth), unit k (id k + 1) in store slot k, all preferring the front.

use super::*;

/// A unit at (rank, column), `width` × `depth` cells.
#[derive(Clone, Copy)]
struct U(i32, i32, i32, i32);

fn one(rank: i32, column: i32) -> U {
    U(rank, column, 1, 1)
}

struct Built {
    squads: Squads,
    columns: Columns,
}

/// Squads of `widths`, each at front 5.0 facing `direction` 1, and the units in store slots
/// from 0 in the order given.
fn built(widths: &[i32], units: &[U]) -> Built {
    let mut b = Built { squads: Squads::default(), columns: Columns::default() };
    for (n, &width) in widths.iter().enumerate() {
        b.squads.open(n as i64 + 1);
        b.squads.width[n] = width;
        b.squads.direction[n] = 1;
        b.squads.front_distance[n] = 5.0;
    }
    for (k, u) in units.iter().enumerate() {
        b.columns.grow();
        b.columns.rank[k] = u.0;
        b.columns.column[k] = u.1;
    }
    b
}

/// The closers for store slots `slots` of `units`, each preferring band `preferred`.
fn closers(units: &[U], slots: &[usize]) -> Vec<Closer> {
    slots
        .iter()
        .map(|&k| Closer {
            id: k as i64 + 1,
            slot: k,
            foot: (units[k].2, units[k].3),
            preferred: 0,
        })
        .collect()
}

fn ranks(b: &Built, slots: &[usize]) -> Vec<i32> {
    slots.iter().map(|&k| b.columns.rank[k]).collect()
}

fn ids(living: &[Closer], moved: &[usize]) -> Vec<i64> {
    let mut out: Vec<i64> = moved.iter().map(|&n| living[n].id).collect();
    out.sort();
    out
}

#[test]
fn step_up_closes_the_column_after_a_front_death() {
    let units = [one(0, 0), one(1, 0), one(2, 0)];
    let mut b = built(&[1], &units);
    let living = closers(&units, &[1, 2]); // the front has fallen

    let moved = compact(&mut b.squads, &mut b.columns, 0, &living, &[]);

    assert_eq!(ids(&living, &moved), vec![2, 3]);
    assert_eq!(ranks(&b, &[1, 2]), vec![0, 1]);
    assert_eq!(b.squads.front_distance[0], 5.0);
}

#[test]
fn a_wide_unit_only_steps_up_when_its_whole_footprint_is_clear() {
    let units = [one(0, 0), one(0, 1), U(1, 0, 2, 1)];
    let mut b = built(&[2], &units);
    let living = closers(&units, &[0, 2]);

    assert!(compact(&mut b.squads, &mut b.columns, 0, &living, &[]).is_empty());
    assert_eq!(ranks(&b, &[0, 2]), vec![0, 1]); // column 0 is still held at rank 0
}

#[test]
fn a_unit_steps_up_over_passes_and_the_units_behind_follow_while_clear() {
    // column 0's two front ranks fall; the climber has two ranks to close, the wide unit
    // behind it spans columns 0 and 1
    let units = [one(0, 0), one(1, 0), one(2, 0), one(0, 1), one(1, 1), U(3, 0, 2, 1)];
    let mut b = built(&[2], &units);
    let living = closers(&units, &[2, 3, 4, 5]);

    let moved = compact(&mut b.squads, &mut b.columns, 0, &living, &[]);

    assert_eq!(ids(&living, &moved), vec![3, 6]);
    assert_eq!(ranks(&b, &[2, 4, 5]), vec![0, 1, 2]); // column 1's rank 1 still holds it
}

#[test]
fn no_one_steps_up_into_a_place_a_move_under_way_is_heading_for() {
    // the front still stands at column 2; the mover is on its way to the fallen's place
    let units = [one(0, 0), one(1, 0), one(1, 1), one(0, 2)];
    let mut b = built(&[3], &units);
    let living = closers(&units, &[1, 2, 3]);
    let movers = [Moving { slot: 2, to: (0, 0), foot: (1, 1) }];

    assert!(compact(&mut b.squads, &mut b.columns, 0, &living, &movers).is_empty());
    assert_eq!(ranks(&b, &[1, 2]), vec![1, 1]); // mid-move: it lands where its move ends
}

#[test]
fn only_front_preferring_units_step_into_the_front_rank() {
    // column 1's front stands; column 0's has fallen, and the unit behind it prefers the back
    let units = [one(0, 1), one(1, 0), one(2, 0)];
    let mut b = built(&[2], &units);
    let mut living = closers(&units, &[0, 1, 2]);
    living[1].preferred = 1;

    assert!(compact(&mut b.squads, &mut b.columns, 0, &living, &[]).is_empty());
    assert_eq!(ranks(&b, &[1, 2]), vec![1, 2]);
}

#[test]
fn a_squad_whose_front_has_all_fallen_re_anchors_on_its_foremost_rank() {
    let units = [one(0, 0), U(1, 0, 1, 1), U(2, 0, 1, 1)];
    let mut b = built(&[1], &units);
    let mut living = closers(&units, &[1, 2]);
    for c in &mut living {
        c.preferred = 1; // none may step into the front
    }

    let moved = compact(&mut b.squads, &mut b.columns, 0, &living, &[]);

    assert!(moved.is_empty());
    assert_eq!(ranks(&b, &[1, 2]), vec![0, 1]);
    assert_eq!(b.squads.front_distance[0], 5.0 - 1.0 / 64.0); // nobody moves on the route
}

#[test]
fn the_lists_order_does_not_change_who_steps_up() {
    let units = [one(0, 0), one(1, 0), one(2, 0), one(0, 1), one(1, 1), U(3, 0, 2, 1)];
    let mut a = built(&[2], &units);
    let mut b = built(&[2], &units);
    let living = closers(&units, &[2, 3, 4, 5]);
    let reversed: Vec<Closer> = living.iter().rev().cloned().collect();

    let one_way = compact(&mut a.squads, &mut a.columns, 0, &living, &[]);
    let other_way = compact(&mut b.squads, &mut b.columns, 0, &reversed, &[]);

    assert_eq!(ids(&living, &one_way), ids(&reversed, &other_way));
    assert_eq!(a.columns.rank, b.columns.rank);
}

/// A leader of two in rank 0 (slots 0, 1) and a fallen unit of its (slot 2, at rank 1); a
/// joining squad of `joining` in rank 0 from slot 3.
fn joined(leader_width: i32, joining: i32) -> (Built, Vec<U>) {
    let mut units = vec![one(0, 0), one(0, 1), one(1, 0)];
    units.extend((0..joining).map(|c| one(0, c)));
    let b = built(&[leader_width, joining], &units);
    (b, units)
}

#[test]
fn a_joining_squad_falls_in_behind_the_leaders_back_rank_centred_on_the_wider_and_steps_up() {
    let (mut b, units) = joined(2, 4);
    let leader = closers(&units, &[0, 1]);
    let joining = closers(&units, &[3, 4, 5, 6]);
    b.columns.attack_cooldown[3] = 0;

    let moved = reinforce(
        &mut b.squads,
        &mut b.columns,
        Joining { leader: 0, joining: 1, leader_units: &[0, 1, 2] },
        &leader,
        &joining,
        &[],
    );

    assert_eq!(moved, vec![2, 5]); // the outer two, where the leader has no front
    assert_eq!(b.squads.width[0], 4);
    assert_eq!((b.columns.column[0], b.columns.column[1]), (1, 2)); // the leader centred
    assert_eq!(b.columns.column[2], 1); // its fallen too
    assert_eq!(ranks(&b, &[3, 4, 5, 6]), vec![0, 1, 1, 0]);
    assert_eq!(b.columns.column[3..7].to_vec(), vec![0, 1, 2, 3]);
    assert_eq!(b.columns.attack_cooldown[3], 1);
    assert_eq!(b.squads.centre_shift[0], 0.0); // the leader stays put
}

#[test]
fn a_narrower_joining_squad_is_centred_behind_and_steps_up_where_the_front_has_fallen() {
    let (mut b, units) = joined(3, 1);
    // the leader is three wide, its middle fallen: units 0 and 1 at columns 0 and 2
    b.columns.column[1] = 2;
    let leader = closers(&units, &[0, 1]);
    let joining = closers(&units, &[3]);

    let moved = reinforce(
        &mut b.squads,
        &mut b.columns,
        Joining { leader: 0, joining: 1, leader_units: &[0, 1, 2] },
        &leader,
        &joining,
        &[],
    );

    assert_eq!(moved, vec![2]); // the joining unit, third of the leader's living now
    assert_eq!((b.columns.rank[3], b.columns.column[3]), (0, 1));
    assert_eq!(b.squads.width[0], 3);
}

#[test]
fn an_odd_widening_shifts_the_centre_by_the_half_cell_left_over() {
    let (mut b, units) = joined(2, 5);
    let leader = closers(&units, &[0, 1]);
    let joining = closers(&units, &[3, 4, 5, 6, 7]);

    reinforce(
        &mut b.squads,
        &mut b.columns,
        Joining { leader: 0, joining: 1, leader_units: &[0, 1, 2] },
        &leader,
        &joining,
        &[],
    );

    assert_eq!((b.columns.column[0], b.columns.column[1]), (1, 2));
    assert_eq!(b.squads.centre_shift[0], 0.5);
    assert_eq!(b.squads.width[0], 5);
}

#[test]
fn joining_units_are_no_longer_loose_or_fleeing() {
    let (mut b, units) = joined(2, 2);
    let leader = closers(&units, &[0, 1]);
    let joining = closers(&units, &[3, 4]);
    entries::loosen(&mut b.squads, &mut b.columns, 1, (4, 3), V2::ZERO);
    entries::flee(&mut b.squads, &mut b.columns, 1, (5, 4), 1.0, V2::ZERO);

    reinforce(
        &mut b.squads,
        &mut b.columns,
        Joining { leader: 0, joining: 1, leader_units: &[0, 1, 2] },
        &leader,
        &joining,
        &[],
    );

    assert!(b.squads.loose[1].is_empty() && b.squads.fleeing[1].is_empty());
    assert_eq!(b.columns.flags[3] & entries::LOOSE, 0);
    assert_eq!(b.columns.flags[4] & entries::FLEEING, 0);
}

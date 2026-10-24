//! Sight's scenarios (Decisions 87 and 134), from the gdUnit tests they replace
//! (test_formation_sight, test_formation_terrain_sight, test_formation_perception): squads
//! built here, on open ground or a wood.

use super::*;
use crate::content::tests::repo_content;
use crate::content::Content;
use crate::ground::{GroundTuning, Paint, Walker};
use crate::roster::Member;

fn scout(id: i64, slot: usize, detection: f64) -> Member {
    Member {
        id,
        slot,
        foot: (1, 1),
        preferred: 0,
        attack_range: 0,
        kind: 0,
        turn_rate: 90.0,
        backward_pace: 0.5,
        detection,
        max_stamina: 100.0,
        walker: Walker { height: 1.0, swimmer: 0, climber: 0, swims: false, climbs: false },
    }
}

/// One squad per entry (faction, its units' points), each unit seeing `detection` cells.
fn built(sides: &[(u32, &[V2])], detection: f64) -> (Squads, Columns, Roster) {
    let mut squads = Squads::default();
    let mut columns = Columns::default();
    let mut roster = Roster::default();
    for (s, &(faction, at)) in sides.iter().enumerate() {
        squads.open(s as i64 + 1);
        let mut living = Vec::new();
        for &p in at {
            let slot = columns.position.len();
            columns.grow();
            columns.position[slot] = p;
            living.push(scout(slot as i64 + 1, slot, detection));
        }
        roster.listed.push(Listed {
            squad: s,
            id: s as i64 + 1,
            faction,
            living,
            ..Listed::default()
        });
    }
    (squads, columns, roster)
}

/// A 32 by 32 ground with a wood (blocking sight) over cells x 10 to 19.
fn wood() -> Ground {
    let content = Content::load(&repo_content()).unwrap_or_else(|error| panic!("{error}"));
    let mut ground = Ground::new(32, 32, GroundTuning::of(&content.tuning));
    ground.paint((10, 0, 10, 32), &Paint { blocks_sight: Some(true), ..Paint::default() });
    ground
}

#[test]
fn a_squad_is_seen_within_the_best_detection_of_the_other_and_not_beyond() {
    let (_, columns, roster) =
        built(&[(0, &[V2::new(0.5, 0.5), V2::new(3.5, 0.5)]), (1, &[V2::new(11.5, 0.5)])], 8.0);
    assert!(detects(&columns, &roster.listed[0], &roster.listed[1], None));
    let (_, columns, roster) = built(&[(0, &[V2::new(0.5, 0.5)]), (1, &[V2::new(11.5, 0.5)])], 8.0);
    assert!(!detects(&columns, &roster.listed[0], &roster.listed[1], None));
}

#[test]
fn a_sight_line_through_more_than_two_screening_cells_is_blocked() {
    let ground = wood();
    assert!(clear(&ground, V2::new(8.5, 0.5), V2::new(11.5, 0.5)), "two cells in: seen");
    assert!(!clear(&ground, V2::new(8.5, 0.5), V2::new(14.5, 0.5)), "deeper in: hidden");
    assert!(clear(&ground, V2::new(8.5, 0.5), V2::new(8.5, 5.5)), "beside the wood");
    let (_, columns, roster) = built(&[(0, &[V2::new(8.5, 0.5)]), (1, &[V2::new(14.5, 0.5)])], 8.0);
    assert!(detects(&columns, &roster.listed[0], &roster.listed[1], None));
    assert!(!detects(&columns, &roster.listed[0], &roster.listed[1], Some(&ground)));
}

#[test]
fn each_formation_sees_the_standing_hostiles_it_detects() {
    let sides: [(u32, &[V2]); 4] = [
        (0, &[V2::new(0.5, 0.5)]),
        (1, &[V2::new(4.5, 0.5)]),
        (1, &[V2::new(30.5, 0.5)]),
        (0, &[V2::new(2.5, 0.5)]),
    ];
    let (mut squads, columns, roster) = built(&sides, 8.0);
    let seen = see(&squads, &columns, &roster, None);
    assert_eq!(seen, vec![vec![1], vec![0, 3], vec![], vec![1]]);
    squads.state[1] = locks::ROUTING;
    let seen = see(&squads, &columns, &roster, None);
    assert_eq!(seen, vec![vec![], vec![0, 3], vec![], vec![]], "a rout is not seen");
}

#[test]
fn a_squad_with_none_living_sees_nothing_and_is_not_seen() {
    let sides: [(u32, &[V2]); 2] = [(0, &[V2::new(0.5, 0.5)]), (1, &[])];
    let (squads, columns, roster) = built(&sides, 8.0);
    assert_eq!(see(&squads, &columns, &roster, None), vec![Vec::<usize>::new(), vec![]]);
}

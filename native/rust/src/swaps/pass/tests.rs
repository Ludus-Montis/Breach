//! The shuffle's scenarios (Decisions 46 and 49), from test_formation_shuffle.gd, on a squad
//! built here: squad 1, one column wide, at front 5.0, holding; unit 100 preferring the back
//! in its front rank and unit 101 preferring the front behind it, both one cell at speed 1.

use super::*;
use crate::events::{self, SWAPPED};

const TICK: i64 = 5;
/// [tick seconds, cells a second at speed 1, scrum_crowding], as FormationSimulation's.
const PACE: [f64; 3] = [0.1, 0.125, 0.6];
/// UnitDef.Position.BACK.
const BACK: i64 = 2;

struct Built {
    squads: Squads,
    field: Field,
}

fn built(reforming: i32) -> Built {
    let (mut squads, mut field) = (Squads::default(), Field::default());
    squads.open(1);
    squads.width[0] = 1;
    squads.combat_width[0] = 1;
    squads.direction[0] = 1;
    squads.front_distance[0] = 5.0;
    squads.reforming[0] = reforming;
    for (rank, (id, band)) in [(100, BACK), (101, 0)].into_iter().enumerate() {
        let slot = field.hold(id) as usize;
        field.columns.rank[slot] = rank as i32;
        field.columns.speed[slot] = 1.0;
        field.columns.sheet[slot].footprint_width = 1;
        field.columns.sheet[slot].footprint_depth = 1;
        field.columns.sheet[slot].preferred_position = band;
        squads.members[0].push(slot as i32);
    }
    Built { squads, field }
}

impl Built {
    fn rank(&self, id: i64) -> i32 {
        self.field.columns.rank[self.field.slot_of(id).unwrap() as usize]
    }

    /// A tick of the pass; what it logged.
    fn step(&mut self) -> Vec<i64> {
        let mut log = Log::default();
        step(&mut self.squads, &mut self.field, &[0], (PACE, TICK), &mut log);
        log.take()
    }

    /// Ticks until the log isn't empty (at most 100); what it logged.
    fn until_told(&mut self) -> Vec<i64> {
        for _ in 0..100 {
            let logged = self.step();
            if !logged.is_empty() {
                return logged;
            }
        }
        vec![]
    }
}

#[test]
fn a_stronger_claim_behind_swaps_forward_past_the_unit_ahead_and_it_is_told() {
    let mut b = built(1);
    assert!(b.step().is_empty());
    assert!(!b.squads.swaps[0].is_empty(), "the swap began");
    assert_eq!((b.rank(100), b.rank(101)), (0, 1), "no one moves while it is under way");

    let logged = b.until_told();

    assert_eq!(logged, vec![SWAPPED as i64, TICK, 1, 0, 101, 1, 100]);
    assert_eq!((b.rank(101), b.rank(100)), (0, 1));
    assert_eq!(events::KINDS[SWAPPED].0, "swapped");
}

#[test]
fn a_squad_re_forming_stops_once_nothing_starts_and_nothing_is_under_way() {
    let mut b = built(1);
    b.step();
    assert_eq!(b.squads.reforming[0], 1, "re-forming while its swap is under way");

    b.until_told();

    assert_eq!(b.squads.reforming[0], 0, "the tick its last swap ended");
    assert!(b.squads.swaps[0].is_empty());
}

#[test]
fn a_squad_not_re_forming_starts_no_move() {
    let mut b = built(0);
    assert!(b.step().is_empty());
    assert!(b.squads.swaps[0].is_empty());
    assert_eq!((b.rank(100), b.rank(101)), (0, 1));
}

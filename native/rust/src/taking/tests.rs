//! FormationTaking's sending scenarios (spec 30 round 3 part 8), from
//! test_formation_taking.gd, on squads built here: each unit in the next store slot, seeing
//! 40 cells. Faction 0 is the player's, 1 the kingdom's; the bodies are the kingdom's.

use super::*;
use crate::maths::V2;

const SEED: i64 = 7;
const REACH: f64 = 6.0;

/// The store the units stand in.
#[derive(Default)]
struct Built {
    columns: Columns,
    squads: Vec<Sending>,
    downed: Vec<Taker>,
}

impl Built {
    fn unit(&mut self, id: i64, faction: u32, at: (f32, f32)) -> Taker {
        let slot = self.columns.position.len();
        self.columns.grow();
        self.columns.position[slot] = V2::new(at.0, at.1);
        Taker { id, slot, faction, detection: 40.0 }
    }

    /// A free player unit of squad `squad` (made as needed) at `at`.
    fn taker(&mut self, squad: usize, id: i64, at: (f32, f32)) {
        let unit = self.unit(id, 0, at);
        while self.squads.len() <= squad {
            self.squads.push(Sending::default());
        }
        self.squads[squad].free.push(unit);
    }

    /// A downed body of `faction` at `at`.
    fn body(&mut self, id: i64, faction: u32, at: (f32, f32)) {
        let unit = self.unit(id, faction, at);
        self.downed.push(unit);
    }

    /// Each squad's takers sent out, by [the unit's id, the body's id].
    fn sent(&self) -> Vec<Vec<(i64, i64)>> {
        send(&self.columns, &self.squads, &self.downed, SEED, REACH)
            .iter()
            .zip(&self.squads)
            .map(|(pairs, squad)| {
                pairs.iter().map(|&(u, b)| (squad.free[u].id, self.downed[b].id)).collect()
            })
            .collect()
    }
}

#[test]
fn a_free_unit_is_sent_to_a_downed_foe_within_reach() {
    let mut b = Built::default();
    b.taker(0, 1, (0.0, 0.0));
    b.body(9, 1, (4.0, 0.0));
    assert_eq!(b.sent(), vec![vec![(1, 9)]]);
}

#[test]
fn a_body_beyond_reach_or_sight_or_a_friend_is_left() {
    let mut b = Built::default();
    b.taker(0, 1, (0.0, 0.0));
    b.body(9, 1, (6.5, 0.0)); // beyond take reach
    b.body(8, 0, (1.0, 0.0)); // a friend
    let mut blind = Built::default();
    blind.taker(0, 1, (0.0, 0.0));
    blind.squads[0].free[0].detection = 3.0;
    blind.body(9, 1, (4.0, 0.0)); // beyond what it sees
    assert_eq!(b.sent(), vec![vec![]]);
    assert_eq!(blind.sent(), vec![vec![]]);
}

#[test]
fn a_body_claimed_already_is_left() {
    let mut b = Built::default();
    b.taker(0, 1, (0.0, 0.0));
    b.body(9, 1, (4.0, 0.0));
    b.squads[0].claimed.push(0);
    assert_eq!(b.sent(), vec![vec![]]);
}

#[test]
fn each_unit_takes_one_body_nearest_first() {
    let mut b = Built::default();
    b.taker(0, 1, (0.0, 0.0));
    b.taker(0, 2, (1.0, 0.0));
    b.taker(0, 3, (2.0, 0.0));
    b.body(9, 1, (3.0, 0.0)); // 1 from the third, 2 from the second
    b.body(8, 1, (5.0, 0.0)); // 3 from the third, 4 from the second
    assert_eq!(b.sent(), vec![vec![(3, 9), (2, 8)]]);
}

#[test]
fn two_groups_may_send_to_the_same_body() {
    let mut b = Built::default();
    b.taker(0, 1, (0.0, 0.0));
    b.taker(1, 2, (1.0, 0.0));
    b.body(9, 1, (3.0, 0.0));
    assert_eq!(b.sent(), vec![vec![(1, 9)], vec![(2, 9)]]);
}

/// Three takers and two bodies, the bodies as near each taker as the others, with the
/// lists in their order or reversed: who takes which, by [the unit's id, the body's id].
fn picks(reversed: bool) -> Vec<(i64, i64)> {
    let mut b = Built::default();
    let mut takers = vec![(1, (0.0, 1.0)), (2, (0.0, -1.0)), (3, (2.0, 0.0))];
    let mut bodies = vec![(9, (1.0, 0.0)), (8, (3.0, 1.0))];
    if reversed {
        takers.reverse();
        bodies.reverse();
    }
    for (id, at) in takers {
        b.taker(0, id, at);
    }
    for (id, at) in bodies {
        b.body(id, 1, at);
    }
    let mut sent = b.sent().remove(0);
    sent.sort();
    sent
}

#[test]
fn who_takes_which_does_not_hang_on_the_lists() {
    assert_eq!(picks(false), picks(true));
    assert_eq!(picks(false).len(), 2);
}

//! RoutBlows' scenarios (Decision 82), on units built here: routers of the player's
//! (faction 0) fleeing on no route, along y = 0 at x = `along`, and the kingdom's (faction 1)
//! fighters standing where they are put. Each fighter's cooldown starts at 1 unless set.

use super::*;
use crate::ground::Walker;
use crate::roster::{Listed, Member};

const SEED: i64 = 7;
const REACH: f64 = 1.5;
const GREM: Walker = Walker { height: 1.0, swimmer: 0, climber: 0, swims: false, climbs: false };

fn member(id: i64, slot: usize) -> Member {
    Member {
        id,
        slot,
        foot: (1, 1),
        preferred: 0,
        attack_range: 0,
        kind: 0,
        turn_rate: 360.0,
        backward_pace: 0.5,
        detection: 8.0,
        max_stamina: 100.0,
        walker: GREM,
    }
}

#[derive(Default)]
struct Built {
    columns: Columns,
    roster: Roster,
    routing: Vec<Routing>,
    fighters: Vec<Fighter>,
}

impl Built {
    fn formation(&mut self, faction: u32) -> usize {
        let n = self.roster.listed.len();
        self.roster.listed.push(Listed {
            squad: n,
            id: n as i64 + 1,
            faction,
            ..Listed::default()
        });
        n
    }

    fn slot(&mut self, listed: usize, id: i64) -> usize {
        let slot = self.columns.position.len();
        self.columns.grow();
        self.roster.listed[listed].living.push(member(id, slot));
        slot
    }

    /// A routing formation of `faction` with routers at these distances along (no route).
    fn routing(&mut self, faction: u32, ids_along: &[(i64, f64)]) -> usize {
        let listed = self.formation(faction);
        for &(id, along) in ids_along {
            let slot = self.slot(listed, id);
            self.columns.along[slot] = along;
        }
        self.routing.push(Routing { listed, route: None });
        listed
    }

    /// A fighter of a new formation of `faction` at `at`, its cooldown at `cooldown`.
    fn fighter(&mut self, faction: u32, id: i64, at: (f32, f32), cooldown: i32) -> usize {
        let listed = self.formation(faction);
        let slot = self.slot(listed, id);
        self.columns.position[slot] = V2::new(at.0, at.1);
        self.columns.attack_cooldown[slot] = cooldown;
        self.fighters.push(Fighter { listed, slot, reset: 10 });
        slot
    }

    fn run(&mut self) -> Vec<(usize, usize, usize)> {
        blows(&mut self.columns, &self.roster, &self.routing, &self.fighters, REACH, SEED)
    }
}

#[test]
fn a_hostile_fighter_within_reach_strikes_the_nearest_router_and_starts_again() {
    let mut b = Built::default();
    b.routing(0, &[(1, 10.0), (2, 11.0)]);
    let slot = b.fighter(1, 50, (11.5, 0.0), 1);
    assert_eq!(b.run(), [(0, 0, 1)]);
    assert_eq!(b.columns.target_id[slot], 2);
    assert_eq!(b.columns.attack_cooldown[slot], 10);
}

#[test]
fn its_cooldown_runs_down_before_it_strikes() {
    let mut b = Built::default();
    b.routing(0, &[(1, 10.0)]);
    let slot = b.fighter(1, 50, (11.0, 0.0), 2);
    assert!(b.run().is_empty());
    assert_eq!(b.columns.target_id[slot], 1);
    assert_eq!(b.columns.attack_cooldown[slot], 1);
    assert_eq!(b.run(), [(0, 0, 0)]);
}

#[test]
fn none_beyond_reach_or_of_its_own_side() {
    let mut b = Built::default();
    b.routing(0, &[(1, 10.0)]);
    let far = b.fighter(1, 50, (12.0, 0.0), 1);
    let friend = b.fighter(0, 51, (10.5, 0.0), 1);
    assert!(b.run().is_empty());
    for slot in [far, friend] {
        assert_eq!(b.columns.target_id[slot], 0);
        assert_eq!(b.columns.attack_cooldown[slot], 1);
    }
}

#[test]
fn a_router_stands_on_its_route_plus_its_offset() {
    let mut b = Built::default();
    let listed = b.routing(0, &[(1, 5.0)]);
    let slot = b.roster.listed[listed].living[0].slot;
    b.columns.flee_offset[slot] = V2::new(0.0, 2.0);
    b.routing[0].route = Some(Route::new(vec![V2::new(0.0, 4.0), V2::new(20.0, 4.0)], 0.0));
    b.fighter(1, 50, (5.0, 7.0), 1);
    assert_eq!(b.run(), [(0, 0, 0)]);
}

#[test]
fn two_routers_equally_near_go_by_their_draws_whatever_the_lists_order() {
    let pick = |ids: [i64; 2]| {
        let mut b = Built::default();
        b.routing(0, &[(ids[0], 10.0)]);
        b.routing(0, &[(ids[1], 12.0)]);
        let slot = b.fighter(1, 50, (11.0, 0.0), 5);
        b.run();
        b.columns.target_id[slot]
    };
    let drawn = if contest::draw(SEED, 1) < contest::draw(SEED, 2) { 1 } else { 2 };
    assert_eq!(pick([1, 2]), drawn);
    assert_eq!(pick([2, 1]), drawn);
}

#[test]
fn each_fighter_strikes_once_a_tick_however_many_it_reaches() {
    let mut b = Built::default();
    b.routing(0, &[(1, 10.0), (2, 10.5), (3, 11.0)]);
    b.fighter(1, 50, (10.5, 0.5), 1);
    b.fighter(1, 51, (10.0, -0.5), 1);
    assert_eq!(b.run().len(), 2);
}

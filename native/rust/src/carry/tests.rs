//! FormationCarry's pick-up scenarios (Decision 126), from test_formation_carry.gd,
//! test_formation_near_passes.gd's bearers paired and test_formation_wounds_order.gd's who
//! bears whom, on squads built here: each unit in the next store slot, strength 10, no gear,
//! one cell across. Faction 0 is the player's, 1 the kingdom's.

use super::*;
use crate::content::tests::repo_content;
use crate::content::Content;

const SEED: i64 = 7;

fn tuning() -> CarryTuning {
    let content = Content::load(&repo_content()).unwrap_or_else(|error| panic!("{error}"));
    CarryTuning::of(&content.tuning)
}

/// The store the units stand in.
#[derive(Default)]
struct Built {
    columns: Columns,
    squads: Vec<Carrying>,
}

impl Built {
    /// Squad `squad` of `faction`, made as needed; `tending` if it tends its downed.
    fn squad(&mut self, squad: usize, faction: u32, tending: bool) {
        while self.squads.len() <= squad {
            self.squads.push(Carrying::default());
        }
        self.squads[squad].faction = faction;
        self.squads[squad].tending = tending;
    }

    fn slot(&mut self, at: (f32, f32)) -> usize {
        let slot = self.columns.position.len();
        self.columns.grow();
        self.columns.position[slot] = V2::new(at.0, at.1);
        slot
    }

    /// A living unit of squad `squad` standing at `at`.
    fn unit(&mut self, squad: usize, id: i64, at: (f32, f32)) -> At {
        let slot = self.slot(at);
        let faction = self.squads[squad].faction;
        let unit = Bearer {
            id,
            slot,
            faction,
            carrying: false,
            gear_weight: 0.0,
            strength: 10,
            hauler: 0,
        };
        self.squads[squad].living.push(unit);
        (squad, self.squads[squad].living.len() - 1)
    }

    /// A downed body of squad `squad` lying at `at`.
    fn body(&mut self, squad: usize, id: i64, at: (f32, f32)) -> At {
        let slot = self.slot(at);
        let faction = self.squads[squad].faction;
        self.squads[squad].downed.push(Body { id, slot, faction, cells: 1 });
        (squad, self.squads[squad].downed.len() - 1)
    }

    fn pick_up(&self, t: &CarryTuning) -> Vec<(At, At)> {
        pick_up(&self.columns, &self.squads, SEED, t)
    }

    /// The ids picked up, [body, bearer] each, in order.
    fn ids(&self, t: &CarryTuning) -> Vec<(i64, i64)> {
        let picked = self.pick_up(t);
        let squads = &self.squads;
        picked
            .iter()
            .map(|&(b, u)| (squads[b.0].downed[b.1].id, squads[u.0].living[u.1].id))
            .collect()
    }
}

/// A downed player unit at the origin, its friend a cell off in a formation that tends its
/// downed if `tending`: [the body, the bearer].
fn scene(tending: bool) -> (Built, At, At) {
    let mut b = Built::default();
    b.squad(0, 0, false);
    b.squad(1, 0, tending);
    let body = b.body(0, 1, (0.0, 0.0));
    let bearer = b.unit(1, 2, (1.0, 0.0));
    (b, body, bearer)
}

#[test]
fn a_formation_that_tends_its_downed_picks_one_up() {
    let t = tuning();
    let (b, body, bearer) = scene(true);
    assert_eq!(b.pick_up(&t), vec![(body, bearer)]);
}

#[test]
fn none_are_picked_up_by_a_formation_that_leaves_them_or_with_a_foe_near() {
    let t = tuning();
    let (leaving, _, _) = scene(false);
    let (mut watched, _, _) = scene(true);
    watched.squad(2, 1, false);
    watched.unit(2, 9, (0.0, 3.0)); // within guard reach of the body
    assert!(leaving.pick_up(&t).is_empty());
    assert!(watched.pick_up(&t).is_empty());
}

#[test]
fn a_foe_beyond_guard_reach_or_a_friend_near_leaves_it_free() {
    let t = tuning();
    let (mut b, body, bearer) = scene(true);
    b.squad(2, 1, false);
    b.unit(2, 9, (0.0, 4.5));
    b.unit(0, 3, (0.0, 1.0)); // a friend of the body's, not of a tending formation
    assert_eq!(b.pick_up(&t), vec![(body, bearer)]);
}

#[test]
fn a_bearer_beyond_reach_or_bearing_already_leaves_it() {
    let t = tuning();
    let (mut far, _, bearer) = scene(true);
    far.columns.position[far.squads[1].living[bearer.1].slot] = V2::new(2.5, 0.0);
    let (mut bearing, _, bearer) = scene(true);
    bearing.squads[1].living[bearer.1].carrying = true;
    assert!(far.pick_up(&t).is_empty());
    assert!(bearing.pick_up(&t).is_empty());
}

#[test]
fn a_unit_too_weak_to_move_under_a_body_leaves_it() {
    let t = tuning();
    let (mut b, body, _) = scene(true);
    b.squads[body.0].downed[body.1].cells = 4; // a brute's body: too heavy for a grem
    assert!(b.pick_up(&t).is_empty());
}

#[test]
fn a_body_weighs_its_cells_and_a_hauler_bears_more() {
    let t = tuning();
    let (mut b, body, bearer) = scene(true);
    let lying = b.squads[body.0].downed[body.1].clone();
    let unit = &mut b.squads[bearer.0].living[bearer.1];
    assert_eq!(stage(unit, None, &t), 0);
    assert_eq!(stage(unit, Some(&lying), &t), 1); // 15 of a limit of 20
    unit.gear_weight = 10.0;
    assert_eq!(stage(unit, Some(&lying), &t), 2);
    unit.gear_weight = 30.0;
    assert_eq!(stage(unit, Some(&lying), &t), 3); // past twice its limit
    unit.hauler = 1;
    assert_eq!(stage(unit, Some(&lying), &t), 2);
}

#[test]
fn the_nearest_pairs_go_first_each_body_and_bearer_once() {
    let t = tuning();
    let mut b = Built::default();
    b.squad(0, 0, false);
    b.squad(1, 0, true);
    b.body(0, 1, (0.0, 0.0));
    b.body(0, 2, (3.0, 0.0));
    b.unit(1, 3, (1.0, 0.0)); // 1 from the first, 2 from the second
    b.unit(1, 4, (1.5, 0.0)); // 1.5 from each
    assert_eq!(b.ids(&t), vec![(1, 3), (2, 4)]);
}

/// One bearer between two downed friends, and a second formation's bearer as near the
/// first, with the squads and the lists in their order or reversed.
fn bearers(reversed: bool) -> Vec<(i64, i64)> {
    let mut b = Built::default();
    let order = if reversed { [2, 1, 0] } else { [0, 1, 2] };
    for (s, &n) in order.iter().enumerate() {
        b.squad(s, 0, n != 0);
        match n {
            0 => {
                let bodies = [(1, (-1.0, 0.0)), (2, (1.0, 0.0))];
                let listed: Vec<_> =
                    if reversed { bodies.iter().rev().collect() } else { bodies.iter().collect() };
                for &(id, at) in listed {
                    b.body(s, id, at);
                }
            }
            1 => {
                b.unit(s, 3, (0.0, 0.0));
            }
            _ => {
                b.unit(s, 4, (-2.0, 0.0));
            }
        }
    }
    let mut ids = b.ids(&tuning());
    ids.sort();
    ids
}

#[test]
fn who_bears_whom_does_not_hang_on_the_lists() {
    assert_eq!(bearers(false), bearers(true));
    assert!(!bearers(false).is_empty());
}

#[test]
fn the_pairs_are_those_a_look_at_all_finds() {
    let t = tuning();
    let mut b = Built::default();
    for s in 0..4 {
        b.squad(s, (s % 2) as u32, s >= 2);
    }
    // each side on its own ground, 30 cells apart, one foe of each among the other's
    let mut id = 1;
    for n in 0..48 {
        let side = n % 2;
        let at = ((n / 2 % 6) as f32 * 1.3 + side as f32 * 30.0, (n / 12) as f32 * 1.7);
        if n % 3 == 0 {
            b.body(side, id, at);
        } else {
            b.unit(side + 2 * (n % 4 / 2), id, at);
        }
        id += 1;
    }
    b.unit(1, id, (2.0, 2.0));
    b.unit(0, id + 1, (32.0, 4.0));
    let mut expected = vec![];
    let standing: Vec<&Bearer> = b.squads.iter().flat_map(|s| &s.living).collect();
    let at = |slot: usize| b.columns.position[slot];
    for (s, squad) in b.squads.iter().enumerate() {
        if !squad.tending {
            continue;
        }
        for (bs, owner) in b.squads.iter().enumerate() {
            for (bu, body) in owner.downed.iter().enumerate() {
                let guarded = standing.iter().any(|o| {
                    o.faction != body.faction
                        && at(o.slot).distance_to(at(body.slot)) as f64 <= t.guard_reach
                });
                if guarded || body.faction != squad.faction {
                    continue;
                }
                for (u, unit) in squad.living.iter().enumerate() {
                    let gap = at(unit.slot).distance_to(at(body.slot)) as f64;
                    if gap <= t.reach && stage(unit, Some(body), &t) < 3 {
                        expected.push(((bs, bu), (s, u)));
                    }
                }
            }
        }
    }
    let mut got: Vec<(At, At)> =
        pairs(&b.columns, &b.squads, SEED, &t).iter().map(|p| (p.1, p.2)).collect();
    got.sort();
    expected.sort();
    assert!(!expected.is_empty());
    assert_eq!(got, expected);
}

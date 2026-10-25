//! The rout's searches through grids (spec 30 round 3) choose exactly as a look through
//! every unit in the lists' order would (Decision 97): on a field of eight friendly
//! formations (every third routing, every other led, one destroyed) and an enemy, on a bent
//! route, their units on a half-cell lattice so that many stand equally near, some loose a
//! little off their places, each search gives what the look at every unit gives, for router
//! point after router point. From test_rout_friends.gd.

use super::*;
use crate::entries;
use crate::ground::Walker;
use crate::roster::{Listed, Member};

const SEED: i64 = 23;
const GREM: Walker = Walker { height: 1.0, swimmer: 0, climber: 0, swims: false, climbs: false };

struct Field {
    squads: Squads,
    columns: Columns,
    roster: Roster,
    routing: Vec<usize>,
    leaders: Vec<i64>,
    route: Route,
    bodies: Vec<Vec<V2>>,
}

fn field() -> Field {
    let mut squads = Squads::default();
    let mut columns = Columns::default();
    let mut roster = Roster::default();
    let (mut routing, mut leaders) = (Vec::new(), Vec::new());
    let mut id = 100;
    for n in 0..9usize {
        let squad = squads.open(n as i64 + 1) as usize;
        let faction = if n == 8 { 1 } else { 0 };
        squads.state[squad] = match n {
            _ if n % 3 == 0 && n < 8 => locks::ROUTING,
            5 => locks::DESTROYED,
            _ => locks::HOLDING,
        };
        if squads.state[squad] == locks::ROUTING {
            routing.push(n);
        }
        let mut listed = Listed { squad, id: n as i64 + 1, faction, ..Listed::default() };
        for u in 0..12usize {
            let slot = columns.position.len();
            columns.grow();
            let x = (n % 4) as f32 * 3.0 + (u % 4) as f32 * 0.5;
            let y = (n / 4) as f32 * 2.5 + (u / 4) as f32 * 0.5;
            columns.position[slot] = V2::new(x, y);
            let foot = if u % 5 == 0 { (2, 2) } else { (1, 1) };
            listed.living.push(Member {
                id,
                slot,
                foot,
                preferred: 0,
                attack_range: 0,
                kind: 0,
                turn_rate: 360.0,
                backward_pace: 0.5,
                detection: 8.0,
                max_stamina: 100.0,
                walker: GREM,
            });
            if u % 7 == 3 {
                entries::loosen(&mut squads, &mut columns, squad, (id, slot), V2::new(x + 0.25, y));
            }
            leaders.push(if n % 2 == 1 && u == 0 { 1 } else { 0 });
            id += 1;
        }
        roster.listed.push(listed);
    }
    let route = Route::new(vec![V2::new(-4.0, 1.0), V2::new(6.0, 3.0), V2::new(16.0, 0.0)], 0.0);
    let loose = roster.loose(&squads);
    let bodies = (0..roster.listed.len())
        .map(|n| {
            roster.listed[n].living.iter().map(|m| roster::at(&columns, &loose[n], m)).collect()
        })
        .collect();
    Field { squads, columns, roster, routing, leaders, route, bodies }
}

impl Field {
    fn friends(&self) -> Friends<'_> {
        Friends::new(&self.roster, &self.squads, &self.columns, &self.routing, &self.leaders, SEED)
    }

    /// Every (listed, member) of `n`'s faction, in the lists' order, that `counts` takes.
    fn every(&self, n: usize, counts: impl Fn(usize) -> bool) -> Vec<(usize, usize)> {
        let faction = self.roster.listed[n].faction;
        let mut out = Vec::new();
        for (o, l) in self.roster.listed.iter().enumerate() {
            if l.faction == faction && counts(o) {
                out.extend((0..l.living.len()).map(|u| (o, u)));
            }
        }
        out
    }

    fn member(&self, (o, u): (usize, usize)) -> &Member {
        &self.roster.listed[o].living[u]
    }

    fn body(&self, unit: (usize, usize)) -> V2 {
        self.bodies[unit.0][unit.1]
    }

    fn routs(&self, o: usize) -> bool {
        self.routing.contains(&o)
    }

    fn stands(&self, o: usize) -> bool {
        !self.routs(o) && self.squads.state[self.roster.listed[o].squad] != locks::DESTROYED
    }

    fn draw(&self, unit: (usize, usize)) -> i64 {
        contest::draw(SEED, self.member(unit).id)
    }
}

/// The router points looked from: a quarter-cell lattice over the field and round it.
fn points() -> Vec<V2> {
    let mut out = Vec::new();
    for i in -8..64 {
        for j in -8..36 {
            out.push(V2::new(i as f32 * 0.25, j as f32 * 0.25));
        }
    }
    out
}

/// The least of `units` by `key`, the first on a tie.
fn least<K: PartialOrd>(
    units: Vec<(usize, usize)>,
    key: impl Fn((usize, usize)) -> K,
) -> Option<(usize, usize)> {
    let mut best: Option<(K, (usize, usize))> = None;
    for unit in units {
        let k = key(unit);
        if best.as_ref().is_none_or(|(held, _)| k < *held) {
            best = Some((k, unit));
        }
    }
    best.map(|(_, unit)| unit)
}

#[test]
fn crushing_finds_the_friend_every_unit_would() {
    let f = field();
    let mut friends = f.friends();
    let n = f.routing[0];
    let mut hits = 0;
    for at in points() {
        let close: Vec<(usize, usize)> = f
            .every(n, |o| !f.routs(o))
            .into_iter()
            .filter(|&u| (f.columns.position[f.member(u).slot].distance_to(at) as f64) < 1.0)
            .collect();
        let whole = least(close, |u| {
            let gap = f.columns.position[f.member(u).slot].distance_to(at) as f64;
            (snapped(gap, EPSILON), f.draw(u))
        });
        hits += whole.is_some() as usize;
        assert_eq!(friends.crushed(n, at), whole, "at {at:?}");
    }
    assert!(hits > 100);
}

#[test]
fn catching_finds_the_friend_every_unit_would() {
    let f = field();
    let mut friends = f.friends();
    let n = f.routing[1];
    let home = 0.0;
    let mut found = 0;
    for led in [false, true] {
        for reach in [0.5, 1.5] {
            for at in points() {
                let whole = least(
                    f.every(n, |o| f.stands(o) && (!led || friends.leadership(o) >= 1))
                        .into_iter()
                        .filter(|&u| {
                            let there = f.body(u);
                            let gap = there.distance_to(at) as f64 - f.member(u).radius();
                            gap <= reach && behind(Some(&f.route), at, there, home)
                        })
                        .collect(),
                    |u| {
                        let gap = f.body(u).distance_to(at) as f64 - f.member(u).radius();
                        (snapped(gap, EPSILON), f.draw(u))
                    },
                )
                .map(|u| u.0);
                found += whole.is_some() as usize;
                let near = friends.friend_near(n, (Some(&f.route), home), at, reach, led);
                assert_eq!(near, whole, "at {at:?}, reach {reach}, led {led}");
            }
        }
    }
    assert!(found > 100);
}

#[test]
fn a_refuge_is_the_friend_every_unit_gives_and_turning_aside_its_ranks_edge() {
    let f = field();
    let mut friends = f.friends();
    let n = f.routing[2];
    let mut found = 0;
    for (along, home) in [(14.0, 0.0), (2.0, 20.0)] {
        for at in points() {
            let whole = least(
                f.every(n, |o| f.stands(o))
                    .into_iter()
                    .filter(|&u| (f.route.distance_of(f.body(u)) - along) * (home - along) > 0.0)
                    .collect(),
                |u| (snapped(f.body(u).distance_to(at) as f64, EPSILON), f.draw(u)),
            )
            .map(|u| u.0);
            found += whole.is_some() as usize;
            assert_eq!(friends.refuge(n, &f.route, at, (along, home)), whole, "at {at:?}");
        }
    }
    assert!(found > 100);
    // turning aside: 0 heading into the ranks, else to half a cell inside their edge
    let friend = 1;
    let side = V2::new(0.0, -1.0);
    let across: Vec<f64> = (0..12).map(|u| f.body((friend, u)).dot(side) as f64).collect();
    let (low, high) = across.iter().fold((f64::INFINITY, -f64::INFINITY), |(l, h), &a| {
        (if a < l { a } else { l }, if a > h { a } else { h })
    });
    for at in points() {
        let mine = at.dot(side) as f64;
        let short = friends.short_of(friend, at, side);
        if mine >= low - 0.4 && mine <= high + 0.4 {
            assert_eq!(short, 0.0, "at {at:?}");
        } else if mine < low - 0.6 || mine > high + 0.6 {
            assert!(short != 0.0, "at {at:?}");
        }
    }
}

#[test]
fn an_enemy_near_is_as_every_enemy_gives() {
    let f = field();
    let friends = f.friends();
    let n = f.routing[0];
    let mut near = 0;
    for at in points() {
        let points = [at, at.add(V2::new(1.0, 0.5))];
        let whole = f.roster.listed[8].living.iter().any(|m| {
            let there = f.columns.position[m.slot];
            points.iter().any(|p| there.distance_to(*p) as f64 <= 2.0)
        });
        near += whole as usize;
        assert_eq!(friends.enemy_near(n, &points, 2.0), whole, "at {at:?}");
    }
    assert!(near > 50);
}

#[test]
fn a_faction_is_alone_with_none_but_its_routing_squads() {
    let mut f = field();
    assert!(!f.friends().alone(0));
    f.routing = (0..8).collect();
    assert!(f.friends().alone(0));
    assert!(!f.friends().alone(8));
}

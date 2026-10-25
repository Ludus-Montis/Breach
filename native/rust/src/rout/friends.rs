//! What a rout's routers look for near them (RoutFriends and RoutCatch, spec 39 step 7b):
//! friends to crush, catch or flee to, and enemies near, found through grids rather than by
//! looking at every unit on the field (spec 30 round 3). Each faction of the routers has two
//! indexes, taken where the units stand as the call begins:
//! - **crush**: the living units of its squads not routing, at their points;
//! - **stand**: those of its standing squads (neither routing nor destroyed), where their
//!   bodies are (ScrumReach.at).
//!
//! A faction is **alone** with no squad but its routing ones: nothing to crush, flee to or
//! be caught by. Each search picks as a look through every unit in the lists' order would:
//! the nearest by its distance snapped to 0.000001, then by the units' seeded draws (Decision
//! 97). Squads the rout passes stand where they are through it, so the indexes hold for the
//! call.

use crate::columns::Columns;
use crate::contest;
use crate::grid::{self, Grid};
use crate::locks;
use crate::maths::{clampf, snapped, V2};
use crate::roster::{self, Roster};
use crate::route::Route;
use crate::squads::Squads;

const EPSILON: f64 = 0.000001;

/// A faction's units indexed where they stand: (listed index, member index) of each.
struct Index {
    units: Vec<(usize, usize)>,
    points: Vec<V2>,
    grid: Grid,
    widest: f64,
}

impl Index {
    /// The living units of the listed squads `counts` takes, in the lists' order, at
    /// `points` (each listed squad's, by member).
    fn gathered(roster: &Roster, points: &[Vec<V2>], counts: impl Fn(usize) -> bool) -> Index {
        let mut units = Vec::new();
        let mut at = Vec::new();
        for (n, l) in roster.listed.iter().enumerate() {
            if counts(n) {
                units.extend((0..l.living.len()).map(|u| (n, u)));
                at.extend_from_slice(&points[n]);
            }
        }
        Index::of(units, at, roster)
    }

    fn of(units: Vec<(usize, usize)>, points: Vec<V2>, roster: &Roster) -> Index {
        let widest = units.iter().fold(0.0, |w: f64, &(n, u)| {
            let r = roster.listed[n].living[u].radius();
            if w > r {
                w
            } else {
                r
            }
        });
        Index { grid: Grid::build(&points), units, points, widest }
    }

    /// Indices within `reach` and the margin of `point`, in the list's order: a superset.
    fn near(&self, point: V2, reach: f64) -> Vec<usize> {
        let mut out = Vec::new();
        self.grid.near(point, reach, |i| {
            out.push(i as usize);
            true
        });
        out.sort_unstable();
        out
    }
}

/// The rout's view of the field for one call: each listed squad's state, and where each
/// of its living units stands (its point) and where its body is (ScrumReach.at), as the call
/// began.
pub struct Friends<'a> {
    roster: &'a Roster,
    states: Vec<i32>,
    points: Vec<Vec<V2>>,
    bodies: Vec<Vec<V2>>,
    /// Each listed squad's best living leader's leadership (FormationMorale.leadership).
    leadership: Vec<i64>,
    /// Each listed squad: whether it was routing as the tick's rout began.
    routing: Vec<bool>,
    seed: i64,
    /// By faction code: its crush and stand indexes, once asked for.
    crush: Vec<Option<Index>>,
    stand: Vec<Option<Index>>,
}

impl<'a> Friends<'a> {
    /// `leaders`: every listed squad's living units' leadership, in the lists' order.
    pub fn new(
        roster: &'a Roster,
        squads: &Squads,
        columns: &Columns,
        routing: &[usize],
        leaders: &[i64],
        seed: i64,
    ) -> Friends<'a> {
        let mut from = 0;
        let leadership = roster
            .listed
            .iter()
            .map(|l| {
                let to = from + l.living.len();
                let best = leaders[from..to].iter().fold(0, |b, &x| if x > b { x } else { b });
                from = to;
                best
            })
            .collect();
        let mut flags = vec![false; roster.listed.len()];
        for &n in routing {
            flags[n] = true;
        }
        let factions = roster.listed.iter().map(|l| l.faction as usize + 1).max().unwrap_or(0);
        let loose = roster.loose(squads);
        Friends {
            roster,
            states: roster.listed.iter().map(|l| squads.state[l.squad]).collect(),
            points: roster
                .listed
                .iter()
                .map(|l| l.living.iter().map(|m| columns.position[m.slot]).collect())
                .collect(),
            bodies: roster
                .listed
                .iter()
                .enumerate()
                .map(|(n, l)| l.living.iter().map(|m| roster::at(columns, &loose[n], m)).collect())
                .collect(),
            leadership,
            routing: flags,
            seed,
            crush: (0..factions).map(|_| None).collect(),
            stand: (0..factions).map(|_| None).collect(),
        }
    }

    pub fn leadership(&self, n: usize) -> i64 {
        self.leadership[n]
    }

    /// True if the listed squad's faction has no squad but its routing ones.
    pub fn alone(&self, n: usize) -> bool {
        let faction = self.roster.listed[n].faction;
        !self
            .roster
            .listed
            .iter()
            .enumerate()
            .any(|(o, l)| l.faction == faction && !self.routing[o])
    }

    /// True if the listed squad is neither routing nor destroyed (RoutCatch.stands).
    pub fn stands(&self, n: usize) -> bool {
        let state = self.states[n];
        state != locks::ROUTING && state != locks::DESTROYED
    }

    fn crush_index(&mut self, faction: u32) -> &Index {
        let (roster, routing, points) = (self.roster, &self.routing, &self.points);
        self.crush[faction as usize].get_or_insert_with(|| {
            let counts = |n: usize| roster.listed[n].faction == faction && !routing[n];
            Index::gathered(roster, points, counts)
        })
    }

    fn stand_index(&mut self, faction: u32) -> &Index {
        let (roster, routing, bodies, states) =
            (self.roster, &self.routing, &self.bodies, &self.states);
        self.stand[faction as usize].get_or_insert_with(|| {
            let counts = |n: usize| {
                roster.listed[n].faction == faction && !routing[n] && states[n] != locks::DESTROYED
            };
            Index::gathered(roster, bodies, counts)
        })
    }

    fn draw(&self, (n, u): (usize, usize)) -> i64 {
        contest::draw(self.seed, self.roster.listed[n].living[u].id)
    }

    /// The friend's unit nearest a router of `n` at `at`, one it runs into (nearer than a
    /// cell), ties by their draws (RoutFriends.crushed): (listed index, member index).
    pub fn crushed(&mut self, n: usize, at: V2) -> Option<(usize, usize)> {
        let faction = self.roster.listed[n].faction;
        let index = self.crush_index(faction);
        let mut best: Option<((f64, i64), usize)> = None;
        let found = index.near(at, 1.0 + grid::MARGIN);
        let index = self.crush[faction as usize].as_ref().expect("built above");
        for f in found {
            let gap = index.points[f].distance_to(at) as f64;
            if gap >= 1.0 || index.units[f].0 == n {
                continue;
            }
            let key = (snapped(gap, EPSILON), self.draw(index.units[f]));
            if best.is_none_or(|(held, _)| key < held) {
                best = Some((key, f));
            }
        }
        best.map(|(_, f)| index.units[f])
    }

    /// The nearest standing friend of `n` with a unit's body within `reach` of `at` (to the
    /// body's edge, Decision 106), that a router there has got behind; if `led`, only one
    /// with a leader (RoutCatch.friend_near). `route` and `home` are the router's squad's.
    pub fn friend_near(
        &mut self,
        n: usize,
        (route, home): (Option<&Route>, f64),
        at: V2,
        reach: f64,
        led: bool,
    ) -> Option<usize> {
        let faction = self.roster.listed[n].faction;
        let index = self.stand_index(faction);
        let found = index.near(at, reach + index.widest + grid::MARGIN);
        let index = self.stand[faction as usize].as_ref().expect("built above");
        let mut best: Option<((f64, i64), usize)> = None;
        for f in found {
            let (o, u) = index.units[f];
            if o == n || (led && self.leadership[o] < 1) {
                continue;
            }
            let there = index.points[f];
            let gap = there.distance_to(at) as f64 - self.roster.listed[o].living[u].radius();
            if gap > reach {
                continue;
            }
            let key = (snapped(gap, EPSILON), self.draw((o, u)));
            if best.is_none_or(|(held, _)| key < held) && behind(route, at, there, home) {
                best = Some((key, o));
            }
        }
        best.map(|(_, o)| o)
    }

    /// The standing friend of `n` whose unit is nearest `at`, among those between the router
    /// and home on its route - `along` its distance along it, `home` home's - (RoutFriends.
    /// refuge): looked for ring by ring outwards.
    pub fn refuge(
        &mut self,
        n: usize,
        route: &Route,
        at: V2,
        (along, home): (f64, f64),
    ) -> Option<usize> {
        let faction = self.roster.listed[n].faction;
        self.stand_index(faction);
        let index = self.stand[faction as usize].as_ref().expect("built above");
        let centre = grid::cell_of(at);
        // (snapped gap, draw, index, distance)
        let mut best: Option<(f64, i64, usize, f64)> = None;
        for ring in 0..=index.grid.last_ring(centre) {
            let within = best.map_or(f64::INFINITY, |b| b.3 + grid::MARGIN);
            if grid::floor_of(ring) > within {
                break;
            }
            let mut found = Vec::new();
            index.grid.ring(centre, ring, |i| found.push(i as usize));
            for f in found {
                let apart = index.points[f].distance_to(at) as f64;
                let gap = snapped(apart, EPSILON);
                if best.is_some_and(|b| gap > b.0) {
                    continue;
                }
                let draw = self.draw(index.units[f]);
                if best.is_some_and(|b| gap == b.0 && (draw, f) > (b.1, b.2)) {
                    continue;
                }
                let (o, _) = index.units[f];
                let ahead = (route.distance_of(index.points[f]) - along) * (home - along) > 0.0;
                if o != n && ahead {
                    best = Some((gap, draw, f, apart));
                }
            }
        }
        best.map(|b| index.units[b.2].0)
    }

    /// How far across its route (along `side`) a router at `at` must turn aside to run into
    /// the friend's ranks: 0 if it is already heading into them (RoutFriends.short_of).
    pub fn short_of(&self, friend: usize, at: V2, side: V2) -> f64 {
        let mut lowest = f64::NAN;
        let mut highest = f64::NAN;
        for &there in &self.bodies[friend] {
            let across = there.sub(at).dot(side) as f64;
            if lowest.is_nan() || across < lowest {
                lowest = across;
            }
            if highest.is_nan() || across > highest {
                highest = across;
            }
        }
        clampf(0.0, lowest - 0.5, highest + 0.5)
    }

    /// Cells from `at` to the friend's nearest living unit (RoutCatch.gap).
    pub fn gap(&self, friend: usize, at: V2) -> f64 {
        let mut least = f64::INFINITY;
        for &there in &self.bodies[friend] {
            let d = there.distance_to(at) as f64;
            if d < least {
                least = d;
            }
        }
        least
    }

    /// True if any enemy of `n` stands within `near` of any of `points` (its routers'):
    /// each enemy looks through a grid of them (RoutFriends.enemy_near).
    pub fn enemy_near(&self, n: usize, points: &[V2], near: f64) -> bool {
        let grid = Grid::build(points);
        let faction = self.roster.listed[n].faction;
        for (o, other) in self.roster.listed.iter().enumerate() {
            if other.faction == faction {
                continue;
            }
            for &at in &self.points[o] {
                let mut found = false;
                grid.near(at, near + grid::MARGIN, |i| {
                    found = at.distance_to(points[i as usize]) as f64 <= near;
                    !found
                });
                if found {
                    return true;
                }
            }
        }
        false
    }
}

/// True if a router at `at` has got behind a friend's unit at `there`: past it, on the side
/// of its route towards `home`, so the friend stands between it and the enemy (RoutCatch.
/// behind). With no route, always.
pub fn behind(route: Option<&Route>, at: V2, there: V2, home: f64) -> bool {
    let Some(route) = route else { return true };
    let mine = route.distance_of(at);
    let theirs = route.distance_of(there);
    (mine - theirs) * (home - theirs) >= 0.0
}

#[cfg(test)]
mod tests;

//! What is near a unit in the scrum (spec 39 step 4c; ScrumNear, ScrumSlots.bodies and
//! UnitSteer's look round its way): the bodies as they stood before anyone walked, which a
//! walker steps round, and a faction's foes indexed where they stand, which a unit looks
//! through for the one it touches or the nearest. A grid (`grid.rs`) finds the few near; each
//! search then picks exactly as a look through the whole list in its order would (Decision
//! 97).

use crate::columns::Columns;
use crate::contest;
use crate::grid::{self, Grid};
use crate::maths::{bearing_vector, minf, snapped, V2};
use crate::reach;
use crate::roster::{self, Body, Member, Roster};
use crate::squads::Squads;
use crate::steer;

const EPSILON: f64 = 0.000001;

/// How a walker steers round bodies (BattleTuning's `bodies_steer_look` and
/// `bodies_steer_clear`) and the battle's seed its draws are taken in.
#[derive(Clone, Copy, Debug)]
pub struct Steering {
    pub look: f64,
    pub clear: f64,
    pub seed: i64,
}

/// The bodies as they stood before anyone walked, and a grid over them.
pub struct Crowd {
    pub bodies: Vec<Body>,
    grid: Grid,
    widest: f64,
}

impl Crowd {
    pub fn of(bodies: Vec<Body>) -> Crowd {
        let points: Vec<V2> = bodies.iter().map(|b| b.at).collect();
        let widest = bodies.iter().fold(0.0, |w: f64, b| if b.radius > w { b.radius } else { w });
        Crowd { grid: Grid::build(&points), bodies, widest }
    }

    /// UnitSteer.toward: the point the unit at `at` (of squad `squad`, an id) heads for on
    /// its way to `goal`, round the nearest body in its way. Looks only at the bodies near
    /// its way, in the lists' order, which picks as a look at every body would.
    pub fn steer(&self, unit: &Member, squad: i64, at: V2, goal: V2, steering: &Steering) -> V2 {
        let own = unit.radius();
        let way = goal.sub(at);
        let length = way.length();
        let mut near = Vec::new();
        if length as f64 >= EPSILON {
            let ahead = V2::new(way.x / length, way.y / length);
            let end = at.add(ahead.scale(minf(length as f64, steering.look)));
            let middle = V2::new((at.x + end.x) / 2.0, (at.y + end.y) / 2.0);
            let half = at.distance_to(end) as f64 / 2.0;
            let reach = half + own + self.widest + grid::MARGIN + grid::MARGIN;
            self.grid.near(middle, reach, |i| {
                near.push(i as usize);
                true
            });
            near.sort_unstable();
        }
        let bodies: Vec<steer::Body> = near
            .iter()
            .map(|&i| {
                let b = self.bodies[i];
                steer::Body { at: b.at, radius: b.radius, squad: b.squad }
            })
            .collect();
        let seed = steering.seed;
        let draw = |i: usize| contest::draw(seed, self.bodies[near[i]].id);
        let own_draw = || contest::draw(seed, unit.id);
        let (look, clear) = (steering.look, steering.clear);
        steer::toward(at, goal, own, squad, &bodies, look, clear, seed, &own_draw, &draw)
    }
}

/// Units indexed where they stand (ScrumNear.index): a faction's foes, or one squad's units.
pub struct Foes {
    /// (listed index, member index) of each.
    pub units: Vec<(usize, usize)>,
    pub points: Vec<V2>,
    grid: Grid,
    widest: f64,
}

impl Foes {
    /// The units of the roster `units` names, where each stands now.
    pub fn index(
        roster: &Roster,
        squads: &Squads,
        columns: &Columns,
        units: Vec<(usize, usize)>,
    ) -> Foes {
        let loose = roster.loose(squads);
        let points: Vec<V2> = units
            .iter()
            .map(|&(n, u)| roster::at(columns, &loose[n], &roster.listed[n].living[u]))
            .collect();
        let widest = units.iter().fold(0.0, |w: f64, &(n, u)| {
            let r = roster.listed[n].living[u].radius();
            if w > r {
                w
            } else {
                r
            }
        });
        Foes { grid: Grid::build(&points), units, points, widest }
    }

    /// The living units of squads hostile to `faction`, not routing (ScrumBlows.hostile_units).
    pub fn of(roster: &Roster, squads: &Squads, columns: &Columns, faction: u32) -> Foes {
        Foes::index(roster, squads, columns, roster.hostiles(squads, faction))
    }

    /// The living units of the listed squad `n`.
    pub fn of_squad(roster: &Roster, squads: &Squads, columns: &Columns, n: usize) -> Foes {
        let units = (0..roster.listed[n].living.len()).map(|u| (n, u)).collect();
        Foes::index(roster, squads, columns, units)
    }

    fn member<'a>(&self, roster: &'a Roster, f: usize) -> &'a Member {
        let (n, u) = self.units[f];
        &roster.listed[n].living[u]
    }

    /// Indices of those near `point`, within `reach` (cells) and margin of it, in the list's
    /// order (BodyGrid.near); a superset of those within it.
    pub fn near(&self, point: V2, reach: f64) -> Vec<usize> {
        let mut out = Vec::new();
        self.grid.near(point, reach, |i| {
            out.push(i as usize);
            true
        });
        out.sort_unstable();
        out
    }

    /// The foes whose bodies' edges may come within `reach` of `point`, in the list's order
    /// (ScrumNear.around).
    pub fn around(&self, point: V2, reach: f64) -> Vec<usize> {
        self.near(point, reach + self.widest + grid::MARGIN)
    }

    /// The one nearest `at` (ScrumNear.nearest): by its distance snapped to 0.000001, then
    /// its unit's draw, then the list's order; none if there are none.
    pub fn nearest(&self, roster: &Roster, at: V2, seed: i64) -> Option<usize> {
        let mut best: Option<(f64, usize)> = None;
        let mut best_draw: Option<i64> = None;
        for f in 0..self.points.len() {
            let gap = snapped(self.points[f].distance_to(at) as f64, EPSILON);
            let better = match best {
                None => true,
                Some((g, _)) if gap != g => gap < g,
                Some((_, b)) => {
                    let mine = *best_draw
                        .get_or_insert_with(|| contest::draw(seed, self.member(roster, b).id));
                    contest::draw(seed, self.member(roster, f).id) < mine
                }
            };
            if better {
                best = Some((gap, f));
                best_draw = None;
            }
        }
        best.map(|(_, f)| f)
    }

    /// The foe of `close` (indices, in the list's order) the unit at `at` facing `bearing`
    /// would strike (ScrumBlows._pick): one it touches (`touch`, reach_contact), in its
    /// front (`arc`, reach_front_arc_degrees) first, then the nearest, then by the seeded
    /// draw. Where it stands, or none if it touches none.
    #[allow(clippy::too_many_arguments)]
    pub fn nearest_touching(
        &self,
        roster: &Roster,
        unit: &Member,
        at: V2,
        bearing: f64,
        close: &[usize],
        (touch, arc): (f64, f64),
        seed: i64,
    ) -> Option<V2> {
        let ahead = bearing_vector(bearing);
        let front = reach::front(arc);
        let mut best: Option<(usize, (i32, f64))> = None;
        for &f in close {
            let other = self.member(roster, f);
            let there = self.points[f];
            if reach::gap(at, unit.radius(), there, other.radius()) > touch {
                continue;
            }
            let side = if reach::in_front(ahead, at, there, front) { 0 } else { 1 };
            let key = (side, at.distance_to(there) as f64);
            let better = match best {
                None => true,
                Some((b, best_key)) => {
                    let best_id = self.member(roster, b).id;
                    less(key, best_key)
                        || (!less(best_key, key)
                            && contest::draw(seed, other.id) < contest::draw(seed, best_id))
                }
            };
            if better {
                best = Some((f, key));
            }
        }
        best.map(|(f, _)| self.points[f])
    }

    /// True if the unit at `at` touches any of `close` (ScrumBlows.touches_any).
    pub fn touches_any(
        &self,
        roster: &Roster,
        unit: &Member,
        at: V2,
        close: &[usize],
        touch: f64,
    ) -> bool {
        close.iter().any(|&f| {
            let other = self.member(roster, f);
            reach::gap(at, unit.radius(), self.points[f], other.radius()) <= touch
        })
    }
}

/// GDScript's `[front, distance] < [front, distance]`.
fn less(a: (i32, f64), b: (i32, f64)) -> bool {
    a.0 < b.0 || (a.0 == b.0 && a.1 < b.1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ground::Walker;
    use crate::roster::Listed;

    fn member(id: i64, slot: usize) -> Member {
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

    /// One squad (slot 0) of units (id, where it stands), in that order.
    fn squad_at(at: &[(i64, V2)]) -> (Roster, Squads, Columns) {
        let mut squads = Squads::default();
        squads.open(1);
        let mut columns = Columns::default();
        let mut living = Vec::new();
        for (n, &(id, p)) in at.iter().enumerate() {
            columns.grow();
            columns.position[n] = p;
            living.push(member(id, n));
        }
        let roster = Roster {
            listed: vec![Listed { squad: 0, id: 1, faction: 0, living, ..Listed::default() }],
        };
        (roster, squads, columns)
    }

    #[test]
    fn the_nearest_is_the_nearest_then_by_draw_whatever_the_order() {
        let at = [
            (1, V2::new(3.0, 0.0)),
            (2, V2::new(1.0, 0.0)),
            (3, V2::new(-1.0, 0.0)),
            (4, V2::new(0.0, 2.0)),
        ];
        let (roster, squads, columns) = squad_at(&at);
        let index = Foes::of_squad(&roster, &squads, &columns, 0);
        let found = index.nearest(&roster, V2::ZERO, 7).unwrap();
        let lower = if contest::draw(7, 2) < contest::draw(7, 3) { 1 } else { 2 };
        assert_eq!(found, lower); // the two a cell off tie: the lower draw
        let mut reversed = at.to_vec();
        reversed.reverse();
        let (roster, squads, columns) = squad_at(&reversed);
        let index = Foes::of_squad(&roster, &squads, &columns, 0);
        let again = index.nearest(&roster, V2::ZERO, 7).unwrap();
        assert_eq!(index.points[again], at[found].1);
    }

    #[test]
    fn nothing_is_nearest_among_none() {
        let (roster, squads, columns) = squad_at(&[]);
        let index = Foes::of_squad(&roster, &squads, &columns, 0);
        assert_eq!(index.nearest(&roster, V2::ZERO, 7), None);
    }
}

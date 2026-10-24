//! Regrouping after a fight (ScrumRegroup, Decisions 88 and 92, spec 27 rounds 5 and 6;
//! spec 39 step 4c): units of squads out of the fight walk back to their places - in the
//! squad's stance, if it holds one - at its re-form pace, and rejoin its frame once in place
//! and facing its way; one held off its place trades places or takes it (`trade`, Decision
//! 110). A drilled retreat's units back away facing the foe they touch (a fighting
//! withdrawal, Decision 95). Every squad's units face the foes they touched as the phase
//! began, and step round the bodies as they stood then, so no squad listed earlier moves
//! first and changes what a later one sees (Decision 97). Chasers and withdrawing units are
//! left to the pursuit and the withdrawal.

use std::collections::HashSet;

use crate::columns::Columns;
use crate::contest;
use crate::entries;
use crate::frame::Frame;
use crate::grid::{self, Grid};
use crate::ground::Ground;
use crate::locks;
use crate::maths::{bearing_vector, minf, V2};
use crate::motion;
use crate::mover;
use crate::reach;
use crate::roster::{self, Body, Member, Roster};
use crate::squads::Squads;
use crate::steer;
use crate::trade;

const EPSILON: f64 = 0.000001;

/// A squad of the roster out of the fight (not fighting or routing), in the lists' order:
/// its index in the roster, whether it is a drilled retreat (ordered to retreat, meeting
/// threats as a line), its re-form pace (FormationDiscipline), whether it is withdrawing,
/// and where its loose units walk back to on the ground (TerrainStep.returns; none: their
/// places).
pub struct Regrouper {
    pub listed: usize,
    pub drilled: bool,
    pub reform: f64,
    pub withdrawing: bool,
    pub homes: Vec<(i64, V2)>,
}

/// The tick's numbers: the march pace (cells a tick at speed 1), the tick's seconds, the
/// battle's seed, `reach_contact`, `reach_front_arc_degrees`, the steering's look and
/// clearance (`bodies_steer_look`, `bodies_steer_clear`) and `scrum_trade_seconds`.
pub struct Rules {
    pub pace: f64,
    pub seconds: f64,
    pub seed: i64,
    pub reach: f64,
    pub arc: f64,
    pub look: f64,
    pub clear: f64,
    pub trade_seconds: f64,
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

    /// UnitSteer.toward: the point the unit at `at` heads for on its way to `goal`, round
    /// the nearest body in its way. Looks only at the bodies near its way, in the lists'
    /// order, which picks as a look at every body would.
    pub fn steer(&self, unit: &Member, squad: i64, at: V2, goal: V2, rules: &Rules) -> V2 {
        let own = unit.radius();
        let way = goal.sub(at);
        let length = way.length();
        let mut near = Vec::new();
        if length as f64 >= EPSILON {
            let unit_way = V2::new(way.x / length, way.y / length);
            let end = at.add(unit_way.scale(minf(length as f64, rules.look)));
            let middle = V2::new((at.x + end.x) / 2.0, (at.y + end.y) / 2.0);
            let half = at.distance_to(end) as f64 / 2.0;
            let mut found = Vec::new();
            let reach = half + own + self.widest + grid::MARGIN + grid::MARGIN;
            self.grid.near(middle, reach, |i| {
                found.push(i as usize);
                true
            });
            found.sort_unstable();
            near = found;
        }
        let bodies: Vec<steer::Body> = near
            .iter()
            .map(|&i| {
                let b = self.bodies[i];
                steer::Body { at: b.at, radius: b.radius, squad: b.squad }
            })
            .collect();
        let seed = rules.seed;
        let draw = |i: usize| contest::draw(seed, self.bodies[near[i]].id);
        let own_draw = || contest::draw(seed, unit.id);
        steer::toward(
            at,
            goal,
            own,
            squad,
            &bodies,
            rules.look,
            rules.clear,
            seed,
            &own_draw,
            &draw,
        )
    }
}

/// The foes of one faction, indexed where they stand (ScrumNear.index).
pub struct Foes {
    /// (listed index, member index) of each.
    pub units: Vec<(usize, usize)>,
    pub points: Vec<V2>,
    grid: Grid,
    widest: f64,
}

impl Foes {
    pub fn of(roster: &Roster, squads: &Squads, columns: &Columns, faction: u32) -> Foes {
        let loose = roster.loose(squads);
        let units = roster.hostiles(squads, faction);
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

    /// The foes whose bodies' edges may come within `reach` of `point`, in the list's order
    /// (ScrumNear.around).
    pub fn around(&self, point: V2, reach: f64) -> Vec<usize> {
        let mut out = Vec::new();
        self.grid.near(point, reach + self.widest + grid::MARGIN, |i| {
            out.push(i as usize);
            true
        });
        out.sort_unstable();
        out
    }

    /// The foe of `close` (indices, in the list's order) the unit at `at` facing `bearing`
    /// would strike (ScrumBlows._pick): one it touches, in its front first, then the
    /// nearest, then by the seeded draw. Where it stands, or none if it touches none.
    pub fn nearest_touching(
        &self,
        roster: &Roster,
        unit: &Member,
        at: V2,
        bearing: f64,
        close: &[usize],
        rules: &Rules,
    ) -> Option<V2> {
        let ahead = bearing_vector(bearing);
        let front = reach::front(rules.arc);
        let mut best: Option<(usize, (i32, f64))> = None;
        for &f in close {
            let (n, u) = self.units[f];
            let other = &roster.listed[n].living[u];
            let there = self.points[f];
            if reach::gap(at, unit.radius(), there, other.radius()) > rules.reach {
                continue;
            }
            let key = (if reach::in_front(ahead, at, there, front) { 0 } else { 1 }, {
                at.distance_to(there) as f64
            });
            let better = match best {
                None => true,
                Some((b, best_key)) => {
                    let (bn, bu) = self.units[b];
                    let best_id = roster.listed[bn].living[bu].id;
                    less(key, best_key)
                        || (!less(best_key, key)
                            && contest::draw(rules.seed, other.id)
                                < contest::draw(rules.seed, best_id))
                }
            };
            if better {
                best = Some((f, key));
            }
        }
        best.map(|(f, _)| self.points[f])
    }
}

/// GDScript's `[front, distance] < [front, distance]`.
fn less(a: (i32, f64), b: (i32, f64)) -> bool {
    a.0 < b.0 || (a.0 == b.0 && a.1 < b.1)
}

/// ScrumRegroup.step: units of the squads out of the fight walk back to their places and
/// trade places where their own ranks hold them off.
pub fn step(
    squads: &mut Squads,
    columns: &mut Columns,
    roster: &Roster,
    regroupers: &[Regrouper],
    ground: Option<&Ground>,
    rules: &Rules,
) {
    let mut faced = Vec::with_capacity(regroupers.len());
    let mut hostile: Vec<(u32, Foes)> = Vec::new(); // faction -> its foes, built once
    for r in regroupers {
        let squad = roster.listed[r.listed].squad;
        let held = squads.loose[squad].iter().any(|&id| !squads.chasing(squad, id));
        if squads.state[squad] == locks::MOVING && held {
            squads.state[squad] = locks::HOLDING; // it stands while its units regroup
        }
        faced.push(foes_faced(squads, columns, roster, r, &mut hostile, rules));
    }
    let walking =
        regroupers.iter().any(|r| !squads.loose[roster.listed[r.listed].squad].is_empty());
    let crowd = if walking { Crowd::of(roster.bodies(squads, columns)) } else { Crowd::of(vec![]) };
    for (r, foes) in regroupers.iter().zip(&faced) {
        walk_back(squads, columns, roster, r, foes, &crowd, ground, rules);
        trade_in(squads, columns, roster, r.listed, rules);
    }
}

/// unit id -> where the foe it touches stands, for a drilled retreat's loose units (none
/// for any other squad).
fn foes_faced(
    squads: &Squads,
    columns: &Columns,
    roster: &Roster,
    r: &Regrouper,
    hostile: &mut Vec<(u32, Foes)>,
    rules: &Rules,
) -> Vec<(i64, V2)> {
    let mut out = Vec::new();
    let listed = &roster.listed[r.listed];
    if !r.drilled {
        return out;
    }
    let found = match hostile.iter().position(|(f, _)| *f == listed.faction) {
        Some(k) => k,
        None => {
            hostile.push((listed.faction, Foes::of(roster, squads, columns, listed.faction)));
            hostile.len() - 1
        }
    };
    let near = &hostile[found].1;
    let loose: HashSet<i64> = squads.loose[listed.squad].iter().copied().collect();
    for &id in &squads.loose[listed.squad] {
        let Some(unit) = listed.living.iter().find(|m| m.id == id) else { continue };
        let where_ = roster::at(columns, &loose, unit);
        let close = near.around(where_, unit.radius() + rules.reach);
        let bearing = columns.bearing[unit.slot];
        if let Some(at) = near.nearest_touching(roster, unit, where_, bearing, &close, rules) {
            out.push((id, at));
        }
    }
    out
}

/// The squad's loose units walk back to their places, round the bodies in their way, paying
/// the ground they cross; in place and facing its way, each rejoins its frame unless it
/// holds a stance.
#[allow(clippy::too_many_arguments)]
fn walk_back(
    squads: &mut Squads,
    columns: &mut Columns,
    roster: &Roster,
    r: &Regrouper,
    foes: &[(i64, V2)],
    crowd: &Crowd,
    ground: Option<&Ground>,
    rules: &Rules,
) {
    let listed = &roster.listed[r.listed];
    let squad = listed.squad;
    if squads.loose[squad].is_empty() {
        return;
    }
    let seconds = rules.seconds;
    let basis = Frame::kept(squads, squad); // once a squad: no unit's walk moves its frame
    let stance = squads.stance[squad];
    let heading = stance.map_or(squads.heading[squad], |s| s.heading);
    for id in squads.loose[squad].clone() {
        if squads.chasing(squad, id) || r.withdrawing {
            continue; // away on its own: the pursuit's or the withdrawal's
        }
        let Some(unit) = listed.living.iter().find(|m| m.id == id) else { continue };
        let slot = unit.slot;
        let place = match r.homes.iter().find(|(h, _)| *h == id) {
            Some(&(_, home)) => home,
            None => basis.place((columns.rank[slot], columns.column[slot]), unit.foot),
        };
        let step = columns.speed[slot] * rules.pace * r.reform;
        let mut at = columns.loose_at[slot];
        let foe_at = match foes.iter().find(|(f, _)| *f == id) {
            Some(&(_, foe)) => foe,
            None => {
                // it takes its place the quicker way, arriving facing its squad's
                motion::look(&unit.mover(columns), at, place, heading, step / seconds)
            }
        };
        let gap = at.distance_to(place) as f64;
        trade::track(columns, slot, gap, seconds);
        let arrived = gap < EPSILON;
        if !arrived {
            // a drilled retreat backs away facing the foe it touches
            let to = crowd.steer(unit, listed.id, at, place, rules);
            let mover = unit.mover(columns);
            let walker = &unit.walker;
            let (stepped, bearing) =
                mover::step(ground, walker, &mover, at, to, step, seconds, Some(foe_at));
            columns.bearing[slot] = bearing;
            at = stepped;
            columns.loose_at[slot] = at; // placement: its step, by the mover (Decision 139)
        }
        columns.next[slot] = at;
        columns.goal_foe[slot] = 0;
        columns.goal_slot[slot] = -1;
        let faced = arrived && {
            let (bearing, faces) = motion::turning_to(&unit.mover(columns), heading, seconds);
            columns.bearing[slot] = bearing;
            faces
        };
        if stance.is_none() && faced {
            entries::unloose(squads, columns, squad, (id, slot)); // in its place, facing its way
        }
    }
}

/// ScrumTrade.trade for the listed squad: its living units, in its order.
fn trade_in(squads: &mut Squads, columns: &mut Columns, roster: &Roster, n: usize, rules: &Rules) {
    let listed = &roster.listed[n];
    if squads.loose[listed.squad].is_empty() {
        return; // nobody to trade
    }
    let units: Vec<trade::Trader> = listed
        .living
        .iter()
        .map(|m| trade::Trader {
            id: m.id,
            slot: m.slot,
            kind: m.kind,
            preferred: m.preferred,
            foot: m.foot,
            living: true,
        })
        .collect();
    let trading = trade::Trading {
        squad: listed.squad,
        units: &units,
        seed: rules.seed,
        after: rules.trade_seconds - EPSILON,
    };
    trade::trade(squads, columns, &trading);
}

#[cfg(test)]
#[path = "regroup/tests.rs"]
mod tests;

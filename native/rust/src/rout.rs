//! Rout (FormationRout, Decision 82, spec 27 round 3; spec 39 step 7b): a panicked flight,
//! not an orderly about-face.
//! - **Breaking:** at 0 morale a formation routs. Every lock on it ends, and its units take
//!   fleeing entries where they stand. Who breaks is decided before the tick's rout begins,
//!   so one break's shock can't cascade within the tick (Decision 97).
//! - **Flight:** its units flee one by one back along its route towards home (`flight.rs`).
//! - **Crush:** a router running into a friend's unit (nearer than a cell) hurts both, and
//!   that friend's formation takes panic shock, once for each routing formation.
//! - **Rally:** a router that reaches a friendly formation with a leader joins its rear. One
//!   that has run into a standing friend is held there, and after `steady_rally_seconds`
//!   with that friend steady joins it. The rallies are all decided before any join, and
//!   join nearest first, ties by their draws.
//! - **Re-form:** a routing formation whose own leader lives, with no enemy within
//!   `enemy_near` for `rally_seconds`, re-forms where its best leader stands and holds.
//! - **Home:** routers that reach home leave the field; a formation left with none scatters.
//!
//! Each function here decides one phase; the pass (`pass.rs`, spec 39 step 8b5-xxviii) runs
//! them in order over the tick and lands what each decided: the hurts, the shocks, the joins
//! and the events. The searches are `friends.rs`'s.

use crate::columns::Columns;
use crate::content::CELLS_PER_TILE;
use crate::contest;
use crate::entries::{self, Key, Value};
use crate::ground::Ground;
use crate::locks;
use crate::maths::{is_equal_approx, is_zero_approx, snapped, V2};
use crate::motion;
use crate::perception;
use crate::roster::Roster;
use crate::rout_blows::{where_on, Routing};
use crate::squads::Squads;

pub mod flight;
pub mod friends;
pub mod pass;

use flight::Flight;
use friends::Friends;

const CELLS: f64 = CELLS_PER_TILE as f64;
/// SkirmishUnit.Order's hold.
const HOLD: i32 = 1;

/// BattleTuning's rout numbers, and the tick's pace (cells a tick at speed 1) and length.
#[derive(Clone, Copy, Debug, Default)]
pub struct RoutTuning {
    pub pace: f64,
    pub tick_seconds: f64,
    pub fan_degrees: f64,
    pub fan_cells: f64,
    pub rally_reach: f64,
    pub steady_rally_seconds: f64,
    pub rally_seconds: f64,
    pub enemy_near: f64,
    pub reformed_morale: i64,
    pub steady_at: i64,
}

impl RoutTuning {
    /// From [pace, tick_seconds, rout_fan_degrees, rout_fan_cells, rout_rally_reach,
    /// rout_steady_rally_seconds, rout_rally_seconds, rout_enemy_near,
    /// rout_reformed_morale, morale_steady_at].
    pub fn of_slice(t: &[f64]) -> RoutTuning {
        RoutTuning {
            pace: t[0],
            tick_seconds: t[1],
            fan_degrees: t[2],
            fan_cells: t[3],
            rally_reach: t[4],
            steady_rally_seconds: t[5],
            rally_seconds: t[6],
            enemy_near: t[7],
            reformed_morale: t[8] as i64,
            steady_at: t[9] as i64,
        }
    }
}

/// What the flight did, in the order it happened: `routing` a formation's index in the
/// routing given, `member` a router's in its living.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Fled {
    /// The router crushed the friend's unit `hit` (listed index, member index); `panic`
    /// the first time this formation's routers crush that friend.
    Crushed { routing: usize, member: usize, hit: (usize, usize), panic: bool },
    /// The router reached home and its fleeing entry is gone.
    Home { routing: usize, member: usize },
    /// The formation has no router left: destroyed.
    Scattered { routing: usize },
}

/// What the rallies decided: who joins whom, in the order they join (routing index, member
/// index, friend's listed index), and the listed squads whose band was first read here
/// with no morale read yet (GDScript sets their morale to their ceiling).
#[derive(Debug, Default, PartialEq)]
pub struct Rallied {
    pub joins: Vec<(usize, usize, usize)>,
    pub first_read: Vec<usize>,
}

/// A join's place in line: its router's gap to the friend (snapped), then its draw.
type JoinKey = (f64, i64);

/// The listed squads that break this tick, in the lists' order: standing, not arrived, with
/// a unit living and at 0 morale.
pub fn breaking(squads: &Squads, roster: &Roster) -> Vec<usize> {
    let mut out = Vec::new();
    for (n, listed) in roster.listed.iter().enumerate() {
        let state = squads.state[listed.squad];
        let standing = state != locks::ROUTING && state != locks::DESTROYED;
        if standing
            && state != locks::ARRIVED
            && !listed.living.is_empty()
            && squads.morale[listed.squad] == 0
        {
            out.push(n);
        }
    }
    out
}

/// Every router not held flees a step, crushing friends in its way; those reaching home
/// leave. `leaders`: every listed squad's living units' leadership, in the lists' order.
#[allow(clippy::too_many_arguments)]
pub fn flee(
    squads: &mut Squads,
    columns: &mut Columns,
    roster: &Roster,
    routing: &[Routing],
    leaders: &[i64],
    tuning: &RoutTuning,
    seed: i64,
    ground: Option<&Ground>,
) -> Vec<Fled> {
    let listed: Vec<usize> = routing.iter().map(|r| r.listed).collect();
    let mut friends = Friends::new(roster, squads, columns, &listed, leaders, seed);
    let mut out = Vec::new();
    for (r, formation) in routing.iter().enumerate() {
        let n = formation.listed;
        let s = roster.listed[n].squad;
        let route = formation.route.as_ref();
        let flight = Flight {
            listed: n,
            route,
            home: squads.home_distance[s] * CELLS,
            pace: tuning.pace,
            seed,
            fan_degrees: tuning.fan_degrees,
            fan_cells: tuning.fan_cells,
            ground,
        };
        let alone = friends.alone(n);
        let mut panicked: Vec<i64> = Vec::new();
        let living = &roster.listed[n].living;
        let mut fled = 0;
        for (u, m) in living.iter().enumerate() {
            if columns.caught[m.slot] > 0 {
                continue; // held by a steady friend it ran into
            }
            let at = where_on(route, columns.along[m.slot], columns.flee_offset[m.slot]);
            flight::step(columns, &flight, m, at, (!alone).then_some(&mut friends));
            if !alone {
                let at = where_on(route, columns.along[m.slot], columns.flee_offset[m.slot]);
                if let Some(hit) = friends.crushed(n, at) {
                    let id = roster.listed[hit.0].id;
                    let panic = !panicked.contains(&id);
                    if panic {
                        panicked.push(id);
                    }
                    out.push(Fled::Crushed { routing: r, member: u, hit, panic });
                }
            }
            if is_equal_approx(columns.along[m.slot], flight.home) {
                entries::unflee(squads, columns, s, (m.id, m.slot));
                out.push(Fled::Home { routing: r, member: u });
                fled += 1;
            }
        }
        if fled == living.len() {
            squads.state[s] = locks::DESTROYED;
            out.push(Fled::Scattered { routing: r });
        }
    }
    out
}

/// Each router's rally, decided before any joins: one with a led friend near, or one held
/// long enough by a steady friend, joins it. Counts each held router's time caught.
/// `morale`: each listed squad's morale (below 0: not read yet) and, when not read yet, its
/// ceiling.
#[allow(clippy::too_many_arguments)]
pub fn rallies(
    squads: &Squads,
    columns: &mut Columns,
    roster: &Roster,
    routing: &[Routing],
    leaders: &[i64],
    morale: &[(i64, i64)],
    tuning: &RoutTuning,
    seed: i64,
) -> Rallied {
    let listed: Vec<usize> = routing.iter().map(|r| r.listed).collect();
    let mut friends = Friends::new(roster, squads, columns, &listed, leaders, seed);
    let mut steady: Vec<Option<bool>> = vec![None; roster.listed.len()];
    let mut out = Rallied::default();
    let mut joins: Vec<(JoinKey, (usize, usize, usize))> = Vec::new();
    for (r, formation) in routing.iter().enumerate() {
        let n = formation.listed;
        let s = roster.listed[n].squad;
        let way = (formation.route.as_ref(), squads.home_distance[s] * CELLS);
        let alone = friends.alone(n);
        for (u, m) in roster.listed[n].living.iter().enumerate() {
            let at = where_on(way.0, columns.along[m.slot], columns.flee_offset[m.slot]);
            let draw = contest::draw(seed, m.id);
            let leader = if alone {
                None
            } else {
                friends.friend_near(n, way, at, tuning.rally_reach, true)
            };
            if let Some(leader) = leader {
                joins.push(((snapped(friends.gap(leader, at), 0.000001), draw), (r, u, leader)));
                continue;
            }
            let friend = if alone {
                None
            } else {
                holder(&mut friends, columns, roster, (n, m.slot), way, at, 2.0 * m.radius())
            };
            let calm = friend.is_some_and(|f| {
                *steady[f].get_or_insert_with(|| {
                    let (now, ceiling) = morale[f];
                    if now < 0 {
                        out.first_read.push(f);
                    }
                    (if now < 0 { ceiling } else { now }) >= tuning.steady_at
                })
            });
            let held = columns.caught[m.slot];
            let caught = match friend {
                None => 0,
                Some(_) if calm => held + 1,
                Some(_) => held.max(1),
            };
            entries::keep(columns, m.slot, Key::Caught, Value::Int(caught));
            let by = friend.map_or(Value::None, |f| Value::Int(roster.listed[f].id));
            entries::keep(columns, m.slot, Key::HeldBy, by);
            let long = caught as f64 * tuning.tick_seconds >= tuning.steady_rally_seconds;
            if let (Some(friend), true, true) = (friend, calm, long) {
                joins.push(((snapped(friends.gap(friend, at), 0.000001), draw), (r, u, friend)));
            }
        }
    }
    joins.sort_by(|a, b| a.0.partial_cmp(&b.0).expect("gaps are numbers"));
    out.joins = joins.into_iter().map(|(_, join)| join).collect();
    out
}

/// The friend holding a router (RoutCatch.holder): the one that caught it while it stands,
/// else the nearest friend within `reach` of `at` it has got behind. `who`: the router's
/// squad's listed index and the router's store slot.
fn holder(
    friends: &mut Friends,
    columns: &Columns,
    roster: &Roster,
    (n, slot): (usize, usize),
    way: (Option<&crate::route::Route>, f64),
    at: V2,
    reach: f64,
) -> Option<usize> {
    if entries::holds_value(columns, slot, Key::HeldBy) {
        let id = columns.held_by[slot];
        let caught =
            (0..roster.listed.len()).find(|&o| roster.listed[o].id == id && friends.stands(o));
        if caught.is_some() {
            return caught;
        }
    }
    friends.friend_near(n, way, at, reach, false)
}

/// Each routing formation whose own leader lives counts its ticks with no enemy near, and
/// re-forms once it has had `rally_seconds` of them. Returns the routing indices of those
/// that re-formed, in order. `slot_of` finds a unit's store slot by its id.
#[allow(clippy::too_many_arguments)]
pub fn regroup(
    squads: &mut Squads,
    columns: &mut Columns,
    roster: &Roster,
    routing: &[Routing],
    leaders: &[i64],
    tuning: &RoutTuning,
    seed: i64,
    mut slot_of: impl FnMut(i64) -> Option<usize>,
) -> Vec<usize> {
    let listed: Vec<usize> = routing.iter().map(|r| r.listed).collect();
    let friends = Friends::new(roster, squads, columns, &listed, leaders, seed);
    let mut starts = Vec::with_capacity(roster.listed.len());
    let mut from = 0;
    for l in &roster.listed {
        starts.push(from);
        from += l.living.len();
    }
    let mut out = Vec::new();
    for (r, formation) in routing.iter().enumerate() {
        let n = formation.listed;
        let living = &roster.listed[n].living;
        if living.is_empty() || friends.leadership(n) == 0 {
            continue;
        }
        let route = formation.route.as_ref();
        let points: Vec<V2> = living
            .iter()
            .map(|m| where_on(route, columns.along[m.slot], columns.flee_offset[m.slot]))
            .collect();
        let s = roster.listed[n].squad;
        let near = friends.enemy_near(n, &points, tuning.enemy_near);
        squads.rally_ticks[s] = if near { 0 } else { squads.rally_ticks[s] + 1 };
        if squads.rally_ticks[s] as f64 * tuning.tick_seconds < tuning.rally_seconds {
            continue;
        }
        let mut best: Option<((i64, i64), usize)> = None;
        for (u, m) in living.iter().enumerate() {
            let key = (-leaders[starts[n] + u], contest::draw(seed, m.id));
            if best.is_none_or(|(held, _)| key < held) {
                best = Some((key, m.slot));
            }
        }
        let (_, leader) = best.expect("a living unit");
        reform(squads, columns, s, route, columns.along[leader] / CELLS, &mut slot_of);
        squads.morale[s] = tuning.reformed_morale as i32;
        out.push(r);
    }
    out
}

/// Re-forms the squad in `s` with its front at `front` (tiles along its route), facing the
/// enemy's end of its route again, and holds.
fn reform(
    squads: &mut Squads,
    columns: &mut Columns,
    s: usize,
    route: Option<&crate::route::Route>,
    front: f64,
    slot_of: &mut impl FnMut(i64) -> Option<usize>,
) {
    squads.front_distance[s] = front;
    let cells = squads.front_distance[s] * CELLS;
    squads.position[s] = where_on(route, cells, V2::ZERO);
    entries::unflee_all(squads, columns, s, slot_of);
    let direction = if is_zero_approx(squads.home_distance[s]) { 1 } else { -1 };
    squads.direction[s] = direction;
    let way = match route {
        Some(route) => route.heading_at(cells).scale(direction as f64),
        None => V2::new(1.0, 0.0).scale(direction as f64),
    };
    squads.heading[s] = motion::bearing_to(V2::ZERO, way, squads.heading[s]);
    squads.order[s] = HOLD;
    squads.state[s] = locks::HOLDING;
    squads.reforming[s] = 1;
}

/// The breaking formations rout, in the order given: every lock on each ends, it drops its
/// flanks, and its units take fleeing entries where they stand. Returns, for each, the
/// listed friends of its faction still standing a unit that see it break, in the lists'
/// order.
pub fn breaks(
    squads: &mut Squads,
    columns: &mut Columns,
    roster: &Roster,
    breaking: &[Routing],
    ground: Option<&Ground>,
) -> Vec<Vec<usize>> {
    let slots: Vec<usize> = roster.listed.iter().map(|l| l.squad).collect();
    let mut out = Vec::new();
    for formation in breaking {
        let n = formation.listed;
        let squad = &roster.listed[n];
        let s = squad.squad;
        locks::release(squads, s, &slots);
        squads.flanks[s].clear();
        squads.state[s] = locks::ROUTING;
        squads.rally_ticks[s] = 0;
        let along = squads.front_distance[s] * CELLS;
        let on = where_on(formation.route.as_ref(), along, V2::ZERO);
        for m in &squad.living {
            let offset = columns.position[m.slot].sub(on);
            entries::flee(squads, columns, s, (m.id, m.slot), along, offset);
        }
        let seen: Vec<usize> = (0..roster.listed.len())
            .filter(|&o| {
                let friend = &roster.listed[o];
                o != n
                    && friend.faction == squad.faction
                    && !friend.living.is_empty()
                    && perception::detects(columns, friend, squad, ground)
            })
            .collect();
        out.push(seen);
    }
    out
}

#[cfg(test)]
pub(crate) mod tests;

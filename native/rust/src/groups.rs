//! Groups of a command forming up (FormationGroups, spec 30 round 3, part 4; spec 39 step
//! 5a). Two friendly groups at ease whose units come within `group_join` of each other - of
//! one command; or of different commands, both having fought and either both standing or
//! marching the same way along the same route - form up by one rule:
//! - of one command, they are one group again: the larger takes in the other;
//! - otherwise each keeps its own orders, unless a leader's trait says otherwise: a leader
//!   that "gathers" takes in a leaderless group it meets; a leader that "joins" brings its
//!   group into the other's command;
//! - two leaderless groups merge only if either is set to merge, the one with more cells'
//!   worth of units taking in the other;
//! - two led groups each keep their own, and units each started under the other's command
//!   go back to it.
//!
//! Every meeting is decided from one snapshot, nearest first; ties in size go to the groups'
//! seeded draws, never to ids or the order they are listed in (Decision 97). Meetings are
//! decided in turn, each on the groups as the ones before left them. Taking a group in and
//! sending units back (the events, the places, the commands) are GDScript's, in the order
//! given.

use std::cmp::Ordering;
use std::collections::HashSet;

use crate::columns::Columns;
use crate::contest;
use crate::grid::{Grid, MARGIN};
use crate::locks::{ARRIVED, HOLDING};
use crate::maths::{maxf, minf, snapped, V2};
use crate::roster::Roster;
use crate::squads::Squads;

const EPSILON: f64 = 0.000001;
/// Pairs of units two groups meeting may have and still be looked at pair by pair (gap).
const ALL_PAIRS: usize = 256;
/// SkirmishUnit.Order.HOLD.
const HOLD: i32 = 1;

/// What of a unit forming up reads that the crate doesn't hold: whether it leads
/// (leadership > 0), whether it has the "joins" and "gathers" traits, and the command it
/// started under (a code; `None`: none yet).
#[derive(Clone, Copy, Debug, Default)]
pub struct Follower {
    pub leads: bool,
    pub joins: bool,
    pub gathers: bool,
    pub origin: Option<u32>,
}

/// A group at ease with living units, in the lists' order, as the roster's squad of the
/// same index: its command and its route (codes: the same code, the same one), and per
/// living unit (in its order) what forming up reads of it.
pub struct Group {
    pub command: u32,
    pub route: u32,
    pub followers: Vec<Follower>,
}

/// What a meeting does, in turn: a group taken in by another (roster indices), or a led
/// group's units (ids, in its order) going back to the command of the group they started
/// under.
#[derive(Clone, Debug, PartialEq)]
pub enum Meeting {
    Taken { taker: usize, taken: usize },
    Returned { from: usize, home: usize, units: Vec<i64> },
}

/// A unit of a group as meetings move it: its id, cells' worth and what it carries.
#[derive(Clone, Copy)]
struct Held {
    id: i64,
    cells: i64,
    follower: Follower,
}

/// FormationGroups._form_up's decisions: what each meeting does, in the order it lands.
pub fn form_up(
    squads: &Squads,
    columns: &Columns,
    roster: &Roster,
    groups: &[Group],
    join: f64,
    seed: i64,
) -> Vec<Meeting> {
    let reaches: Vec<f64> = (0..groups.len())
        .map(|g| squads.width[roster.listed[g].squad] as f64 + back_row(columns, roster, g) as f64)
        .collect();
    let positions: Vec<V2> = roster.listed.iter().map(|l| squads.position[l.squad]).collect();
    let mut grids: Vec<Option<(Grid, f64)>> = (0..groups.len()).map(|_| None).collect();
    let mut meetings = Vec::new(); // (gap, lower draw, higher draw, one, other)
    for (one, other) in pairs_near(&positions, &reaches, join) {
        if roster.listed[one].faction != roster.listed[other].faction
            || !together(squads, roster, groups, one, other)
        {
            continue;
        }
        let gap = gap(columns, roster, one, other, &mut grids[other], join);
        if gap <= join + EPSILON {
            let a = contest::squad_draw(seed, roster.listed[one].id);
            let b = contest::squad_draw(seed, roster.listed[other].id);
            meetings.push((snapped(gap, 0.000001), a.min(b), a.max(b), one, other));
        }
    }
    meetings.sort_by(|a, b| {
        a.0.partial_cmp(&b.0).unwrap_or(Ordering::Equal).then(a.1.cmp(&b.1)).then(a.2.cmp(&b.2))
    });
    let mut held: Vec<Vec<Held>> = (0..groups.len())
        .map(|g| {
            let living = &roster.listed[g].living;
            (0..living.len())
                .map(|u| Held {
                    id: living[u].id,
                    cells: living[u].foot.0 * living[u].foot.1,
                    follower: groups[g].followers[u],
                })
                .collect()
        })
        .collect();
    let draws: Vec<i64> = roster.listed.iter().map(|l| contest::squad_draw(seed, l.id)).collect();
    let decide = Decide { roster, groups, draws: &draws };
    let mut gone = HashSet::new();
    let mut out = Vec::new();
    for &(_, _, _, one, other) in &meetings {
        if gone.contains(&one) || gone.contains(&other) {
            continue;
        }
        match decide.taker(&held, one, other) {
            Some(taker) => {
                let taken = if taker == one { other } else { one };
                let command = groups[taker].command;
                let mut moving = std::mem::take(&mut held[taken]);
                for unit in &mut moving {
                    unit.follower.origin.get_or_insert(command);
                }
                held[taker].extend(moving);
                gone.insert(taken);
                out.push(Meeting::Taken { taker, taken });
            }
            None if led(&held[one]) && led(&held[other]) => {
                out.extend(return_origins(&mut held, groups, one, other));
                out.extend(return_origins(&mut held, groups, other, one));
            }
            None => {}
        }
    }
    out
}

/// What a meeting is decided on: the roster, the groups and each group's draw.
struct Decide<'a> {
    roster: &'a Roster,
    groups: &'a [Group],
    draws: &'a [i64],
}

impl Decide<'_> {
    /// Which of two meeting groups takes in the other, or none if each keeps its own orders.
    fn taker(&self, held: &[Vec<Held>], one: usize, other: usize) -> Option<usize> {
        if self.groups[one].command == self.groups[other].command {
            return Some(self.larger(held, one, other));
        }
        let has = |g: usize, joins: bool| {
            held[g].iter().any(|u| {
                u.follower.leads && if joins { u.follower.joins } else { u.follower.gathers }
            })
        };
        let (one_joins, other_joins) = (has(one, true), has(other, true));
        if one_joins != other_joins {
            return Some(if one_joins { other } else { one }); // into the other's command
        }
        if one_joins {
            return Some(self.larger(held, one, other));
        }
        let one_gathers = has(one, false) && !led(&held[other]);
        let other_gathers = has(other, false) && !led(&held[one]);
        if one_gathers != other_gathers {
            return Some(if one_gathers { one } else { other });
        }
        let merging = self.roster.listed[one].merges || self.roster.listed[other].merges;
        if !led(&held[one]) && !led(&held[other]) && merging {
            return Some(self.larger(held, one, other));
        }
        None
    }

    /// The larger of two groups by cells' worth of units; a tie by their seeded draws.
    fn larger(&self, held: &[Vec<Held>], one: usize, other: usize) -> usize {
        let worth = |g: usize| held[g].iter().map(|u| u.cells).sum::<i64>();
        if (worth(one), self.draws[one]) > (worth(other), self.draws[other]) {
            one
        } else {
            other
        }
    }
}

fn led(held: &[Held]) -> bool {
    held.iter().any(|u| u.follower.leads)
}

/// Units of `from` that started under `home`'s command go back to it (both led), unless
/// none or all of them did.
fn return_origins(
    held: &mut [Vec<Held>],
    groups: &[Group],
    from: usize,
    home: usize,
) -> Vec<Meeting> {
    let command = groups[home].command;
    let back = |u: &Held| u.follower.origin == Some(command);
    let count = held[from].iter().filter(|u| back(u)).count();
    if count == 0 || count == held[from].len() {
        return vec![];
    }
    let (going, staying): (Vec<Held>, Vec<Held>) = held[from].iter().partition(|u| back(u));
    held[from] = staying;
    let units = going.iter().map(|u| u.id).collect();
    held[home].extend(going);
    vec![Meeting::Returned { from, home, units }]
}

/// True if the two groups may form up: one command's; or, of different commands, both
/// having fought (coming back together after a fight, not two forces merely posted near
/// each other) and either both standing or both marching the same way along the same
/// route - not passing each other on ways of their own.
fn together(squads: &Squads, roster: &Roster, groups: &[Group], one: usize, other: usize) -> bool {
    if groups[one].command == groups[other].command {
        return true;
    }
    let (a, b) = (roster.listed[one].squad, roster.listed[other].squad);
    if squads.fought[a] == 0 || squads.fought[b] == 0 {
        return false;
    }
    let standing = |s: usize| squads.state[s] == HOLDING || squads.state[s] == ARRIVED;
    if standing(a) && standing(b) {
        return true;
    }
    groups[one].route == groups[other].route
        && squads.direction[a] == squads.direction[b]
        && squads.order[a] == squads.order[b]
        && squads.order[a] != HOLD
}

/// The group's back row, from its units' ranks and depths.
fn back_row(columns: &Columns, roster: &Roster, g: usize) -> i64 {
    let mut rows = 0;
    for m in &roster.listed[g].living {
        rows = rows.max(columns.rank[m.slot] as i64 + m.foot.1);
    }
    rows
}

/// GroupPairs.near: the pairs (i < j) of groups whose positions lie within their reaches and
/// `join` of each other, i ascending and, for each i, j ascending. Found through a grid of
/// cells as wide as the largest such distance, so every pair lies in neighbouring cells.
pub fn pairs_near(positions: &[V2], reaches: &[f64], join: f64) -> Vec<(usize, usize)> {
    let mut cell = join;
    for &reach in reaches {
        cell = maxf(cell, 2.0 * reach + join);
    }
    let key = |p: V2| ((p.x as f64 / cell).floor() as i64, (p.y as f64 / cell).floor() as i64);
    let mut cells: std::collections::BTreeMap<(i64, i64), Vec<usize>> = Default::default();
    for (index, &p) in positions.iter().enumerate() {
        cells.entry(key(p)).or_default().push(index);
    }
    let mut pairs = Vec::new();
    for i in 0..positions.len() {
        let here = key(positions[i]);
        let mut found = Vec::new();
        for dx in -1..=1 {
            for dy in -1..=1 {
                for &j in cells.get(&(here.0 + dx, here.1 + dy)).map_or(&[][..], |v| v.as_slice()) {
                    let apart = positions[i].distance_to(positions[j]) as f64;
                    if j > i && apart <= reaches[i] + reaches[j] + join {
                        found.push(j);
                    }
                }
            }
        }
        found.sort_unstable();
        pairs.extend(found.into_iter().map(|j| (i, j)));
    }
    pairs
}

/// Cells between the two groups' nearest bodies, exact up to `join`; beyond it, more than
/// `join`. Small groups look at every pair of units; otherwise only the other's units that
/// could come within `join` of each unit, through a grid of them (made once, in `grid`).
fn gap(
    columns: &Columns,
    roster: &Roster,
    one: usize,
    other: usize,
    grid: &mut Option<(Grid, f64)>,
    join: f64,
) -> f64 {
    let (ours, theirs) = (&roster.listed[one].living, &roster.listed[other].living);
    let at = &columns.position;
    let mut least = f64::INFINITY;
    let mut look = |unit: &crate::roster::Member, k: usize| {
        let friend = &theirs[k];
        let apart = at[unit.slot].distance_to(at[friend.slot]) as f64;
        least = minf(least, apart - unit.radius() - friend.radius());
    };
    if ours.len() * theirs.len() > ALL_PAIRS {
        let (grid, widest) = grid.get_or_insert_with(|| {
            let points: Vec<V2> = theirs.iter().map(|m| at[m.slot]).collect();
            let widest = theirs.iter().map(|m| m.radius()).fold(0.0, maxf);
            (Grid::build(&points), widest)
        });
        for unit in ours {
            let within = join + unit.radius() + *widest + MARGIN;
            grid.near(at[unit.slot], within, |k| {
                look(unit, k as usize);
                true
            });
        }
    } else {
        for unit in ours {
            for k in 0..theirs.len() {
                look(unit, k);
            }
        }
    }
    maxf(least, 0.0)
}

#[cfg(test)]
mod tests;

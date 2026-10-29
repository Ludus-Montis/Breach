//! A group's units fallen behind go on as a group of their own (GroupSplit, spec 30 round 3,
//! part 4; spec 39 step 5a): a cluster of the group's units (each within `group_join` of
//! another) cut off from the rest, all further from their places than `walk_lost` - the frame
//! no longer waits for them (FormationWalk), unless no man is left behind - splits off. Each
//! new group's front is where its foremost unit stands along the route, and its units line
//! up across it by where they stand. Splits come in the order of their units' least seeded
//! draws, never by ids or lists (Decision 97). Nobody moves until every split is found, so
//! this decides them all; `groups::pass` makes the new groups (spec 39 step 8b5-xxx).

use std::cmp::Ordering;
use std::collections::HashSet;

use crate::columns::Columns;
use crate::contest;
use crate::grid::{Grid, MARGIN};
use crate::march::pour::{self, Frame, Pourer};
use crate::maths::{bearing_vector, maxf, snapped, V2};
use crate::roster::Roster;
use crate::route::Route;
use crate::squads::Squads;

/// A squad at ease on a route that may split, in the lists' order: its index in the roster,
/// whether its frame waits for every unit (no man left behind), its route, and per living
/// unit (in its order) whether it is out taking a downed foe and how far it is through a
/// swap (FormationShuffle.offset: ranks forward, columns across).
pub struct Candidate {
    pub listed: usize,
    pub waits: bool,
    pub route: Route,
    pub taking: Vec<bool>,
    pub offsets: Vec<V2>,
}

/// The numbers a split reads: `walk_lost`, `group_join`, the fight's seed and the cells in a
/// tile.
pub struct Rules {
    pub lost: f64,
    pub join: f64,
    pub seed: i64,
    pub cells_per_tile: f64,
}

/// A group to split off: the candidate it leaves (its index in the candidates), its units
/// (indices into the roster squad's living) in the order they line up, and its front (tiles
/// along the route).
#[derive(Clone, Debug, PartialEq)]
pub struct Split {
    pub candidate: usize,
    pub members: Vec<usize>,
    pub front: f64,
}

/// GroupSplit.step's decisions: every group splitting off, in the order they are numbered.
pub fn step(
    squads: &Squads,
    columns: &Columns,
    roster: &Roster,
    candidates: &[Candidate],
    rules: &Rules,
) -> Vec<Split> {
    let mut splits = Vec::new(); // (least draw, candidate, cluster)
    for (n, candidate) in candidates.iter().enumerate() {
        let behind = behind(squads, columns, roster, candidate, rules);
        for cluster in clusters(&behind, columns, roster, candidate.listed, rules.join) {
            let living = &roster.listed[candidate.listed].living;
            let least =
                cluster.iter().map(|&u| contest::draw(rules.seed, living[u].id)).min().unwrap();
            splits.push((least, n, cluster));
        }
    }
    splits.sort_by_key(|s| s.0);
    splits
        .into_iter()
        .map(|(_, n, cluster)| group(squads, columns, roster, &candidates[n], n, cluster, rules))
        .collect()
}

/// The candidate's units in their places, fallen behind them and cut off from the rest: no
/// unit of its cluster stands near its place. None if the frame waits for every unit, or
/// if every cluster is behind (the frame is where they are going).
fn behind(
    squads: &Squads,
    columns: &Columns,
    roster: &Roster,
    candidate: &Candidate,
    rules: &Rules,
) -> Vec<usize> {
    if candidate.waits {
        return vec![];
    }
    let listed = &roster.listed[candidate.listed];
    let slot = listed.squad;
    let (loose, fleeing): (HashSet<i64>, HashSet<i64>) = (
        squads.loose[slot].iter().copied().collect(),
        squads.fleeing[slot].iter().copied().collect(),
    );
    let placed: Vec<usize> = (0..listed.living.len())
        .filter(|&u| {
            let id = listed.living[u].id;
            !loose.contains(&id) && !fleeing.contains(&id) && !candidate.taking[u]
        })
        .collect();
    let frame = Frame {
        anchor: squads.position[slot],
        heading: squads.heading[slot],
        width: squads.width[slot],
        centre_shift: squads.centre_shift[slot],
    };
    let far: Vec<bool> = (0..listed.living.len())
        .map(|u| {
            let m = &listed.living[u];
            let unit = Pourer {
                id: m.id,
                at: V2::ZERO,
                rank: columns.rank[m.slot],
                column: columns.column[m.slot],
                footprint_width: m.foot.0 as i32,
                footprint_depth: m.foot.1 as i32,
                offset: candidate.offsets[u],
                walker: m.walker,
                taking: None,
            };
            columns.position[m.slot].distance_to(pour::place(&frame, &unit)) as f64 > rules.lost
        })
        .collect();
    if !placed.iter().any(|&u| far[u]) {
        return vec![]; // nobody has fallen behind: nothing to cluster
    }
    let mut out = Vec::new();
    let mut kept = false;
    for cluster in clusters(&placed, columns, roster, candidate.listed, rules.join) {
        if cluster.iter().all(|&u| far[u]) {
            out.extend(cluster);
        } else {
            kept = true;
        }
    }
    if kept {
        out
    } else {
        vec![]
    }
}

/// `units` (indices into the listed squad's living) in clusters: each within `join` + 1 of
/// another of its cluster, found through a grid of where they stand.
fn clusters(
    units: &[usize],
    columns: &Columns,
    roster: &Roster,
    listed: usize,
    join: f64,
) -> Vec<Vec<usize>> {
    let living = &roster.listed[listed].living;
    let points: Vec<V2> = units.iter().map(|&u| columns.position[living[u].slot]).collect();
    let join = join + 1.0;
    let grid = Grid::build(&points);
    let mut seen = vec![false; units.len()];
    let mut out = Vec::new();
    for start in 0..units.len() {
        if seen[start] {
            continue;
        }
        seen[start] = true;
        let mut cluster = vec![start];
        let mut i = 0;
        while i < cluster.len() {
            let at = points[cluster[i]];
            grid.near(at, join + MARGIN, |found| {
                let found = found as usize;
                if !seen[found] && points[found].distance_to(at) as f64 <= join {
                    seen[found] = true;
                    cluster.push(found);
                }
                true
            });
            i += 1;
        }
        out.push(cluster.into_iter().map(|k| units[k]).collect());
    }
    out
}

/// The group a cluster makes: its units lined up across the squad's heading by where they
/// stand (ties by their draws), its front where its foremost unit stands along the route.
fn group(
    squads: &Squads,
    columns: &Columns,
    roster: &Roster,
    candidate: &Candidate,
    n: usize,
    cluster: Vec<usize>,
    rules: &Rules,
) -> Split {
    let listed = &roster.listed[candidate.listed];
    let slot = listed.squad;
    let across = bearing_vector(squads.heading[slot]).orthogonal();
    let mut keyed: Vec<(f64, i64, usize)> = cluster
        .into_iter()
        .map(|u| {
            let m = &listed.living[u];
            let along = columns.position[m.slot].dot(across) as f64;
            (snapped(along, 0.0001), contest::draw(rules.seed, m.id), u)
        })
        .collect();
    keyed.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(Ordering::Equal).then(a.1.cmp(&b.1)));
    let direction = squads.direction[slot] as f64;
    let mut front = f64::NEG_INFINITY;
    for &(_, _, u) in &keyed {
        let at = columns.position[listed.living[u].slot];
        front = maxf(front, candidate.route.distance_of(at) * direction);
    }
    Split {
        candidate: n,
        members: keyed.into_iter().map(|k| k.2).collect(),
        front: front * direction / rules.cells_per_tile,
    }
}

#[cfg(test)]
mod tests;

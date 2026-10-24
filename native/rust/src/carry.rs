//! Bearing the wounded (FormationCarry, Decision 126; spec 39 step 6c): who picks up which
//! downed friend. Every free unit of a formation that tends its downed, beside a friendly
//! body no foe stands near (`guard_reach`) and that it can still move bearing, may pick it
//! up: the nearest pairs first, ties by the body's then the bearer's seeded draws - never
//! the lists (Decision 97) - each body and bearer once. GDScript keeps the bodies following
//! their bearers, dropping and setting them down, coming home, the bearers' pace and the
//! events, and lands what this decides.

use std::collections::HashSet;

use crate::columns::Columns;
#[cfg(test)]
use crate::content::BattleTuning;
use crate::contest;
use crate::grid::{Grid, MARGIN};
use crate::maths::{snapped, V2};
use crate::wounds::At;

/// The tuning the pick-up reads (BattleTuning's).
#[derive(Clone, Debug)]
pub struct CarryTuning {
    pub reach: f64,
    pub guard_reach: f64,
    pub body_weight: f64,
    pub per_strength: f64,
    pub easy: f64,
    pub most: f64,
    pub hauler: f64,
}

impl CarryTuning {
    /// The tuning as the binding is given it, in this order.
    pub fn of_slice(t: &[f64]) -> CarryTuning {
        CarryTuning {
            reach: t[0],
            guard_reach: t[1],
            body_weight: t[2],
            per_strength: t[3],
            easy: t[4],
            most: t[5],
            hauler: t[6],
        }
    }

    /// The battle tuning's.
    #[cfg(test)]
    pub fn of(t: &BattleTuning) -> CarryTuning {
        CarryTuning::of_slice(&[
            t.wounds_reach,
            t.wounds_guard_reach,
            t.wounds_body_weight,
            t.load_per_strength,
            t.load_easy,
            t.load_most,
            t.load_hauler,
        ])
    }
}

/// A living unit as the pick-up is given it: its id, store slot, faction (a code), whether
/// it already bears a body, the weight of its gear, its strength and its "hauler" level.
#[derive(Clone, Debug)]
pub struct Bearer {
    pub id: i64,
    pub slot: usize,
    pub faction: u32,
    pub carrying: bool,
    pub gear_weight: f64,
    pub strength: i64,
    pub hauler: i64,
}

/// A downed body lying (not borne): its id, store slot, faction (a code) and the cells it
/// covers (its footprint's width times its depth).
#[derive(Clone, Debug)]
pub struct Body {
    pub id: i64,
    pub slot: usize,
    pub faction: u32,
    pub cells: i64,
}

/// A squad as the pick-up is given it: its faction (a code), whether it tends its downed
/// now (told to, and not fighting or routing), its living and its downed bodies lying.
#[derive(Clone, Debug, Default)]
pub struct Carrying {
    pub faction: u32,
    pub tending: bool,
    pub living: Vec<Bearer>,
    pub downed: Vec<Body>,
}

/// The bodies picked up this tick, in the order they were: [the body, its bearer], each by
/// [its squad, its index in that squad's downed or living].
pub fn pick_up(
    columns: &Columns,
    squads: &[Carrying],
    seed: i64,
    tuning: &CarryTuning,
) -> Vec<(At, At)> {
    let mut pairs = pairs(columns, squads, seed, tuning);
    pairs.sort_by(|a, b| a.0.partial_cmp(&b.0).expect("a gap is a number"));
    let mut borne: HashSet<At> = HashSet::new(); // asked, never iterated
    let mut bearing: HashSet<At> = HashSet::new();
    let mut out = vec![];
    for (_, body, bearer) in pairs {
        if borne.contains(&body) || bearing.contains(&bearer) {
            continue;
        }
        borne.insert(body);
        bearing.insert(bearer);
        out.push((body, bearer));
    }
    out
}

/// A pair's key: [the gap, the body's draw, the bearer's draw].
type Key = (f64, i64, i64);

/// [key, body, bearer] for each friendly body a free unit of a formation that tends its
/// downed could pick up, as all stand.
pub fn pairs(
    columns: &Columns,
    squads: &[Carrying],
    seed: i64,
    tuning: &CarryTuning,
) -> Vec<(Key, At, At)> {
    let mut out = vec![];
    if !squads.iter().any(|s| s.tending) {
        return out;
    }
    let free = unguarded(columns, squads, tuning);
    for (s, squad) in squads.iter().enumerate() {
        if !squad.tending {
            continue;
        }
        let points: Vec<V2> = squad.living.iter().map(|u| columns.position[u.slot]).collect();
        let grid = Grid::build(&points);
        for &(bs, bu) in
            free.iter().filter(|&&(bs, bu)| squads[bs].downed[bu].faction == squad.faction)
        {
            let body = &squads[bs].downed[bu];
            let there = columns.position[body.slot];
            let mut found = vec![];
            grid.near(there, tuning.reach + MARGIN, |i| {
                found.push(i as usize);
                true
            });
            for u in found {
                let unit = &squad.living[u];
                let gap = points[u].distance_to(there) as f64;
                if unit.carrying || gap > tuning.reach || stage(unit, Some(body), tuning) >= 3 {
                    continue;
                }
                let key = (
                    snapped(gap, 0.000001),
                    contest::draw(seed, body.id),
                    contest::draw(seed, unit.id),
                );
                out.push((key, (bs, bu), (s, u)));
            }
        }
    }
    out
}

/// Each downed body with no foe standing within `guard_reach`, in the squads' order.
fn unguarded(columns: &Columns, squads: &[Carrying], tuning: &CarryTuning) -> Vec<At> {
    let standing: Vec<&Bearer> = squads.iter().flat_map(|s| &s.living).collect();
    let points: Vec<V2> = standing.iter().map(|u| columns.position[u.slot]).collect();
    let grid = Grid::build(&points);
    let mut free = vec![];
    for (s, squad) in squads.iter().enumerate() {
        for (u, body) in squad.downed.iter().enumerate() {
            let there = columns.position[body.slot];
            let mut guarded = false;
            grid.near(there, tuning.guard_reach + MARGIN, |i| {
                let other = standing[i as usize];
                guarded = other.faction != body.faction
                    && points[i as usize].distance_to(there) as f64 <= tuning.guard_reach;
                !guarded
            });
            if !guarded {
                free.push((s, u));
            }
        }
    }
    free
}

/// The unit's load stage bearing `body` (none: its gear alone) (UnitArms.stage_of): 0 easy,
/// 1 mild, 2 steep, 3 it can't move.
pub fn stage(unit: &Bearer, body: Option<&Body>, tuning: &CarryTuning) -> usize {
    let borne = body.map_or(0.0, |b| b.cells as f64 * tuning.body_weight);
    let carried = unit.gear_weight + borne;
    let limit = unit.strength as f64 * tuning.per_strength;
    let most = tuning.most + unit.hauler as f64 * tuning.hauler;
    if carried <= limit * tuning.easy {
        0
    } else if carried <= limit {
        1
    } else if carried <= limit * most {
        2
    } else {
        3
    }
}

#[cfg(test)]
mod tests;

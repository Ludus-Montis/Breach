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
use crate::entries;
use crate::frame::Frame;
use crate::ground::Ground;
use crate::locks;
use crate::maths::V2;
use crate::motion;
use crate::mover;
use crate::near::{Crowd, Foes, Steering};
use crate::roster::{self, Roster};
use crate::squads::Squads;
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

impl Rules {
    fn steering(&self) -> Steering {
        Steering { look: self.look, clear: self.clear, seed: self.seed }
    }
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
        let touch = (rules.reach, rules.arc);
        if let Some(at) =
            near.nearest_touching(roster, unit, where_, bearing, &close, touch, rules.seed)
        {
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
            let to = crowd.steer(unit, listed.id, at, place, &rules.steering());
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

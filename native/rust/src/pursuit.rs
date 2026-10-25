//! Retreat and pursuit (ScrumPursuit and FormationPursuit, Decisions 95, 103, 109 and 112;
//! spec 39 step 4c). When a formation retreats out of a fight, each unit of its enemies near
//! it may break ranks to chase it on its own, by its discipline and a seeded roll (`tempt`),
//! as far from where it broke away as its discipline leashes it. A chaser walks after the
//! nearest unit of the squad it chases (ties by the units' draws, Decision 97) while it can
//! see it; at its leash, too tired, out of sight or its quarry gone, it straggles back to its
//! place wherever its formation now is, and rejoins it there (`chase`). Chasers all pick
//! their quarry before any moves. A formation pursuing as a whole waits for its rearmost
//! (`lagging`) and closes on where its quarry's units stand (`spread`).

use std::collections::HashSet;

use crate::columns::Columns;
use crate::entries;
use crate::frame::Frame;
use crate::ghash::{self, Key};
use crate::ground::Ground;
use crate::maths::{bearing_vector, V2};
use crate::mover;
use crate::near::{Crowd, Foes, Steering};
use crate::reach;
use crate::rect::Rect;
use crate::roster::{self, Member, Roster};
use crate::squads::{Chaser, Squads};

pub mod whole;

const EPSILON: f64 = 0.000001;

/// A squad of the roster with units out chasing, in the lists' order: its index in the
/// roster, and where its loose units walk back to on the ground (TerrainStep.returns; none:
/// their places).
pub struct Chasing {
    pub listed: usize,
    pub homes: Vec<(i64, V2)>,
}

/// The tick's numbers: cells a second at speed 1, the tick's seconds, the share of its most
/// stamina below which a chaser gives up (`stamina_give_up`), and how it steers.
pub struct Rules {
    pub cells_per_second: f64,
    pub seconds: f64,
    pub give_up: f64,
    pub steering: Steering,
}

/// A chaser's walk this tick: its squad (listed index), the unit, where it heads, and
/// whether it is on its way back.
struct Walk<'a> {
    listed: usize,
    unit: &'a Member,
    heads: V2,
    returning: bool,
}

/// ScrumPursuit.step: each chaser walks at the march pace, paying the ground, after the
/// nearest unit of the squad it chases, or back to its place once its chase is over.
pub fn chase(
    squads: &mut Squads,
    columns: &mut Columns,
    roster: &Roster,
    chasing: &[Chasing],
    ground: Option<&Ground>,
    rules: &Rules,
) {
    let mut walks = Vec::new();
    let mut quarries: Vec<(usize, Foes)> = Vec::new(); // foe -> its units, where they stand
    for c in chasing {
        let listed = &roster.listed[c.listed];
        let squad = listed.squad;
        let basis = Frame::kept(squads, squad); // once a squad: no chase moves its frame
        for chaser in squads.chasers[squad].clone() {
            let id = chaser.unit;
            let unit = listed.living.iter().find(|m| m.id == id);
            let Some(unit) = unit.filter(|_| squads.loose[squad].contains(&id)) else {
                squads.chasers[squad].retain(|c| c.unit != id); // fallen, or back in its ranks
                continue;
            };
            let slot = unit.slot;
            let mut heads = match c.homes.iter().find(|(h, _)| *h == id) {
                Some(&(_, home)) => home,
                None => basis.place((columns.rank[slot], columns.column[slot]), unit.foot),
            };
            let mut returning = chaser.returning;
            if !returning {
                let at = columns.loose_at[slot];
                let quarry = quarry(squads, columns, roster, &chaser, unit, &mut quarries, rules);
                if done(&chaser, at, unit, quarry) {
                    back(squads, squad, id);
                    returning = true;
                } else if let Some(quarry) = quarry {
                    heads = quarry;
                }
            }
            walks.push(Walk { listed: c.listed, unit, heads, returning });
        }
    }
    if walks.is_empty() {
        return;
    }
    let crowd = Crowd::of(roster.bodies(squads, columns));
    let full: Vec<f64> = walks
        .iter()
        .map(|w| columns.speed[w.unit.slot] * rules.cells_per_second * rules.seconds)
        .collect(); // read before any walks: no chase changes a pace
    for (walk, full) in walks.iter().zip(full) {
        let (unit, slot) = (walk.unit, walk.unit.slot);
        let squad = roster.listed[walk.listed].squad;
        let at = columns.loose_at[slot];
        let to = crowd.steer(unit, roster.listed[walk.listed].id, at, walk.heads, &rules.steering);
        let mover = unit.mover(columns);
        let seconds = rules.seconds;
        let (stepped, bearing) =
            mover::step(ground, &unit.walker, &mover, at, to, full, seconds, None);
        columns.bearing[slot] = bearing;
        columns.loose_at[slot] = stepped; // placement: its step, by the mover (Decision 139)
        columns.next[slot] = stepped;
        if walk.returning && (stepped.distance_to(walk.heads) as f64) < EPSILON {
            squads.chasers[squad].retain(|c| c.unit != unit.id); // back: it rejoins its formation
            entries::unloose(squads, columns, squad, (unit.id, slot));
        }
    }
}

/// The chaser gives up and makes its way back.
fn back(squads: &mut Squads, squad: usize, unit_id: i64) {
    if let Some(chaser) = squads.chasers[squad].iter_mut().find(|c| c.unit == unit_id) {
        chaser.returning = true;
    }
}

/// Where the nearest unit of the chaser's foe stands, or none if the chase is over before it
/// looks: its foe gone, or it too tired. `quarries` keeps each foe's index.
fn quarry(
    squads: &Squads,
    columns: &Columns,
    roster: &Roster,
    chaser: &Chaser,
    unit: &Member,
    quarries: &mut Vec<(usize, Foes)>,
    rules: &Rules,
) -> Option<V2> {
    let foe = roster.by_id(chaser.foe)?;
    if roster.listed[foe].living.is_empty() {
        return None; // destroyed
    }
    if columns.stamina[unit.slot] < unit.max_stamina * rules.give_up {
        return None; // too tired (FormationStamina.gives_up)
    }
    let k = match quarries.iter().position(|(f, _)| *f == foe) {
        Some(k) => k,
        None => {
            quarries.push((foe, Foes::of_squad(roster, squads, columns, foe)));
            quarries.len() - 1
        }
    };
    let at = columns.loose_at[unit.slot];
    let index = &quarries[k].1;
    Some(index.nearest(roster, at, rules.steering.seed).map_or(at, |f| index.points[f]))
}

/// True if a chase is over: its quarry gone, the chaser at its leash, or the quarry out of
/// its sight.
fn done(chaser: &Chaser, at: V2, unit: &Member, quarry: Option<V2>) -> bool {
    let Some(quarry) = quarry else { return true };
    at.distance_to(chaser.from) as f64 >= chaser.leash
        || quarry.distance_to(at) as f64 > unit.detection
}

/// What tempts the units of an enemy of a retreating squad: per living unit of it, in its
/// order, the chance it breaks ranks and the leash it then chases on (FormationDiscipline);
/// how near (cells) the retreating squad it must be (`pursuit_tempted_within`); the tick and
/// the battle's seed.
pub struct Temptation<'a> {
    pub chances: &'a [(f64, f64)],
    pub within: f64,
    pub tick: i64,
    pub seed: i64,
}

/// ScrumPursuit._tempt: each of the enemy's units (listed `enemy`) near the retreating squad
/// (listed `fleeing`) may break ranks to chase it.
pub fn tempt(
    squads: &mut Squads,
    columns: &mut Columns,
    roster: &Roster,
    (enemy, fleeing): (usize, usize),
    temptation: &Temptation,
) {
    let near = Foes::of_squad(roster, squads, columns, fleeing);
    let listed = &roster.listed[enemy];
    let foe = roster.listed[fleeing].id;
    for (unit, &(chance, leash)) in listed.living.iter().zip(temptation.chances) {
        let loose: HashSet<i64> = squads.loose[listed.squad].iter().copied().collect();
        let at = roster::at(columns, &loose, unit);
        let nearest = near.nearest(roster, at, temptation.seed).map_or(at, |f| near.points[f]);
        if nearest.distance_to(at) as f64 > temptation.within {
            continue;
        }
        let keys = [Key::Int(temptation.tick), Key::Int(unit.id), Key::Str("chase")];
        if ghash::uniform(temptation.seed, &keys) >= chance {
            continue;
        }
        if !loose.contains(&unit.id) {
            entries::loosen(squads, columns, listed.squad, (unit.id, unit.slot), at);
        }
        let chaser = Chaser { unit: unit.id, foe, from: at, leash, returning: false };
        squads.chase(listed.squad, chaser);
    }
}

/// FormationPursuit._lagging: true if any of the listed squad's units not out chasing lags
/// more than `lag` cells behind its place along its heading - a formation pursues as a body,
/// its frame waiting for its rearmost (Decision 103).
pub fn lagging(squads: &Squads, columns: &Columns, roster: &Roster, n: usize, lag: f64) -> bool {
    let listed = &roster.listed[n];
    let squad = listed.squad;
    let forward = bearing_vector(squads.heading[squad]);
    let basis = Frame::kept(squads, squad);
    let loose: HashSet<i64> = squads.loose[squad].iter().copied().collect();
    listed.living.iter().any(|unit| {
        if squads.chasing(squad, unit.id) {
            return false; // one out on its own makes its own way (Decision 112)
        }
        let anchor = basis.place((columns.rank[unit.slot], columns.column[unit.slot]), unit.foot);
        let behind = anchor.sub(roster::at(columns, &loose, unit)).dot(forward) as f64;
        behind > lag
    })
}

/// FormationPursuit._spread: the cells the listed squad's units cover where they stand, its
/// frame's heading turning each footprint; none for a squad with none living.
pub fn spread(squads: &Squads, columns: &Columns, roster: &Roster, n: usize) -> Option<Rect> {
    let listed = &roster.listed[n];
    let heading = squads.heading[listed.squad];
    let loose: HashSet<i64> = squads.loose[listed.squad].iter().copied().collect();
    let area = |unit: &Member| {
        let (corner, size) =
            reach::area(roster::at(columns, &loose, unit), unit.foot.0, unit.foot.1, heading);
        Rect::new(corner, size)
    };
    let first = area(listed.living.first()?);
    Some(listed.living.iter().fold(first, |out, unit| out.merge(area(unit))))
}

#[cfg(test)]
#[path = "pursuit/tests.rs"]
mod tests;

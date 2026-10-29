//! The swaps under way of a squad re-forming (FormationShuffle, Decisions 46, 49 and 51;
//! spec 39 step 8a2-iii): each tick, last first, a swap any of whose units has fallen is
//! cancelled; the rest count down, and one whose time is up ends: its units take the places
//! it was heading for, the squad widens to take in any now standing outside its columns
//! (renumbering every column, the swaps still under way included, its centre shifted to
//! match), and it closes up the gaps the move left (compact). Which moves start is
//! `march::shuffle`'s. Since step 8b4-v the crate holds each squad's swaps under way too
//! (`Held`, `Squads::swaps`): which units each moves and where, the ticks left, the units it
//! passed; the "swapped" and "stepped_up" events stay GDScript's.

use serde::{Deserialize, Serialize};

use crate::columns::Columns;
use crate::compact::{self, Closer, Moving};
use crate::march::front::Cell;
use crate::march::shuffle::Started;
use crate::squads::Squads;

/// SkirmishUnit.State's first state not standing (DEAD): those before it stand.
const DEAD: i32 = 4;

/// A unit in a swap under way: its store slot, whether it still stands, the place (rank,
/// column) it is heading for and its footprint (width, depth).
#[derive(Clone, Copy, Debug)]
pub struct SwapUnit {
    pub slot: usize,
    pub alive: bool,
    pub to: (i32, i32),
    pub foot: (i32, i32),
}

/// A swap under way: the ticks it has left and its units, the mover first.
#[derive(Clone, Debug)]
pub struct Swap {
    pub ticks: i64,
    pub units: Vec<SwapUnit>,
}

/// What a tick of the swaps did: the swaps still under way (their index and ticks left, in
/// the list's order), how many columns every place moved left by the widenings, and the
/// swaps that ended, in the order they did, each with the units that stepped up after it
/// (their index into the squad's living units and the rank each stepped up to).
#[derive(Debug, Default, PartialEq)]
pub struct Ends {
    pub kept: Vec<(usize, i64)>,
    pub left: i32,
    pub ended: Vec<(usize, Vec<(usize, i32)>)>,
}

/// FormationShuffle.step's swaps for the squad in squad slot `squad`: `living` its standing
/// units as compact takes them, `units` the store slots of all its units (the fallen too:
/// a widening renumbers the whole frame), `swaps` its swaps under way in the list's order.
pub fn end(
    squads: &mut Squads,
    columns: &mut Columns,
    squad: usize,
    living: &[Closer],
    units: &[usize],
    swaps: &[Swap],
) -> Ends {
    let mut ends = Ends::default();
    let mut under_way: Vec<Option<Swap>> = swaps.iter().cloned().map(Some).collect();
    let mut within = false; // every living unit known to stand in the squad's columns
    for index in (0..under_way.len()).rev() {
        let Some(swap) = under_way[index].as_mut() else { continue };
        if swap.units.iter().any(|u| !u.alive) {
            under_way[index] = None;
            continue;
        }
        swap.ticks -= 1;
        if swap.ticks > 0 {
            continue;
        }
        let Some(ended) = under_way[index].take() else { continue };
        for u in &ended.units {
            (columns.rank[u.slot], columns.column[u.slot]) = u.to;
        }
        if !within || !ended.units.iter().all(|u| inside(squads, columns, squad, u)) {
            let left = widen(squads, columns, squad, living, units);
            for u in under_way.iter_mut().flatten().flat_map(|s| s.units.iter_mut()) {
                u.to.1 -= left;
            }
            ends.left += left;
            within = true;
        }
        let movers: Vec<Moving> = under_way
            .iter()
            .flatten()
            .flat_map(|s| &s.units)
            .map(|u| Moving { slot: u.slot, to: u.to, foot: u.foot })
            .collect();
        let moved = compact::compact(squads, columns, squad, living, &movers);
        let stepped = moved.into_iter().map(|n| (n, columns.rank[living[n].slot])).collect();
        ends.ended.push((index, stepped));
    }
    ends.kept = under_way
        .iter()
        .enumerate()
        .filter_map(|(index, swap)| swap.as_ref().map(|s| (index, s.ticks)))
        .collect();
    ends
}

/// True if the unit stands within the squad's columns.
fn inside(squads: &Squads, columns: &Columns, squad: usize, unit: &SwapUnit) -> bool {
    let column = columns.column[unit.slot];
    column >= 0 && column + unit.foot.0 <= squads.width[squad]
}

/// Takes in any columns living units now stand in outside 0..width (a spread, Decision 51):
/// renumbers the column of every unit in `units` and shifts the centre to match. Returns
/// how far left the columns' new 0 is of the old (0 if it is unchanged).
fn widen(
    squads: &mut Squads,
    columns: &mut Columns,
    squad: usize,
    living: &[Closer],
    units: &[usize],
) -> i32 {
    let width = squads.width[squad];
    let (mut left, mut right) = (0, width);
    for c in living {
        let column = columns.column[c.slot];
        left = left.min(column);
        right = right.max(column + c.foot.0);
    }
    if left == 0 && right == width {
        return 0;
    }
    squads.centre_shift[squad] += left as f64 + (right - left - width) as f64 / 2.0;
    for &slot in units {
        columns.column[slot] -= left;
    }
    squads.width[squad] = right - left;
    left
}

#[cfg(test)]
mod tests;

/// A unit in a swap the crate holds: its id and store slot, the place (rank, column) it is
/// heading for and the one it stood in as the swap began, and its footprint (width, depth).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct HeldUnit {
    pub id: i64,
    pub slot: usize,
    pub to: Cell,
    pub from: Cell,
    pub foot: (i32, i32),
}

/// A swap under way as the crate holds it (FormationShuffle's swaps): the ticks it has left
/// and takes in all, its units (the mover first) and the ids of the units it passed.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Held {
    pub ticks: i64,
    pub total: i64,
    pub units: Vec<HeldUnit>,
    pub passed: Vec<i64>,
}

/// A held swap that ended this tick: its mover's id (0: it had none), the ids of the units
/// it passed, and the units that stepped up after it (their index into the squad's living
/// units and the rank each stepped up to).
#[derive(Debug, PartialEq)]
pub struct Ended {
    pub mover: i64,
    pub passed: Vec<i64>,
    pub stepped: Vec<(usize, i32)>,
}

/// `end` over the swaps the squad in `squad` holds, a unit standing while its state is
/// before DEAD: those still under way are kept, counted down and moved left with the
/// widenings; those that ended or were cancelled are dropped. Returns those that ended, in
/// the order they did.
pub fn end_held(
    squads: &mut Squads,
    columns: &mut Columns,
    squad: usize,
    living: &[Closer],
    units: &[usize],
) -> Vec<Ended> {
    let was = std::mem::take(&mut squads.swaps[squad]);
    let swaps: Vec<Swap> = was
        .iter()
        .map(|held| Swap {
            ticks: held.ticks,
            units: held
                .units
                .iter()
                .map(|u| SwapUnit {
                    slot: u.slot,
                    alive: columns.state[u.slot] < DEAD,
                    to: u.to,
                    foot: u.foot,
                })
                .collect(),
        })
        .collect();
    let ends = end(squads, columns, squad, living, units, &swaps);
    squads.swaps[squad] = ends
        .kept
        .iter()
        .map(|&(index, ticks)| {
            let mut kept = was[index].clone();
            kept.ticks = ticks;
            for u in &mut kept.units {
                u.to.1 -= ends.left;
                u.from.1 -= ends.left;
            }
            kept
        })
        .collect();
    ends.ended
        .into_iter()
        .map(|(index, stepped)| Ended {
            mover: was[index].units.first().map_or(0, |u| u.id),
            passed: was[index].passed.clone(),
            stepped,
        })
        .collect()
}

/// The places the squad's held swaps claim (each unit's place it heads for and footprint, in
/// order) and, of their units, the indices of those in `living` (store slots), in order: what
/// `march::shuffle::start_moves` reads of the moves under way.
pub fn claims(
    squads: &Squads,
    squad: usize,
    living: &[usize],
) -> (Vec<(Cell, i32, i32)>, Vec<usize>) {
    let (mut claimed, mut busy) = (Vec::new(), Vec::new());
    for u in squads.swaps[squad].iter().flat_map(|held| &held.units) {
        claimed.push((u.to, u.foot.0, u.foot.1));
        busy.extend(living.iter().position(|&slot| slot == u.slot));
    }
    (claimed, busy)
}

/// Holds the moves `started` for the squad in `squad`, each unit given by its index into
/// `living` ([id, store slot] each), where it stands (`places`) and its footprint (`feet`).
pub fn begin(
    squads: &mut Squads,
    squad: usize,
    started: &[Started],
    living: &[(i64, usize)],
    places: &[Cell],
    feet: &[(i32, i32)],
) {
    for one in started {
        let units = one
            .plan
            .to
            .iter()
            .map(|&(n, to)| HeldUnit {
                id: living[n].0,
                slot: living[n].1,
                to,
                from: places[n],
                foot: feet[n],
            })
            .collect();
        let passed = one.plan.passed.iter().map(|&n| living[n].0).collect();
        squads.swaps[squad].push(Held { ticks: one.ticks, total: one.ticks, units, passed });
    }
}

/// How far each unit in the squad's held swaps is through its move, [ranks forward, columns
/// across], by id: the first swap it is in.
pub fn offsets(squads: &Squads, squad: usize) -> Vec<(i64, f32, f32)> {
    let mut out: Vec<(i64, f32, f32)> = Vec::new();
    for held in &squads.swaps[squad] {
        let progress = 1.0 - held.ticks as f64 / held.total as f64;
        for u in &held.units {
            if out.iter().any(|&(id, _, _)| id == u.id) {
                continue;
            }
            let ranks = f64::from(u.from.0 - u.to.0) * progress;
            let across = f64::from(u.to.1 - u.from.1) * progress;
            out.push((u.id, ranks as f32, across as f32));
        }
    }
    out
}

/// The squad's held swaps' units as compact takes them (`Moving`), in order.
pub fn moving(squads: &Squads, squad: usize) -> Vec<Moving> {
    squads.swaps[squad]
        .iter()
        .flat_map(|held| &held.units)
        .map(|u| Moving { slot: u.slot, to: u.to, foot: u.foot })
        .collect()
}

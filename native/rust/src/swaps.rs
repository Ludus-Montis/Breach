//! The swaps under way of a squad re-forming (FormationShuffle, Decisions 46, 49 and 51;
//! spec 39 step 8a2-iii): each tick, last first, a swap any of whose units has fallen is
//! cancelled; the rest count down, and one whose time is up ends: its units take the places
//! it was heading for, the squad widens to take in any now standing outside its columns
//! (renumbering every column, the swaps still under way included, its centre shifted to
//! match), and it closes up the gaps the move left (compact). Which moves start is
//! `march::shuffle`'s; the swaps' list, its events and the "swapped" and "stepped_up" events
//! stay GDScript's until the crate holds the tick's state (step 8b).

use crate::columns::Columns;
use crate::compact::{self, Closer, Moving};
use crate::squads::Squads;

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

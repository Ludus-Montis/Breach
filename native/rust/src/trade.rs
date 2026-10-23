//! Trading places while regrouping (ScrumTrade, Decision 110; spec 39 step 4b): a unit
//! walking back to its place that has come no nearer than its closest for
//! `scrum_trade_seconds` - its own ranks in the way - trades places with the friend whose
//! place is nearest it, if that place is nearer it than its own and the two are
//! interchangeable (one kind, one band, one footprint); the friend walks to the place it
//! left. With no such friend, its own place is the nearest: it takes it. Places stay one
//! unit each. Stalled units trade in the order of their seeded draws, never the list's
//! (Decision 97).

use crate::columns::Columns;
use crate::contest;
use crate::entries::{self, Key, Value};
use crate::frame::Frame;
use crate::grid::{self, Grid};
use crate::maths::{snapped, V2};
use crate::squads::Squads;

/// Cells nearer than its closest yet that count as getting nearer.
const PROGRESS: f64 = 0.001;

/// Notes how near the loose unit in store slot `slot` stands to its place, `gap` cells off:
/// a new closest, or `seconds` more stuck.
pub fn track(columns: &mut Columns, slot: usize, gap: f64, seconds: f64) {
    if gap < columns.closest[slot] - PROGRESS {
        entries::keep(columns, slot, Key::Closest, Value::Real(gap));
        entries::keep(columns, slot, Key::Stuck, Value::Real(0.0));
    } else {
        let stuck = columns.stuck[slot] + seconds;
        entries::keep(columns, slot, Key::Stuck, Value::Real(stuck));
    }
}

/// A unit of the trading squad: its id, store slot, kind (its definition, as a code),
/// preferred band and footprint (width, depth), and whether it is living. The living come
/// first, in the squad's order, then any loose units that aren't.
pub struct Trader {
    pub id: i64,
    pub slot: usize,
    pub kind: u32,
    pub preferred: i64,
    pub foot: (i64, i64),
    pub living: bool,
}

impl Trader {
    fn place(&self, columns: &Columns) -> (i32, i32) {
        (columns.rank[self.slot], columns.column[self.slot])
    }

    fn swaps_with(&self, other: &Trader) -> bool {
        self.kind == other.kind && self.preferred == other.preferred && self.foot == other.foot
    }
}

/// The squad trading: its store slot, its units, the battle's seed and the seconds stuck
/// after which a unit trades.
pub struct Trading<'a> {
    pub squad: usize,
    pub units: &'a [Trader],
    pub seed: i64,
    pub after: f64,
}

/// The places as the living hold them: each one's anchor, a grid over them, who holds each
/// (an index into the units), where each unit's is, and where each unit comes in the
/// living units.
struct Places {
    points: Vec<V2>,
    grid: Grid,
    held: Vec<usize>,
    of: Vec<Option<usize>>,
    order: Vec<usize>,
}

impl Places {
    fn of_living(columns: &Columns, trading: &Trading, frame: &Frame) -> Places {
        let living: Vec<usize> =
            (0..trading.units.len()).filter(|&u| trading.units[u].living).collect();
        let points: Vec<V2> = living
            .iter()
            .map(|&u| frame.place(trading.units[u].place(columns), trading.units[u].foot))
            .collect();
        let mut of = vec![None; trading.units.len()];
        for (index, &u) in living.iter().enumerate() {
            of[u] = Some(index);
        }
        let order = of.iter().map(|index| index.unwrap_or(usize::MAX)).collect();
        Places { grid: Grid::build(&points), points, held: living, of, order }
    }

    /// Notes that the two traded places: anchors stay with the places.
    fn traded(&mut self, unit: usize, friend: usize) {
        let (mine, theirs) = (self.of[unit].unwrap(), self.of[friend].unwrap());
        self.held[mine] = friend;
        self.held[theirs] = unit;
        self.of[unit] = Some(theirs);
        self.of[friend] = Some(mine);
    }
}

/// Each stalled unit of the squad trades places, if a friend's place is nearer it; one that
/// can't is as near its place as the press allows, and takes it there and then.
pub fn trade(squads: &mut Squads, columns: &mut Columns, trading: &Trading) {
    let squad = trading.squad;
    let units = trading.units;
    let index_of = |id: i64| units.iter().position(|u| u.id == id);
    let mut stalled: Vec<usize> = squads.loose[squad]
        .iter()
        .filter_map(|&id| index_of(id))
        .filter(|&u| columns.stuck[units[u].slot] >= trading.after)
        .collect();
    if stalled.is_empty() {
        return;
    }
    stalled.sort_by_key(|&u| contest::draw(trading.seed, units[u].id));
    let frame = Frame::kept(squads, squad); // once: a trade moves units, not the frame
                                            // kept as they trade, unless one stalled holds none: then looked at afresh each time
    let mut kept = stalled
        .iter()
        .all(|&u| units[u].living)
        .then(|| Places::of_living(columns, trading, &frame));
    for &u in &stalled {
        let unit = &units[u];
        if !squads.loose[squad].contains(&unit.id) {
            continue;
        }
        let at = columns.loose_at[unit.slot];
        let fresh;
        let places = match &kept {
            Some(places) => places,
            None => {
                fresh = Places::of_living(columns, trading, &frame);
                &fresh
            }
        };
        if let Some(friend) = nearest_place(squads, columns, trading, (u, at), (places, &frame)) {
            swap(squads, columns, squad, unit, &units[friend]);
            if let Some(places) = kept.as_mut() {
                places.traded(u, friend);
            }
        } else if squads.stance[squad].is_none() {
            entries::unloose(squads, columns, squad, (unit.id, unit.slot));
            columns.position[unit.slot] = frame.place(unit.place(columns), unit.foot);
        }
    }
}

/// The interchangeable friend whose place is nearest `at`, and nearer than the unit's own;
/// none if none. Ties by the friends' draws, then the living units' order.
fn nearest_place(
    squads: &Squads,
    columns: &Columns,
    trading: &Trading,
    me: (usize, V2),
    lookup: (&Places, &Frame),
) -> Option<usize> {
    let (u, at) = me;
    let (places, frame) = lookup;
    let unit = &trading.units[u];
    let own = frame.place(unit.place(columns), unit.foot).distance_to(at) as f64;
    let mut best: Option<(usize, (f64, i64, usize))> = None;
    places.grid.near(at, own + grid::MARGIN, |index| {
        let index = index as usize;
        let friend = places.held[index];
        let other = &trading.units[friend];
        if friend == u || !other.swaps_with(unit) || squads.chasing(trading.squad, other.id) {
            return true;
        }
        let there = places.points[index].distance_to(at) as f64;
        if there >= own - PROGRESS {
            return true;
        }
        let key =
            (snapped(there, 0.000001), contest::draw(trading.seed, other.id), places.order[friend]);
        if best.is_none_or(|(_, held)| key < held) {
            best = Some((friend, key));
        }
        true
    });
    best.map(|(friend, _)| friend)
}

/// The two swap places; the friend, loose if it wasn't, walks to its new one.
fn swap(squads: &mut Squads, columns: &mut Columns, squad: usize, unit: &Trader, friend: &Trader) {
    let place = unit.place(columns);
    let theirs = friend.place(columns);
    (columns.rank[unit.slot], columns.column[unit.slot]) = theirs;
    (columns.rank[friend.slot], columns.column[friend.slot]) = place;
    if !squads.loose[squad].contains(&friend.id) {
        let at = columns.position[friend.slot];
        entries::loosen(squads, columns, squad, (friend.id, friend.slot), at);
    }
    for moved in [unit.slot, friend.slot] {
        entries::forget(columns, moved, &[Key::Closest, Key::Stuck]);
    }
}

#[cfg(test)]
mod tests;

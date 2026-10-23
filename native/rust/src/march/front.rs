//! Who stands where in a squad's frame (SquadPlaces), and where a unit behind the front
//! steps up to when a front place falls free (ShuffleFront, Decision 49): the nearest free
//! place in the front rank, within the squad's columns or - for one joined in a fight -
//! spread within the combat width (Decision 51). The free places are found once a planning
//! (`open`), so each unit weighs those few, not every column (bounded-work).

use std::collections::{BTreeMap, BTreeSet};

use super::shuffle::{Plan, Ranked, Squad};

/// ShuffleFront.EPSILON.
const EPSILON: f64 = 0.000001;

/// A cell of a frame: (rank, column).
pub type Cell = (i32, i32);

/// The cells a footprint `width` by `depth` covers at `place`.
pub fn cells_of(place: Cell, width: i32, depth: i32) -> impl Iterator<Item = Cell> {
    (place.0..place.0 + depth)
        .flat_map(move |rank| (place.1..place.1 + width).map(move |column| (rank, column)))
}

/// SquadPlaces.occupancy: the cells the living units' footprints cover, each with the
/// units on it in their order, as they stand while the moves are planned.
pub struct Occupancy {
    pub cells: BTreeMap<Cell, Vec<usize>>,
}

impl Occupancy {
    pub fn of(units: &[Ranked]) -> Occupancy {
        let mut cells: BTreeMap<Cell, Vec<usize>> = BTreeMap::new();
        for (index, unit) in units.iter().enumerate() {
            for cell in cells_of(unit.place, unit.footprint_width, unit.footprint_depth) {
                cells.entry(cell).or_default().push(index);
            }
        }
        Occupancy { cells }
    }

    fn on(&self, cell: Cell) -> &[usize] {
        self.cells.get(&cell).map_or(&[], Vec::as_slice)
    }

    /// SquadPlaces.vacant: whether the unit's footprint at `place` is free of everyone but
    /// the units `moving`, and of every cell `claimed`.
    pub fn vacant(
        &self,
        place: Cell,
        unit: &Ranked,
        moving: &[usize],
        claimed: &BTreeSet<Cell>,
    ) -> bool {
        cells_of(place, unit.footprint_width, unit.footprint_depth).all(|cell| {
            !claimed.contains(&cell) && self.on(cell).iter().all(|other| moving.contains(other))
        })
    }

    /// SquadPlaces.ahead: the units in the row directly ahead of `units[me]`, across its
    /// columns, in their order.
    pub fn ahead(&self, units: &[Ranked], me: usize) -> Vec<usize> {
        let unit = &units[me];
        let mut out: Vec<usize> = Vec::new();
        for column in unit.place.1..unit.place.1 + unit.footprint_width {
            for &other in self.on((unit.place.0 - 1, column)) {
                if other != me && !out.contains(&other) {
                    out.push(other);
                }
            }
        }
        out.sort_unstable();
        out
    }
}

/// The cells claimed by the moves planned so far, and the columns they span.
pub struct Claimed {
    pub cells: BTreeSet<Cell>,
    pub left: i32,
    pub right: i32,
}

impl Claimed {
    pub fn new() -> Claimed {
        Claimed { cells: BTreeSet::new(), left: 1 << 30, right: -(1 << 30) }
    }

    /// FormationShuffle._claim: claims the cells a footprint `width` by `depth` covers at
    /// `place`.
    pub fn claim(&mut self, place: Cell, width: i32, depth: i32) {
        self.cells.extend(cells_of(place, width, depth));
        if depth > 0 && width > 0 {
            self.left = self.left.min(place.1);
            self.right = self.right.max(place.1 + width);
        }
    }
}

/// ShuffleFront.open: the front rank's columns whose first cell nobody stands in or has
/// claimed, across every column any unit may reach: places only fill as moves are
/// planned, so a place free to a unit later is among them.
pub fn open(squad: &Squad, units: &[Ranked], claimed: &Claimed, occupied: &Occupancy) -> Vec<i32> {
    let reach =
        units.iter().fold(squad.combat_width, |reach, unit| reach.max(unit.footprint_width));
    (-reach..squad.width + reach)
        .filter(|&column| {
            let cell = (0, column);
            !occupied.cells.contains_key(&cell) && !claimed.cells.contains(&cell)
        })
        .collect()
}

/// ShuffleFront.free_place: a front-preferring unit's move to the nearest free place in the
/// front rank (`open`'s columns); none if there is none.
pub fn free_place(
    squad: &Squad,
    units: &[Ranked],
    me: usize,
    claimed: &Claimed,
    occupied: &Occupancy,
    open: &[i32],
) -> Option<Plan> {
    let unit = &units[me];
    if unit.preferred_position != 0 {
        return None;
    }
    let spreading = squad.fighting && unit.joined && squad.combat_width > 0;
    let reach = if spreading { squad.combat_width } else { unit.footprint_width };
    let at = unit.place;
    let mut best: Option<Plan> = None;
    for &column in open {
        if column < -reach || column > squad.width + reach - unit.footprint_width {
            continue;
        }
        let place = (0, column);
        let inside = column >= 0 && column + unit.footprint_width <= squad.width;
        if !inside && !within_combat_width(squad, column, unit, claimed) {
            continue;
        }
        if !occupied.vacant(place, unit, &[me], &claimed.cells) {
            continue;
        }
        let distance = at.0.max((column - at.1).abs());
        if best.as_ref().is_none_or(|b| distance < b.distance) {
            best = Some(Plan { mover: me, passed: Vec::new(), to: vec![(me, place)], distance });
        }
    }
    best
}

/// ShuffleFront._within_combat_width: whether the unit at `column` keeps the line, planned
/// moves included, within the combat width, and no more than half a column past either
/// edge of the lane (Decision 51).
fn within_combat_width(squad: &Squad, column: i32, unit: &Ranked, claimed: &Claimed) -> bool {
    if squad.combat_width <= 0 {
        return false;
    }
    let left = 0.min(column).min(claimed.left);
    let right = squad.width.max(column + unit.footprint_width).max(claimed.right);
    let edge = squad.combat_width as f64 / 2.0 + 0.5;
    let lateral = column as f64 - squad.width as f64 / 2.0 + squad.centre_shift;
    right - left <= squad.combat_width
        && lateral >= -edge - EPSILON
        && lateral + unit.footprint_width as f64 <= edge + EPSILON
}

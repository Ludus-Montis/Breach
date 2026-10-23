//! Re-forming (FormationShuffle, Decisions 46 and 49, specs/22-formation-feel-test.md):
//! after a reinforcement or a death, units move toward the places their band prefers.
//! - **Into free space first.** A front-preferring unit behind the front takes the nearest
//!   free place in the front rank (`front`).
//! - **Otherwise through.** A unit with a stronger claim (its band further forward, or the
//!   same band and a higher priority) moves forward a rank past the units directly ahead in
//!   its columns. Passed units that fit take its back row; any too wide or too deep move
//!   back, through the formation, to the nearest free space that fits them.
//! - **How long:** as far as the furthest unit travels, at the slowest involved unit's
//!   speed, slowed by crowding while the squad fights.
//!
//! This plans the moves a re-forming squad can start; the squad keeps its swaps under way,
//! counts them down and ends them (FormationShuffle.step).

use super::front::{self, Cell, Claimed, Occupancy};
use crate::maths::minf;

/// FormationShuffle.SEARCH_RANKS: how far behind its place a passed unit may look for free
/// space.
const SEARCH_RANKS: i32 = 64;
/// FormationShuffle.TERRAIN_FACTOR.
const TERRAIN_FACTOR: f64 = 1.0;
/// FormationShuffle.EPSILON.
const EPSILON: f64 = 0.000001;
/// SkirmishSquad.RANK_DEPTH: a rank in tiles.
const RANK_DEPTH: f64 = 1.0 / 64.0;

/// A living unit of the squad, in its order: its place (rank, column), footprint, band and
/// priority, pace this tick, and whether it joined as a reinforcement.
#[derive(Clone, Copy, Debug)]
pub struct Ranked {
    pub place: Cell,
    pub footprint_width: i32,
    pub footprint_depth: i32,
    pub preferred_position: i32,
    pub position_priority: i32,
    pub speed: f64,
    pub joined: bool,
}

/// What of the squad a re-form reads: its columns, its lane's combat width, its centre
/// shift and whether it fights.
#[derive(Clone, Copy)]
pub struct Squad {
    pub width: i32,
    pub combat_width: i32,
    pub centre_shift: f64,
    pub fighting: bool,
}

/// A move: who moves (indices into the living units), where each goes (the mover first,
/// then the passed in order), and how many ranks the furthest travels.
#[derive(Clone, Debug, PartialEq)]
pub struct Plan {
    pub mover: usize,
    pub passed: Vec<usize>,
    pub to: Vec<(usize, Cell)>,
    pub distance: i32,
}

/// A planned move and the ticks it takes.
#[derive(Clone, Debug, PartialEq)]
pub struct Started {
    pub plan: Plan,
    pub ticks: i64,
}

/// FormationShuffle.stronger: whether `unit` has the stronger claim to a forward place.
fn stronger(unit: &Ranked, other: &Ranked) -> bool {
    if unit.preferred_position != other.preferred_position {
        return unit.preferred_position < other.preferred_position;
    }
    unit.position_priority > other.position_priority
}

/// PlaceOrder.front_first: the indices of `places`, front rank first, each rank left to
/// right. Two on one place (which a frame never holds) keep their order.
pub fn front_first(places: &[Cell]) -> Vec<usize> {
    let mut order: Vec<usize> = (0..places.len()).collect();
    order.sort_by_key(|&index| places[index]);
    order
}

/// FormationShuffle._start_moves: every move the squad can start, front first, given the
/// moves under way (`swaps`: where each of their units goes, with its footprint's width and
/// depth; `busy`: the living units in them). `pace` is [ticks' seconds, cells a second at speed 1, crowding in a fight].
pub fn start_moves(
    squad: &Squad,
    units: &[Ranked],
    swaps: &[(Cell, i32, i32)],
    busy: &[usize],
    pace: [f64; 3],
) -> Vec<Started> {
    let [tick_seconds, travel_scale, scrum_crowding] = pace;
    let mut claimed = Claimed::new();
    for &(place, width, depth) in swaps {
        claimed.claim(place, width, depth);
    }
    let mut busy = busy.to_vec();
    let occupied = Occupancy::of(units);
    let places: Vec<Cell> = units.iter().map(|unit| unit.place).collect();
    let order = front_first(&places);
    let ordered: Vec<Ranked> = order.iter().map(|&index| units[index]).collect();
    let open = front::open(squad, &ordered, &claimed, &occupied);
    let mut started = Vec::new();
    for &me in &order {
        if units[me].place.0 == 0 || busy.contains(&me) {
            continue;
        }
        let plan = front::free_place(squad, units, me, &claimed, &occupied, &open)
            .or_else(|| through(squad, units, me, &busy, &claimed, &occupied));
        let Some(plan) = plan else {
            continue;
        };
        let slowest =
            plan.to.iter().fold(f64::INFINITY, |least, &(unit, _)| minf(least, units[unit].speed));
        if slowest <= 0.0 {
            continue;
        }
        let crowding = if squad.fighting { scrum_crowding } else { 1.0 };
        let pace = travel_scale * slowest * TERRAIN_FACTOR * crowding;
        let seconds = plan.distance as f64 * RANK_DEPTH / pace;
        let ticks = ((seconds / tick_seconds - EPSILON).ceil() as i64).max(1);
        for &(unit, place) in &plan.to {
            busy.push(unit);
            claimed.claim(place, units[unit].footprint_width, units[unit].footprint_depth);
        }
        started.push(Started { plan, ticks });
    }
    started
}

/// FormationShuffle._through: moving forward a rank past weaker units ahead; none if it
/// can't (Decision 49).
fn through(
    squad: &Squad,
    units: &[Ranked],
    me: usize,
    busy: &[usize],
    claimed: &Claimed,
    occupied: &Occupancy,
) -> Option<Plan> {
    let ahead = occupied.ahead(units, me);
    let unit = &units[me];
    if ahead.is_empty() || ahead.iter().any(|&a| busy.contains(&a) || !stronger(unit, &units[a])) {
        return None;
    }
    let at = unit.place;
    let mut to = vec![(me, (at.0 - 1, at.1))];
    let back_row = at.0 + unit.footprint_depth - 1;
    let mut moving = ahead.clone();
    moving.push(me);
    let mut taken = Claimed::new(); // the cells this move claims, over those claimed already
    taken.claim(to[0].1, unit.footprint_width, unit.footprint_depth);
    let mut distance = 1;
    for &passed in &ahead {
        let was = units[passed].place;
        let mut place = (back_row, was.1);
        if !fits_back_row(unit, at, &units[passed], was) {
            place = nearest_free_behind(squad, units, passed, &moving, &taken, occupied, claimed)?;
            distance = distance.max((place.0 - was.0).max((place.1 - was.1).abs()));
        }
        to.push((passed, place));
        taken.claim(place, units[passed].footprint_width, units[passed].footprint_depth);
    }
    Some(Plan { mover: me, passed: ahead, to, distance })
}

/// FormationShuffle._nearest_free_behind: the nearest place behind the unit where its whole
/// footprint is free of this move's claims (`taken`), everyone not `moving`, and the moves
/// planned before it (`claimed`).
fn nearest_free_behind(
    squad: &Squad,
    units: &[Ranked],
    me: usize,
    moving: &[usize],
    taken: &Claimed,
    occupied: &Occupancy,
    claimed: &Claimed,
) -> Option<Cell> {
    let unit = &units[me];
    let at = unit.place;
    let mut best: Option<Cell> = None;
    let mut best_key = (1i32 << 30, 0i32, 0i32);
    for rank in at.0 + 1..at.0 + SEARCH_RANKS {
        for column in 0..squad.width - unit.footprint_width + 1 {
            let place = (rank, column);
            let lateral = (column - at.1).abs();
            let key = ((rank - at.0).max(lateral), lateral, rank);
            if key < best_key
                && occupied.vacant(place, unit, moving, &taken.cells)
                && front::cells_of(place, unit.footprint_width, unit.footprint_depth)
                    .all(|cell| !claimed.cells.contains(&cell))
            {
                best = Some(place);
                best_key = key;
            }
        }
    }
    best
}

/// FormationShuffle._fits_back_row: whether a passed unit (at `was`) fits the row the mover
/// (at `at`) leaves: one rank deep, directly ahead, within its columns.
fn fits_back_row(unit: &Ranked, at: Cell, passed: &Ranked, was: Cell) -> bool {
    passed.footprint_depth == 1
        && was.0 == at.0 - 1
        && was.1 >= at.1
        && was.1 + passed.footprint_width <= at.1 + unit.footprint_width
}

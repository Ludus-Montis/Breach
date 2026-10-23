//! Fights start where units meet (ScrumEngage, Decision 88, spec 27 round 6; spec 39 step
//! 4b): two hostile squads whose units come within `reach_engage` of each other lock into a
//! fight, whatever their faces. A squad already fighting is joined by the one that reached
//! it, and keeps its own foe. Any standing squad may be engaged; only one free to fight
//! (locks::can_engage, unlocked, unflanked, not fighting) seeks it (Decision 111). Every free
//! squad picks the nearest hostile it has come within reach of, all from one snapshot before
//! any lock lands, a tie going to the squads' seeded draws - never to the order they are
//! listed in (Decision 97).

use crate::columns::Columns;
use crate::contest;
use crate::grid::{self, Grid};
use crate::locks::{self, Locking, DESTROYED, FIGHTING};
use crate::maths::{maxf, minf, snapped, V2};
use crate::reach;
use crate::rect::Rect;
use crate::squads::Squads;

/// Cells added to how far apart two squads' boxes may be and still have units within
/// reach: far above float rounding, so no pair is missed.
const MARGIN: f64 = 0.01;
/// The hair added to `reach_engage` in every test against it.
const HAIR: f64 = 0.000001;

/// A listed squad as engaging reads it: its store slot, faction, whether units joined it on
/// the march (for its lock), and its living units: their store slots, where each stands
/// (ScrumReach.at) and its body's radius.
pub struct Side {
    pub squad: usize,
    pub faction: u32,
    pub joined: bool,
    pub slots: Vec<i32>,
    pub at: Vec<V2>,
    pub radii: Vec<f64>,
}

/// A side's box round where its living units stand, and its widest body's radius.
struct Span {
    area: Rect,
    widest: f64,
}

fn span(side: &Side) -> Span {
    let mut area = Rect::default();
    let mut widest = 0.0;
    for (n, &at) in side.at.iter().enumerate() {
        area = if n == 0 { Rect::new(at, V2::ZERO) } else { area.expand(at) };
        widest = maxf(widest, side.radii[n]);
    }
    Span { area, widest }
}

/// How far apart two boxes are (0 where they meet): the gap's axes in GDScript floats, its
/// length in a `Vector2`.
fn apart(one: Rect, other: Rect) -> f64 {
    let (one_end, other_end) = (one.end(), other.end());
    let across = maxf(
        maxf(
            other.position.x as f64 - one_end.x as f64,
            one.position.x as f64 - other_end.x as f64,
        ),
        0.0,
    );
    let down = maxf(
        maxf(
            other.position.y as f64 - one_end.y as f64,
            one.position.y as f64 - other_end.y as f64,
        ),
        0.0,
    );
    V2::new(across as f32, down as f32).length() as f64
}

/// Locks every free squad of `sides` (in the lists' order) onto the nearest hostile its
/// units have come within `reach_engage` of. Returns each lock as (its side's index, the
/// foe's id), in the lists' order.
pub fn step(
    squads: &mut Squads,
    columns: &mut Columns,
    sides: &[Side],
    reach_engage: f64,
    seed: i64,
) -> Vec<(usize, i64)> {
    let spans: Vec<Span> = sides.iter().map(span).collect();
    let mut grids: Vec<Option<Grid>> = sides.iter().map(|_| None).collect();
    let mut picks = Vec::new();
    for (n, side) in sides.iter().enumerate() {
        if !free(squads, side.squad) {
            continue;
        }
        if let Some(foe) = nearest(squads, sides, (n, &spans), &mut grids, (reach_engage, seed)) {
            picks.push((n, squads.id[sides[foe].squad]));
        }
    }
    for &(n, foe) in &picks {
        let side = &sides[n];
        let locking = Locking { squad: side.squad, living: &side.slots, joined: side.joined };
        locks::lock(squads, columns, &locking, foe);
    }
    picks
}

fn free(squads: &Squads, squad: usize) -> bool {
    locks::can_engage(squads, squad)
        && squads.engaged_with[squad] == 0
        && squads.flanks[squad].is_empty()
        && squads.state[squad] != FIGHTING
}

/// The index in `sides` of the hostile nearest side `mine.0` within reach_engage (between
/// their nearest units' bodies), ties by the squads' draws; none if none is. A side whose
/// box is too far from its own for any two units to be in reach is passed over. `rules`:
/// reach_engage and the battle's seed.
fn nearest(
    squads: &Squads,
    sides: &[Side],
    mine: (usize, &[Span]),
    grids: &mut [Option<Grid>],
    rules: (f64, i64),
) -> Option<usize> {
    let (n, spans) = mine;
    let (reach_engage, seed) = rules;
    let side = &sides[n];
    let reach = reach_engage + HAIR;
    let mut best: Option<(usize, (f64, i64))> = None;
    for (m, other) in sides.iter().enumerate() {
        if other.faction == side.faction || squads.state[other.squad] == DESTROYED {
            continue;
        }
        let far = reach + spans[n].widest + spans[m].widest + MARGIN;
        if other.at.is_empty() || apart(spans[n].area, spans[m].area) > far {
            continue;
        }
        let theirs = grids[m].get_or_insert_with(|| Grid::build(&other.at));
        let gap = gap(side, other, theirs, spans[m].widest, reach);
        let key = (snapped(gap, 0.000001), contest::squad_draw(seed, squads.id[other.squad]));
        if gap <= reach && best.is_none_or(|(_, held)| key < held) {
            best = Some((m, key));
        }
    }
    best.map(|(m, _)| m)
}

/// Cells between the two sides' nearest units' bodies (0 where they touch or overlap), if
/// within `reach`; past it, some gap past it, or infinity. `theirs`: a grid of where the
/// other's units stand, `widest` their widest body's radius.
fn gap(side: &Side, other: &Side, theirs: &Grid, widest: f64, reach: f64) -> f64 {
    let mut least = f64::INFINITY;
    for (n, &at) in side.at.iter().enumerate() {
        let radius = side.radii[n];
        theirs.near(at, radius + reach + widest + grid::MARGIN, |found| {
            let found = found as usize;
            least = minf(least, reach::gap(at, radius, other.at[found], other.radii[found]));
            true
        });
    }
    least
}

#[cfg(test)]
mod tests {
    use super::*;

    const REACH: f64 = 0.5;

    /// A squad of `faction` with grems (radius 0.5) at `points`, its units' store slots
    /// from `first`.
    fn side(squad: usize, faction: u32, first: i32, points: &[(f32, f32)]) -> Side {
        Side {
            squad,
            faction,
            joined: false,
            slots: (first..first + points.len() as i32).collect(),
            at: points.iter().map(|&(x, y)| V2::new(x, y)).collect(),
            radii: vec![0.5; points.len()],
        }
    }

    fn store(ids: &[i64]) -> (Squads, Columns) {
        let mut squads = Squads::default();
        for &id in ids {
            squads.open(id);
        }
        let mut columns = Columns::default();
        for _ in 0..12 {
            columns.grow();
        }
        (squads, columns)
    }

    #[test]
    fn squads_whose_units_come_within_reach_lock_whatever_their_faces() {
        let (mut squads, mut columns) = store(&[1, 2]);
        let sides = [side(0, 0, 0, &[(0.0, 0.0), (0.0, 1.0)]), side(1, 1, 2, &[(1.4, 1.0)])];
        let picks = step(&mut squads, &mut columns, &sides, REACH, 3);
        assert_eq!(picks, vec![(0, 2), (1, 1)]);
        assert_eq!((squads.engaged_with[0], squads.state[0]), (2, FIGHTING));
        assert_eq!((columns.attack_cooldown[0], columns.attack_cooldown[2]), (1, 1));
    }

    #[test]
    fn squads_out_of_reach_or_friends_or_not_free_stay_apart() {
        let (mut squads, mut columns) = store(&[1, 2, 3]);
        let sides = [
            side(0, 0, 0, &[(0.0, 0.0)]),
            side(1, 0, 1, &[(1.0, 0.0)]), // a friend, touching
            side(2, 1, 2, &[(2.4, 0.0)]), // in reach of squad 2, not of squad 1
        ];
        assert_eq!(step(&mut squads, &mut columns, &sides, REACH, 3), vec![(1, 3), (2, 2)]);
        let (mut squads, mut columns) = store(&[1, 2]);
        squads.order[0] = locks::RETREAT; // not free: engaged, never engaging
        let sides = [side(0, 0, 0, &[(0.0, 0.0)]), side(1, 1, 1, &[(1.0, 0.0)])];
        assert_eq!(step(&mut squads, &mut columns, &sides, REACH, 3), vec![(1, 1)]);
    }

    #[test]
    fn the_nearest_hostile_wins_and_a_tie_goes_to_the_draws_not_the_lists() {
        let (mut squads, mut columns) = store(&[1, 2, 3]);
        let sides = [
            side(0, 0, 0, &[(0.0, 0.0)]),
            side(1, 1, 1, &[(1.2, 0.0)]),
            side(2, 1, 2, &[(-1.2, 0.0)]),
        ];
        let tie = step(&mut squads, &mut columns, &sides, REACH, 3)[0];
        let lower = if contest::squad_draw(3, 2) < contest::squad_draw(3, 3) { 2 } else { 3 };
        assert_eq!(tie, (0, lower));
        let (mut squads, mut columns) = store(&[1, 3, 2]);
        let swapped = [
            side(0, 0, 0, &[(0.0, 0.0)]),
            side(1, 1, 1, &[(-1.2, 0.0)]),
            side(2, 1, 2, &[(1.2, 0.0)]),
        ];
        assert_eq!(step(&mut squads, &mut columns, &swapped, REACH, 3)[0], (0, lower));
        let (mut squads, mut columns) = store(&[1, 2, 3]);
        let nearer = [
            side(0, 0, 0, &[(0.0, 0.0)]),
            side(1, 1, 1, &[(1.3, 0.0)]),
            side(2, 1, 2, &[(-1.1, 0.0)]),
        ];
        assert_eq!(step(&mut squads, &mut columns, &nearer, REACH, 3)[0], (0, 3));
    }
}

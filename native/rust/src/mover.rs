//! The one way a unit moves (UnitMover, Decision 139) and the ground's toll on its step
//! (TerrainStep, Decision 85): every step a unit takes goes from where it stands towards
//! where it means to go, at its pace by facing (`motion`), and pays the ground it crosses -
//! at the share of its pace the ground allows, and where its way is barred, sliding along
//! the barrier on whichever axis is open and brings it nearer, or staying where neither
//! is. No ground (`None`) is open ground. The maths is the GDScript's, operation for
//! operation (`maths`). Cells.

use crate::ground::{Ground, Walker};
use crate::maths::V2;
use crate::motion::{self, Mover};

/// TerrainStep.EPSILON, UnitMover's.
const EPSILON: f64 = 0.000001;

/// TerrainStep.way: the point the walker at `at` steps towards on its way to `goal`, and
/// the share of its pace the ground allows: straight there; or, where that way is barred,
/// along the barrier on whichever axis is open and brings it nearer; `(at, 0)` if none
/// is. It looks `reach` cells ahead.
pub fn way(ground: Option<&Ground>, walker: &Walker, at: V2, goal: V2, reach: f64) -> (V2, f64) {
    let to_goal = goal.sub(at);
    let Some(ground) = ground else {
        return (goal, 1.0);
    };
    if (to_goal.length() as f64) < EPSILON {
        return (goal, 1.0);
    }
    let straight = ground.crossing(walker, at, at.add(to_goal.normalized().scale(reach)));
    if straight > 0.0 {
        return (goal, straight);
    }
    let mut best = (at, 0.0);
    let mut nearest = to_goal.length() as f64;
    for along in [V2::new(to_goal.x, 0.0), V2::new(0.0, to_goal.y)] {
        if (along.length() as f64) < EPSILON {
            continue;
        }
        let share = ground.crossing(walker, at, at.add(along.normalized().scale(reach)));
        let left = at.add(along).distance_to(goal) as f64;
        if share > 0.0 && left < nearest - EPSILON {
            best = (at.add(along), share);
            nearest = left;
        }
    }
    best
}

/// TerrainStep.pay, UnitMover.pay: where the walker stepping from `at` to `to` - its step
/// on open ground - gets: as far along its way as the ground's share of that step takes it.
pub fn pay(ground: Option<&Ground>, walker: &Walker, at: V2, to: V2) -> V2 {
    if ground.is_none() {
        return to;
    }
    let step = at.distance_to(to) as f64;
    let (going, share) = way(ground, walker, at, to, step.max(1.0));
    at.move_toward(going, step * share)
}

/// UnitMover.share: the share of an open-ground step from `at` to `stepped` the ground lets
/// the walker make: 1 on open ground, 0 where it's barred.
pub fn share(ground: Option<&Ground>, walker: &Walker, at: V2, stepped: V2) -> f64 {
    let wanted = at.distance_to(stepped) as f64;
    if ground.is_none() || wanted < EPSILON {
        return 1.0;
    }
    at.distance_to(pay(ground, walker, at, stepped)) as f64 / wanted
}

/// UnitMover.step: where the unit at `at` gets walking towards `to` with `full` cells at
/// full pace, turning as it goes (towards its heading, or to keep facing `look`) over
/// `seconds`, and its bearing then.
#[allow(clippy::too_many_arguments)]
pub fn step(
    ground: Option<&Ground>,
    walker: &Walker,
    unit: &Mover,
    at: V2,
    to: V2,
    full: f64,
    seconds: f64,
    look: Option<V2>,
) -> (V2, f64) {
    let (walked, bearing) = motion::walk(unit, at, to, full, seconds, look);
    (pay(ground, walker, at, walked), bearing)
}

/// UnitMover.slide: as step, for a unit that doesn't turn as it moves.
pub fn slide(
    ground: Option<&Ground>,
    walker: &Walker,
    unit: &Mover,
    at: V2,
    to: V2,
    full: f64,
) -> V2 {
    pay(ground, walker, at, motion::advance(unit, at, to, full))
}

#[cfg(test)]
mod tests;

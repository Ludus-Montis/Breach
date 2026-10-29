//! A squad's march along its route (FormationMarch, Decisions 74, 75 and 85; spec 39 step
//! 3d): where its front gets along its route, which way its order takes it, reaching either
//! end (`end`, step 8a2-v), the pace the ground under its front rank allows, and its units
//! walking to their places (`walk`), pouring through a gap (`pour`), sweeping round bends
//! (`sweep`), climbing and swimming (`modes`) and re-forming (`shuffle`, `front`). The maths is the GDScript's, operation for
//! operation (`maths`): points in f32, the rest in f64.
//! Distances along a route are in tiles, the rest in cells.

pub mod front;
pub mod modes;
pub mod pour;
pub mod shuffle;
pub mod sweep;
pub mod sync;
#[cfg(test)]
mod tests;
pub mod walk;

use crate::ground::{Ground, Walker};
use crate::maths::{bearing_vector, clampf, is_equal_approx, is_zero_approx, minf, signf, V2};

/// SkirmishUnit.Order.
pub const ADVANCE: i32 = 0;
pub const RETREAT: i32 = 2;

/// Where a squad stands along its route and where its order takes it.
#[derive(Clone, Copy)]
pub struct Along {
    /// Tiles along its route of its front, and of its home end (0 or the route's length).
    pub front: f64,
    pub home: f64,
    pub order: i32,
    /// +1 up its route, -1 down it.
    pub direction: i32,
}

impl Along {
    /// The end away from home, on a route `route_end` tiles long.
    fn enemy_end(&self, route_end: f64) -> f64 {
        if is_zero_approx(self.home) {
            route_end
        } else {
            0.0
        }
    }
}

/// FormationMarch.toward: where the squad's front gets moving `delta` tiles along its route
/// of `route_end` tiles: never past either end, nor past the point its order takes it to.
pub fn toward(along: &Along, delta: f64, route_end: f64) -> f64 {
    let next = clampf(along.front + delta, 0.0, route_end);
    let enemy_end = along.enemy_end(route_end);
    let target = if along.order == RETREAT { along.home } else { enemy_end };
    let before = signf(target - along.front);
    if before != 0.0 && signf(target - next) != before {
        return target; // it would pass its target: it stops there
    }
    next
}

/// FormationMarch.travel_sign: which way the squad's order takes it along its route, +1 up
/// it, -1 down it. Advancing heads for the end away from home, anything else for home.
pub fn travel_sign(along: &Along, route_end: f64) -> i32 {
    let target = if along.order == ADVANCE { along.enemy_end(route_end) } else { along.home };
    if is_equal_approx(target, along.front) {
        return along.direction;
    }
    if target > along.front {
        1
    } else {
        -1
    }
}

/// How a squad's march ends this tick (FormationMarch.check_ends).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum End {
    /// Not at either end: it marches on.
    Under,
    /// Advancing, its front at the enemy's end (tiles): it arrives there.
    Arrived(f64),
    /// Retreating, its front home or every unit of it home: it holds at home (tiles).
    Returned(f64),
}

/// FormationMarch.check_ends: an advancing squad whose front is at the enemy's end of its
/// route of `route_end` tiles arrives; a retreating one whose front is home, or (on a route)
/// whose units are all home wherever its front stands (`all_home`, asked only then), returns
/// and holds there as it faces, rather than about-face to march back to home.
pub fn end(along: &Along, route_end: f64, on_route: bool, all_home: impl FnOnce() -> bool) -> End {
    let enemy_end = along.enemy_end(route_end);
    if along.order == ADVANCE && is_equal_approx(along.front, enemy_end) {
        End::Arrived(enemy_end)
    } else if along.order == RETREAT
        && (is_equal_approx(along.front, along.home) || (on_route && all_home()))
    {
        End::Returned(along.home)
    } else {
        End::Under
    }
}

/// FormationMarch.pace's share: the pace of the worst ground its front rank steps into
/// facing `heading` that it can cross (one barred there pours through), each walker at its
/// point; 0 where none of them can go.
pub fn pace(ground: &Ground, walkers: &[Walker], points: &[V2], heading: f64) -> f64 {
    let ahead = bearing_vector(heading);
    let mut worst = f64::INFINITY;
    for (walker, &at) in walkers.iter().zip(points) {
        let factor = ground.crossing(walker, at, at.add(ahead));
        if factor > 0.0 {
            worst = minf(worst, factor);
        }
    }
    if worst == f64::INFINITY {
        0.0
    } else {
        worst
    }
}

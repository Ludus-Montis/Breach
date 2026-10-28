//! A formation sweeps round its route's bends (FormationSweep, Decision 105, spec 30 round
//! 1): as it marches, its frame turns towards its route's heading where its front is, no
//! faster than its outer file can march the arc - pivoting on its front centre, the file
//! half its width out covers a quarter circle in the time a wheel took. A 5 degree bend and
//! a 90 degree bend follow the same rule, so it doesn't halt to turn; at a sharp bend it
//! cuts the corner a little.

use crate::geometry::CELLS;
use crate::maths::{clampf, maxf, V2};
use crate::motion::{bearing_to, fposmod, rad_to_deg};
use crate::route::Route;
use crate::squads::Squads;

/// Degrees a second a frame `width` columns wide may turn marching at `cells_per_second`.
pub fn rate(width: i32, cells_per_second: f64) -> f64 {
    let arm = maxf(width as f64 / 2.0, 0.5); // its outer file's distance from the pivot
    rad_to_deg(cells_per_second / arm)
}

/// The bearing of `route` at `cells` along it, travelled in `direction` (FormationMarch.face:
/// `heading` if the way has no length).
pub fn wanted(route: &Route, cells: f64, direction: i32, heading: f64) -> f64 {
    let way = route.heading_at(cells).scale(direction as f64);
    bearing_to(V2::ZERO, way, heading)
}

/// The frame's heading after turning from `heading` towards `wanted`, by what its rate
/// allows in `seconds`: the short way round.
pub fn step(heading: f64, wanted: f64, width: i32, cells_per_second: f64, seconds: f64) -> f64 {
    let left = fposmod(wanted - heading + 180.0, 360.0) - 180.0;
    let reach = rate(width, cells_per_second) * seconds;
    fposmod(heading + clampf(left, -reach, reach), 360.0)
}

/// FormationMarch.face for the squad in `slot`: its frame turned to its route's bearing where
/// its front stands (none: as it is).
pub fn face_held(squads: &mut Squads, slot: usize) {
    if let Some(route) = &squads.route[slot] {
        let cells = squads.front_distance[slot] * CELLS;
        squads.heading[slot] = wanted(route, cells, squads.direction[slot], squads.heading[slot]);
    }
}

/// FormationSweep.step for the squad in `slot`, marching at `cells_per_second` for `seconds`:
/// its frame turned towards its route's bearing where its front stands (none: as it is).
pub fn sweep_held(squads: &mut Squads, slot: usize, cells_per_second: f64, seconds: f64) {
    if let Some(route) = &squads.route[slot] {
        let (cells, heading) = (squads.front_distance[slot] * CELLS, squads.heading[slot]);
        let wanted = wanted(route, cells, squads.direction[slot], heading);
        let width = squads.width[slot];
        squads.heading[slot] = step(heading, wanted, width, cells_per_second, seconds);
    }
}

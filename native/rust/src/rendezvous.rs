//! Planned rendezvous (FormationRendezvous, Decision 87, spec 27 round 2; spec 39 step 5b):
//! the overlord's timing, given before departure, so waves on different routes reach their
//! points together. The march is predicted as the simulation runs it: a cell pace per tick,
//! slowed by the ground along the route (Decision 85), sweeping round bends (Decision 105)
//! as fast as its files, walking to their places, keep within their slack (FormationWalk,
//! spec 30 round 3). So it holds until something interferes on the way (a fight, a queue, a
//! gap it pours through).

use crate::ground::{Ground, Walker};
use crate::maths::{bearing_vector, clampf, maxf, V2};
use crate::motion::{bearing_to, fposmod};
use crate::route::Route;

/// The ground the march is predicted over: a unit of height 1 that neither swims nor climbs
/// (TerrainWalker.grounded(1.0)).
const GROUNDED: Walker =
    Walker { height: 1.0, swimmer: 0, climber: 0, swims: false, climbs: false };
/// No march is predicted for longer.
const MOST_TICKS: i64 = 1_000_000;

/// A march to predict: how far along the route it goes (cells), the squad's width, its pace
/// (cells a second) and a tick's seconds, where it sets out (cells along the route) and the
/// slack its units walk to their places within (cells, FormationWalk).
pub struct March {
    pub cells: f64,
    pub width: i32,
    pub cells_per_second: f64,
    pub tick_seconds: f64,
    pub from: f64,
    pub slack: f64,
}

/// The march as it goes: its front's travel, its heading and its front rank's files.
struct Going {
    step: f64,
    sweep: f64,
    travelled: f64,
    facing: f64,
    files: Vec<V2>,
}

/// FormationRendezvous.ticks_to: the ticks for `march` to come its cells along `route`, over
/// `ground` if given (open ground if not).
pub fn ticks_to(route: &Route, march: &March, ground: Option<&Ground>) -> i64 {
    let facing = bearing_to(V2::ZERO, route.heading_at(march.from), 0.0);
    let half = maxf(march.width as f64 / 2.0, 0.5);
    let mut going = Going {
        step: march.cells_per_second * march.tick_seconds,
        sweep: march.cells_per_second / half * (180.0 / std::f64::consts::PI) * march.tick_seconds,
        travelled: march.from,
        facing,
        files: places(route, march.width, march.from, facing),
    };
    let mut ticks = 0;
    while going.travelled < march.cells - 0.000001 && ticks < MOST_TICKS {
        tick(route, march, &mut going, ground);
        ticks += 1;
    }
    ticks
}

/// One tick of the predicted march: the frame steps and sweeps by the share its lagging
/// files allow (FormationWalk.share), and the front rank's files walk to their places.
fn tick(route: &Route, march: &March, going: &mut Going, ground: Option<&Ground>) {
    let here = route.point_at(going.travelled);
    let ahead = route.heading_at(going.travelled);
    let pace = match ground {
        Some(g) => maxf(0.05, g.crossing(&GROUNDED, here, here.add(ahead))),
        None => 1.0,
    };
    let now = places(route, march.width, going.travelled, going.facing);
    let furthest = going
        .files
        .iter()
        .zip(&now)
        .fold(0.0, |most, (file, place)| maxf(most, file.distance_to(*place) as f64));
    let keeping = clampf((march.slack - furthest) / (march.slack / 2.0), 0.0, 1.0);
    let wanted = bearing_to(V2::ZERO, ahead, going.facing);
    let left = fposmod(wanted - going.facing + 180.0, 360.0) - 180.0; // the short way
    let reach = going.sweep * keeping;
    going.facing = fposmod(going.facing + clampf(left, -reach, reach), 360.0);
    going.travelled += going.step * pace * keeping;
    let next = places(route, march.width, going.travelled, going.facing);
    for (file, place) in going.files.iter_mut().zip(next) {
        *file = file.move_toward(place, going.step * pace);
    }
}

/// The front rank's places, a line `width` wide facing `heading` at `cells` along `route`.
fn places(route: &Route, width: i32, cells: f64, heading: f64) -> Vec<V2> {
    let anchor = route.point_at(cells);
    let ahead = bearing_vector(heading);
    (0..width)
        .map(|column| {
            let across = column as f64 - width as f64 / 2.0 + 0.5;
            anchor.sub(ahead.orthogonal().scale(across)).sub(ahead.scale(0.5))
        })
        .collect()
}

/// FormationRendezvous.waits: the ticks each wave should wait before setting out so all
/// arrive together, from each one's predicted ticks.
pub fn waits(predicted: &[i64]) -> Vec<i64> {
    let longest = predicted.iter().copied().fold(0, i64::max);
    predicted.iter().map(|&ticks| longest - ticks).collect()
}

#[cfg(test)]
mod tests;

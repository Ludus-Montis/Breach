//! Routes and the detours learned on them (FormationRoute, Decisions 74 and 75; RoutePatch,
//! spec 30), operation for operation as the GDScript had them: waypoints and the points on
//! them in f32 (`Vector2`), distances along a route in f64 (GDScript floats), each leg's
//! length a `Vector2.distance_to`. A squad keeps its distance along its route and takes its
//! place and heading from it. Distances are in cells.

use serde::{Deserialize, Serialize};
use std::cmp::Ordering;

use crate::maths::V2;

/// FormationRoute's guard against a leg of no length.
const LEAST_LEG: f64 = 0.000001;

/// A path of cells across the map, and how far either side of its centre line squads may
/// spread (the lane's combat width, halved).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Route {
    pub points: Vec<V2>,
    pub half_width: f64,
}

/// A learned detour: between `from_along` and `to_along` cells along a route, follow
/// `waypoints`.
#[derive(Clone, Debug, PartialEq)]
pub struct Patch {
    pub from_along: f64,
    pub to_along: f64,
    pub waypoints: Vec<V2>,
}

impl Route {
    pub fn new(points: Vec<V2>, half_width: f64) -> Route {
        Route { points, half_width }
    }

    pub fn length(&self) -> f64 {
        legs(&self.points)
    }

    /// The point at a distance along the route, clamped to its ends.
    pub fn point_at(&self, distance: f64) -> V2 {
        let Some(&last) = self.points.last() else {
            return V2::ZERO;
        };
        let mut remaining = distance.max(0.0);
        for leg in self.points.windows(2) {
            let length = leg[0].distance_to(leg[1]) as f64;
            if remaining < length {
                return leg[0].lerp(leg[1], remaining / length);
            }
            remaining -= length;
        }
        last
    }

    /// The unit direction of travel at a distance; at a bend, the leg leaving it.
    pub fn heading_at(&self, distance: f64) -> V2 {
        let count = self.points.len();
        if count < 2 {
            return V2::new(1.0, 0.0);
        }
        let mut remaining = distance.max(0.0);
        for leg in self.points.windows(2) {
            let length = leg[0].distance_to(leg[1]) as f64;
            if remaining < length {
                return leg[1].sub(leg[0]).normalized();
            }
            remaining -= length;
        }
        self.points[count - 1].sub(self.points[count - 2]).normalized()
    }

    /// The distance along the route of the point on it nearest `point`.
    pub fn distance_of(&self, point: V2) -> f64 {
        let mut best = 0.0;
        let mut best_gap = f64::INFINITY;
        let mut walked = 0.0;
        for leg in self.points.windows(2) {
            let (from, to) = (leg[0], leg[1]);
            let length = from.distance_to(to) as f64;
            let guard = length.max(LEAST_LEG);
            let way = to.sub(from);
            let unit = V2::new(way.x / guard as f32, way.y / guard as f32);
            let along = (point.sub(from).dot(unit) as f64).clamp(0.0, length);
            let gap = point.distance_to(from.lerp(to, along / guard)) as f64;
            if gap < best_gap {
                best_gap = gap;
                best = walked + along;
            }
            walked += length;
        }
        best
    }
}

impl Patch {
    /// The patch a detour makes: from where its first waypoint lies along `route` to where
    /// its last does, through the waypoints between.
    pub fn of_detour(route: &Route, detour: &[V2]) -> Patch {
        let inner = if detour.len() > 2 { detour[1..detour.len() - 1].to_vec() } else { vec![] };
        Patch {
            from_along: route.distance_of(detour[0]),
            to_along: route.distance_of(detour[detour.len() - 1]),
            waypoints: inner,
        }
    }

    /// `route` with each patch's stretch replaced by its detour. Patches apply in order
    /// along the route; one starting inside an earlier one's stretch gives way to it.
    pub fn patched(route: &Route, patches: &[Patch]) -> Route {
        let mut ordered: Vec<&Patch> =
            patches.iter().filter(|patch| patch.to_along > patch.from_along).collect();
        ordered.sort_by(|one, other| before(one, other));
        let corners = &route.points;
        let mut out = Vec::new();
        let mut cut = f64::NEG_INFINITY; // where the last patch rejoined
        let mut corner = 0;
        let mut walked = 0.0;
        for patch in ordered {
            if patch.from_along < cut {
                continue; // inside an earlier patch's stretch
            }
            while corner < corners.len() && walked < patch.from_along {
                if walked > cut {
                    out.push(corners[corner]);
                }
                corner += 1;
                walked = along(corners, corner);
            }
            out.push(route.point_at(patch.from_along));
            out.extend_from_slice(&patch.waypoints);
            out.push(route.point_at(patch.to_along));
            cut = patch.to_along;
        }
        for rest in corner..corners.len() {
            if along(corners, rest) > cut {
                out.push(corners[rest]);
            }
        }
        Route::new(out, route.half_width)
    }
}

/// Whether `one` comes before `other` along the route: by where it leaves, then where it
/// rejoins, then its waypoints' places (x, then y), then the fewer waypoints.
fn before(one: &Patch, other: &Patch) -> Ordering {
    if one.from_along != other.from_along {
        return order(one.from_along < other.from_along);
    }
    if one.to_along != other.to_along {
        return order(one.to_along < other.to_along);
    }
    for (a, b) in one.waypoints.iter().zip(&other.waypoints) {
        if a != b {
            return order(a.x < b.x || (a.x == b.x && a.y < b.y));
        }
    }
    one.waypoints.len().cmp(&other.waypoints.len())
}

fn order(first: bool) -> Ordering {
    if first {
        Ordering::Less
    } else {
        Ordering::Greater
    }
}

/// How far along the polyline `corners` its corner `index` lies (infinite past its end).
fn along(corners: &[V2], index: usize) -> f64 {
    if index >= corners.len() {
        return f64::INFINITY;
    }
    legs(&corners[..=index])
}

/// The length of the polyline `points`, its legs added in order from 0.
fn legs(points: &[V2]) -> f64 {
    points.windows(2).fold(0.0, |total, leg| total + leg[0].distance_to(leg[1]) as f64)
}

#[cfg(test)]
mod tests;

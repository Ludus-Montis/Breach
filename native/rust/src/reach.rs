//! ScrumReach (spec 39 step 4a): who touches whom in the scrum (Decisions 88 and 106, spec
//! 30). A unit's body is the circle in its footprint, and it reaches an enemy whose body
//! comes within a touch of its own (`reach_contact`). A unit's front is the arc within some
//! degrees of its bearing (`reach_front_arc_degrees`); a blow from anywhere else is a flank
//! blow. Cells, in GDScript's precision: points in f32, distances and radii in f64.

use crate::det_math;
use crate::maths::{bearing_vector, maxf, V2};

/// The radius of a body with this footprint: the circle in it (a grem 0.5, a brute 1).
pub fn radius(width: i64, depth: i64) -> f64 {
    width.min(depth) as f64 / 2.0
}

/// Cells between two bodies standing at `here` and `there` (0 where they touch or overlap).
pub fn gap(here: V2, radius: f64, there: V2, other: f64) -> f64 {
    apart(here.distance_to(there) as f64, radius, other)
}

/// `gap` for two bodies whose centres are `distance` apart.
#[inline]
pub fn apart(distance: f64, radius: f64, other: f64) -> f64 {
    maxf(distance - radius - other, 0.0)
}

/// The least dot of a way with a unit's bearing that still lies in its front:
/// `cos(deg_to_rad(arc))` less a hair (ScrumReach.in_front's).
pub fn front(arc_degrees: f64) -> f64 {
    det_math::cos(arc_degrees * (std::f64::consts::PI / 180.0)) - 0.000001
}

/// True if `point` lies in the front of a unit at `from` facing `ahead` (its bearing's
/// vector), `front` as `front()` gives it; a point on it is in front.
#[inline]
pub fn in_front(ahead: V2, from: V2, point: V2, front: f64) -> bool {
    let direction = point.sub(from);
    if (direction.length() as f64) < 0.000001 {
        return true;
    }
    direction.normalized().dot(ahead) as f64 >= front
}

/// The cells a unit with this footprint covers, centred at `at`, turned to its squad's
/// `heading`: [its box's corner, its size] (the box round its turned footprint).
pub fn area(at: V2, width: i64, depth: i64, heading: f64) -> (V2, V2) {
    let way = bearing_vector(heading);
    let ahead = V2::new(way.x.abs(), way.y.abs());
    let right = V2::new(ahead.y, ahead.x);
    let (w, d) = (width as f32, depth as f32);
    let size = V2::new(right.x * w + ahead.x * d, right.y * w + ahead.y * d);
    (at.sub(V2::new(size.x / 2.0, size.y / 2.0)), size)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(x: f32, y: f32) -> V2 {
        V2::new(x, y)
    }

    #[test]
    fn a_body_is_the_circle_in_its_footprint() {
        assert_eq!(radius(1, 1), 0.5);
        assert_eq!(radius(2, 2), 1.0);
        assert_eq!(radius(2, 1), 0.5);
    }

    #[test]
    fn the_gap_is_between_the_edges_and_never_below_nothing() {
        assert_eq!(gap(v(0.0, 0.0), 0.5, v(3.0, 0.0), 1.0), 1.5);
        assert_eq!(gap(v(0.0, 0.0), 0.5, v(0.8, 0.0), 0.5), 0.0); // overlapping
        assert_eq!(
            gap(v(0.0, 0.0), 0.5, v(0.0, 4.0), 0.5),
            gap(v(0.0, 4.0), 0.5, v(0.0, 0.0), 0.5)
        );
    }

    #[test]
    fn the_front_is_the_arc_round_its_bearing() {
        let f = front(60.0);
        let north = bearing_vector(0.0);
        assert!(in_front(north, v(0.0, 0.0), v(0.0, -2.0), f)); // dead ahead
        assert!(!in_front(north, v(0.0, 0.0), v(0.0, 2.0), f)); // behind
        assert!(!in_front(north, v(0.0, 0.0), v(2.0, 0.0), f)); // at its side
        assert!(in_front(north, v(0.0, 0.0), v(1.0, -1.8), f)); // inside 60 degrees
        assert!(!in_front(north, v(0.0, 0.0), v(1.8, -1.0), f)); // outside it
        assert!(in_front(north, v(1.0, 1.0), v(1.0, 1.0), f)); // a point on it
        assert!(in_front(bearing_vector(90.0), v(0.0, 0.0), v(3.0, 0.1), f)); // facing east
    }

    #[test]
    fn a_footprint_turns_with_its_squad() {
        // 2 wide and 1 deep, centred at (5, 5): across the way north, along it east.
        assert_eq!(area(v(5.0, 5.0), 2, 1, 0.0), (v(4.0, 4.5), v(2.0, 1.0)));
        assert_eq!(area(v(5.0, 5.0), 2, 1, 90.0), (v(4.5, 4.0), v(1.0, 2.0)));
        assert_eq!(area(v(5.0, 5.0), 2, 1, 180.0), area(v(5.0, 5.0), 2, 1, 0.0));
    }
}

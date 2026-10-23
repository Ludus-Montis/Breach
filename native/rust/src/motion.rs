//! How a unit turns (UnitMotion's bearing_to, pace and turn; UnitShuffle's look and
//! turning), operation for operation as the GDScript has them: `Vector2` maths in f32,
//! GDScript floats in f64, `fposmod`, `clampf`, `maxf` and `rad_to_deg` as Godot computes
//! them, and DetMath's atan2 and acos (`det_math`). Bearings are degrees clockwise from
//! north.

use crate::det_math;
use crate::maths::{bearing_vector, V2};

/// UnitMotion.EPSILON and UnitShuffle.EPSILON.
const EPSILON: f64 = 0.000001;

/// What of a unit its turning reads.
#[derive(Clone, Copy)]
pub struct Mover {
    pub bearing: f64,
    pub turn_rate: f64,
    pub backward_pace: f64,
}

/// Godot's `fposmod`: `fmod`, moved into the divisor's sign, and -0 made 0.
#[inline]
pub fn fposmod(x: f64, y: f64) -> f64 {
    let mut value = x % y;
    if (value < 0.0 && y > 0.0) || (value > 0.0 && y < 0.0) {
        value += y;
    }
    value + 0.0
}

/// Godot's `rad_to_deg`: `x * (180 / PI)`.
#[inline]
pub fn rad_to_deg(x: f64) -> f64 {
    x * (180.0 / std::f64::consts::PI)
}

/// Godot's `clampf`.
#[inline]
fn clampf(v: f64, low: f64, high: f64) -> f64 {
    if v < low {
        low
    } else if v > high {
        high
    } else {
        v
    }
}

/// Godot's `maxf`.
#[inline]
fn maxf(a: f64, b: f64) -> f64 {
    if a > b {
        a
    } else {
        b
    }
}

/// UnitMotion.bearing_to: the bearing of the way from `from` to `to` (`current` if they
/// coincide).
pub fn bearing_to(from: V2, to: V2, current: f64) -> f64 {
    let way = to.sub(from);
    if (way.length() as f64) < EPSILON {
        return current;
    }
    fposmod(rad_to_deg(det_math::atan2(way.x as f64, -way.y as f64)), 360.0)
}

/// UnitMotion.pace: the share of its speed the unit makes along `heading` facing `bearing`.
pub fn pace(unit: &Mover, heading: V2, bearing: f64) -> f64 {
    if (heading.length() as f64) < EPSILON {
        return 1.0;
    }
    let cosine = clampf(heading.normalized().dot(bearing_vector(bearing)) as f64, -1.0, 1.0);
    let off = rad_to_deg(det_math::acos(cosine)) / 180.0;
    1.0 - (1.0 - unit.backward_pace) * off
}

/// UnitShuffle._turning: seconds the unit takes to turn from one bearing to another.
fn turning(unit: &Mover, from: f64, to: f64) -> f64 {
    let cosine = clampf(bearing_vector(from).dot(bearing_vector(to)) as f64, -1.0, 1.0);
    rad_to_deg(det_math::acos(cosine)) / maxf(unit.turn_rate, EPSILON)
}

/// UnitShuffle.look: the point the unit should look towards walking from `at` to `to`, to
/// arrive facing `bearing` soonest; `speed` is its full speed in cells a second.
pub fn look(unit: &Mover, at: V2, to: V2, bearing: f64, speed: f64) -> V2 {
    let way = to.sub(at);
    let ahead = at.add(bearing_vector(bearing));
    let length = way.length() as f64;
    if length < EPSILON || speed < EPSILON {
        return ahead;
    }
    let heading = bearing_to(at, to, unit.bearing);
    let facing_pace = maxf(pace(unit, way, bearing), EPSILON);
    let shuffling = turning(unit, unit.bearing, bearing) + length / (speed * facing_pace);
    let walking =
        turning(unit, unit.bearing, heading) + length / speed + turning(unit, heading, bearing);
    if shuffling <= walking {
        ahead
    } else {
        to
    }
}

/// UnitMotion._apart: degrees from `from` to `to` the short way round, in (-180, 180].
fn apart(from: f64, to: f64) -> f64 {
    let turn = fposmod(to - from, 360.0);
    if turn > 180.0 + EPSILON {
        turn - 360.0
    } else {
        turn
    }
}

/// UnitMotion.turn: the unit's bearing after turning towards `wanted` by what its turn
/// rate allows in `seconds`.
pub fn turn(unit: &Mover, wanted: f64, seconds: f64) -> f64 {
    turning_to(unit, wanted, seconds).0
}

/// As turn, and whether it then faces `wanted`.
pub fn turning_to(unit: &Mover, wanted: f64, seconds: f64) -> (f64, bool) {
    let left = apart(unit.bearing, wanted);
    let reach = unit.turn_rate * seconds;
    if left.abs() < EPSILON || reach >= left.abs() - EPSILON {
        return (fposmod(wanted, 360.0), true);
    }
    let sign = if left > 0.0 {
        1.0
    } else if left < 0.0 {
        -1.0
    } else {
        0.0
    };
    (fposmod(unit.bearing + sign * reach, 360.0), false)
}

/// UnitMotion.move: the unit's next point moving from `at` to `to` with `full_step` cells
/// at full pace, at the pace its present bearing allows (it turns separately).
pub fn advance(unit: &Mover, at: V2, to: V2, full_step: f64) -> V2 {
    at.move_toward(to, full_step * pace(unit, to.sub(at), unit.bearing))
}

/// UnitMotion.walk: the unit's next point walking from `at` to `to` with `full_step` cells
/// at full pace, and its bearing, turning as it goes - towards its heading, or to keep
/// facing `look` if given.
pub fn walk(
    unit: &Mover,
    at: V2,
    to: V2,
    full_step: f64,
    seconds: f64,
    look: Option<V2>,
) -> (V2, f64) {
    let heading = to.sub(at);
    let wanted = bearing_to(at, look.unwrap_or(to), unit.bearing);
    let turned = turn(unit, wanted, seconds);
    (at.move_toward(to, full_step * pace(unit, heading, turned)), turned)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn godot_s_helpers() {
        assert_eq!(rad_to_deg(1.0).to_bits(), 0x404ca5dc1a63c1f8); // printed by Godot
        assert_eq!(fposmod(-90.0, 360.0), 270.0);
        assert_eq!(fposmod(-0.0, 360.0).to_bits(), 0);
        assert_eq!(fposmod(720.0, 360.0).to_bits(), 0);
        assert_eq!(bearing_to(V2::ZERO, V2::new(1.0, 0.0), 7.0), 90.0);
        assert_eq!(bearing_to(V2::ZERO, V2::new(0.0, 1.0), 7.0), 180.0);
        assert_eq!(bearing_to(V2::ZERO, V2::ZERO, 7.0), 7.0);
    }

    #[test]
    fn turning_is_limited_by_the_turn_rate() {
        let unit = Mover { bearing: 350.0, turn_rate: 100.0, backward_pace: 0.4 };
        assert_eq!(turn(&unit, 20.0, 0.1), 0.0); // 10 of the 30 degrees, clockwise
        assert_eq!(turn(&unit, 355.0, 0.1), 355.0);
        assert_eq!(turn(&unit, 170.0, 0.1), 0.0); // an about-turn goes clockwise
    }

    const TICK: f64 = 0.1;

    /// A unit facing north at a grem's turn rate and backward pace (test_unit_motion).
    fn unit(bearing: f64) -> Mover {
        Mover { bearing, turn_rate: 450.0, backward_pace: 0.4 }
    }

    fn ticks_to_face(mut unit: Mover, wanted: f64) -> i32 {
        for tick in 1..100 {
            let (bearing, faces) = turning_to(&unit, wanted, TICK);
            unit.bearing = bearing;
            if faces {
                return tick;
            }
        }
        100
    }

    #[test]
    fn a_quarter_turn_takes_two_ticks_and_about_turn_four() {
        assert_eq!(ticks_to_face(unit(0.0), 90.0), 2);
        assert_eq!(ticks_to_face(unit(0.0), 180.0), 4);
        assert_eq!(ticks_to_face(unit(0.0), 270.0), 2); // the short way round
    }

    #[test]
    fn a_unit_turns_to_any_angle_not_a_step() {
        assert_eq!(turning_to(&unit(0.0), 30.0, TICK), (30.0, true)); // within 45 degrees
        assert_eq!(turning_to(&unit(30.0), 330.0, TICK), (345.0, false)); // 60 the other way
        assert!((bearing_to(V2::ZERO, V2::new(1.0, -1.0), 0.0) - 45.0).abs() < 0.0001);
    }

    #[test]
    fn pace_falls_from_ahead_to_its_backward_pace() {
        let north = unit(0.0);
        assert!((pace(&north, V2::new(0.0, -1.0), 0.0) - 1.0).abs() < 0.001);
        assert!((pace(&north, V2::new(1.0, 0.0), 0.0) - 0.7).abs() < 0.001);
        assert!((pace(&north, V2::new(0.0, 1.0), 0.0) - 0.4).abs() < 0.001);
        assert_eq!(pace(&north, V2::ZERO, 0.0), 1.0);
    }

    #[test]
    fn backing_away_keeps_facing_the_foe() {
        let foe = Some(V2::new(5.0, 0.0));
        let (at, bearing) = walk(&unit(0.0), V2::new(5.0, 5.0), V2::new(5.0, 10.0), 1.0, TICK, foe);
        assert_eq!(bearing, 0.0); // still facing north, at the foe
        assert!((at.y - 5.4).abs() < 0.001); // at its backward pace
    }

    #[test]
    fn walking_turns_towards_its_heading_and_moving_does_not() {
        let to = V2::new(10.0, 0.0);
        let (at, bearing) = walk(&unit(0.0), V2::ZERO, to, 1.0, TICK, None);
        assert_eq!(bearing, 45.0); // a quarter turn takes two ticks
        let turned = Mover { bearing, ..unit(0.0) };
        assert_eq!(at, V2::ZERO.move_toward(to, pace(&turned, to, 45.0)));
        assert_eq!(advance(&unit(0.0), V2::ZERO, to, 1.0), V2::new(0.7, 0.0));
    }

    #[test]
    fn a_grem_is_nimbler_than_a_brute() {
        use crate::content::{tests::repo_content, Content};
        let content = Content::load(&repo_content()).unwrap_or_else(|error| panic!("{error}"));
        let of = |name: &str| {
            let unit = &content.units[name];
            Mover { bearing: 0.0, turn_rate: unit.turn_rate, backward_pace: unit.backward_pace }
        };
        let (grem, brute) = (of("grem"), of("grem_brute"));
        assert!(ticks_to_face(grem, 180.0) < ticks_to_face(brute, 180.0));
        assert!(pace(&grem, V2::new(0.0, 1.0), 0.0) > pace(&brute, V2::new(0.0, 1.0), 0.0));
    }

    #[test]
    fn the_quarter_bearings_are_exact() {
        let quarters =
            [(0.0, (0.0, -1.0)), (90.0, (1.0, 0.0)), (180.0, (0.0, 1.0)), (270.0, (-1.0, 0.0))];
        for (bearing, (x, y)) in quarters {
            for turns in [-3.0, -1.0, 0.0, 1.0, 5.0] {
                let vector = bearing_vector(bearing + 360.0 * turns);
                assert_eq!(
                    (vector.x.to_bits(), vector.y.to_bits()),
                    (V2::new(x, y).x.to_bits(), V2::new(x, y).y.to_bits()),
                    "{bearing} {turns}"
                );
            }
        }
    }
}

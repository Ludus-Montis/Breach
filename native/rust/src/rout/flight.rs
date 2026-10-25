//! How a disorderly flight spreads (RoutFlight, Decision 99, spec 27 round 10; spec 39 step
//! 7b): a fleeing unit makes for the nearest safety. With a standing friendly formation
//! between it and home (its refuge), it steers for that friend, which catches it. Otherwise
//! a rout's units don't keep to their route's line but fan out from it, each by its own
//! angle (seeded by the battle and the unit) up to `fan_degrees` either side, until
//! `fan_cells` out or the ground to the side can't be crossed. Its steps pay the ground they
//! cross (the one mover, Decision 139): a wood or a ford slows a flight as it slows a march.

use crate::columns::Columns;
use crate::det_math;
use crate::entries::{self, Key, Value};
use crate::ghash::{self, Key as HashKey};
use crate::ground::Ground;
use crate::maths::{clampf, move_toward, signf, V2};
use crate::mover;
use crate::roster::Member;
use crate::route::Route;

use super::friends::Friends;

/// GDScript's `deg_to_rad`.
pub fn deg_to_rad(degrees: f64) -> f64 {
    degrees * (std::f64::consts::PI / 180.0)
}

/// The unit's own flight angle (radians) off its route's line, at full disorder
/// (RoutFlight.fan).
pub fn fan(id: i64, seed: i64, fan_degrees: f64) -> f64 {
    let roll = ghash::uniform(seed, &[HashKey::Int(id), HashKey::Str("fan")]) * 2.0 - 1.0;
    deg_to_rad(fan_degrees) * roll
}

/// What a router's step needs of its formation and the tick.
pub struct Flight<'a> {
    pub listed: usize,
    pub route: Option<&'a Route>,
    /// Cells along its route of home.
    pub home: f64,
    /// Cells a tick at speed 1.
    pub pace: f64,
    pub seed: i64,
    pub fan_degrees: f64,
    pub fan_cells: f64,
    pub ground: Option<&'a Ground>,
}

/// A router's step (RoutFlight.step): towards home along its route and out to the side, or
/// towards its refuge, from `at`, where it stands. `friends` is None when its faction has no
/// friend standing.
pub fn step(
    columns: &mut Columns,
    flight: &Flight,
    unit: &Member,
    at: V2,
    friends: Option<&mut Friends>,
) {
    let slot = unit.slot;
    let full = columns.speed[slot] * flight.pace;
    let mut aside = det_math::sin(fan(unit.id, flight.seed, flight.fan_degrees)) * full;
    let fanned = columns.fanned[slot];
    let along = columns.along[slot];
    let heading = flight.route.map_or(V2::new(1.0, 0.0), |r| r.heading_at(along));
    let side = heading.orthogonal();
    let refuge = match (flight.route, friends) {
        (Some(route), Some(friends)) => friends
            .refuge(flight.listed, route, at, (along, flight.home))
            .map(|friend| friends.short_of(friend, at, side)),
        _ => None,
    };
    if let Some(short) = refuge {
        // it makes for the friend, turning aside only if it would miss it
        let most = det_math::sin(deg_to_rad(flight.fan_degrees)) * full;
        aside = clampf(short, -most, most);
    }
    let open =
        flight.ground.is_none_or(|g| g.crossing(&unit.walker, at, at.add(side.scale(aside))) > 0.0);
    if (refuge.is_none() && (fanned + aside).abs() > flight.fan_cells) || !open {
        aside = 0.0;
    }
    let ahead = (full * full - aside * aside).sqrt() * signf(flight.home - along);
    let stepped = at.add(side.scale(aside)).add(heading.scale(ahead));
    let paid = mover::share(flight.ground, &unit.walker, at, stepped);
    entries::keep(columns, slot, Key::Fanned, Value::Real(fanned + aside * paid));
    columns.flee_offset[slot] = columns.flee_offset[slot].add(side.scale(aside).scale(paid));
    columns.along[slot] = move_toward(along, flight.home, ahead.abs() * paid);
}

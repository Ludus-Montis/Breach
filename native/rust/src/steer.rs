//! Steering round bodies (UnitSteer, Decision 114, spec 30 round 2): a unit walking to a
//! goal whose straight way there runs into a body that won't part for it - a foe's, or
//! another squad's - within `look` cells steps round the nearest such body, on the side
//! the body leans least into its way, aiming to graze it just clear (`clear`). A body dead
//! centre in its way is passed on the side a seeded draw picks (Decision 97: no
//! handedness, never the list). A body on the goal itself is left to the bodies parting.
//! The nearest is the least by [distance along, draw], which never ties, so the order the
//! bodies are weighed in can't matter. The maths is the GDScript's, operation for
//! operation (`maths`, DetMath's asin, sin and cos). Cells.

use crate::det_math;
use crate::ghash::{self, Key};
use crate::maths::{rotated, snapped, V2};

/// UnitSteer.EPSILON.
const EPSILON: f64 = 0.000001;

/// A body that may stand in the way: where, its radius and its squad.
#[derive(Clone, Copy, Debug)]
pub struct Body {
    pub at: V2,
    pub radius: f64,
    pub squad: i64,
}

/// The nearest body in the way so far: its index, how far along the way and aside it
/// stands, and how near the two may come.
#[derive(Clone, Copy, Debug)]
pub struct Nearest {
    along: f64,
    body: usize,
    at: V2,
    aside: f64,
    clearance: f64,
}

/// The way a unit of radius `own`, of squad `squad`, means to go from `at` to `goal`.
pub struct Way {
    at: V2,
    goal: V2,
    ahead: V2,
    across: V2,
    own: f64,
    squad: i64,
    /// How far ahead it looks: the way's length or `look`, the less.
    span: f64,
}

impl Way {
    /// None where the unit is on its goal (it heads for the goal).
    pub fn new(at: V2, goal: V2, own: f64, squad: i64, look: f64) -> Option<Way> {
        let way = goal.sub(at);
        let length = way.length();
        if (length as f64) < EPSILON {
            return None;
        }
        let ahead = V2::new(way.x / length, way.y / length);
        let span = if (length as f64) < look { length as f64 } else { look };
        Some(Way { at, goal, ahead, across: V2::new(ahead.y, -ahead.x), own, squad, span })
    }

    /// Where its look ends.
    pub fn end(&self) -> V2 {
        self.at.add(self.ahead.scale(self.span))
    }

    /// The unit's own radius.
    pub fn own(&self) -> f64 {
        self.own
    }

    /// Weighs body `index` against the nearest so far; `draw` gives a body's draw, read
    /// only on a tie.
    pub fn weigh(
        &self,
        nearest: &mut Option<Nearest>,
        index: usize,
        body: Body,
        draw: &dyn Fn(usize) -> i64,
    ) {
        if body.squad == self.squad {
            return; // itself, or its own squad's: they part for it
        }
        let clearance = self.own + body.radius;
        if (body.at.distance_to(self.goal) as f64) < clearance - EPSILON {
            return; // it stands on the goal: bodies part
        }
        let along = body.at.sub(self.at).dot(self.ahead) as f64;
        let aside = body.at.sub(self.at).dot(self.across) as f64;
        if along <= EPSILON || along >= self.span || aside.abs() >= clearance - EPSILON {
            return;
        }
        let key = snapped(along, EPSILON);
        let nearer = match nearest {
            None => true,
            Some(best) if key != best.along => key < best.along,
            Some(best) => draw(index) < draw(best.body),
        };
        if nearer {
            *nearest = Some(Nearest { along: key, body: index, at: body.at, aside, clearance });
        }
    }

    /// The point the unit heads for: the goal, or past the nearest body in its way,
    /// `clear` beyond touching it. `own_draw` and `draw` are the draws for a dead-centre
    /// body's side, in the battle's `seed`.
    pub fn past(
        &self,
        nearest: Option<Nearest>,
        clear: f64,
        seed: i64,
        own_draw: &dyn Fn() -> i64,
        draw: &dyn Fn(usize) -> i64,
    ) -> V2 {
        let Some(best) = nearest else {
            return self.goal;
        };
        let mut side = -sign(best.aside);
        if best.aside.abs() < EPSILON {
            let keys = [Key::Int(own_draw()), Key::Int(draw(best.body)), Key::Str("steer")];
            side = if ghash::uniform(seed, &keys) < 0.5 { 1.0 } else { -1.0 };
        }
        round(self.at, best.at, best.clearance + clear, self.across.scale(side))
    }
}

/// UnitSteer.toward over a list of bodies: the point the unit at `at` heads for this step
/// on its way to `goal`.
#[allow(clippy::too_many_arguments)]
pub fn toward(
    at: V2,
    goal: V2,
    own: f64,
    squad: i64,
    bodies: &[Body],
    look: f64,
    clear: f64,
    seed: i64,
    own_draw: &dyn Fn() -> i64,
    draw: &dyn Fn(usize) -> i64,
) -> V2 {
    let Some(way) = Way::new(at, goal, own, squad, look) else {
        return goal;
    };
    let mut nearest = None;
    for (index, &body) in bodies.iter().enumerate() {
        way.weigh(&mut nearest, index, body, draw);
    }
    way.past(nearest, clear, seed, own_draw, draw)
}

/// Godot's `signf`.
#[inline]
fn sign(x: f64) -> f64 {
    if x > 0.0 {
        1.0
    } else if x < 0.0 {
        -1.0
    } else {
        0.0
    }
}

/// UnitSteer._round: where the way from `at` grazes a circle of `reach` about `centre` on
/// the `side` given, or the point beside the centre that way if already that near.
pub fn round(at: V2, centre: V2, reach: f64, side: V2) -> V2 {
    let to_centre = centre.sub(at);
    let gap = to_centre.length() as f64;
    if gap <= reach + EPSILON {
        return centre.add(side.scale(reach));
    }
    let angle = det_math::asin(reach / gap);
    let unit = to_centre.normalized();
    let mut turned = rotated(unit, angle);
    let other = rotated(unit, -angle);
    if other.dot(side) > turned.dot(side) {
        turned = other;
    }
    at.add(turned.scale((gap * gap - reach * reach).sqrt()))
}

#[cfg(test)]
mod tests;

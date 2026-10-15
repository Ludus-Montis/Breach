//! The scrum's walk (FormationScrum._walk) on the field's own state. Each fighting unit
//! steps once a tick: one making for a slot towards its next point (the plan's AIM and
//! PRESS), one with no slot that touches no foe (STAND, IDLE) back towards its place in its squad's frame (SquadFrame.place, with
//! ScrumStance's anchor), looking where UnitShuffle says. Either steps round the nearest
//! body in its way that won't part for it (UnitSteer), at the pace its bearing allows
//! (UnitMotion.move), slowed by the bodies lying underfoot (GroundBodies.underfoot). The
//! steps are the GDScript reference's, bit for bit: the same maths in the same precision
//! (`maths`, `motion`, DetMath's asin, sin and cos), the body stepped round the least by
//! the full key [distance along, draw], which never ties, and the footing a least over the
//! bodies lying near, so the grids' visiting order can't matter. Every unit reads the
//! bodies as they stood before any stepped (the scrum's snapshot), so the walk's own
//! order can't either.

use crate::det_math;
use crate::field::{Field, DESTROYED, ROUTING};
use crate::ghash::{self, Key};
use crate::grid::{Grid, MARGIN};
use crate::maths::{bearing_vector, snapped, V2};
use crate::motion::{self, Mover};
use crate::scrum::{AIM, IDLE, PRESS, STAND, TOUCH};

/// UnitSteer.EPSILON, UnitMotion.EPSILON.
const EPSILON: f64 = 0.000001;
/// Godot's CMP_EPSILON (Vector2.move_toward).
const CMP_EPSILON: f32 = 0.00001;

/// BattleTuning's and the tick's numbers.
pub struct Params {
    pub seed: i64,
    pub seconds: f64,
    /// bodies_steer_look, bodies_steer_clear.
    pub look: f64,
    pub clear: f64,
    /// bodies_ground_drag, bodies_ground_block.
    pub drag: f64,
    pub block: f64,
}

/// The fighting squads' units, as the slot search's call had them (BodyField.seek), with
/// what it made of each and what their walk needs. Where each stands is the field's, as
/// that call's sync left it: nothing moves between it and the walk.
pub struct WalkIn<'a> {
    pub member_counts: &'a [i32],
    /// Per squad: its pace (cells a tick at speed 1, crowded or not), heading, width and
    /// centre shift (its frame, ScrumStance.anchor) - 4 floats - and its frame's anchor.
    pub squad_floats: &'a [f64],
    pub squad_anchors: &'a [V2],
    /// Each unit's flat index in the last sync.
    pub members: &'a [i32],
    /// The plan's codes (scrum::TOUCH...IDLE) and next points.
    pub codes: &'a [u8],
    pub nexts: &'a [V2],
    pub speeds: &'a [f64],
    /// Per unit: bearing, turn rate, backward pace (only the bearing and backward pace of
    /// one making for a slot, nothing of one touching a foe: they aren't read).
    pub motion: &'a [f64],
    /// Per unit: column, rank, footprint width and depth.
    pub frames: &'a [f64],
    /// The bodies lying on the ground: where, radius, footprint area.
    pub lying_ats: &'a [V2],
    pub lying_radii: &'a [f64],
    pub lying_areas: &'a [f64],
}

/// Per unit: where it stands after its step, and where it looks (its next point, making
/// for a slot; UnitShuffle's, walking back to its place; nothing, touching a foe).
pub struct WalkOut {
    pub ats: Vec<V2>,
    pub towards: Vec<V2>,
}

/// Bodies indexed where they stand, with the widest radius among them.
struct Index {
    bodies: Vec<u32>,
    grid: Grid,
    widest: f64,
}

/// The bodies lying on the ground, indexed.
struct Lying<'a> {
    input: &'a WalkIn<'a>,
    grid: Grid,
    widest: f64,
}

/// Every unit's step this tick.
pub fn walk(field: &Field, input: &WalkIn, params: &Params) -> WalkOut {
    let crowd = crowd(field);
    let lying = Lying {
        input,
        grid: Grid::build(input.lying_ats),
        widest: input.lying_radii.iter().copied().fold(0.0, f64::max),
    };
    let count = input.members.len();
    let ats = input
        .members
        .iter()
        .map(|&flat| field.at[field.flat[flat as usize] as usize]);
    let mut out = WalkOut {
        ats: ats.collect(),
        towards: vec![V2::ZERO; count],
    };
    let mut member = 0usize;
    for (s, &units) in input.member_counts.iter().enumerate() {
        for _ in 0..units {
            if input.codes[member] != TOUCH {
                step(field, &crowd, &lying, input, params, s, member, &mut out);
            }
            member += 1;
        }
    }
    out
}

/// One unit's step (and look, back to its place).
#[allow(clippy::too_many_arguments)]
fn step(
    field: &Field,
    crowd: &Index,
    lying: &Lying,
    input: &WalkIn,
    params: &Params,
    s: usize,
    m: usize,
    out: &mut WalkOut,
) {
    let body = field.flat[input.members[m] as usize] as usize;
    let at = field.at[body];
    let unit = Mover {
        bearing: input.motion[3 * m],
        turn_rate: input.motion[3 * m + 1],
        backward_pace: input.motion[3 * m + 2],
    };
    let full = input.speeds[m] * input.squad_floats[4 * s];
    let code = input.codes[m];
    let goal = if code == AIM || code == PRESS {
        out.towards[m] = input.nexts[m];
        input.nexts[m]
    } else {
        debug_assert!(code == STAND || code == IDLE);
        let heading = input.squad_floats[4 * s + 1];
        let place = place(input, s, m, heading);
        out.towards[m] = motion::look(&unit, at, place, heading, full / params.seconds);
        place
    };
    let to = toward(field, crowd, body, at, goal, params);
    let footing = footing(lying, field, body, at, to, params);
    let full_step = full * footing;
    out.ats[m] = move_toward(
        at,
        to,
        (full_step * motion::pace(&unit, to.sub(at), unit.bearing)) as f32,
    );
}

/// ScrumSlots.bodies: the bodies standing (not a routing or destroyed squad's), indexed.
fn crowd(field: &Field) -> Index {
    let mut bodies = Vec::new();
    for &sq in &field.order {
        let squad = &field.squads[sq as usize];
        if squad.flags & (ROUTING | DESTROYED) == 0 {
            bodies.extend_from_slice(&squad.members);
        }
    }
    let points: Vec<V2> = bodies.iter().map(|&b| field.at[b as usize]).collect();
    let widest = bodies
        .iter()
        .map(|&b| field.radius[b as usize])
        .fold(0.0, f64::max);
    Index {
        grid: Grid::build(&points),
        bodies,
        widest,
    }
}

/// SquadFrame.place: the centre of the unit's place in its squad's frame.
fn place(input: &WalkIn, s: usize, m: usize, heading: f64) -> V2 {
    let (width, shift) = (input.squad_floats[4 * s + 2], input.squad_floats[4 * s + 3]);
    let f = &input.frames[4 * m..4 * m + 4];
    let ahead = bearing_vector(heading);
    let across = f[0] - width / 2.0 + shift + f[2] / 2.0;
    let back = f[1] + f[3] / 2.0;
    let orthogonal = V2::new(ahead.y, -ahead.x);
    input.squad_anchors[s]
        .add(orthogonal.scale(-across))
        .sub(ahead.scale(back))
}

/// UnitSteer.toward: the point the unit at `at` heads for this step on its way to `goal`:
/// the goal, or a point beside the nearest body in its way that won't part for it.
fn toward(field: &Field, crowd: &Index, me: usize, at: V2, goal: V2, p: &Params) -> V2 {
    let way = goal.sub(at);
    let length = way.length();
    if (length as f64) < EPSILON {
        return goal;
    }
    let ahead = V2::new(way.x / length, way.y / length);
    let across = V2::new(ahead.y, -ahead.x);
    let own = field.radius[me];
    let span = if (length as f64) < p.look {
        length as f64
    } else {
        p.look
    };
    let end = at.add(ahead.scale(span));
    // Every body that could stand within a breadth of the way (a box round it).
    let half = ((end.x - at.x).abs().max((end.y - at.y).abs()) as f64) / 2.0;
    let centre = V2::new((at.x + end.x) / 2.0, (at.y + end.y) / 2.0);
    let reach = half + own + crowd.widest + 2.0 * MARGIN;
    let mut best: Option<(f64, i64, usize, f64, f64)> = None;
    crowd.grid.near(centre, reach, |i| {
        let b = crowd.bodies[i as usize] as usize;
        if field.squad[b] == field.squad[me] {
            return true; // itself, or its own squad's: they part for it
        }
        let point = field.at[b];
        let clearance = own + field.radius[b];
        if (point.distance_to(goal) as f64) < clearance - EPSILON {
            return true; // it stands on the goal: bodies part
        }
        let along = point.sub(at).dot(ahead) as f64;
        let aside = point.sub(at).dot(across) as f64;
        if along <= EPSILON || along >= span || aside.abs() >= clearance - EPSILON {
            return true;
        }
        let key = (snapped(along, EPSILON), field.draw[b]);
        let better = match best {
            None => true,
            // as GDScript compares [along, draw]: item by item
            Some(o) if key.0 != o.0 => key.0 < o.0,
            Some(o) => key.1 < o.1,
        };
        if better {
            best = Some((key.0, key.1, b, aside, clearance));
        }
        true
    });
    let Some((_, draw, b, aside, clearance)) = best else {
        return goal;
    };
    let mut side = -sign(aside);
    if aside.abs() < EPSILON {
        let keys = [Key::Int(field.draw[me]), Key::Int(draw), Key::Str("steer")];
        side = if ghash::uniform(p.seed, &keys) < 0.5 {
            1.0
        } else {
            -1.0
        };
    }
    round(at, field.at[b], clearance + p.clear, across.scale(side))
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
fn round(at: V2, centre: V2, reach: f64, side: V2) -> V2 {
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

/// DetMath.rotated: `v` turned by `angle` radians, worked in f64, then made a Vector2.
#[inline]
fn rotated(v: V2, angle: f64) -> V2 {
    let (s, c) = (det_math::sin(angle), det_math::cos(angle));
    let (x, y) = (v.x as f64, v.y as f64);
    V2::new((x * c - y * s) as f32, (x * s + y * c) as f32)
}

/// FormationScrum._footing: the share of its pace the unit keeps stepping from `at`
/// towards `to` over the bodies on the ground.
fn footing(lying: &Lying, field: &Field, me: usize, at: V2, to: V2, p: &Params) -> f64 {
    let way = to.sub(at);
    if (way.length() as f64) < EPSILON {
        return 1.0;
    }
    underfoot(lying, field, me, at.add(way.normalized().scale(0.5)), p)
}

/// GroundBodies.underfoot: the share (0 to 1) of its pace the unit keeps stepping onto
/// `step` - slowed by the weight of what it crosses against its own, blocked by one
/// bodies_ground_block times its mass - the least over the bodies lying there.
fn underfoot(lying: &Lying, field: &Field, me: usize, step: V2, p: &Params) -> f64 {
    let own = field.radius[me];
    let area = field.area[me];
    let input = lying.input;
    let mut worst = 1.0f64;
    let mut blocked = false;
    lying.grid.near(step, own + lying.widest + MARGIN, |i| {
        let i = i as usize;
        let reach = own + input.lying_radii[i];
        if (input.lying_ats[i].distance_to(step) as f64) >= reach {
            return true;
        }
        let weight = input.lying_areas[i] / area;
        if weight >= p.block {
            blocked = true;
            return false;
        }
        let share = 1.0 / (1.0 + weight * p.drag);
        if share < worst {
            worst = share;
        }
        true
    });
    if blocked {
        0.0
    } else {
        worst
    }
}

/// Godot's `Vector2.move_toward`.
#[inline]
fn move_toward(from: V2, to: V2, delta: f32) -> V2 {
    let way = to.sub(from);
    let length = way.length();
    if length <= delta || length < CMP_EPSILON {
        to
    } else {
        from.add(V2::new(way.x / length * delta, way.y / length * delta))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn move_toward_is_godot_s() {
        let to = V2::new(3.0, 4.0);
        assert_eq!(move_toward(V2::ZERO, to, 10.0), to);
        assert_eq!(move_toward(V2::ZERO, to, 1.0), V2::new(0.6, 0.8));
    }

    #[test]
    fn rounding_a_body_grazes_it_on_the_side_given() {
        let at = V2::ZERO;
        let centre = V2::new(0.0, -4.0);
        let left = round(at, centre, 1.0, V2::new(-1.0, 0.0));
        let right = round(at, centre, 1.0, V2::new(1.0, 0.0));
        assert!(left.x < 0.0 && right.x > 0.0);
        assert!((left.x + right.x).abs() < 1e-6);
        // already within reach: beside the centre
        assert_eq!(
            round(at, V2::new(0.0, -0.5), 1.0, V2::new(1.0, 0.0)),
            V2::new(1.0, -0.5)
        );
    }
}

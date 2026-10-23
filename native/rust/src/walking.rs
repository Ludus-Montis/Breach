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

use crate::field::{Field, DESTROYED, ROUTING};
use crate::grid::{Grid, MARGIN};
use crate::lying::Lying;
use crate::maths::{bearing_vector, V2};
use crate::motion::{self, Mover};
use crate::scrum::{AIM, IDLE, PRESS, STAND, TOUCH};
use crate::steer::{Body, Way};

/// UnitMotion.EPSILON.
const EPSILON: f64 = 0.000001;

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

/// Every unit's step this tick.
pub fn walk(field: &Field, input: &WalkIn, params: &Params) -> WalkOut {
    let crowd = crowd(field);
    let lying = Lying::new(input.lying_ats, input.lying_radii, input.lying_areas);
    let count = input.members.len();
    let ats = input.members.iter().map(|&flat| field.at[field.flat[flat as usize] as usize]);
    let mut out = WalkOut { ats: ats.collect(), towards: vec![V2::ZERO; count] };
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
    out.ats[m] = at.move_toward(to, full_step * motion::pace(&unit, to.sub(at), unit.bearing));
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
    let widest = bodies.iter().map(|&b| field.radius[b as usize]).fold(0.0, f64::max);
    Index { grid: Grid::build(&points), bodies, widest }
}

/// SquadFrame.place: the centre of the unit's place in its squad's frame.
fn place(input: &WalkIn, s: usize, m: usize, heading: f64) -> V2 {
    let (width, shift) = (input.squad_floats[4 * s + 2], input.squad_floats[4 * s + 3]);
    let f = &input.frames[4 * m..4 * m + 4];
    let ahead = bearing_vector(heading);
    let across = f[0] - width / 2.0 + shift + f[2] / 2.0;
    let back = f[1] + f[3] / 2.0;
    let orthogonal = V2::new(ahead.y, -ahead.x);
    input.squad_anchors[s].add(orthogonal.scale(-across)).sub(ahead.scale(back))
}

/// UnitSteer.toward: the point the unit at `at` heads for this step on its way to `goal`:
/// the goal, or a point beside the nearest body in its way that won't part for it.
fn toward(field: &Field, crowd: &Index, me: usize, at: V2, goal: V2, p: &Params) -> V2 {
    let squad = |b: usize| field.squad[b] as i64;
    let Some(way) = Way::new(at, goal, field.radius[me], squad(me), p.look) else {
        return goal;
    };
    // Every body that could stand within a breadth of the way (a box round it).
    let end = way.end();
    let half = ((end.x - at.x).abs().max((end.y - at.y).abs()) as f64) / 2.0;
    let centre = V2::new((at.x + end.x) / 2.0, (at.y + end.y) / 2.0);
    let reach = half + way.own() + crowd.widest + 2.0 * MARGIN;
    let draw = |b: usize| field.draw[b];
    let mut nearest = None;
    crowd.grid.near(centre, reach, |i| {
        let b = crowd.bodies[i as usize] as usize;
        let body = Body { at: field.at[b], radius: field.radius[b], squad: squad(b) };
        way.weigh(&mut nearest, b, body, &draw);
        true
    });
    way.past(nearest, p.clear, p.seed, &|| field.draw[me], &draw)
}

/// FormationScrum._footing: the share of its pace the unit keeps stepping from `at`
/// towards `to` over the bodies on the ground.
fn footing(lying: &Lying, field: &Field, me: usize, at: V2, to: V2, p: &Params) -> f64 {
    let way = to.sub(at);
    if (way.length() as f64) < EPSILON {
        return 1.0;
    }
    let step = at.add(way.normalized().scale(0.5));
    lying.underfoot(field.radius[me], field.area[me], step, p.drag, p.block)
}

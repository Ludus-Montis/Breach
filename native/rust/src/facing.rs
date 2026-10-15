//! The scrum's facing (FormationScrum._faces and the turns after it) on the field's own
//! state. Each fighting unit turns once a tick, at its turn rate, towards the foe it
//! touches - one in its front first, then the nearest, then by the seeded draw
//! (ScrumBlows._pick) - or else so as to arrive facing what it will do: the foe at the
//! slot it makes for (UnitShuffle.look), or the point its walk chose (`toward`). The
//! choices and bearings are the GDScript reference's, bit for bit: the same maths in the
//! same precision (`motion`, `det_math`), and the touching foe the least by the full key
//! [front, distance, draw], which never ties, so the grid's visiting order can't matter.

use crate::field::Field;
use crate::grid::MARGIN;
use crate::maths::{bearing_vector, V2};
use crate::motion::{self, Mover};
use crate::scrum::{foes_of, Foes};

/// The unit makes for a slot: it has a next point and a foe there.
pub const GOAL: u8 = 1;
/// The unit's walk chose a point to look towards.
pub const TOWARD: u8 = 2;

/// BattleTuning's and the tick's numbers.
pub struct Params {
    pub reach: f64,
    /// The least dot of a point's way and a bearing for the point to be in front
    /// (ScrumReach.in_front: cos of the front arc, less 0.000001).
    pub front: f64,
    pub seconds: f64,
    /// Cells a tick at speed 1 (ctx["pace"]).
    pub pace: f64,
    pub crowding: f64,
}

/// The fighting squads' units, as the slot search's call had them (BodyField.seek),
/// with where each stands now and what it turns by.
pub struct FaceIn<'a> {
    pub foe_counts: &'a [i32],
    pub foe_ids: &'a [i64],
    pub member_counts: &'a [i32],
    /// Each unit's flat index in the last sync.
    pub members: &'a [i32],
    pub ats: &'a [V2],
    /// Per unit: bearing, turn rate, backward pace.
    pub motion: &'a [f64],
    pub speeds: &'a [f64],
    pub flags: &'a [u8],
    pub nexts: &'a [V2],
    pub foe_ats: &'a [V2],
    pub towards: &'a [V2],
}

/// Every unit's bearing after this tick's turn (unchanged for one with nothing to face).
/// The units' points are written into the field first: they have walked since the sync.
pub fn face(field: &mut Field, input: &FaceIn, params: &Params) -> Vec<f64> {
    for (m, &flat) in input.members.iter().enumerate() {
        let body = field.flat[flat as usize] as usize;
        field.at[body] = input.ats[m];
    }
    let mut out = Vec::with_capacity(input.members.len());
    let (mut member, mut foe_at) = (0usize, 0usize);
    for (f, &count) in input.member_counts.iter().enumerate() {
        let ids = &input.foe_ids[foe_at..foe_at + input.foe_counts[f] as usize];
        foe_at += ids.len();
        let foes = foes_of(field, ids);
        for _ in 0..count {
            out.push(bearing_after(field, &foes, input, params, member));
            member += 1;
        }
    }
    out
}

/// One unit's bearing after its turn.
fn bearing_after(field: &Field, foes: &Foes, input: &FaceIn, params: &Params, m: usize) -> f64 {
    let body = field.flat[input.members[m] as usize] as usize;
    let at = field.at[body];
    let unit = Mover {
        bearing: input.motion[3 * m],
        turn_rate: input.motion[3 * m + 1],
        backward_pace: input.motion[3 * m + 2],
    };
    let flags = input.flags[m];
    let radius = field.radius[body];
    let foe = touching(field, foes, at, radius, unit.bearing, params.reach, params.front);
    let mut look = foe.map(|f| field.at[f as usize]);
    if look.is_none() && flags & GOAL != 0 {
        let speed = input.speeds[m] * params.pace * params.crowding / params.seconds;
        let (next, foe_at) = (input.nexts[m], input.foe_ats[m]);
        let bearing = motion::bearing_to(next, foe_at, unit.bearing);
        look = Some(motion::look(&unit, at, next, bearing, speed));
    }
    if look.is_none() && flags & TOWARD != 0 {
        look = Some(input.towards[m]);
    }
    match look {
        Some(point) => {
            let wanted = motion::bearing_to(at, point, unit.bearing);
            motion::turn(&unit, wanted, params.seconds)
        }
        None => unit.bearing,
    }
}

/// ScrumBlows._pick: the foe (its body) the unit strikes - the least by [in its front (0)
/// or not (1), distance, the foe's draw] of those it touches.
pub fn touching(
    field: &Field,
    foes: &Foes,
    at: V2,
    radius: f64,
    bearing: f64,
    reach: f64,
    front_dot: f64,
) -> Option<u32> {
    let ahead = bearing_vector(bearing);
    let mut best: Option<(u8, f64, i64, u32)> = None;
    foes.grid.near(at, radius + reach + foes.widest + MARGIN, |i| {
        let f = foes.bodies[i as usize] as usize;
        let there = field.at[f];
        let apart = at.distance_to(there) as f64;
        let gap = apart - radius - field.radius[f];
        if (if gap > 0.0 { gap } else { 0.0 }) > reach {
            return true; // not touching (distances are never NaN)
        }
        let front = if in_front(ahead, at, there, front_dot) { 0 } else { 1 };
        let key = (front, apart, field.draw[f], f as u32);
        if best.as_ref().is_none_or(|b| before(&key, b)) {
            best = Some(key);
        }
        true
    });
    best.map(|b| b.3)
}

/// `a < b` as GDScript compares the Arrays [front, distance, draw]: item by item.
#[inline]
fn before(a: &(u8, f64, i64, u32), b: &(u8, f64, i64, u32)) -> bool {
    if a.0 != b.0 {
        return a.0 < b.0;
    }
    if a.1 < b.1 {
        return true;
    }
    if b.1 < a.1 {
        return false;
    }
    a.2 < b.2
}

/// ScrumReach.in_front: `point` lies in the front of a unit at `from` facing `ahead`.
#[inline]
pub fn in_front(ahead: V2, from: V2, point: V2, front: f64) -> bool {
    let direction = point.sub(from);
    if (direction.length() as f64) < 0.000001 {
        return true;
    }
    direction.normalized().dot(ahead) as f64 >= front
}

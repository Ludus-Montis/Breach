//! Pouring through a gap (FormationPour, spec 30 round 3, part 2): a unit whose place lies
//! on ground it can't walk makes for the nearest point across its frame it can, and where
//! several such points fall on one another, or on a place a unit already stands for, they
//! file in behind it: each squeezed unit, front first, then nearest its frame's centre
//! line, then by its seeded draw (Decision 97), takes the first point back from its own - a
//! body's breadth at a time, against its formation's way - that it can walk and that no
//! target taken before holds. So a formation queues at a ford and pours through it rather
//! than packing its units onto one point. Cells.

use std::cmp::Ordering;
use std::collections::BTreeMap;

use crate::contest;
use crate::ground::{Ground, Mode, Walker};
use crate::maths::{bearing_vector, minf, signf, snapped, V2};

/// FormationPour.EPSILON.
const EPSILON: f64 = 0.000001;
/// Cells a unit squeezes in at a time, looking for ground to stand on.
const SQUEEZE_STEP: f64 = 0.5;

/// A squad's frame (StoredSquad.frame): its front centre, heading, columns and centre shift.
#[derive(Clone, Copy)]
pub struct Frame {
    pub anchor: V2,
    pub heading: f64,
    pub width: i32,
    pub centre_shift: f64,
}

/// One of the squad's units that makes for a point: its id, where it stands, its place
/// (rank, column), footprint, how far it is through a swap (FormationShuffle.offset: ranks
/// forward, columns across), its walker, and where the downed foe it is out taking lies.
#[derive(Clone, Copy)]
pub struct Pourer {
    pub id: i64,
    pub at: V2,
    pub rank: i32,
    pub column: i32,
    pub footprint_width: i32,
    pub footprint_depth: i32,
    pub offset: V2,
    pub walker: Walker,
    pub taking: Option<V2>,
}

impl Pourer {
    /// ScrumReach.radius: the circle in its footprint.
    fn radius(&self) -> f64 {
        minf(self.footprint_width as f64, self.footprint_depth as f64) / 2.0
    }
}

/// FormationPour.place: where the unit belongs in its squad's frame now, any swap under
/// way included.
pub fn place(frame: &Frame, unit: &Pourer) -> V2 {
    let ahead = bearing_vector(frame.heading);
    let shift =
        ahead.scale(unit.offset.x as f64).sub(ahead.orthogonal().scale(unit.offset.y as f64));
    let across = unit.column as f64 - frame.width as f64 / 2.0
        + frame.centre_shift
        + unit.footprint_width as f64 / 2.0;
    let back = unit.rank as f64 + unit.footprint_depth as f64 / 2.0;
    let at = frame.anchor.add(ahead.orthogonal().scale(-across)).sub(ahead.scale(back));
    at.add(shift)
}

/// FormationPour.walkable: whether the walker can walk on the point - not swim or climb to
/// it. Anywhere on open ground.
pub fn walkable(ground: Option<&Ground>, walker: &Walker, point: V2) -> bool {
    let Some(ground) = ground else {
        return true;
    };
    ground.crossing(walker, point, point) > 0.0
        && ground.mode(walker, point, point) == Mode::Walking
}

/// What a unit makes for, before any files in: [its place, its point, squeezed].
struct Aim {
    place: V2,
    point: V2,
    squeezed: bool,
}

/// FormationPour.targets: where each unit makes for, in order: one out taking a downed foe,
/// beside it (`wounds_reach`); another, its place, or - where its place lies on ground it
/// can't walk - the nearest it can walk, squeezing across the frame towards its centre
/// line, the squeezed filing in behind one another; with none to walk, its place if it can
/// swim or climb there, else the nearest it can stand on.
pub fn targets(
    ground: Option<&Ground>,
    frame: &Frame,
    units: &[Pourer],
    seed: i64,
    wounds_reach: f64,
) -> Vec<V2> {
    let aims: Vec<Aim> = units
        .iter()
        .map(|unit| match unit.taking {
            Some(body) => {
                Aim { place: unit.at, point: beside(unit.at, body, wounds_reach), squeezed: false }
            }
            None => {
                let own = place(frame, unit);
                let point = aim(ground, frame, unit, own);
                Aim { place: own, point, squeezed: !point.is_equal_approx(own) }
            }
        })
        .collect();
    queue(ground, frame, units, &aims, seed)
}

/// FormationPour._beside: beside the downed foe lying at `body` that the unit at `unit` is
/// out taking.
fn beside(unit: V2, body: V2, wounds_reach: f64) -> V2 {
    let off = unit.sub(body);
    let near = minf(off.length() as f64, wounds_reach * 0.5);
    body.add(if (off.length() as f64) > EPSILON { off.normalized().scale(near) } else { off })
}

/// FormationPour._aim: the unit's own point: its place, or the nearest it can stand on.
fn aim(ground: Option<&Ground>, frame: &Frame, unit: &Pourer, own: V2) -> V2 {
    let Some(ground) = ground else {
        return own;
    };
    if walkable(Some(ground), &unit.walker, own) {
        return own;
    }
    if let Some(walk) = squeeze(ground, frame, unit, own, true) {
        return walk; // ground it can walk, near by
    }
    if ground.crossing(&unit.walker, own, own) > 0.0 {
        return own; // none to walk: it swims or climbs to its place
    }
    squeeze(ground, frame, unit, own, false).unwrap_or(own)
}

/// FormationPour._squeeze: the nearest point across the frame from `own` towards its centre
/// line where the unit can stand - walking it, if `walking`.
fn squeeze(ground: &Ground, frame: &Frame, unit: &Pourer, own: V2, walking: bool) -> Option<V2> {
    let across = bearing_vector(frame.heading).orthogonal();
    let offset = own.sub(frame.anchor).dot(across) as f64;
    let steps = (offset.abs() / SQUEEZE_STEP).ceil() as i64;
    for step in 1..=steps {
        let by = across.scale(signf(offset)).scale(minf(step as f64 * SQUEEZE_STEP, offset.abs()));
        let point = own.sub(by);
        let ok = if walking {
            walkable(Some(ground), &unit.walker, point)
        } else {
            ground.crossing(&unit.walker, point, point) > 0.0
        };
        if ok {
            return Some(point);
        }
    }
    None
}

/// The targets taken, bucketed by where they lie, and where each queue was last taken up.
struct Taken {
    bucket: f64,
    cells: BTreeMap<(i32, i32), Vec<(V2, f64)>>,
    ends: BTreeMap<(u32, u32, u64), i64>,
}

impl Taken {
    /// FormationPour._taken: the targets the unsqueezed hold.
    fn of(units: &[Pourer], aims: &[Aim]) -> Taken {
        let widest = units.iter().fold(0.5, |widest, unit| f64::max(widest, unit.radius()));
        let mut taken =
            Taken { bucket: widest * 2.0, cells: BTreeMap::new(), ends: BTreeMap::new() };
        for (unit, aim) in units.iter().zip(aims) {
            if !aim.squeezed {
                taken.take(aim.point, unit.radius());
            }
        }
        taken
    }

    fn home(&self, at: V2) -> (i32, i32) {
        let bucket = self.bucket as f32;
        ((at.x / bucket).floor() as i32, (at.y / bucket).floor() as i32)
    }

    fn take(&mut self, at: V2, radius: f64) {
        let home = self.home(at);
        self.cells.entry(home).or_default().push((at, radius));
    }

    /// FormationPour._free: true if no target taken lies within `own` and its own radius of
    /// `at`.
    fn free(&self, at: V2, own: f64) -> bool {
        let (hx, hy) = self.home(at);
        for dy in -1..=1 {
            for dx in -1..=1 {
                for &(held, radius) in self.cells.get(&(hx + dx, hy + dy)).into_iter().flatten() {
                    if (at.distance_to(held) as f64) < own + radius - EPSILON {
                        return false;
                    }
                }
            }
        }
        true
    }
}

/// A float's bits as a dictionary key compares it: -0 as 0.
fn key_bits(value: f32) -> u32 {
    (value + 0.0).to_bits()
}

/// FormationPour._queue: the unsqueezed keep their points; the squeezed file in behind
/// theirs, in turn.
fn queue(
    ground: Option<&Ground>,
    frame: &Frame,
    units: &[Pourer],
    aims: &[Aim],
    seed: i64,
) -> Vec<V2> {
    let mut out: Vec<V2> = aims.iter().map(|aim| aim.point).collect();
    let ahead = bearing_vector(frame.heading);
    let mut turns: Vec<((f64, f64, i64), usize)> = Vec::new(); // [front first, nearest the centre line, draw]
    for (index, aim) in aims.iter().enumerate() {
        if !aim.squeezed {
            continue;
        }
        let off = aim.place.sub(frame.anchor);
        let front = -snapped(off.dot(ahead) as f64, EPSILON);
        let centre = snapped((off.dot(ahead.orthogonal()) as f64).abs(), EPSILON);
        turns.push(((front, centre, contest::draw(seed, units[index].id)), index));
    }
    if turns.is_empty() {
        return out; // nobody squeezed: every unit makes for its own place
    }
    let mut taken = Taken::of(units, aims);
    turns.sort_by(|a, b| {
        let (ka, kb) = (a.0, b.0);
        ka.0.partial_cmp(&kb.0)
            .unwrap_or(Ordering::Equal)
            .then(ka.1.partial_cmp(&kb.1).unwrap_or(Ordering::Equal))
            .then(ka.2.cmp(&kb.2))
    });
    for &(_, index) in &turns {
        let unit = &units[index];
        let point = file_in(ground, unit, aims[index].point, ahead, &mut taken, aims.len() as i64);
        out[index] = point;
        taken.take(point, unit.radius());
    }
    out
}

/// FormationPour._file_in: the first point from `point` back against `ahead`, a body's
/// breadth at a time, that the unit can walk and no target taken holds; `point` if none is
/// within `limit` steps. A queue is taken up where its last unit filed in, so a long one
/// costs a step a unit.
fn file_in(
    ground: Option<&Ground>,
    unit: &Pourer,
    point: V2,
    ahead: V2,
    taken: &mut Taken,
    limit: i64,
) -> V2 {
    let breadth = unit.radius() * 2.0;
    let queue_key = (key_bits(point.x), key_bits(point.y), (breadth + 0.0).to_bits());
    let first = taken.ends.get(&queue_key).copied().unwrap_or(0);
    for step in first..=limit {
        let at = point.sub(ahead.scale(breadth * step as f64));
        if (step == 0 || walkable(ground, &unit.walker, at)) && taken.free(at, unit.radius()) {
            taken.ends.insert(queue_key, step);
            return at;
        }
    }
    point
}

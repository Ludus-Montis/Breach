//! The scrum's seeking (ScrumSeek.plan with SlotSearch, ScrumSlots and ScrumNear) on the
//! field's own state. Each fighting squad's free units touch a foe and stand, or make for
//! the nearest open slot round their foes' bodies, or wait a body's breadth short of the
//! nearest taken one. The claims are settled in rounds (spec 42, Decision 161): every
//! seeker not yet placed proposes a slot against the claims settled so far, and a proposal
//! wins unless one with a higher contest key lies within a body's breadth of it
//! (`settling`). Losers propose again; after `ROUNDS`, the rest press. A seeker's own
//! search is the GDScript reference's:
//! - slots, keys and claims in the same precision (`maths`);
//! - the same keys compared item by item, a full tie going to the slot listed first;
//! - the seeded draws by Godot's own `hash()` (`ghash`), drawn only when a tie needs them
//!   (the order of comparisons is unchanged by drawing lazily).
//!
//! Who seeks and where a unit's place is are gathered on the field from GDScript's say of
//! each squad (`seeking.rs`); the ground a slot lies on is asked of GDScript (`within`) only
//! when there is terrain.

use std::cmp::Ordering;
use std::collections::HashMap;

use rayon::prelude::*;

use crate::contest::{self, Contest};
use crate::field::{Field, DESTROYED, ROUTING};
use crate::ghash::{self, Key};
use crate::grid::{cell_of, floor_of, Grid, MARGIN};
use crate::maths::{bearing_vector, snapped, V2};
use crate::reach;
use crate::settling::{self, Proposal};
use crate::threads::Threads;

/// Tolerance for bodies just touching (ScrumSlots.EPSILON).
const EPSILON: f64 = 0.001;
/// Keys are snapped to this (snappedf(.., 0.000001)).
const SNAP: f64 = 0.000001;

/// A unit touches a foe: it stands (goal null, next where it is).
pub const TOUCH: u8 = 0;
/// A unit not free to seek: goal null, next unchanged.
pub const STAND: u8 = 1;
/// A seeker makes for a slot (goal, next = the slot, foe_at).
pub const AIM: u8 = 2;
/// A seeker waits a body's breadth short of a taken slot (goal, next, foe_at).
pub const PRESS: u8 = 3;
/// A seeker with nowhere to go: goal null, next where it is.
pub const IDLE: u8 = 4;

/// Rounds of proposing and settling claims before the rest fall back to pressing (spec 42).
const ROUNDS: usize = 3;

const FREE: u8 = 0;
const OWN: u8 = 1;
const TAKEN: u8 = 2;

/// The tick's numbers: BattleTuning's and the battle's.
pub struct Params {
    pub tick: i64,
    pub seed: i64,
    pub die: i64,
    pub reach: f64,
    pub leash: f64,
    /// Whether each squad's search (its seekers, their keys and rings) and each round's
    /// proposals and settling run on the pool; the plan is the same either way.
    pub threaded: Threads,
}

/// The fighting squads and their units, from GDScript's rules (see BodyField.seek).
pub struct PlanIn<'a> {
    pub fighters: &'a [i64],
    pub foe_counts: &'a [i32],
    pub foe_ids: &'a [i64],
    pub member_counts: &'a [i32],
    /// Each unit's flat index in the last sync.
    pub members: &'a [i32],
    pub may_seek: &'a [u8],
    pub speeds: &'a [f64],
    pub anchors: &'a [V2],
    pub goal_foes: &'a [i64],
    /// -1: no goal.
    pub goal_slots: &'a [i32],
}

/// Per unit of PlanIn.members: what it does (TOUCH...IDLE) and where it makes for.
#[derive(Default)]
pub struct PlanOut {
    pub codes: Vec<u8>,
    pub goal_foes: Vec<i64>,
    pub goal_slots: Vec<i32>,
    pub nexts: Vec<V2>,
    pub foe_ats: Vec<V2>,
}

struct Slot {
    point: V2,
    foe: u32,
    number: i32,
}

/// The slots round one squad's foes for seekers of one radius (SlotSearch.ring).
struct Ring {
    radius: f64,
    slots: Vec<Slot>,
    by_foe: HashMap<i64, (u32, u32)>,
    status: Vec<u8>,
    own: Vec<u32>,
    dead: Vec<bool>,
    grid: Grid,
    /// The slots each body alone stands on.
    owned: HashMap<u32, Vec<u32>>,
    /// The slots no body stands on, less those found claimed at the last refresh, and a
    /// grid over them; `gone`: how many of them have been found claimed since.
    free: Vec<u32>,
    free_grid: Grid,
    gone: usize,
}

/// A fighting squad's foes, indexed.
pub struct Foes {
    pub bodies: Vec<u32>,
    pub grid: Grid,
    pub widest: f64,
}

struct Seeker {
    member: usize,
    body: u32,
    ring: usize,
    /// Its key in the contest for slots (ScrumContest.key).
    key: Contest,
}

/// The bodies standing (ScrumSlots.bodies: not a routing or destroyed squad's), indexed.
#[derive(Default)]
struct Crowd {
    bodies: Vec<u32>,
    points: Vec<V2>,
    grid: Grid,
    widest: f64,
}

/// What the plan keeps on the field between ticks (`buffers.rs`): its crowd, claims and
/// seekers, each cleared and filled afresh by every plan.
#[derive(Default)]
pub struct Buffers {
    crowd: Crowd,
    claims: Claims,
    seekers: Vec<Seeker>,
}

impl Buffers {
    /// True once a plan has kept something in them.
    pub fn warm(&self) -> bool {
        self.crowd.bodies.capacity() > 0 || self.seekers.capacity() > 0
    }
}

/// Points claimed this tick, in a grid remade after each round's claims.
#[derive(Default)]
struct Claims {
    points: Vec<V2>,
    grid: Grid,
}

impl Claims {
    fn clear(&mut self) {
        self.points.clear();
        self.grid.rebuild(&self.points);
    }

    fn extend(&mut self, points: impl IntoIterator<Item = V2>) {
        self.points.extend(points);
        self.grid.rebuild(&self.points);
    }

    /// True if a claim lies within a body's breadth of `point` (SlotSearch._claimed).
    fn near(&self, point: V2, radius: f64) -> bool {
        let mut near = false;
        self.grid.near(point, 2.0 * radius + MARGIN, |i| {
            near = (self.points[i as usize].distance_to(point) as f64) < breadth(radius);
            !near
        });
        near
    }
}

/// How near two claims of seekers of `radius` may lie: a body's breadth, just touching.
fn breadth(radius: f64) -> f64 {
    2.0 * radius - EPSILON
}

/// What a seeker proposes in a round: to aim at a slot, or (no slot open) to press toward
/// the nearest one, or idle (None). Each with the slots it found claimed.
enum Choice {
    Aim(usize),
    Press(Option<usize>),
}

/// Everything one plan works over.
struct Plan<'a> {
    field: &'a Field,
    input: &'a PlanIn<'a>,
    params: &'a Params,
    crowd: &'a Crowd,
    rings: Vec<Ring>,
    claims: Claims,
    within: &'a (dyn Fn(usize, V2) -> bool + Sync),
    out: PlanOut,
}

/// ScrumSeek.plan: every fighting squad's units, their claims settled in rounds, in buffers
/// kept from an earlier plan (or new ones): the plan is the same either way.
pub fn plan(
    field: &Field,
    input: &PlanIn,
    params: &Params,
    within: &(dyn Fn(usize, V2) -> bool + Sync),
    kept: &mut Buffers,
) -> PlanOut {
    let count = input.members.len();
    let Buffers { crowd, claims, seekers } = kept;
    fill_crowd(field, crowd);
    claims.clear();
    let search = Search { field, input, params, crowd };
    let squads = search.squads();
    let mut plan = Plan {
        field,
        input,
        params,
        crowd,
        rings: Vec::new(),
        claims: std::mem::take(claims),
        within,
        out: PlanOut {
            codes: vec![STAND; count],
            goal_foes: vec![0; count],
            goal_slots: vec![-1; count],
            nexts: vec![V2::ZERO; count],
            foe_ats: vec![V2::ZERO; count],
        },
    };
    plan.take(squads, seekers);
    let threaded = params.threaded.on(count);
    let mut left: Vec<&Seeker> = seekers.iter().collect();
    for round in 0..ROUNDS {
        if left.is_empty() {
            break;
        }
        let proposed = plan.each(&left, threaded, |plan, s| plan.propose(s, round == 0));
        left = plan.settle(left, proposed, threaded);
    }
    let fallbacks = plan.each(&left, threaded, |plan, s| plan.pick(s, true, &mut Vec::new()));
    for (seeker, slot) in left.into_iter().zip(fallbacks) {
        plan.press(seeker, slot);
    }
    *claims = plan.claims;
    plan.out
}

/// The crowd as the field stands, in `crowd`'s buffers.
fn fill_crowd(field: &Field, crowd: &mut Crowd) {
    crowd.bodies.clear();
    for &sq in &field.order {
        let squad = &field.squads[sq as usize];
        if squad.flags & (ROUTING | DESTROYED) == 0 {
            crowd.bodies.extend_from_slice(&squad.members);
        }
    }
    crowd.points.clear();
    crowd.points.extend(crowd.bodies.iter().map(|&b| field.at[b as usize]));
    crowd.widest = crowd.bodies.iter().map(|&b| field.radius[b as usize]).fold(0.0, f64::max);
    crowd.grid.rebuild(&crowd.points);
}

/// Compares two f64 keys item by item, as GDScript compares Arrays.
fn keys(a: &[f64; 3], b: &[f64; 3]) -> Ordering {
    for i in 0..3 {
        if a[i] < b[i] {
            return Ordering::Less;
        }
        if b[i] < a[i] {
            return Ordering::Greater;
        }
    }
    Ordering::Equal
}

/// The best slot so far of one search.
struct Best {
    slot: usize,
    key: [f64; 3],
    draw: i64,
    roll: Option<f64>,
}

impl<'a> Plan<'a> {
    /// The squads' searches taken in squad order into `out`: their rings listed one after
    /// another, each seeker pointing at its own, and the units touching a foe standing.
    fn take(&mut self, squads: Vec<Squad>, out: &mut Vec<Seeker>) {
        out.clear();
        for squad in squads {
            let first = self.rings.len();
            self.rings.extend(squad.rings);
            for (member, at) in squad.touching {
                self.out.codes[member] = TOUCH;
                self.out.nexts[member] = at;
            }
            out.extend(squad.seekers.into_iter().map(|s| Seeker { ring: first + s.ring, ..s }));
        }
    }

    /// `propose` (or another search) for each seeker, in their order, on the pool or not.
    fn each<T: Send>(
        &self,
        seekers: &[&Seeker],
        threaded: bool,
        search: impl Fn(&Self, &Seeker) -> T + Sync,
    ) -> Vec<T> {
        if threaded {
            seekers.par_iter().map(|s| search(self, s)).collect()
        } else {
            seekers.iter().map(|s| search(self, s)).collect()
        }
    }

    /// A seeker's proposal against the claims settled so far: in the first round the slot
    /// it made for if still open (ScrumSeek._kept), else the nearest open slot; with none
    /// open, it presses. Returns the slots found claimed on the way.
    fn propose(&self, seeker: &Seeker, first: bool) -> (Choice, Vec<usize>) {
        let mut dead = Vec::new();
        if let Some(slot) = first.then(|| self.kept(seeker)).flatten() {
            return (Choice::Aim(slot), dead);
        }
        let choice = match self.pick(seeker, false, &mut dead) {
            Some(slot) => Choice::Aim(slot),
            None => Choice::Press(self.pick(seeker, true, &mut dead)),
        };
        (choice, dead)
    }

    /// One round settled: the aims no higher-keyed aim lies within a body's breadth of
    /// claim, and with the presses are written out in the seekers' order; the slots found
    /// claimed are marked dead for all. Returns the seekers whose aims lost.
    fn settle<'s>(
        &mut self,
        seekers: Vec<&'s Seeker>,
        proposed: Vec<(Choice, Vec<usize>)>,
        threaded: bool,
    ) -> Vec<&'s Seeker> {
        let proposals: Vec<Proposal<Contest>> = seekers
            .iter()
            .zip(&proposed)
            .filter_map(|(s, (choice, _))| match *choice {
                Choice::Aim(slot) => {
                    let ring = &self.rings[s.ring];
                    let reach = breadth(ring.radius);
                    Some(Proposal { key: s.key, at: ring.slots[slot].point, reach })
                }
                Choice::Press(_) => None,
            })
            .collect();
        let before = |a: &Contest, b: &Contest| contest::order(a, b) == Ordering::Less;
        let mut wins = settling::winners(&proposals, before, threaded).into_iter();
        let (mut left, mut claimed) = (Vec::new(), Vec::new());
        for (seeker, (choice, dead)) in seekers.into_iter().zip(proposed) {
            self.bury(seeker.ring, &dead);
            match choice {
                Choice::Aim(slot) if wins.next() == Some(true) => {
                    claimed.push(self.aim(seeker, slot));
                }
                Choice::Aim(_) => left.push(seeker),
                Choice::Press(slot) => self.press(seeker, slot),
            }
        }
        self.claims.extend(claimed);
        for ring in 0..self.rings.len() {
            self.refresh(ring);
        }
        left
    }

    /// The slots found claimed are gone for all.
    fn bury(&mut self, ring: usize, dead: &[usize]) {
        let ring = &mut self.rings[ring];
        for &slot in dead {
            if !ring.dead[slot] {
                ring.dead[slot] = true;
                if ring.status[slot] == FREE {
                    ring.gone += 1;
                }
            }
        }
    }

    /// SlotSearch.open: in the seeker's leash on ground it can cross, no other body on it,
    /// no claim near it.
    fn open(&self, seeker: &Seeker, point: V2, radius: f64) -> bool {
        if !self.within(seeker, point) {
            return false;
        }
        let field = self.field;
        let crowd = self.crowd;
        let mut clear = true;
        crowd.grid.near(point, crowd.widest + radius + MARGIN, |i| {
            let b = crowd.bodies[i as usize];
            let r = field.radius[b as usize];
            clear = b == seeker.body
                || field.at[b as usize].distance_to(point) as f64 >= r + radius - EPSILON;
            clear
        });
        clear && !self.claims.near(point, radius)
    }

    /// ScrumSlots._within: within the leash of its place, on ground it can cross.
    fn within(&self, seeker: &Seeker, point: V2) -> bool {
        let place = self.input.anchors[seeker.member];
        if point.distance_to(place) as f64 > self.params.leash {
            return false;
        }
        (self.within)(seeker.member, point)
    }

    /// ScrumSeek._kept: the slot it made for, if it is still open.
    fn kept(&self, seeker: &Seeker) -> Option<usize> {
        let goal = self.input.goal_slots[seeker.member];
        let ring = &self.rings[seeker.ring];
        let &(first, count) = ring.by_foe.get(&self.input.goal_foes[seeker.member])?;
        if goal < 0 || goal as u32 >= count {
            return None;
        }
        let slot = (first + goal as u32) as usize;
        let (point, radius) = (ring.slots[slot].point, ring.radius);
        self.open(seeker, point, radius).then_some(slot)
    }

    /// SlotSearch.pick: the nearest open slot (crowded: the nearest within its leash,
    /// open or not), equals by key, then the slot listed first. The slots it finds claimed
    /// go to `dead`.
    fn pick(&self, seeker: &Seeker, crowded: bool, dead: &mut Vec<usize>) -> Option<usize> {
        let field = self.field;
        let at = field.at[seeker.body as usize];
        let place = self.input.anchors[seeker.member];
        let ahead = bearing_vector(field.bearing[seeker.body as usize]);
        let bound = self.params.leash + at.distance_to(place) as f64;
        let centre = cell_of(at);
        let frame = (at, place, ahead);
        let mut best: Option<Best> = None;
        let ring = &self.rings[seeker.ring];
        if !crowded {
            for &slot in ring.owned.get(&seeker.body).into_iter().flatten() {
                self.consider(seeker, slot as usize, false, frame, (&mut best, dead));
            }
        }
        let last =
            if crowded { ring.grid.last_ring(centre) } else { ring.free_grid.last_ring(centre) };
        let mut found = Vec::new();
        for number in 0..=last {
            let floor = floor_of(number) - MARGIN;
            if floor > bound || best.as_ref().is_some_and(|b| floor > b.key[0]) {
                break;
            }
            found.clear();
            if crowded {
                ring.grid.ring(centre, number, |i| found.push(i as usize));
            } else {
                ring.free_grid.ring(centre, number, |i| found.push(ring.free[i as usize] as usize));
            }
            for &slot in &found {
                self.consider(seeker, slot, crowded, frame, (&mut best, dead));
            }
        }
        best.map(|b| b.slot)
    }

    /// Remakes a ring's free grid without the slots found claimed, once they are half of it.
    fn refresh(&mut self, ring: usize) {
        let ring = &mut self.rings[ring];
        if ring.gone * 2 <= ring.free.len() {
            return;
        }
        let dead = &ring.dead;
        ring.free.retain(|&i| !dead[i as usize]);
        let points: Vec<V2> = ring.free.iter().map(|&i| ring.slots[i as usize].point).collect();
        ring.free_grid = Grid::build(&points);
        ring.gone = 0;
    }

    /// SlotSearch._consider: the slot becomes the best if it qualifies and its key is
    /// lower, or the same and it is listed first; found claimed, it goes to `dead`.
    fn consider(
        &self,
        seeker: &Seeker,
        slot: usize,
        crowded: bool,
        (at, place, ahead): (V2, V2, V2),
        (best, dead): (&mut Option<Best>, &mut Vec<usize>),
    ) {
        let ring = &self.rings[seeker.ring];
        let point = ring.slots[slot].point;
        let distance = snapped(at.distance_to(point) as f64, SNAP);
        if best.as_ref().is_some_and(|b| distance > b.key[0]) {
            return;
        }
        if !crowded {
            let mine = ring.status[slot] == OWN && ring.own[slot] == seeker.body;
            if ring.dead[slot] || !(ring.status[slot] == FREE || mine) {
                return;
            }
        }
        if !self.within(seeker, point) {
            return;
        }
        if !crowded && self.claims.near(point, ring.radius) {
            dead.push(slot); // claimed: gone for all
            return;
        }
        let key = [
            distance,
            -snapped(point.sub(at).dot(ahead) as f64, SNAP),
            snapped(point.distance_to(place) as f64, SNAP),
        ];
        let draw = self.field.draw[ring.slots[slot].foe as usize];
        let better = match best.as_mut() {
            None => true,
            Some(b) => match keys(&key, &b.key).then(draw.cmp(&b.draw)) {
                Ordering::Less => true,
                Ordering::Greater => false,
                Ordering::Equal => {
                    let mine = self.roll(seeker, slot);
                    let theirs = *b.roll.get_or_insert_with(|| self.roll(seeker, b.slot));
                    mine < theirs || (mine == theirs && slot < b.slot)
                }
            },
        };
        if better {
            *best = Some(Best { slot, key, draw, roll: None });
        }
    }

    /// The seeded draw between two mirror-image slots (BattleRolls.uniform(seed,
    /// [seeker, foe, slot, "slot"])).
    fn roll(&self, seeker: &Seeker, slot: usize) -> f64 {
        let field = self.field;
        let s = &self.rings[seeker.ring].slots[slot];
        let keys = [
            Key::Int(field.unit_id[seeker.body as usize]),
            Key::Int(field.unit_id[s.foe as usize]),
            Key::Int(s.number as i64),
            Key::Str("slot"),
        ];
        ghash::uniform(self.params.seed, &keys)
    }

    /// ScrumSeek._aim: the seeker makes for the slot; returns the point it claims.
    fn aim(&mut self, seeker: &Seeker, slot: usize) -> V2 {
        let s = &self.rings[seeker.ring].slots[slot];
        let m = seeker.member;
        self.out.codes[m] = AIM;
        self.out.goal_foes[m] = self.field.unit_id[s.foe as usize];
        self.out.goal_slots[m] = s.number;
        self.out.nexts[m] = s.point;
        self.out.foe_ats[m] = self.field.at[s.foe as usize];
        s.point
    }

    /// ScrumSeek._press: it waits a body's breadth short of the slot, claiming nothing.
    fn press(&mut self, seeker: &Seeker, slot: Option<usize>) {
        let m = seeker.member;
        let field = self.field;
        let at = field.at[seeker.body as usize];
        let Some(slot) = slot else {
            self.out.codes[m] = IDLE;
            self.out.nexts[m] = at;
            return;
        };
        let s = &self.rings[seeker.ring].slots[slot];
        let back = at.sub(s.point);
        let breadth = 2.0 * field.radius[seeker.body as usize];
        self.out.codes[m] = PRESS;
        self.out.goal_foes[m] = field.unit_id[s.foe as usize];
        self.out.goal_slots[m] = s.number;
        self.out.nexts[m] = if back.length() as f64 <= breadth {
            at
        } else {
            s.point.add(back.normalized().scale(breadth))
        };
        self.out.foe_ats[m] = field.at[s.foe as usize];
    }
}

/// What a squad's search reads: nothing it or another squad's search writes.
struct Search<'a> {
    field: &'a Field,
    input: &'a PlanIn<'a>,
    params: &'a Params,
    crowd: &'a Crowd,
}

/// One squad's search: its seekers (each `ring` its own, from 0), their rings, and the
/// units touching a foe, with where they stand.
#[derive(Default)]
struct Squad {
    seekers: Vec<Seeker>,
    rings: Vec<Ring>,
    touching: Vec<(usize, V2)>,
}

impl Search<'_> {
    /// The fighting squads' units: those touching a foe stand, those not free to seek keep
    /// to their places, the rest seek (ScrumSeek._seekers_of).
    /// Each squad apart, in squad order (on the pool, with `Params::threaded`).
    fn squads(&self) -> Vec<Squad> {
        let mut squads = Vec::with_capacity(self.input.fighters.len());
        let (mut member, mut foe_at) = (0usize, 0usize);
        for f in 0..self.input.fighters.len() {
            let foe_ids = &self.input.foe_ids[foe_at..foe_at + self.input.foe_counts[f] as usize];
            foe_at += foe_ids.len();
            let units = self.input.member_counts[f] as usize;
            squads.push((member, units, foe_ids));
            member += units;
        }
        let one = |&(first, units, ids): &(usize, usize, &[i64])| self.squad(first, units, ids);
        if self.params.threaded.on(self.input.members.len()) {
            squads.par_iter().map(one).collect()
        } else {
            squads.iter().map(one).collect()
        }
    }

    /// One squad's units from `first`: a ring for each radius among its seekers.
    fn squad(&self, first: usize, units: usize, foe_ids: &[i64]) -> Squad {
        let foes = foes_of(self.field, foe_ids);
        let mut out = Squad::default();
        let mut radii: Vec<u64> = Vec::new();
        for member in first..first + units {
            let Some(mut s) = self.seeker(member, &foes, &mut out.touching) else {
                continue;
            };
            let radius = self.field.radius[s.body as usize];
            s.ring = match radii.iter().position(|&r| r == radius.to_bits()) {
                Some(ring) => ring,
                None => {
                    out.rings.push(self.ring(&foes, radius));
                    radii.push(radius.to_bits());
                    radii.len() - 1
                }
            };
            out.seekers.push(s);
        }
        out
    }

    /// One unit: it stands touching a foe, keeps to its place, or seeks (Some).
    fn seeker(
        &self,
        member: usize,
        foes: &Foes,
        touching: &mut Vec<(usize, V2)>,
    ) -> Option<Seeker> {
        let field = self.field;
        let body = field.flat[self.input.members[member] as usize];
        let b = body as usize;
        let (at, radius) = (field.at[b], field.radius[b]);
        if self.touches(at, radius, foes) {
            touching.push((member, at));
            return None;
        }
        if self.input.may_seek[member] == 0 || foes.bodies.is_empty() {
            return None; // STAND
        }
        let speed = self.input.speeds[member];
        let arrival = gap_to(field, foes, at, radius) / speed.max(0.01);
        let p = self.params;
        let rolled = contest::roll(p.seed, p.tick, field.unit_id[b], p.die);
        let key = Contest::of(arrival, speed, field.initiative[b], rolled, field.draw[b]);
        Some(Seeker { member, body, ring: 0, key })
    }

    /// ScrumBlows.touches_any: a foe's body within reach_contact of the unit's.
    fn touches(&self, at: V2, radius: f64, foes: &Foes) -> bool {
        let field = self.field;
        let reach = self.params.reach;
        let mut touching = false;
        foes.grid.near(at, radius + reach + foes.widest + MARGIN, |i| {
            let f = foes.bodies[i as usize] as usize;
            let apart = at.distance_to(field.at[f]) as f64;
            touching = reach::apart(apart, radius, field.radius[f]) <= reach;
            !touching
        });
        touching
    }

    /// SlotSearch.ring: the slots round the foes for a seeker of `radius`, with the bodies
    /// standing on each (none: free; only one: its own).
    fn ring(&self, foes: &Foes, radius: f64) -> Ring {
        let field = self.field;
        let mut slots = Vec::new();
        let mut by_foe = HashMap::new();
        for &foe in &foes.bodies {
            let f = foe as usize;
            let reach = field.radius[f] + radius;
            let count = slot_count(reach, radius);
            by_foe.entry(field.unit_id[f]).or_insert((slots.len() as u32, count as u32));
            for number in 0..count {
                let way = bearing_vector(field.bearing[f] + number as f64 * 360.0 / count as f64);
                slots.push(Slot {
                    point: field.at[f].add(way.scale(reach)),
                    foe,
                    number: number as i32,
                });
            }
        }
        let mut status = Vec::with_capacity(slots.len());
        let mut own = Vec::with_capacity(slots.len());
        for slot in &slots {
            let (on, first) = self.standing_on(slot.point, radius);
            status.push(on);
            own.push(first);
        }
        let points: Vec<V2> = slots.iter().map(|s| s.point).collect();
        let dead = vec![false; slots.len()];
        let mut owned: HashMap<u32, Vec<u32>> = HashMap::new();
        let mut free = Vec::new();
        for (i, &on) in status.iter().enumerate() {
            match on {
                FREE => free.push(i as u32),
                OWN => owned.entry(own[i]).or_default().push(i as u32),
                _ => {}
            }
        }
        let free_points: Vec<V2> = free.iter().map(|&i| points[i as usize]).collect();
        Ring {
            radius,
            grid: Grid::build(&points),
            slots,
            by_foe,
            status,
            own,
            dead,
            owned,
            free_grid: Grid::build(&free_points),
            free,
            gone: 0,
        }
    }

    /// Whether bodies stand on `point` for a seeker of `radius`: FREE, OWN (with whose) or
    /// TAKEN (SlotSearch._standing_on, which stops at two).
    fn standing_on(&self, point: V2, radius: f64) -> (u8, u32) {
        let field = self.field;
        let crowd = self.crowd;
        let (mut on, mut first) = (0u8, u32::MAX);
        crowd.grid.near(point, crowd.widest + radius + MARGIN, |i| {
            let b = crowd.bodies[i as usize];
            let r = field.radius[b as usize];
            if (field.at[b as usize].distance_to(point) as f64) < r + radius - EPSILON {
                on += 1;
                first = b;
            }
            on < 2
        });
        (on.min(TAKEN), first)
    }
}

/// How many slots fit round a foe for a seeker of `radius`, their centres `reach` from the
/// foe's (the two radii): as many as fit, at least 3 (6 for a grem round a grem, 9 round a
/// brute).
fn slot_count(reach: f64, radius: f64) -> i64 {
    ((std::f64::consts::PI * reach / radius + 0.000001).floor() as i64).max(3)
}

/// ScrumSeek.foe_units: the living units of the squads with these ids, not routing, in
/// the list's order, indexed where they stand.
pub fn foes_of(field: &Field, ids: &[i64]) -> Foes {
    let mut bodies = Vec::new();
    for &sq in &field.order {
        let squad = &field.squads[sq as usize];
        if squad.flags & ROUTING == 0 && ids.contains(&squad.id) {
            bodies.extend_from_slice(&squad.members);
        }
    }
    let points: Vec<V2> = bodies.iter().map(|&b| field.at[b as usize]).collect();
    let widest = bodies.iter().map(|&b| field.radius[b as usize]).fold(0.0, f64::max);
    Foes { grid: Grid::build(&points), bodies, widest }
}

/// ScrumNear.gap_to: cells from `at` to the nearest foe body's edge, less `radius` (0 if
/// it touches one).
fn gap_to(field: &Field, foes: &Foes, at: V2, radius: f64) -> f64 {
    let mut least = f64::INFINITY;
    let centre = cell_of(at);
    for ring in 0..=foes.grid.last_ring(centre) {
        if floor_of(ring) - foes.widest - radius - MARGIN > least {
            break;
        }
        foes.grid.ring(centre, ring, |i| {
            let f = foes.bodies[i as usize] as usize;
            let edge = field.at[f].distance_to(at) as f64;
            least = least.min(edge - field.radius[f] - radius);
        });
    }
    least.max(0.0)
}

pub mod pass;
pub mod prepare;
pub mod pursue;
pub mod regroup;
pub mod seek;
pub mod step;
pub mod withdraw;

#[cfg(test)]
mod tests;

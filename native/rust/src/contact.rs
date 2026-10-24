//! Contact between squads (FormationContact, FormationFronts and FormationEdges, Decisions
//! 40, 44, 47, 51, 78, 97 and 111; spec 39 step 4c): which hostile front is within melee
//! reach of a squad's, facing it; where an advancing squad must stop (at reach of a hostile
//! front or face, or behind a friend's back rank); whether a squad with no front units holds
//! to skirmish; which friend's back rank a wave has reached to join. Every free squad picks
//! the nearest hostile front it reaches, all from one snapshot, then the locks land together
//! (`fronts`); an attacker whose front reaches a hostile's side or rear locks on there, the
//! nearest holding an edge two reach at once (`edges`). Ties go to the squads' seeded draws,
//! never to the lists' order. Squads are the roster's, by their index in it.

use crate::columns::Columns;
use crate::contest;
use crate::geometry::{self, Shape, RANK_DEPTH};
use crate::locks::{self, Locking};
use crate::maths::snapped;
use crate::roster::Roster;
use crate::squads::Squads;

/// Tiles between engaged fronts: a little over one cell (Decision 68).
pub const MELEE_REACH: f64 = 0.0175;
const EPSILON: f64 = 0.000001;
/// SkirmishUnit.Order.ADVANCE.
const ADVANCE: i32 = 0;

/// The squads as one decision reads them: the store's, and the roster's.
pub struct Field<'a> {
    pub squads: &'a Squads,
    pub columns: &'a Columns,
    pub roster: &'a Roster,
}

impl<'a> Field<'a> {
    fn shape(&self, n: usize) -> Shape<'a> {
        Shape::of(self.squads, self.columns, &self.roster.listed[n])
    }

    fn slot(&self, n: usize) -> usize {
        self.roster.listed[n].squad
    }

    fn hostile(&self, a: usize, b: usize) -> bool {
        self.roster.listed[a].faction != self.roster.listed[b].faction
    }

    /// True if the squad may be engaged (FormationContact.engageable): standing, whatever it
    /// is doing.
    pub fn engageable(&self, n: usize) -> bool {
        self.squads.state[self.slot(n)] != locks::DESTROYED
            && !self.roster.listed[n].living.is_empty()
    }

    /// True if its units stand in its frame (FormationContact.framed): not routing.
    pub fn framed(&self, n: usize) -> bool {
        self.squads.state[self.slot(n)] != locks::ROUTING
    }

    /// True if it seeks combat (FormationContact.can_engage).
    pub fn can_engage(&self, n: usize) -> bool {
        locks::can_engage(self.squads, self.slot(n))
    }

    /// FormationContact.nearest_hostile: the hostile front among `among` within melee reach
    /// of `from`'s, facing it: the nearest, a tie going to the squads' draws.
    pub fn nearest_hostile(&self, from: usize, among: &[usize], seed: i64) -> Option<usize> {
        let mine = self.shape(from);
        let mut best: Option<(usize, (f64, i64))> = None;
        for &other in among {
            if !self.hostile(from, other) || !self.engageable(other) || !self.framed(other) {
                continue;
            }
            let theirs = self.shape(other);
            if !geometry::facing_off(&mine, &theirs) || !geometry::overlaps(&mine, &theirs) {
                continue;
            }
            let gap = geometry::gap(&mine, &theirs).abs();
            let key =
                (snapped(gap, EPSILON), contest::squad_draw(seed, self.roster.listed[other].id));
            if gap <= MELEE_REACH + EPSILON && best.is_none_or(|(_, b)| key < b) {
                best = Some((other, key));
            }
        }
        best.map(|(n, _)| n)
    }

    /// FormationContact.limit: the furthest the advancing squad `mover` may get this tick,
    /// wanting to reach `next` (tiles along its route), judged from where everyone stood as
    /// the tick began. `marching`: the squads (ids) advancing this tick; two hostiles
    /// marching at each other each close at most half the gap between them.
    pub fn limit(&self, mover: usize, next: f64, marching: &[i64]) -> f64 {
        let slot = self.slot(mover);
        let (front, direction) =
            (self.squads.front_distance[slot], self.squads.direction[slot] as f64);
        let mut next = next;
        for other in 0..self.roster.listed.len() {
            if other == mover || !self.can_engage(other) {
                continue;
            }
            let mut room = self.room(mover, other);
            if room == f64::INFINITY {
                continue;
            }
            if marching.contains(&self.roster.listed[other].id) && self.closing_on(mover, other) {
                room /= 2.0; // it closes too: each takes its half
            }
            let reachable = front + direction * if room > 0.0 { room } else { 0.0 };
            next = if direction > 0.0 {
                if next < reachable {
                    next
                } else {
                    reachable
                }
            } else if next > reachable {
                next
            } else {
                reachable
            };
        }
        next
    }

    /// How far the mover may come toward `other` (tiles): to melee reach of a hostile's
    /// front or of its side or rear face, or to just behind a friend; INF if `other` isn't
    /// ahead across the mover's line.
    fn room(&self, mover: usize, other: usize) -> f64 {
        let (mine, theirs) = (self.shape(mover), self.shape(other));
        let hostile = self.hostile(mover, other);
        if hostile && !geometry::facing_off(&mine, &theirs) {
            let face = geometry::face_gap(&mine, &theirs);
            if face > EPSILON && geometry::overlap_across(&mine, &theirs) {
                return face - MELEE_REACH;
            }
            return f64::INFINITY;
        }
        if !ahead(&mine, &theirs) {
            return f64::INFINITY;
        }
        if hostile {
            return geometry::gap(&mine, &theirs) - MELEE_REACH;
        }
        geometry::gap(&mine, &theirs) - depth(&theirs)
    }

    fn closing_on(&self, mover: usize, other: usize) -> bool {
        self.hostile(mover, other) && geometry::facing_off(&self.shape(mover), &self.shape(other))
    }

    /// FormationContact.skirmishing: true if the advancing squad, with no front-preferring
    /// units, has an enemy within its ranged reach: it holds there (Decision 47).
    pub fn skirmishing(&self, mover: usize) -> bool {
        let listed = &self.roster.listed[mover];
        if self.squads.order[listed.squad] != ADVANCE
            || listed.living.iter().any(|u| u.preferred == 0)
        {
            return false;
        }
        let reach = listed.living.iter().map(|u| u.attack_range).max().unwrap_or(0).max(0);
        if reach == 0 {
            return false;
        }
        let mine = self.shape(mover);
        (0..self.roster.listed.len()).any(|other| {
            if !self.hostile(mover, other)
                || !self.can_engage(other)
                || self.roster.listed[other].living.is_empty()
            {
                return false;
            }
            let theirs = self.shape(other);
            geometry::overlaps(&mine, &theirs)
                && geometry::gap(&mine, &theirs).abs() <= reach as f64 * RANK_DEPTH + EPSILON
        })
    }

    /// FormationContact.joinable: the friend whose back rank the mover has reached and may
    /// join - one in combat, or, when the mover merges, one on the march not retreating.
    pub fn joinable(&self, mover: usize) -> Option<usize> {
        let mine = self.shape(mover);
        (0..self.roster.listed.len()).find(|&other| {
            if other == mover || self.hostile(mover, other) || !self.accepts(mover, other) {
                return false;
            }
            let theirs = self.shape(other);
            ahead(&mine, &theirs) && geometry::gap(&mine, &theirs) - depth(&theirs) <= EPSILON
        })
    }

    fn accepts(&self, mover: usize, leader: usize) -> bool {
        let slot = self.slot(leader);
        let state = self.squads.state[slot];
        if state == locks::FIGHTING {
            return true;
        }
        self.roster.listed[mover].merges
            && (state == locks::MOVING || state == locks::HOLDING)
            && self.squads.order[slot] != locks::RETREAT
    }
}

fn ahead(mover: &Shape, other: &Shape) -> bool {
    geometry::gap(mover, other) > EPSILON && geometry::overlaps(mover, other)
}

/// How deep a squad is in tiles: a squad behind it stops that far behind its front.
fn depth(friend: &Shape) -> f64 {
    friend.back_rows() as f64 * RANK_DEPTH
}

fn lock(squads: &mut Squads, columns: &mut Columns, roster: &Roster, n: usize, foe: usize) {
    let listed = &roster.listed[n];
    let living = listed.slots();
    let locking = Locking { squad: listed.squad, living: &living, joined: listed.joined };
    locks::lock(squads, columns, &locking, roster.listed[foe].id);
}

/// FormationFronts.engage: free squads lock onto the hostile fronts they reach, every pick
/// made before any lock lands; a free foe that picked no one locks onto the nearest of those
/// that picked it, unless it is getting away (Decision 111). Returns the picks' locks as
/// (attacker, foe) in the lists' order, for their "engaged" events.
pub fn fronts(
    squads: &mut Squads,
    columns: &mut Columns,
    roster: &Roster,
    seed: i64,
) -> Vec<(usize, usize)> {
    let everyone: Vec<usize> = (0..roster.listed.len()).collect();
    let mut picks = Vec::new();
    {
        let field = Field { squads, columns, roster };
        for attacker in 0..roster.listed.len() {
            let slot = roster.listed[attacker].squad;
            if !field.can_engage(attacker) || squads.engaged_with[slot] != 0 {
                continue;
            }
            if let Some(foe) = field.nearest_hostile(attacker, &everyone, seed) {
                picks.push((attacker, foe));
            }
        }
    }
    for &(attacker, foe) in &picks {
        lock(squads, columns, roster, attacker, foe);
    }
    for &(_, foe) in &picks {
        let field = Field { squads, columns, roster };
        if squads.engaged_with[roster.listed[foe].squad] != 0 || !field.can_engage(foe) {
            continue; // one getting away doesn't turn to fight (Decision 111)
        }
        let pickers: Vec<usize> = picks.iter().filter(|p| p.1 == foe).map(|p| p.0).collect();
        if let Some(nearest) = field.nearest_hostile(foe, &pickers, seed) {
            lock(squads, columns, roster, foe, nearest); // reach is mutual, so one always is
        }
    }
    picks
}

/// A flank lock landed (FormationEdges._lock_on): the victim, the attacker, and the edge.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Flanked {
    pub victim: usize,
    pub attacker: usize,
    pub edge: i32,
}

/// FormationEdges.engage: free attackers whose fronts reach a hostile's side or rear lock
/// on, all picked before any lands, nearest first, so the nearest holds an edge two reach at
/// once. The victim stops and fights, and the units on that edge strike at once. Returns
/// the locks in the order they landed, for their events and shocks.
pub fn edges(
    squads: &mut Squads,
    columns: &mut Columns,
    roster: &Roster,
    tick: i64,
    seed: i64,
) -> Vec<Flanked> {
    let mut picks: Vec<((f64, i64, i64), usize, usize)> = Vec::new();
    {
        let field = Field { squads, columns, roster };
        for attacker in 0..roster.listed.len() {
            let slot = roster.listed[attacker].squad;
            if !field.can_engage(attacker) || squads.engaged_with[slot] != 0 {
                continue;
            }
            if let Some(pick) = target(&field, attacker, seed) {
                picks.push(pick);
            }
        }
    }
    picks.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
    let mut held: Vec<(usize, i32)> = Vec::new(); // edges taken this tick
    let mut out = Vec::with_capacity(picks.len());
    for (_, attacker, victim) in picks {
        let slot = roster.listed[victim].squad;
        let edge =
            geometry::edge_hit(squads.heading[slot], squads.heading[roster.listed[attacker].squad]);
        lock(squads, columns, roster, attacker, victim);
        if !held.contains(&(victim, edge)) {
            held.push((victim, edge)); // a nearer attacker already holds it this tick
            squads.flank(slot, edge, roster.listed[attacker].id, tick);
        }
        squads.state[slot] = locks::FIGHTING;
        let on_edge = Shape::of(squads, columns, &roster.listed[victim]).edge_units(edge);
        for u in on_edge {
            columns.attack_cooldown[roster.listed[victim].living[u].slot] = 1;
        }
        out.push(Flanked { victim, attacker, edge });
    }
    out
}

/// [key, attacker, victim] for the nearest side or rear face the attacker reaches (ties by
/// the squads' draws), if any.
fn target(field: &Field, attacker: usize, seed: i64) -> Option<((f64, i64, i64), usize, usize)> {
    let mine = contest::squad_draw(seed, field.roster.listed[attacker].id);
    let mut best: Option<((f64, i64, i64), usize, usize)> = None;
    for victim in 0..field.roster.listed.len() {
        if !field.hostile(attacker, victim) || !reaches(field, attacker, victim) {
            continue;
        }
        let gap =
            snapped(geometry::face_gap(&field.shape(attacker), &field.shape(victim)), EPSILON);
        let key = (gap, mine, contest::squad_draw(seed, field.roster.listed[victim].id));
        if best.is_none_or(|(b, _, _)| key < b) {
            best = Some((key, attacker, victim));
        }
    }
    best
}

/// True if the attacker's front reaches a hostile's side or rear face, overlapping it.
fn reaches(field: &Field, attacker: usize, victim: usize) -> bool {
    if !field.engageable(victim) || !field.framed(victim) {
        return false;
    }
    let (mine, theirs) = (field.shape(attacker), field.shape(victim));
    if geometry::facing_off(&mine, &theirs) {
        return false; // front to front: the frontal lock
    }
    let gap = geometry::face_gap(&mine, &theirs);
    gap > -EPSILON && gap <= MELEE_REACH + EPSILON && geometry::overlap_across(&mine, &theirs)
}

#[cfg(test)]
#[path = "contact/tests.rs"]
mod tests;

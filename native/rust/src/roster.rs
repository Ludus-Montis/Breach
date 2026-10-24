//! The squads as a pass is given them (spec 39 step 4c): each listed squad, in the lists'
//! order, with its living units in its own order and what of each the crate doesn't hold
//! (its id, store slot, footprint, preferred band, ranged reach, kind, turn rate, backward pace, detection
//! range, most stamina and the walker it crosses the ground as). GDScript keeps who is living (UnitState,
//! Decision 147), so it hands them in; where each stands, how it faces and its place are
//! the store's columns, read where they live.

use std::collections::HashSet;

use crate::columns::Columns;
use crate::ground::Walker;
use crate::locks;
use crate::maths::V2;
use crate::motion::Mover;
use crate::reach;
use crate::squads::Squads;

/// A living unit of a listed squad.
#[derive(Clone, Debug)]
pub struct Member {
    pub id: i64,
    pub slot: usize,
    /// Its footprint: width and depth in cells.
    pub foot: (i64, i64),
    pub preferred: i64,
    /// Its ranged reach in ranks (0: none).
    pub attack_range: i64,
    /// Its definition, as a code (one per definition in the roster).
    pub kind: u32,
    pub turn_rate: f64,
    pub backward_pace: f64,
    pub detection: f64,
    pub max_stamina: f64,
    pub walker: Walker,
}

impl Member {
    /// The radius of its body (ScrumReach.radius).
    pub fn radius(&self) -> f64 {
        reach::radius(self.foot.0, self.foot.1)
    }

    /// Its motion, facing as the store's columns have it (CrateMoves.of).
    pub fn mover(&self, columns: &Columns) -> Mover {
        Mover {
            bearing: columns.bearing[self.slot],
            turn_rate: self.turn_rate,
            backward_pace: self.backward_pace,
        }
    }
}

/// A listed squad: its squad-store slot, its id, its faction (a code), whether units joined
/// it on the march, whether it merges into a friend it catches up with (its command's), and
/// its living units.
#[derive(Clone, Debug, Default)]
pub struct Listed {
    pub squad: usize,
    pub id: i64,
    pub faction: u32,
    pub joined: bool,
    pub merges: bool,
    pub living: Vec<Member>,
}

impl Listed {
    /// Its living units' store slots, in its order.
    pub fn slots(&self) -> Vec<i32> {
        self.living.iter().map(|m| m.slot as i32).collect()
    }
}

/// The squads a pass is given, in the lists' order.
#[derive(Clone, Debug, Default)]
pub struct Roster {
    pub listed: Vec<Listed>,
}

/// A body in the scrum (ScrumSlots.bodies): where it stands, its radius, its squad's id and
/// its unit's id.
#[derive(Clone, Copy, Debug)]
pub struct Body {
    pub at: V2,
    pub radius: f64,
    pub squad: i64,
    pub id: i64,
}

impl Roster {
    /// The listed squad with this id, if listed.
    pub fn by_id(&self, id: i64) -> Option<usize> {
        self.listed.iter().position(|l| l.id == id)
    }

    /// True if the listed squad routs or is destroyed.
    pub fn falling(&self, squads: &Squads, index: usize) -> bool {
        let state = squads.state[self.listed[index].squad];
        state == locks::ROUTING || state == locks::DESTROYED
    }

    /// Who of each listed squad is loose, by id: ScrumReach.at reads its loose point.
    pub fn loose(&self, squads: &Squads) -> Vec<HashSet<i64>> {
        self.listed.iter().map(|l| squads.loose[l.squad].iter().copied().collect()).collect()
    }

    /// [[where, radius, unit], ...] for every living unit of the squads still standing
    /// (ScrumSlots.bodies), in the lists' order.
    pub fn bodies(&self, squads: &Squads, columns: &Columns) -> Vec<Body> {
        let loose = self.loose(squads);
        let mut out = Vec::new();
        for (n, listed) in self.listed.iter().enumerate() {
            if self.falling(squads, n) {
                continue;
            }
            for m in &listed.living {
                let at = at(columns, &loose[n], m);
                out.push(Body { at, radius: m.radius(), squad: listed.id, id: m.id });
            }
        }
        out
    }

    /// [(listed index, member index), ...] of the living units of squads hostile to the
    /// faction, not routing or destroyed (ScrumBlows.hostile_units), in the lists' order.
    pub fn hostiles(&self, squads: &Squads, faction: u32) -> Vec<(usize, usize)> {
        let mut out = Vec::new();
        for (n, listed) in self.listed.iter().enumerate() {
            if listed.faction == faction || self.falling(squads, n) {
                continue;
            }
            out.extend((0..listed.living.len()).map(|u| (n, u)));
        }
        out
    }
}

/// Where the unit stands (ScrumReach.at): its loose point while loose, else its point.
#[inline]
pub fn at(columns: &Columns, loose: &HashSet<i64>, m: &Member) -> V2 {
    if loose.contains(&m.id) {
        columns.loose_at[m.slot]
    } else {
        columns.position[m.slot]
    }
}

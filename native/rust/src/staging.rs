//! Hold-until orders' trigger (FormationStaging, Decision 87, spec 27 round 2; spec 39 step
//! 5b): a staged wave goes once it detects its partner (sees_partner) or its partner fighting
//! (sees_fight); with no partner named, any friendly squad on another route. Led by a
//! coordinated leader (Decision 81) with a meeting point, it times its departure: from each
//! such friend's distance to the point and its pace, as seen, against its own predicted march
//! (FormationRendezvous), it goes when it would arrive no earlier than every one of them, so
//! none is singled out by the order squads are listed in (Decision 97). Holding at the
//! staging point, the fallback and the events are the order's, GDScript's (step 7).

use crate::columns::Columns;
use crate::content::CELLS_PER_TILE;
use crate::ground::Ground;
use crate::locks::FIGHTING;
use crate::maths::{minf, V2};
use crate::perception;
use crate::rendezvous::{self, March};
use crate::roster::{Listed, Roster};
use crate::route::Route;
use crate::squads::Squads;

/// A coordinated leader leaves this many ticks early at most.
const SLACK_TICKS: f64 = 2.0;

/// A staged squad's order: its index in the roster, whether it waits to see a fight (else
/// to see its partner), its partner's squad id (0: none named), each listed squad's route
/// (a code: one per route), the meeting it times its departure by if a coordinated leader
/// leads it, and [cells a second at speed 1, seconds a tick].
pub struct Staged<'a> {
    pub listed: usize,
    pub sees_fight: bool,
    pub partner: i64,
    pub routes: &'a [u32],
    pub meet: Option<Meeting>,
    pub pace: [f64; 2],
}

/// The meeting point, how far along its own route it is (cells), that route, and the slack
/// its units walk to their places within (FormationWalk).
pub struct Meeting {
    pub at: V2,
    pub cells: f64,
    pub route: Route,
    pub slack: f64,
}

/// True if the staged squad's trigger has fired and it is time to go: some friend fires it,
/// and it would arrive no earlier than any of them.
pub fn signalled(
    squads: &Squads,
    columns: &Columns,
    roster: &Roster,
    ground: Option<&Ground>,
    staged: &Staged,
) -> bool {
    let partners = triggered(squads, columns, roster, ground, staged);
    !partners.is_empty()
        && partners.iter().all(|&n| time_to_go(squads, columns, roster, ground, staged, n))
}

/// The listed friends whose sight fires the trigger, in the lists' order.
fn triggered(
    squads: &Squads,
    columns: &Columns,
    roster: &Roster,
    ground: Option<&Ground>,
    staged: &Staged,
) -> Vec<usize> {
    let own = &roster.listed[staged.listed];
    let mut out = Vec::new();
    for (n, other) in roster.listed.iter().enumerate() {
        if n == staged.listed || other.faction != own.faction {
            continue;
        }
        if staged.partner != 0 && other.id != staged.partner {
            continue;
        }
        if staged.partner == 0 && staged.routes[n] == staged.routes[staged.listed] {
            continue; // a wave on its own route is no partner
        }
        let wanted = !staged.sees_fight || squads.state[other.squad] == FIGHTING;
        if wanted && perception::detects(columns, own, other, ground) {
            out.push(n);
        }
    }
    out
}

/// True unless a coordinated leader judges it too soon: it would arrive before the listed
/// friend `partner`.
fn time_to_go(
    squads: &Squads,
    columns: &Columns,
    roster: &Roster,
    ground: Option<&Ground>,
    staged: &Staged,
    partner: usize,
) -> bool {
    let Some(meet) = &staged.meet else {
        return true;
    };
    let other = &roster.listed[partner];
    let partner_speed = speed(columns, other);
    if partner_speed <= 0.0 {
        return true;
    }
    let [cells_per_second, tick_seconds] = staged.pace;
    let away = squads.position[other.squad].distance_to(meet.at) as f64;
    let partner_ticks = away / (partner_speed * cells_per_second * tick_seconds);
    let own = &roster.listed[staged.listed];
    let march = March {
        cells: meet.cells,
        width: squads.width[own.squad],
        cells_per_second: speed(columns, own) * cells_per_second,
        tick_seconds,
        from: squads.front_distance[own.squad] * CELLS_PER_TILE as f64,
        slack: meet.slack,
    };
    let own_ticks = rendezvous::ticks_to(&meet.route, &march, ground);
    own_ticks as f64 >= partner_ticks - SLACK_TICKS
}

/// Its slowest living unit's pace this tick (SkirmishSquad.speed): 0 with none living.
fn speed(columns: &Columns, listed: &Listed) -> f64 {
    let slowest =
        listed.living.iter().fold(f64::INFINITY, |least, m| minf(least, columns.speed[m.slot]));
    if slowest == f64::INFINITY {
        0.0
    } else {
        slowest
    }
}

#[cfg(test)]
mod tests;

//! The scrum's engaging, manoeuvres and stall over the roster (ScrumEngage.step,
//! FormationManoeuvre.step and FormationScrum._stall; spec 39 step 8b5-xi): each reads its
//! squads where the crate holds them and reports its events in the log. Engaging locks free
//! squads onto the hostiles their units reach (`engage`), each lock its squad's "engaged"
//! event with its foe's id; the manoeuvres settle each squad's current one (`manoeuvre`);
//! the stall releases fights nobody has touched or sought a foe in for a while, each
//! release its squad's "disengaged" event.

use crate::columns::Columns;
use crate::contact;
use crate::engage::{self, Side};
use crate::events::{self, Log};
use crate::locks;
use crate::manoeuvre::{self, Case, Stall};
use crate::roster::{self, Roster};
use crate::squads::Squads;

/// ScrumEngage.step on `tick`: free squads lock onto the nearest hostile their units have
/// come within `reach_engage` of, each one's living units standing where ScrumReach.at has
/// them; each pick's "engaged" event goes to `log`, in the lists' order.
pub fn engage(
    squads: &mut Squads,
    columns: &mut Columns,
    roster: &Roster,
    (tick, seed, reach_engage): (i64, i64, f64),
    log: &mut Log,
) {
    let loose = roster.loose(squads);
    let sides: Vec<Side> = roster
        .listed
        .iter()
        .enumerate()
        .map(|(n, listed)| Side {
            squad: listed.squad,
            faction: listed.faction,
            joined: listed.joined,
            slots: listed.slots(),
            at: listed.living.iter().map(|m| roster::at(columns, &loose[n], m)).collect(),
            radii: listed.living.iter().map(|m| m.radius()).collect(),
        })
        .collect();
    for (n, foe) in engage::step(squads, columns, &sides, reach_engage, seed) {
        log.squad(events::ENGAGED, tick, squads, roster.listed[n].squad, &[foe]);
    }
}

/// FormationManoeuvre.step: each listed squad's manoeuvre this tick, in the lists' order.
/// Whether it skirmishes is asked only where no higher priority applies; a squad that
/// narrowed hands in its living units, to walk to their new places.
pub fn manoeuvre(squads: &mut Squads, columns: &mut Columns, roster: &Roster) {
    let mut cases = Vec::with_capacity(roster.listed.len());
    for (n, listed) in roster.listed.iter().enumerate() {
        let s = listed.squad;
        let withdrawing = squads.withdraw[s].is_some();
        let skirmishing = !withdrawing
            && !manoeuvre::fighting(squads, s)
            && contact::Field { squads, columns, roster }.skirmishing(n);
        let living = if squads.narrow_ticks[s] > 0 {
            listed.living.iter().map(|m| (m.id, m.slot)).collect()
        } else {
            Vec::new()
        };
        cases.push(Case { squad: s, withdrawing, skirmishing, living });
    }
    manoeuvre::step(squads, columns, &cases);
}

/// FormationScrum._stall on `tick`: counts each fighting squad quiet unless it or a foe is
/// `active` (squad ids), and releases those stalled `ticks`; each release's "disengaged"
/// event goes to `log`, in the lists' order.
pub fn stall(
    squads: &mut Squads,
    roster: &Roster,
    (active, ticks): (&[i64], i64),
    tick: i64,
    log: &mut Log,
) {
    let listed: Vec<usize> = roster.listed.iter().map(|l| l.squad).collect();
    let factions: Vec<u32> = roster.listed.iter().map(|l| l.faction).collect();
    let foes: Vec<Vec<i64>> = (0..listed.len())
        .map(|n| locks::foe_ids(squads, listed[n], &listed, &factions, factions[n]))
        .collect();
    let stall = Stall { listed: &listed, foes: &foes, active, ticks };
    for s in manoeuvre::stall(squads, &stall) {
        log.squad(events::DISENGAGED, tick, squads, s, &[]);
    }
}

#[cfg(test)]
mod tests;

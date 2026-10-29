//! The strays through the groups as one crate call (spec 39 step 8b5-xxxix): the strays
//! recovery set off adopted (`strays::adopt`), morale's second (`morale::pass`, on the tick
//! that is one), the rout (`rout::pass`, when anything routs or breaks), carrying
//! (`carry::pass`, the units the rout brought home its homes), the bearers carrying set off
//! adopted in turn, and the groups (`groups::pass`), in that order, as FormationSimulation made
//! them one call at a time. The squads listed grow as they did in GDScript's lists: each squad
//! a pass opens is listed after the rest, its living read from the members, every unit as the
//! roster first gave it (`listing`). Every pass's events go to the log in that order; GDScript
//! mirrors each pass's result after it.

#[cfg(test)]
mod tests;

use std::collections::BTreeMap;

use crate::bands::BandTuning;
use crate::carry::pass::{self as carry, Carried, LoadTuning};
use crate::carry::CarryTuning;
use crate::commands::MERGES;
use crate::events::Log;
use crate::field::Field;
use crate::ground::Ground;
use crate::groups::pass::{self as groups, Landed};
use crate::locks::ROUTING;
use crate::morale::{pass as morale, MoraleTuning};
use crate::roster::{Listed, Member, Roster};
use crate::rout::pass::{self as rout, Moved, RoutShocks};
use crate::rout::RoutTuning;
use crate::split;
use crate::squads::Squads;
use crate::strays;

/// Every pass's tuning, and the tick's: morale's on a second's tick (None on the others).
pub struct Rules {
    pub morale: Option<MoraleTuning>,
    pub rout: RoutTuning,
    pub shocks: RoutShocks,
    pub bands: BandTuning,
    pub carry: CarryTuning,
    pub loads: LoadTuning,
    pub groups: split::Rules,
    pub tick: i64,
    pub seed: i64,
}

/// What each pass did, as GDScript mirrors it: the strays adopted ([index in the strays
/// given, the squad slot opened]), the routers that left their squads, what carrying did, the
/// bearers adopted (as the strays, their index in `carried.strays`) and what the groups did,
/// each split's units by id.
#[derive(Debug, Default, PartialEq)]
pub struct Mustered {
    pub adopted: Vec<(usize, usize)>,
    pub routed: Vec<Moved>,
    pub carried: Carried,
    pub bearers: Vec<(usize, usize)>,
    pub grouped: Vec<Landed>,
    /// Each split's units by id (`groups::pass::split_ids`).
    pub split: Vec<Vec<i64>>,
}

/// The passes over `roster`'s squads, on `ground`; `strays` [its squad's slot, its unit's
/// store slot] each, as recovery set them off. Their events in `log`.
pub fn step(
    squads: &mut Squads,
    field: &mut Field,
    roster: &Roster,
    strays: &[(usize, usize)],
    ground: Option<&Ground>,
    rules: &Rules,
    log: &mut Log,
) -> Mustered {
    let (tick, seed) = (rules.tick, rules.seed);
    let known: BTreeMap<usize, Member> =
        roster.listed.iter().flat_map(|l| l.living.iter().map(|m| (m.slot, m.clone()))).collect();
    let adopted = strays::adopt(squads, field, strays, seed);
    let opened: Vec<usize> = adopted.iter().map(|&(_, s)| s).collect();
    let mut roster = listing(roster, squads, field, &known, &opened);
    if let Some(t) = &rules.morale {
        morale::step((squads, &field.columns), &roster, (t, &rules.bands), tick, log);
    }
    let routs = roster.listed.iter().any(|l| {
        let s = l.squad;
        squads.state[s] == ROUTING || squads.morale[s] == 0
    });
    let mut routed = Vec::new();
    if routs {
        let tuning = (&rules.rout, &rules.shocks, &rules.bands);
        routed = rout::step(squads, field, &roster, tuning, (tick, seed), ground, log);
        roster = listing(&roster, squads, field, &known, &[]);
    }
    let homes: Vec<i64> = routed.iter().filter(|m| m.into.is_none()).map(|m| m.id).collect();
    let listed: Vec<usize> = roster.listed.iter().map(|l| l.squad).collect();
    let tuning = (&rules.carry, &rules.loads);
    let carried = carry::step(squads, field, (&listed, &homes), (tick, seed), tuning, log);
    let setting_off: Vec<(usize, usize)> = carried
        .strays
        .iter()
        .map(|&(n, id)| (roster.listed[n].squad, known_slot(&known, id)))
        .collect();
    let bearers = strays::adopt(squads, field, &setting_off, seed);
    let opened: Vec<usize> = bearers.iter().map(|&(_, s)| s).collect();
    let roster = listing(&roster, squads, field, &known, &opened);
    let grouped = groups::step(squads, field, &roster, &rules.groups, tick, log);
    let split = groups::split_ids(&grouped, &roster);
    Mustered { adopted, routed, carried, bearers, grouped, split }
}

/// The store slot of the unit `id`, as the roster gave it.
fn known_slot(known: &BTreeMap<usize, Member>, id: i64) -> usize {
    known.values().find(|m| m.id == id).map_or(usize::MAX, |m| m.slot)
}

/// The squads as GDScript would list them now (CrateRoster.of): `roster`'s, then each squad
/// in the slots `opened`, in that order, its faction coded as a listed squad of the same one
/// is (FormationLocks.factions) and merging as its command does; each one's living read from
/// its members, every unit as `known` (the roster first given) has it.
pub fn listing(
    roster: &Roster,
    squads: &Squads,
    field: &Field,
    known: &BTreeMap<usize, Member>,
    opened: &[usize],
) -> Roster {
    let mut out = roster.clone();
    for &slot in opened {
        let kin = roster.listed.iter().find(|l| squads.faction[l.squad] == squads.faction[slot]);
        let merges = squads.command[slot].is_some_and(|c| squads.commands.gives(c, MERGES));
        let faction = kin.map_or(squads.faction[slot], |l| l.faction);
        let id = squads.id[slot];
        out.listed.push(Listed { squad: slot, id, faction, merges, ..Listed::default() });
    }
    for listed in &mut out.listed {
        let living = squads.living(listed.squad, &field.columns.state);
        listed.living = living.iter().filter_map(|u| known.get(&(*u as usize))).cloned().collect();
        listed.joined = !squads.joined[listed.squad].is_empty();
    }
    out
}

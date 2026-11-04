//! The whole tick as one crate pass (FormationSimulation.step, spec 39 step 8b5-xli): the
//! opening (`opening::step`), the scrum (`scrum::step`), the fight and its aftermath
//! (`aftermath::step`), the strays through the groups (`mustering::step`) and the tick's end
//! (`ending::step`), in that order, every pass's events in the log in that order. The roster
//! given holds every unit the squads hold, living or not; between the passes the squads are
//! listed afresh as CrateRoster.of listed GDScript's (`relisted`): the squads a join took in
//! leave the list, the factions are coded again in the list's order, and each squad's living
//! is read from its members, every unit as the roster gave it. The strays recovery set off go
//! to the mustering, and the squads it opened (and took in) are listed (or dropped) for the
//! end as GDScript's mirrors list them (`ended_listing`).

#[cfg(test)]
mod tests;

use std::collections::BTreeMap;

use crate::aftermath::{self, Aftermath};
use crate::ending::{self, Ended};
use crate::events::Log;
use crate::field::Field;
use crate::groups::pass::Landed;
use crate::mustering::{self, Mustered};
use crate::opening::{self, Opened};
use crate::paths::Over;
use crate::recovery::pass::Came;
use crate::roster::{Member, Roster};
use crate::scrum;
use crate::squads::Squads;

/// Each pass's rules, as its own binding is given them.
pub struct Rules<'a> {
    pub opening: opening::Rules<'a>,
    pub scrum: scrum::step::Rules,
    pub aftermath: aftermath::Rules,
    pub mustering: mustering::Rules,
    pub ending: ending::Rules,
}

/// What each pass did, as GDScript mirrors it, in the passes' order; the scrum's are the
/// pursuers back at their posts and the seek's time.
#[derive(Debug, Default)]
pub struct Ticked {
    pub opened: Opened,
    pub back: Vec<usize>,
    pub seek_usec: i64,
    pub after: Aftermath,
    pub mustered: Mustered,
    pub ended: Ended,
}

/// One tick over the squads of `roster` (every unit each holds, living or not), on `over`'s
/// ground (None: open ground). Their events in `log`.
pub fn step(
    squads: &mut Squads,
    field: &mut Field,
    roster: &Roster,
    mut over: Option<&mut Over>,
    rules: &Rules,
    log: &mut Log,
) -> Ticked {
    let known = known(roster);
    let roster = relisted(roster, squads, field, &known, &[], &[]);
    let opened = opening::step(squads, field, &roster, over.as_deref_mut(), &rules.opening, log);
    let joining: Vec<usize> = opened.joins.iter().map(|&(_, j)| j).collect();
    let roster = relisted(&roster, squads, field, &known, &joining, &[]);
    let ground = over.as_ref().map(|o| o.ground);
    let scrummed = scrum::step::step(squads, field, &roster, ground, &rules.scrum, log);
    let roster = relisted(&roster, squads, field, &known, &[], &[]);
    let after = aftermath::step(squads, field, &roster, ground, &rules.aftermath, log);
    let alone = after.recovered.iter().filter(|l| matches!(l.came, Came::Alone));
    let slot = |id| field.slot_of(id).map_or(usize::MAX, |s| s as usize);
    let strays: Vec<(usize, usize)> =
        alone.map(|l| (roster.listed[l.listed].squad, slot(l.id))).collect();
    let roster = relisted(&roster, squads, field, &known, &[], &[]);
    let mustered = mustering::step(squads, field, &roster, &strays, ground, &rules.mustering, log);
    let listed = ended_listing(&roster, &mustered);
    let ended = ending::step(squads, field, &listed, ground, &rules.ending, log);
    let (back, seek_usec) = (scrummed.back, scrummed.seek_usec);
    Ticked { opened, back, seek_usec, after, mustered, ended }
}

/// Every unit of `roster`, by its store slot.
pub fn known(roster: &Roster) -> BTreeMap<usize, Member> {
    let units = roster.listed.iter().flat_map(|l| l.living.iter());
    units.map(|m| (m.slot, m.clone())).collect()
}

/// The squads as CrateRoster.of would list them now: `roster`'s but those listed at
/// `dropped`, then those in the slots `opened` (`mustering::listing`: each one's living read
/// from its members, every unit as `known` has it), the factions coded in the list's order
/// (FormationLocks.factions).
pub fn relisted(
    roster: &Roster,
    squads: &Squads,
    field: &Field,
    known: &BTreeMap<usize, Member>,
    dropped: &[usize],
    opened: &[usize],
) -> Roster {
    let kept = roster.listed.iter().enumerate().filter(|(n, _)| !dropped.contains(n));
    let kept = Roster { listed: kept.map(|(_, l)| l.clone()).collect() };
    let mut out = mustering::listing(&kept, squads, field, known, opened);
    let mut codes: Vec<u32> = Vec::new();
    for listed in &mut out.listed {
        let faction = squads.faction[listed.squad];
        listed.faction = match codes.iter().position(|&c| c == faction) {
            Some(code) => code as u32,
            None => {
                codes.push(faction);
                codes.len() as u32 - 1
            }
        };
    }
    out
}

/// The squads' slots the end runs over, as GDScript's mirrors leave its lists: `roster`'s
/// (as the mustering was given them), then the strays' and the bearers' squads adopted, then
/// each split's group, less the groups taken in (each named by its index as the groups
/// listed them, the splits after the rest).
pub fn ended_listing(roster: &Roster, mustered: &Mustered) -> Vec<usize> {
    let mut listed: Vec<usize> = roster.listed.iter().map(|l| l.squad).collect();
    listed.extend(mustered.adopted.iter().chain(&mustered.bearers).map(|&(_, s)| s));
    let mut taken = Vec::new();
    for done in &mustered.grouped {
        match *done {
            Landed::Split { slot, .. } => listed.push(slot),
            Landed::Taken { taken: n, .. } => taken.push(listed[n]),
            Landed::Returned { .. } => {}
        }
    }
    listed.retain(|s| !taken.contains(s));
    listed
}

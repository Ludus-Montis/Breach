//! The fight's blows and shots chosen (FormationMelee.blows, spec 39 step 8b5-xvii), in one
//! pass over the squads' roster: every living unit's target cleared as the fight begins;
//! if some squad may strike, the field synced to the scrum (`scrum::seek::sync`) and each
//! unit's pick taken in the sync's order (`blows::picks`): its target set, its cooldown run
//! down, and at 0 started again at its melee time, half again while its squad wavers (its
//! morale's band read at its first blow, Decision 144), a blow unless it retreats and has
//! turned away; then the blows on routers (`rout_blows`), from the front ranks of the squads
//! free to fight; then the shots (`shots`). Each blow's damage is its striker's sheet's: its
//! melee damage, or its ranged damage for a shot. How each lands stays GDScript's.

use std::collections::{HashMap, HashSet};

use crate::bands::{self, BandTuning};
use crate::blows::{self, AWAY, FLANK};
use crate::field::Field;
use crate::locks::{self, DESTROYED, FIGHTING, RETREAT, ROUTING};
use crate::roster::Roster;
use crate::rout_blows::{self, Fighter, Routing};
use crate::scrum::seek;
use crate::shocks;
use crate::shots;
use crate::squads::Squads;

/// What the choice reads of the tick and of BattleTuning.
pub struct Rules {
    /// The ticks a second's strike takes (Decision 120), and the battle's seed.
    pub interval: i64,
    pub seed: i64,
    /// reach_contact, and the front's least dot (ScrumReach.in_front).
    pub reach: f64,
    pub front: f64,
    /// rout_strike_reach.
    pub rout_reach: f64,
    pub bands: BandTuning,
}

/// A unit by its squad's index in the roster and its index in that one's living.
pub type Who = (usize, usize);

/// A blow or a shot: who strikes whom, its damage before it lands, and (a blow) whether it
/// falls on a flank.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Blow {
    pub striker: Who,
    pub target: Who,
    pub damage: i64,
    pub flank: bool,
}

/// The tick's melee blows (the scrum's, then those on routers), its shots, and whether the
/// field was synced to the scrum for them (the landing counts on that sync).
#[derive(Debug, Default, PartialEq)]
pub struct Chosen {
    pub melee: Vec<Blow>,
    pub shots: Vec<Blow>,
    pub synced: bool,
}

/// FormationMelee.blows over the roster's squads.
pub fn choose(squads: &mut Squads, field: &mut Field, roster: &Roster, rules: &Rules) -> Chosen {
    let mut chosen = Chosen::default();
    for member in roster.listed.iter().flat_map(|l| &l.living) {
        field.columns.target_id[member.slot] = 0;
    }
    if any_strike(squads, roster) {
        chosen.melee = scrum(squads, field, roster, rules);
        chosen.synced = true;
    }
    chosen.melee.extend(on_routers(squads, field, roster, rules));
    chosen.shots = shots(squads, field, roster, rules);
    chosen
}

/// ScrumBlowsField._any_strike: a squad standing (not routing nor destroyed) and fighting
/// or retreating strikes any other faction's standing squad, another only one retreating.
fn any_strike(squads: &Squads, roster: &Roster) -> bool {
    let standing =
        || roster.listed.iter().filter(|l| !matches!(squads.state[l.squad], ROUTING | DESTROYED));
    let mut factions = HashSet::new();
    let mut retreating = HashSet::new();
    for listed in standing() {
        factions.insert(listed.faction);
        if squads.order[listed.squad] == RETREAT {
            retreating.insert(listed.faction);
        }
    }
    standing().any(|listed| {
        let s = listed.squad;
        let fights = squads.state[s] == FIGHTING || squads.order[s] == RETREAT;
        let foes = if fights { &factions } else { &retreating };
        foes.len() > usize::from(foes.contains(&listed.faction))
    })
}

/// Ticks between a unit's melee blows: its melee time in ticks, at least 1 (ScrumBlows.ticks).
fn melee_ticks(interval: i64, seconds: f64) -> i64 {
    (interval as f64 * seconds).round().max(1.0) as i64
}

/// The scrum's blows (ScrumBlowsField.blows): each pick taken in the sync's order.
fn scrum(squads: &mut Squads, field: &mut Field, roster: &Roster, rules: &Rules) -> Vec<Blow> {
    seek::sync(squads, field, roster, rules.seed);
    let whos: Vec<Who> = roster
        .listed
        .iter()
        .enumerate()
        .flat_map(|(n, l)| (0..l.living.len()).map(move |m| (n, m)))
        .collect();
    let says: Vec<u8> = roster
        .listed
        .iter()
        .map(|l| {
            let fighting = if squads.state[l.squad] == FIGHTING { blows::FIGHTING } else { 0 };
            fighting | if squads.order[l.squad] == RETREAT { blows::RETREAT } else { 0 }
        })
        .collect();
    let params = blows::Params { reach: rules.reach, front: rules.front };
    let picks = blows::picks(field, &says, &params);
    let mut bands_of: HashMap<usize, i64> = HashMap::new();
    let mut out = Vec::new();
    for pick in picks {
        let (striker, target) = (whos[pick.flat as usize], whos[pick.foe as usize]);
        let listed = &roster.listed[striker.0];
        let slot = listed.living[striker.1].slot;
        let columns = &mut field.columns;
        columns.target_id[slot] = roster.listed[target.0].living[target.1].id;
        columns.attack_cooldown[slot] -= 1;
        if columns.attack_cooldown[slot] > 0 {
            continue;
        }
        let usual = melee_ticks(rules.interval, columns.sheet[slot].melee_seconds);
        let band = *bands_of.entry(striker.0).or_insert_with(|| {
            bands::band(shocks::morale(squads, columns, listed, &rules.bands), &rules.bands)
        });
        columns.attack_cooldown[slot] = bands::interval(band, usual) as i32;
        if pick.flags & AWAY == 0 {
            let damage = columns.sheet[slot].dmg;
            out.push(Blow { striker, target, damage, flank: pick.flags & FLANK != 0 });
        }
    }
    out
}

/// RoutBlows.blows: the front ranks (by column) of the squads free to fight strike the
/// routers they reach, from behind.
fn on_routers(squads: &Squads, field: &mut Field, roster: &Roster, rules: &Rules) -> Vec<Blow> {
    let routing: Vec<Routing> = (0..roster.listed.len())
        .filter(|&n| squads.state[roster.listed[n].squad] == ROUTING)
        .map(|listed| Routing { listed, route: squads.route[roster.listed[listed].squad].clone() })
        .collect();
    if routing.iter().all(|r| roster.listed[r.listed].living.is_empty()) {
        return vec![];
    }
    let columns = &mut field.columns;
    let mut whos = Vec::new();
    let mut fighters = Vec::new();
    for (n, listed) in roster.listed.iter().enumerate() {
        if !locks::can_engage(squads, listed.squad) {
            continue;
        }
        let mut front: Vec<usize> = (0..listed.living.len())
            .filter(|&m| columns.rank[listed.living[m].slot] == 0)
            .collect();
        front.sort_by_key(|&m| columns.column[listed.living[m].slot]);
        for m in front {
            let slot = listed.living[m].slot;
            let reset = melee_ticks(rules.interval, columns.sheet[slot].melee_seconds) as i32;
            whos.push((n, m));
            fighters.push(Fighter { listed: n, slot, reset });
        }
    }
    if fighters.is_empty() {
        return vec![];
    }
    let struck =
        rout_blows::blows(columns, roster, &routing, &fighters, rules.rout_reach, rules.seed);
    struck
        .into_iter()
        .map(|(f, r, m)| Blow {
            striker: whos[f],
            target: (routing[r].listed, m),
            damage: columns.sheet[fighters[f].slot].dmg,
            flank: true,
        })
        .collect()
}

/// FormationCombat.ranged_blows: who may shoot, each at its ranged time, and whom.
fn shots(squads: &Squads, field: &mut Field, roster: &Roster, rules: &Rules) -> Vec<Blow> {
    let columns = &mut field.columns;
    let seconds: Vec<f64> = roster
        .listed
        .iter()
        .flat_map(|l| &l.living)
        .map(|m| columns.sheet[m.slot].ranged_seconds)
        .collect();
    let shooters = shots::shooters(squads, columns, roster, &seconds, rules.interval);
    let out = shots::shots(squads, columns, roster, &shooters, rules.seed);
    out.into_iter()
        .map(|(s, n, u)| {
            let shooter = &shooters[s];
            let slot = roster.listed[shooter.listed].living[shooter.member].slot;
            Blow {
                striker: (shooter.listed, shooter.member),
                target: (n, u),
                damage: columns.sheet[slot].ranged_dmg,
                flank: false,
            }
        })
        .collect()
}

pub mod land;

#[cfg(test)]
mod tests;

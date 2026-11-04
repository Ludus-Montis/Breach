//! The fight as one pass (FormationSimulation._fight, spec 39 step 8b5-xix): the blows and
//! shots chosen (`fight::choose`), then landed (`fight::land`, on the field as the choice
//! synced it), then the hurts (`hurts::land`): each shot, then each blow, in the order
//! landed, the fallen onto the fell queue; then "spat" for each shot and "hit" for each blow
//! in the log, as FormationEvents made them; then each striker's stamina spent on its blow
//! (`stamina::strike`), blows then shots. Then the surrender over the melee blows, each at
//! the damage it dealt (`fight::surrender`, spec 39 step 8b5-xx).

#[cfg(test)]
mod tests;

use super::land::{self, Struck};
use super::surrender;
use super::{Blow, Who};
use crate::events::{Log, HIT, SPAT};
use crate::field::Field;
use crate::ground::Ground;
use crate::hurts::{self, Hurt};
use crate::roster::{Member, Roster};
use crate::squads::Squads;
use crate::stamina;

/// What the pass reads of the tick and of BattleTuning.
pub struct Rules {
    pub choose: super::Rules,
    pub land: land::Rules,
    pub tick: i64,
    /// stamina_blow: what a blow or shot costs its striker, times how fast its load tires it.
    pub blow: f64,
    /// wounds_surrender: the chance a routing unit struck surrenders, times its want of
    /// courage.
    pub surrender: f64,
}

/// What the fight did: its melee blows, each at the damage it dealt, and the units the
/// surrender took, in the order they gave up (which GDScript copies).
#[derive(Debug, Default)]
pub struct Fought {
    #[cfg_attr(not(test), allow(dead_code))] // the tests read the damage dealt
    pub melee: Vec<Blow>,
    pub taken: Vec<Who>,
}

/// The fight over the roster's squads.
pub fn step(
    squads: &mut Squads,
    field: &mut Field,
    roster: &Roster,
    rules: &Rules,
    ground: Option<&Ground>,
    log: &mut Log,
) -> Fought {
    let mut chosen = super::choose(squads, field, roster, &rules.choose);
    let (melee, shots) = (chosen.melee.len(), chosen.shots.len());
    let melee_struck = chosen.melee.iter().map(|blow| struck(roster, blow, false));
    let shots_struck = chosen.shots.iter().map(|shot| struck(roster, shot, true));
    let struck: Vec<Struck> = melee_struck.chain(shots_struck).collect();
    let landed = land::land(squads, field, roster, &struck, &rules.land, chosen.synced, ground);
    let blows = chosen.melee.iter_mut().chain(chosen.shots.iter_mut());
    for (blow, &(dealt, _)) in blows.zip(&landed) {
        blow.damage = dealt;
    }
    let hurts: Vec<Hurt> =
        chosen.shots.iter().chain(&chosen.melee).map(|b| hurt(roster, b)).collect();
    let mut fell = vec![];
    let left = hurts::land(&mut field.columns, &hurts, &mut fell);
    for id in fell {
        field.fell(id);
    }
    for (shot, &(_, how)) in chosen.shots.iter().zip(&landed[melee..]) {
        let shooter = member(roster, shot.striker);
        let kind = i64::from(field.columns.words[shooter.slot].damage_type);
        let target = member(roster, shot.target).id;
        let values = [shooter.id, target, shot.damage, kind, i64::from(how)];
        log.squad(SPAT, rules.tick, squads, roster.listed[shot.striker.0].squad, &values);
    }
    for (n, blow) in chosen.melee.iter().enumerate() {
        let striker = member(roster, blow.striker);
        let faction = field.columns.words[striker.slot].faction_id;
        let (target, how) = (member(roster, blow.target).id, i64::from(landed[n].1));
        let values = [target, blow.damage, blow.flank as i64, how, i64::from(left[shots + n])];
        log.unit(HIT, rules.tick, striker.id, faction, &values);
    }
    let tiring = |s: &Struck| (s.striker, field.columns.sheet[s.striker].tiring);
    let strikers: Vec<(usize, f64)> = struck.iter().map(tiring).collect();
    stamina::strike(&mut field.columns, &strikers, rules.blow);
    let given =
        surrender::Rules { tick: rules.tick, seed: rules.choose.seed, chance: rules.surrender };
    let taken = surrender::surrender(squads, field, roster, &chosen.melee, &given, log);
    Fought { melee: chosen.melee, taken }
}

fn member(roster: &Roster, who: Who) -> &Member {
    &roster.listed[who.0].living[who.1]
}

fn struck(roster: &Roster, blow: &Blow, ranged: bool) -> Struck {
    let (striker, target) = (member(roster, blow.striker).slot, member(roster, blow.target).slot);
    Struck { striker, target, damage: blow.damage, ranged, flank: blow.flank }
}

/// A blow's hurt, at the damage it dealt.
fn hurt(roster: &Roster, blow: &Blow) -> Hurt {
    let target = member(roster, blow.target);
    Hurt { slot: target.slot, id: target.id, damage: blow.damage }
}

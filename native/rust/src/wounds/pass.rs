//! The wounds' tending as one crate pass (FormationWounds.tend, Decision 121; spec 39 step
//! 8b5-xxiv): the listed squads' living units and their downed bodies lying (DOWNED, not
//! borne) are read where they live - each one's id, faction, captor, leader and playing dead,
//! messengers, damage, "cunning" and "thorough" from its sheet and words - and taken (`tend`).
//! Then each body taken lands, in the order found: captured, TAKEN in the STATE column and
//! "captured" (unit, by) logged; sent home, its leader's "messenger" trait one fewer, RELEASED,
//! "sent_home" (unit) logged and every listed squad of the body's side with anyone standing
//! shocked (`shocks::shock_living`, wounds_messenger_shock); finished, DEAD and "finished"
//! (unit) logged. A body a strike took to 0 hp or below goes on the fell queue. GDScript
//! mirrors the states and the messengers.

#[cfg(test)]
mod tests;

use crate::bands::BandTuning;
use crate::events::{self, Log};
use crate::field::Field;
use crate::fight::surrender::TAKEN;
use crate::shocks;
use crate::squads::{Squads, DEAD, DOWNED};
use crate::wounds::{self, Taken, Taking, Tended, WoundsTuning, AVERAGE};

/// SkirmishUnit.State's RELEASED: sent home.
pub const RELEASED: i32 = 7;

/// What the pass reads besides the squads: the takings' tuning, the messenger's shock and
/// morale's bands.
pub struct Tuning<'a> {
    pub wounds: &'a WoundsTuning,
    pub shock: i64,
    pub bands: &'a BandTuning,
}

/// A body taken, as GDScript mirrors it: its squad's index in the lists, its id and the
/// state it comes to, and for one sent home, the leader's [index in the lists, id].
#[derive(Debug, PartialEq)]
pub struct Landed {
    pub listed: usize,
    pub id: i64,
    pub state: i32,
    pub leader: Option<(usize, i64)>,
}

/// The tending over the squads in `listed` (squad slots, in the lists' order), on `tick`;
/// `interval` is ticks a second. Returns the bodies taken, in the order they landed.
pub fn step(
    squads: &mut Squads,
    field: &mut Field,
    listed: &[usize],
    (tick, interval, seed): (i64, i64, i64),
    t: &Tuning,
    log: &mut Log,
) -> Vec<Landed> {
    let tended: Vec<Tended> = listed.iter().map(|&s| tended(squads, field, s)).collect();
    let tending = wounds::tend(&mut field.columns, &tended, interval, seed, t.wounds);
    for id in tending.fell {
        field.fell(id);
    }
    let mut out = vec![];
    for taken in tending.taken {
        let (body, other, state) = match taken {
            Taken::Captured { body, by } => (body, Some(by), TAKEN),
            Taken::SentHome { body, leader } => (body, Some(leader), RELEASED),
            Taken::Finished { body } => (body, None, DEAD),
        };
        let unit = &tended[body.0].downed[body.1];
        let s = listed[body.0];
        field.columns.state[unit.slot] = state;
        let other =
            other.map(|at| (at.0, tended[at.0].living[at.1].id, tended[at.0].living[at.1].slot));
        let mut leader = None;
        match state {
            TAKEN => {
                let by = other.map_or(0, |(_, id, _)| id);
                log.squad(events::CAPTURED, tick, squads, s, &[unit.id, by]);
            }
            RELEASED => {
                let (n, id, slot) = other.unwrap_or_default();
                spend_messenger(field, slot);
                leader = Some((n, id));
                log.squad(events::SENT_HOME, tick, squads, s, &[unit.id]);
                let side = unit.faction;
                for &shaken in listed {
                    let living = squads.living(shaken, &field.columns.state);
                    if squads.faction[shaken] != side || living.is_empty() {
                        continue;
                    }
                    let living: Vec<usize> = living.iter().map(|&u| u as usize).collect();
                    let landed = (t.bands, tick);
                    let lands = (&mut *squads, &field.columns);
                    shocks::shock_living(lands, (shaken, &living), t.shock, landed, log);
                }
            }
            _ => log.squad(events::FINISHED, tick, squads, s, &[unit.id]),
        }
        out.push(Landed { listed: body.0, id: unit.id, state, leader });
    }
    out
}

/// The squad in slot `s` as the takings read it: its living units and its bodies lying.
fn tended(squads: &Squads, field: &Field, s: usize) -> Tended {
    let state = &field.columns.state;
    let lying = squads.members[s].iter().copied().filter(|&u| state[u as usize] == DOWNED);
    Tended {
        living: squads.living(s, state).into_iter().map(|u| taking(field, u as usize)).collect(),
        downed: lying.map(|u| taking(field, u as usize)).collect(),
    }
}

/// The unit in `slot` as the takings read it.
fn taking(field: &Field, slot: usize) -> Taking {
    let (sheet, words) = (&field.columns.sheet[slot], &field.columns.words[slot]);
    let level = |said: &[(u32, f64)], name: &str| field.lexicon.level(said, name);
    let trait_of = |name: &str| level(&words.traits, name).unwrap_or(0.0);
    Taking {
        id: field.unit_at(slot as u32).unwrap_or(0),
        slot,
        faction: words.faction_id,
        captor: trait_of("captor") > 0.0,
        leads: sheet.leadership > 0,
        playing_dead: sheet.playing_dead,
        messengers: trait_of("messenger") as i64,
        dmg: sheet.dmg,
        cunning: trait_of("cunning") as i64,
        thorough: trait_of("thorough") as i64,
        wits: level(&words.attributes, "wits").unwrap_or(AVERAGE),
        constitution: level(&words.attributes, "constitution").unwrap_or(AVERAGE),
        melee_seconds: sheet.melee_seconds,
    }
}

/// The leader in `slot` has one messenger fewer.
fn spend_messenger(field: &mut Field, slot: usize) {
    let code = field.lexicon.find("messenger");
    let traits = &mut field.columns.words[slot].traits;
    if let Some(said) = traits.iter_mut().find(|(word, _)| Some(*word) == code) {
        said.1 -= 1.0;
    }
}

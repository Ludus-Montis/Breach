//! What the wounded come to as one crate pass (FormationRecovery.step, Decisions 121, 125,
//! 126 and 127; spec 39 step 8b5-xxv): the listed squads' living and downed units are read
//! where they live - each one's id, faction, wounds, "hardened", "cunning" and
//! "regenerates_downed", leading and playing dead, regeneration, most hp, wits, constitution
//! and detection from its sheet and words - and recovery runs (`recovery::step`). Then what
//! it decided lands, in its order: one dead of its wounds DEAD in the STATE column and
//! "died_of_wounds" (unit) logged; one beginning to play dead so marked on its sheet and
//! "playing_dead" (unit) logged; each coming to MOVING, no longer playing dead, and joining
//! the formation it found - out of its own squad and enlisted in that one, if another, at a
//! place behind its rearmost, loose where it lies, the squad reforming - or none, to make
//! for home alone (FormationStrays); "came_to" (unit, alone, joined) logged. A unit an hp
//! write took to 0 or below goes on the fell queue. GDScript mirrors the states, the
//! playing dead and the lists.

#[cfg(test)]
mod tests;

use crate::entries;
use crate::events::{self, Log};
use crate::field::Field;
use crate::joins;
use crate::locks::{DESTROYED, ROUTING};
use crate::recovery::{self, Lay, Recovering, Ward};
use crate::squads::{Squads, DEAD};
use crate::wounds::{WoundsTuning, AVERAGE};

/// SkirmishUnit.State's MOVING: up again.
pub const MOVING: i32 = 0;

/// What a downed unit came to.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Came {
    DiedOfWounds,
    PlayingDead,
    /// It came to with no formation in sight, to make for home alone.
    Alone,
    /// It came to and joined the squad at this index in the lists.
    Joined(usize),
}

/// A downed unit something befell, as GDScript mirrors it: its squad's index in the lists,
/// its id and what it came to.
#[derive(Debug, PartialEq)]
pub struct Landed {
    pub listed: usize,
    pub id: i64,
    pub came: Came,
}

/// One tick of `seconds` over the squads in `listed` (squad slots, in the lists' order), on
/// `tick`. Returns what befell the downed, in the order it landed.
pub fn step(
    squads: &mut Squads,
    field: &mut Field,
    listed: &[usize],
    (tick, seconds, seed): (i64, f64, i64),
    tuning: &WoundsTuning,
    log: &mut Log,
) -> Vec<Landed> {
    let wards: Vec<Ward> = listed.iter().map(|&s| ward(squads, field, s)).collect();
    let recovered = recovery::step(&mut field.columns, &wards, seconds, seed, tuning);
    for id in recovered.fell {
        field.fell(id);
    }
    let mut out = vec![];
    for lay in recovered.lay {
        let (at, came, kind) = match lay {
            Lay::DiedOfWounds(at) => (at, Came::DiedOfWounds, events::DIED_OF_WOUNDS),
            Lay::PlayingDead(at) => (at, Came::PlayingDead, events::PLAYING_DEAD),
        };
        let unit = &wards[at.0].downed[at.1];
        if came == Came::DiedOfWounds {
            field.columns.state[unit.slot] = DEAD;
        } else {
            field.columns.sheet[unit.slot].playing_dead = true;
        }
        log.squad(kind, tick, squads, listed[at.0], &[unit.id]);
        out.push(Landed { listed: at.0, id: unit.id, came });
    }
    for rising in recovered.rising {
        let (unit, own) = (&wards[rising.unit.0].downed[rising.unit.1], listed[rising.unit.0]);
        field.columns.state[unit.slot] = MOVING;
        field.columns.sheet[unit.slot].playing_dead = false;
        let joined = rising.joins.map(|n| {
            join(squads, field, (unit.id, unit.slot), own, listed[n]);
            squads.id[listed[n]]
        });
        let told = [unit.id, i64::from(joined.is_none()), joined.unwrap_or(-1)];
        log.squad(events::CAME_TO, tick, squads, own, &told);
        let came = rising.joins.map_or(Came::Alone, Came::Joined);
        out.push(Landed { listed: rising.unit.0, id: unit.id, came });
    }
    out
}

/// The squad in slot `s` as recovery reads it.
fn ward(squads: &Squads, field: &Field, s: usize) -> Ward {
    let state = &field.columns.state;
    let read = |units: Vec<i32>| units.into_iter().map(|u| recovering(field, u as usize));
    Ward {
        id: squads.id[s],
        joinable: !matches!(squads.state[s], DESTROYED | ROUTING),
        living: read(squads.living(s, state)).collect(),
        downed: read(squads.downed(s, state)).collect(),
    }
}

/// The unit in `slot` as recovery reads it.
fn recovering(field: &Field, slot: usize) -> Recovering {
    let (sheet, words) = (&field.columns.sheet[slot], &field.columns.words[slot]);
    let trait_of = |name: &str| field.lexicon.level(&words.traits, name).unwrap_or(0.0);
    let attribute = |name: &str| field.lexicon.level(&words.attributes, name).unwrap_or(AVERAGE);
    Recovering {
        id: field.unit_at(slot as u32).unwrap_or(0),
        slot,
        faction: words.faction_id,
        wounded: sheet.wounded,
        hardened: trait_of("hardened") as i64,
        cunning: trait_of("cunning") as i64,
        leads: sheet.leadership > 0,
        regenerates_downed: trait_of("regenerates_downed") > 0.0,
        playing_dead: sheet.playing_dead,
        regeneration: sheet.regeneration,
        max_hp: sheet.max_hp,
        wits: attribute("wits"),
        constitution: attribute("constitution"),
        detection: sheet.detection,
    }
}

/// The unit (`unit`: its id and store slot) joins the squad in slot `best`, from the squad
/// in `own`: taken out of its own and enlisted, if another, placed a rank behind its new
/// squad's rearmost standing, in the first column, and loose where it lies; that squad
/// reforms.
fn join(squads: &mut Squads, field: &mut Field, unit: (i64, usize), own: usize, best: usize) {
    let slot = unit.1;
    let columns = &field.columns;
    let behind = |u: usize| columns.rank[u] + columns.sheet[u].footprint_depth as i32;
    let standing = squads.living(best, &columns.state).into_iter().map(|u| u as usize);
    let back = standing.filter(|&u| u != slot).map(behind).fold(0, i32::max);
    if best != own {
        entries::unloose(squads, &mut field.columns, own, unit);
        entries::unflee(squads, &mut field.columns, own, unit);
        squads.members[own].retain(|&u| u as usize != slot);
        joins::enlist_one(squads, field, best, slot as i32);
    }
    field.columns.rank[slot] = back;
    field.columns.column[slot] = 0;
    squads.reforming[best] = 1;
    let at = field.columns.position[slot];
    entries::loosen(squads, &mut field.columns, best, unit, at);
}

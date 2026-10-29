//! Bearing the wounded as one crate pass (FormationCarry.step, Decision 126, spec 39 step
//! 8b5-xxix) over the listed squads: the bodies borne by bearers that fled home this tick come
//! home too, out of their squads' members ("fled_home": unit, definition); each body still
//! borne follows its bearer, or is dropped where it is if its bearer is down or gone; a
//! bearer that is down, or whose body is no longer borne, sets its load down; then the free
//! units of the formations that tend their downed pick bodies up (`pick_up`): each body
//! CARRIED, its bearer bearing it ("borne": unit, body), slowed, tired and dodging by its
//! load. A bearer of a formation told to "recover" sets off home with its body alone
//! (FormationStrays). GDScript mirrors what changed.

#[cfg(test)]
mod tests;

use super::{pick_up, stage, Bearer, Body, CarryTuning, Carrying};
use crate::events::{self, Log};
use crate::field::Field;
use crate::locks;
use crate::squads::{Squads, CARRIED, DEAD, DOWNED};

/// BattleTuning's load_pace, load_tiring and load_dodge: each by load stage.
#[derive(Clone, Debug, Default)]
pub struct LoadTuning {
    pub pace: Vec<f64>,
    pub tiring: Vec<f64>,
    pub dodge: Vec<f64>,
}

impl LoadTuning {
    /// The tuning as the binding is given it: the three lists one after another, each as
    /// long.
    pub fn of_slice(t: &[f64]) -> LoadTuning {
        let n = t.len() / 3;
        LoadTuning {
            pace: t[..n].to_vec(),
            tiring: t[n..2 * n].to_vec(),
            dodge: t[2 * n..].to_vec(),
        }
    }
}

/// What the pass did, as GDScript mirrors it: each unit by its squad's index in the lists
/// and its id.
#[derive(Debug, Default, PartialEq)]
pub struct Carried {
    /// The units whose state, carrying, carried_by or load changed, once each, in the order
    /// first changed.
    pub changed: Vec<(usize, i64)>,
    /// The bodies that came home with their bearers, out of their squads.
    pub home: Vec<(usize, i64)>,
    /// The bearers setting off home alone with a body.
    pub strays: Vec<(usize, i64)>,
}

/// A unit the pass keeps track of: its id, store slot and its squad's index in the lists.
#[derive(Clone, Copy)]
struct Held {
    id: i64,
    slot: usize,
    listed: usize,
}

/// One tick of bearing over the squads in slots `listed` on `tick`, ties to the fight's
/// `seed`. `homes`: the ids of the units that fled home this tick ("fled_home"), in order.
pub fn step(
    squads: &mut Squads,
    field: &mut Field,
    (listed, homes): (&[usize], &[i64]),
    (tick, seed): (i64, i64),
    (t, loads): (&CarryTuning, &LoadTuning),
    log: &mut Log,
) -> Carried {
    let mut out = Carried::default();
    let (bodies, bearers) = held(squads, field, listed);
    for &home in homes {
        for body in &bodies {
            let s = listed[body.listed];
            let columns = &field.columns;
            if columns.state[body.slot] != CARRIED || columns.sheet[body.slot].carried_by != home {
                continue;
            }
            squads.members[s].retain(|&u| u as usize != body.slot);
            let definition = columns.objects[body.slot].definition;
            log.squad(events::FLED_HOME, tick, squads, s, &[body.id, definition]);
            out.home.push((body.listed, body.id));
        }
    }
    for body in &bodies {
        follow(field, body, &bearers, &mut out);
    }
    for bearer in &bearers {
        hold(field, bearer, &bodies, (t, loads), &mut out);
    }
    picked(squads, field, listed, (tick, seed), (t, loads), log, &mut out);
    out
}

/// The listed squads' downed (DOWNED or CARRIED) and their units bearing a body, standing or
/// not, in the lists' and members' order.
fn held(squads: &Squads, field: &Field, listed: &[usize]) -> (Vec<Held>, Vec<Held>) {
    let (mut bodies, mut bearers) = (Vec::new(), Vec::new());
    let id = |slot: usize| field.unit_at(slot as u32).unwrap_or(0);
    for (n, &s) in listed.iter().enumerate() {
        for slot in squads.downed(s, &field.columns.state) {
            let slot = slot as usize;
            bodies.push(Held { id: id(slot), slot, listed: n });
        }
        for &slot in &squads.members[s] {
            let slot = slot as usize;
            if field.columns.sheet[slot].carrying != 0 {
                bearers.push(Held { id: id(slot), slot, listed: n });
            }
        }
    }
    (bodies, bearers)
}

/// A body still borne follows its bearer; one whose bearer is down or gone is dropped where
/// it is (FormationCarry._follow).
fn follow(field: &mut Field, body: &Held, bearers: &[Held], out: &mut Carried) {
    let columns = &mut field.columns;
    if columns.state[body.slot] != CARRIED {
        return;
    }
    let by = columns.sheet[body.slot].carried_by;
    match bearers.iter().rev().find(|b| b.id == by) {
        Some(bearer) if columns.state[bearer.slot] < DEAD => {
            // placement: a carried body is where its bearer is (Decision 139)
            columns.position[body.slot] = columns.position[bearer.slot];
        }
        _ => {
            columns.state[body.slot] = DOWNED;
            columns.sheet[body.slot].carried_by = 0;
            changed(out, body);
        }
    }
}

/// A bearer that is down, or whose body is no longer borne (it came to, died or went home),
/// sets its load down (FormationCarry._hold).
fn hold(
    field: &mut Field,
    bearer: &Held,
    bodies: &[Held],
    tuning: (&CarryTuning, &LoadTuning),
    out: &mut Carried,
) {
    let columns = &field.columns;
    let carrying = columns.sheet[bearer.slot].carrying;
    let body = bodies.iter().rev().find(|b| b.id == carrying);
    let borne = body.is_some_and(|b| columns.state[b.slot] == CARRIED);
    if borne && columns.state[bearer.slot] < DEAD {
        return;
    }
    field.columns.sheet[bearer.slot].carrying = 0;
    let unit = bearer_of(field, bearer.slot, bearer.id);
    bear(field, &unit, None, tuning);
    changed(out, bearer);
}

/// The pick-up (FormationCarry._pick_up): who bears whom is `pick_up`'s; this lands it.
fn picked(
    squads: &Squads,
    field: &mut Field,
    listed: &[usize],
    (tick, seed): (i64, i64),
    tuning: (&CarryTuning, &LoadTuning),
    log: &mut Log,
    out: &mut Carried,
) {
    if !listed.iter().any(|&s| tending(squads, s)) {
        return;
    }
    let carrying: Vec<Carrying> = listed.iter().map(|&s| carrying_of(squads, field, s)).collect();
    for (body, bearer) in pick_up(&field.columns, &carrying, seed, tuning.0) {
        let (b, r) = (&carrying[body.0].downed[body.1], &carrying[bearer.0].living[bearer.1]);
        let sheets = &mut field.columns.sheet;
        field.columns.state[b.slot] = CARRIED;
        sheets[b.slot].carried_by = r.id;
        sheets[r.slot].carrying = b.id;
        bear(field, r, Some(b), tuning);
        let s = listed[bearer.0];
        log.squad(events::BORNE, tick, squads, s, &[r.id, b.id]);
        changed(out, &Held { id: b.id, slot: b.slot, listed: body.0 });
        changed(out, &Held { id: r.id, slot: r.slot, listed: bearer.0 });
        if tends(squads, s) == "recover" {
            out.strays.push((bearer.0, r.id));
        }
    }
}

/// The squad in slot `s` as the pick-up is given it: its living, and its downed lying.
fn carrying_of(squads: &Squads, field: &Field, s: usize) -> Carrying {
    let columns = &field.columns;
    let living = squads.living(s, &columns.state).into_iter().map(|u| u as usize);
    let downed = squads.downed(s, &columns.state).into_iter().map(|u| u as usize);
    let id = |slot: usize| field.unit_at(slot as u32).unwrap_or(0);
    Carrying {
        faction: squads.faction[s],
        tending: tending(squads, s),
        living: living.map(|slot| bearer_of(field, slot, id(slot))).collect(),
        downed: downed
            .filter(|&slot| columns.state[slot] == DOWNED)
            .map(|slot| {
                let sheet = &columns.sheet[slot];
                let cells = sheet.footprint_width * sheet.footprint_depth;
                Body { id: id(slot), slot, faction: columns.words[slot].faction_id, cells }
            })
            .collect(),
    }
}

/// The unit in `slot` as a bearer: its faction, whether it bears a body, its gear, its
/// strength (10 if none is said) and its "hauler".
fn bearer_of(field: &Field, slot: usize, id: i64) -> Bearer {
    let (sheet, words) = (&field.columns.sheet[slot], &field.columns.words[slot]);
    let level = |levels, word, none: f64| field.lexicon.level(levels, word).unwrap_or(none);
    Bearer {
        id,
        slot,
        faction: words.faction_id,
        carrying: sheet.carrying != 0,
        gear_weight: sheet.gear_weight,
        strength: level(&words.attributes, "strength", 10.0) as i64,
        hauler: level(&words.traits, "hauler", 0.0) as i64,
    }
}

/// Sets the unit's load stage, pace, tiring and dodge bearing `body` (none: its gear alone)
/// (FormationCarry._bear): its pace a share of its definition's speed (none: its own pace).
fn bear(
    field: &mut Field,
    unit: &Bearer,
    body: Option<&Body>,
    (t, loads): (&CarryTuning, &LoadTuning),
) {
    let at = stage(unit, body, t);
    let definition = field.columns.objects[unit.slot].definition;
    let own = field.columns.sheet[unit.slot].fresh_speed;
    let march = field.speed(definition).unwrap_or(own);
    let sheet = &mut field.columns.sheet[unit.slot];
    sheet.load_stage = at as i64;
    sheet.fresh_speed = march * loads.pace[at];
    sheet.tiring = loads.tiring[at];
    sheet.dodging = loads.dodge[at];
}

/// The command's tends of the squad in slot `s` ("" for none).
fn tends(squads: &Squads, s: usize) -> &str {
    squads.command[s].map_or("", |c| squads.commands.tends[c].as_str())
}

/// True if the squad in slot `s` tends its downed now: told to, and not fighting or routing.
fn tending(squads: &Squads, s: usize) -> bool {
    !tends(squads, s).is_empty()
        && squads.state[s] != locks::FIGHTING
        && squads.state[s] != locks::ROUTING
}

/// Notes the unit as changed, once.
fn changed(out: &mut Carried, unit: &Held) {
    if !out.changed.contains(&(unit.listed, unit.id)) {
        out.changed.push((unit.listed, unit.id));
    }
}

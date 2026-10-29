//! The scrum's preparing and its lines (FormationScrum._prepare and ScrumStance.anticipate_all,
//! spec 39 step 8b5-xii), over the roster. Preparing readies each squad for the tick's
//! scrum: a routing or destroyed one lets its loose units, its stance and its fight go; the
//! fallen leave their loose entries; a fight beginning is marked, and one ending closes the
//! squad's ranks over its dead; a squad fighting, or holding a stance, sends every living
//! unit loose where it stands. Its lines: each squad that may look meets the threats it
//! sees (`stance::anticipate`), each new line its "faced" event in the log.

use crate::discipline::{self, DisciplineTuning};
use crate::entries;
use crate::events::{self, Log};
use crate::field::Field;
use crate::geometry::Shape;
use crate::ground::Ground;
use crate::locks::{DESTROYED, FIGHTING, HOLDING, MOVING, ROUTING};
use crate::maths::{bearing_vector, V2};
use crate::perception;
use crate::ranks::{self, Ranked};
use crate::roster::{Listed, Roster};
use crate::squads::{Squads, Stance};
use crate::stance::{self, Seen};

/// SkirmishUnit.State's first state not standing (DEAD): those before it stand.
const DEAD: i32 = 4;

/// FormationScrum._prepare for each listed squad, in the lists' order, on `tick`.
pub fn prepare(squads: &mut Squads, field: &mut Field, roster: &Roster, tick: i64) {
    for listed in &roster.listed {
        prepare_one(squads, field, listed, tick);
    }
}

fn prepare_one(squads: &mut Squads, field: &mut Field, listed: &Listed, tick: i64) {
    let s = listed.squad;
    if matches!(squads.state[s], ROUTING | DESTROYED) {
        let slots: Vec<Option<usize>> =
            squads.loose[s].iter().map(|&id| field.slot_of(id).map(|at| at as usize)).collect();
        let mut at = slots.into_iter();
        entries::unloose_all(squads, &mut field.columns, s, |_| at.next().flatten());
        squads.stance[s] = None;
        squads.fight_since[s] = -1;
        return;
    }
    for unit_id in squads.loose[s].clone() {
        match field.slot_of(unit_id).map(|at| at as usize) {
            Some(slot) if field.columns.state[slot] < DEAD => {}
            Some(slot) => entries::unloose(squads, &mut field.columns, s, (unit_id, slot)),
            None => squads.loose[s].retain(|&id| id != unit_id),
        }
    }
    let fighting = squads.state[s] == FIGHTING;
    if fighting && squads.fight_since[s] < 0 {
        squads.fight_since[s] = tick;
        squads.fought[s] = 1;
        squads.stall_ticks[s] = 0;
    } else if !fighting {
        if squads.fight_since[s] >= 0 {
            close(squads, field, listed); // the dead leave holes: the living close up
        }
        squads.fight_since[s] = -1;
    }
    if !fighting && squads.stance[s].is_none() {
        return;
    }
    for m in &listed.living {
        if !squads.loose[s].contains(&m.id) {
            let stood = field.columns.position[m.slot];
            entries::loosen(squads, &mut field.columns, s, (m.id, m.slot), stood);
        }
    }
}

/// SquadRanks.close: the living close up over the dead, and the squad's swaps end.
fn close(squads: &mut Squads, field: &mut Field, listed: &Listed) {
    let units: Vec<Ranked> = listed
        .living
        .iter()
        .map(|m| Ranked {
            slot: m.slot,
            preferred: m.preferred,
            foot: (m.foot.0 as i32, m.foot.1 as i32),
        })
        .collect();
    ranks::close(&mut field.columns, &units, squads.width[listed.squad]);
    squads.swaps[listed.squad].clear();
}

/// What the lines read: scrum_anticipate, the battle's seed and FormationDiscipline's tuning.
pub struct Lines<'a> {
    pub anticipate: f64,
    pub seed: i64,
    pub discipline: &'a DisciplineTuning,
}

/// ScrumStance.anticipate_all on `tick`: each listed squad, in turn, meets the threats it
/// sees on `ground`; the stances let go are cleared, and each new one is taken, its anchor
/// at the face of its footprints towards its heading, its "faced" event in `log`.
pub fn anticipate(
    squads: &mut Squads,
    field: &Field,
    roster: &Roster,
    ground: Option<&Ground>,
    (lines, tick): (&Lines, i64),
    log: &mut Log,
) {
    let columns = &field.columns;
    let mut seen: Vec<Seen> = roster
        .listed
        .iter()
        .map(|listed| Seen {
            squad: listed.squad,
            faction: listed.faction,
            living: !listed.living.is_empty(),
            bounds: Shape::of(squads, columns, listed).bounds(),
            meets_threats: looks(squads, listed.squad)
                && discipline::of(columns, listed, lines.discipline)
                    >= lines.discipline.meets_threats,
            detects: Vec::new(),
        })
        .collect();
    for n in 0..seen.len() {
        if seen[n].meets_threats {
            seen[n].detects = sees(squads, field, roster, ground, (&seen, n));
        }
    }
    for facing in stance::anticipate(squads, &seen, (lines.anticipate, lines.seed)) {
        let listed = &roster.listed[facing.seen];
        let shape = Shape::of(squads, columns, listed);
        let centre = shape.bounds().center();
        let outward = bearing_vector(facing.heading);
        let out = shape.reach(outward).1 as f64 - centre.dot(outward) as f64;
        let anchor: V2 = centre.add(outward.scale(out));
        let s = listed.squad;
        squads.stance[s] = Some(Stance { anchor, heading: facing.heading, foe: facing.foe });
        let way = ((facing.heading / 90.0).round() as i64).rem_euclid(4);
        log.squad(events::FACED, tick, squads, s, &[facing.heading.to_bits() as i64, way]);
    }
}

/// ScrumStance._looks: marching, holding or fighting, its front free.
fn looks(squads: &Squads, s: usize) -> bool {
    matches!(squads.state[s], HOLDING | MOVING | FIGHTING) && squads.engaged_with[s] == 0
}

/// ScrumStance._sees: the listed hostiles, living and not routing, that the squad at `n`
/// detects, by their places in the list.
fn sees(
    squads: &Squads,
    field: &Field,
    roster: &Roster,
    ground: Option<&Ground>,
    (seen, n): (&[Seen], usize),
) -> Vec<usize> {
    let listed = &roster.listed;
    (0..listed.len())
        .filter(|&m| {
            seen[m].faction != seen[n].faction
                && seen[m].living
                && squads.state[listed[m].squad] != ROUTING
                && perception::detects(&field.columns, &listed[n], &listed[m], ground)
        })
        .collect()
}

#[cfg(test)]
mod tests;

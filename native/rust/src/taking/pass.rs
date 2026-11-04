//! Taking the downed as one crate pass (FormationTaking.step, spec 39 step 8b5-xxiii): each
//! listed squad's takers whose work is done come back - their bodies no longer downed, the
//! taker gone from the squad or fallen, the squad no longer at ease or threatened, or the
//! hold run out (take_hold seconds, for good under "no man left behind") - then the squads
//! at ease, whose command takes, not under "fall behind, left behind", not threatened and
//! that haven't given up taking send their free units out to the downed foes (`send`), and
//! "sent_to_take" is logged for each taker sent. The hold's start (`taking_since`) and where
//! a squad gave up (`took_until`) are its squad columns; a new fight makes its downed foes
//! worth taking again.

#[cfg(test)]
mod tests;

use crate::commands::TAKES;
use crate::events::{self, Log};
use crate::field::Field;
use crate::locks::{ARRIVED, FIGHTING, HOLDING, MOVING};
use crate::march::walk::Rule;
use crate::moving::share;
use crate::squads::{Squads, DEAD, DOWNED};
use crate::taking::{self, Sending, Taker};

/// What the pass reads besides the squads: take_hold (seconds) and take_reach (cells).
pub struct TakingTuning {
    pub hold: f64,
    pub reach: f64,
}

/// The listed squads (their slots, in the lists' order), each threatened or not (its index
/// in `listed`), on `tick` of `seconds` with the fight's `seed`.
pub fn step(
    squads: &mut Squads,
    field: &Field,
    (listed, threatened): (&[usize], &[bool]),
    (tick, seconds, seed): (i64, f64, i64),
    t: &TakingTuning,
    log: &mut Log,
) {
    let downed = lying(squads, field, listed);
    let mut sending = vec![];
    for (&s, &threat) in listed.iter().zip(threatened) {
        let rule = share::rule(field, &squads.living(s, &field.columns.state));
        recall(squads, field, s, (tick, seconds), (threat, rule), t.hold);
        if takes(squads, s, rule) && !threat && squads.took_until[s] < 0 {
            sending.push(s);
        }
    }
    if sending.is_empty() || downed.is_empty() {
        return;
    }
    let free: Vec<Sending> = sending.iter().map(|&s| sends(squads, field, s, &downed)).collect();
    let sent = taking::send(&field.columns, &free, &downed, seed, t.reach);
    for ((&s, squad), pairs) in sending.iter().zip(&free).zip(sent) {
        for (u, b) in pairs {
            let unit = &squad.free[u];
            squads.taking[s].push((unit.id, downed[b].slot));
            if squads.taking_since[s] < 0 {
                squads.taking_since[s] = tick;
            }
            log.squad(events::SENT_TO_TAKE, tick, squads, s, &[unit.id]);
        }
    }
}

/// The bodies lying (DOWNED, not borne) of every listed squad, in the lists' order and each
/// squad's members' order.
fn lying(squads: &Squads, field: &Field, listed: &[usize]) -> Vec<Taker> {
    let columns = &field.columns;
    let mut out = vec![];
    for &s in listed {
        for &u in &squads.members[s] {
            if columns.state[u as usize] == DOWNED {
                out.push(taker(field, u as usize));
            }
        }
    }
    out
}

/// The unit in `slot` as the takings read it.
fn taker(field: &Field, slot: usize) -> Taker {
    let columns = &field.columns;
    Taker {
        id: field.unit_at(slot as u32).unwrap_or(0),
        slot,
        faction: columns.words[slot].faction_id,
        detection: columns.sheet[slot].detection,
    }
}

fn at_ease(state: i32) -> bool {
    matches!(state, MOVING | HOLDING | ARRIVED)
}

/// Whether the squad in `s` sends takers out at all: at ease, its command takes, and not
/// under "fall behind, left behind".
fn takes(squads: &Squads, s: usize, rule: Rule) -> bool {
    let command = squads.command[s].is_some_and(|c| squads.commands.gives(c, TAKES));
    at_ease(squads.state[s]) && command && rule != Rule::LeftBehind
}

/// The squad in `s`'s takers come back once their bodies are no longer downed, they are
/// gone or fallen, it is threatened or no longer at ease, or the hold has run out.
fn recall(
    squads: &mut Squads,
    field: &Field,
    s: usize,
    (tick, seconds): (i64, f64),
    (threatened, rule): (bool, Rule),
    hold: f64,
) {
    if squads.state[s] == FIGHTING {
        squads.took_until[s] = -1; // a new fight: its downed foes are worth taking again
    }
    if squads.taking[s].is_empty() {
        return;
    }
    let held = (tick - squads.taking_since[s]) as f64 * seconds;
    let out_of_time = rule != Rule::NoManLeft && held >= hold;
    let at_ease = at_ease(squads.state[s]);
    let columns = &field.columns;
    let members = &squads.members[s];
    let stays = |&(id, body): &(i64, usize)| {
        let taker = members.iter().find(|&&u| field.unit_at(u as u32) == Some(id));
        let alive = taker.is_some_and(|&u| columns.state[u as usize] < DEAD);
        !out_of_time && columns.state[body] == DOWNED && alive && !threatened && at_ease
    };
    let kept: Vec<(i64, usize)> = squads.taking[s].iter().copied().filter(stays).collect();
    squads.taking[s] = kept;
    if squads.taking[s].is_empty() {
        squads.taking_since[s] = -1;
        squads.took_until[s] = if out_of_time { tick } else { -1 };
    }
}

/// The squad in `s`'s free units (living, in their places, not out already) and the bodies
/// its takers have claimed, by index in `downed`.
fn sends(squads: &Squads, field: &Field, s: usize, downed: &[Taker]) -> Sending {
    let out = &squads.taking[s];
    let free = squads.living(s, &field.columns.state).into_iter().map(|u| taker(field, u as usize));
    let free = free.filter(|unit| {
        let id = unit.id;
        !squads.loose[s].contains(&id)
            && !squads.fleeing[s].contains(&id)
            && !out.iter().any(|&(taker, _)| taker == id)
    });
    let claimed = out.iter().filter_map(|&(_, body)| downed.iter().position(|d| d.slot == body));
    Sending { free: free.collect(), claimed: claimed.collect() }
}

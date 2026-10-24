//! What becomes of the downed and the caught (FormationWounds, Decision 121, spec 28 part 6;
//! spec 39 step 6b). Units don't leave the downed: once no friend of a downed unit stands
//! near it (`guard_reach`) and a standing foe beside it (`reach`) has no foe of its own near,
//! that foe takes it - captures it if it is a captor, else strikes it on its blows' interval,
//! down into death's door, constitution deep, till it dies. The one exception: a foe whose
//! formation's leader can send a messenger (its "messenger" trait, one a level) lets it go,
//! to carry word home. A routing unit struck may surrender in place, by a seeded chance from
//! its want of courage. Decided from one snapshot, then applied (Decision 97): GDScript keeps
//! the units' states, the messengers' traits, the morale shocks and the events, and lands
//! what this decides.

use crate::columns::Columns;
#[cfg(test)]
use crate::content::BattleTuning;
use crate::contest;
use crate::ghash::{self, Key};
use crate::grid::{Grid, MARGIN};
use crate::maths::{clampf, V2};

/// An attribute's ordinary value (UnitDef.AVERAGE).
pub const AVERAGE: f64 = 10.0;

/// The tuning the downed's passes read (BattleTuning's).
#[derive(Clone, Debug)]
pub struct WoundsTuning {
    pub reach: f64,
    pub guard_reach: f64,
    pub condition: f64,
    pub danger_reach: f64,
    pub play_dead: f64,
    pub cunning: f64,
    pub play_dead_check: f64,
    pub trait_wits: f64,
    pub believe_die: f64,
    pub wake_seconds: f64,
    pub wake_spread: (f64, f64),
    pub condition_cap: f64,
    pub drain_below: f64,
    pub drain: f64,
}

impl WoundsTuning {
    /// The tuning as the binding is given it, in this order.
    pub fn of_slice(t: &[f64]) -> WoundsTuning {
        WoundsTuning {
            reach: t[0],
            guard_reach: t[1],
            condition: t[2],
            danger_reach: t[3],
            play_dead: t[4],
            cunning: t[5],
            play_dead_check: t[6],
            trait_wits: t[7],
            believe_die: t[8],
            wake_seconds: t[9],
            wake_spread: (t[10], t[11]),
            condition_cap: t[12],
            drain_below: t[13],
            drain: t[14],
        }
    }

    /// The battle tuning's.
    #[cfg(test)]
    pub fn of(t: &BattleTuning) -> WoundsTuning {
        WoundsTuning::of_slice(&[
            t.wounds_reach,
            t.wounds_guard_reach,
            t.wounds_condition,
            t.wounds_danger_reach,
            t.wounds_play_dead,
            t.wounds_cunning,
            t.wounds_play_dead_check,
            t.wounds_trait_wits,
            t.wounds_believe_die,
            t.wounds_wake_seconds,
            t.wounds_wake_spread[0],
            t.wounds_wake_spread[1],
            t.condition_cap,
            t.condition_drain_below,
            t.condition_drain,
        ])
    }
}

/// How far below 0 hp a unit lives on, at death's door: its constitution (Decision 121),
/// as GDScript's `int()` takes it.
pub fn depth(constitution: f64) -> i64 {
    constitution as i64
}

/// A unit as the takings are given it: its id, store slot, faction (a code), whether it is a
/// captor, leads, or (downed) plays dead, the messengers it has left, its blows' damage and
/// "cunning" and "thorough", and its wits, constitution and seconds between blows.
#[derive(Clone, Debug)]
pub struct Taking {
    pub id: i64,
    pub slot: usize,
    pub faction: u32,
    pub captor: bool,
    pub leads: bool,
    pub playing_dead: bool,
    pub messengers: i64,
    pub dmg: i64,
    pub cunning: i64,
    pub thorough: i64,
    pub wits: f64,
    pub constitution: f64,
    pub melee_seconds: f64,
}

impl Taking {
    /// A unit of average wits and constitution 6 that strikes for 4 a second.
    #[cfg(test)]
    pub fn plain(id: i64, slot: usize, faction: u32) -> Taking {
        Taking {
            id,
            slot,
            faction,
            captor: false,
            leads: false,
            playing_dead: false,
            messengers: 0,
            dmg: 4,
            cunning: 0,
            thorough: 0,
            wits: AVERAGE,
            constitution: 6.0,
            melee_seconds: 1.0,
        }
    }
}

/// A squad as the takings are given it: its living units and its downed (not borne) bodies,
/// each in its order.
#[derive(Clone, Debug, Default)]
pub struct Tended {
    pub living: Vec<Taking>,
    pub downed: Vec<Taking>,
}

/// A unit by [its squad's index in the lists, its index in that squad's list].
pub type At = (usize, usize);

/// What became of a body taken this tick, in the order its takings were found.
#[derive(Clone, Debug, PartialEq)]
pub enum Taken {
    /// Captured by the taker.
    Captured { body: At, by: At },
    /// Sent home by the leader's messenger.
    SentHome { body: At, leader: At },
    /// Struck down through death's door: dead.
    Finished { body: At },
}

/// What the takings decided, and the ids of the bodies a strike took from above 0 to 0 or
/// below, in the order they fell (the fell queue's).
#[derive(Debug, Default)]
pub struct Tending {
    pub taken: Vec<Taken>,
    pub fell: Vec<i64>,
}

/// This tick's takings of the downed. `interval` is ticks a second (a taker strikes on its
/// melee interval). Writes the takers' cooldowns and targets and the bodies' hp.
pub fn tend(
    columns: &mut Columns,
    squads: &[Tended],
    interval: i64,
    seed: i64,
    tuning: &WoundsTuning,
) -> Tending {
    let mut out = Tending::default();
    let mut downed: Vec<At> = vec![];
    let mut standing: Vec<At> = vec![];
    for (s, squad) in squads.iter().enumerate() {
        downed.extend((0..squad.downed.len()).map(|u| (s, u)));
        standing.extend((0..squad.living.len()).map(|u| (s, u)));
    }
    if downed.is_empty() {
        return out;
    }
    let unit = |at: At, lying: bool| {
        let squad = &squads[at.0];
        if lying {
            &squad.downed[at.1]
        } else {
            &squad.living[at.1]
        }
    };
    // taken and taking in the units' seeded draws' order, never the list's (Decision 97)
    standing.sort_by_key(|&at| contest::draw(seed, unit(at, false).id));
    downed.sort_by_key(|&at| contest::draw(seed, unit(at, true).id));
    let at = |a: At, lying: bool| columns.position[unit(a, lying).slot];
    let standing_points: Vec<V2> = standing.iter().map(|&a| at(a, false)).collect();
    let downed_points: Vec<V2> = downed.iter().map(|&a| at(a, true)).collect();
    let near = Near {
        standing: &standing,
        downed: &downed,
        standing_grid: Grid::build(&standing_points),
        downed_grid: Grid::build(&downed_points),
        standing_points: &standing_points,
        downed_points: &downed_points,
    };
    let mut guarded: Vec<Option<bool>> = vec![None; downed.len()];
    // each body's takers, the bodies in the order a taker first found them
    let mut taken: Vec<(usize, Vec<At>)> = vec![];
    for (n, &taker) in standing.iter().enumerate() {
        let found = within_reach(squads, &near, n, &mut guarded, seed, tuning);
        if let Some(body) = found {
            match taken.iter_mut().find(|(b, _)| *b == body) {
                Some((_, takers)) => takers.push(taker),
                None => taken.push((body, vec![taker])),
            }
        }
    }
    let mut messengers: Vec<Vec<i64>> =
        squads.iter().map(|s| s.living.iter().map(|u| u.messengers).collect()).collect();
    for (body, takers) in taken {
        let body = downed[body];
        if let Some(done) =
            take(columns, squads, body, &takers, &mut messengers, interval, &mut out.fell)
        {
            out.taken.push(done);
        }
    }
    out
}

/// The standing and the downed in their draws' order, each with a grid of where they lie.
struct Near<'a> {
    standing: &'a [At],
    downed: &'a [At],
    standing_grid: Grid,
    downed_grid: Grid,
    standing_points: &'a [V2],
    downed_points: &'a [V2],
}

/// The downed foe (its index in `near.downed`) the `n`th standing unit would take, or none:
/// none unguarded within reach, or a foe still near it.
fn within_reach(
    squads: &[Tended],
    near: &Near,
    n: usize,
    guarded: &mut [Option<bool>],
    seed: i64,
    tuning: &WoundsTuning,
) -> Option<usize> {
    let taker_at = near.standing[n];
    let taker = &squads[taker_at.0].living[taker_at.1];
    let here = near.standing_points[n];
    let mut best: Option<(usize, (f64, i64))> = None;
    let mut found = vec![];
    near.downed_grid.near(here, tuning.reach + MARGIN, |i| {
        found.push(i as usize);
        true
    });
    for b in found {
        let (s, u) = near.downed[b];
        let body = &squads[s].downed[u];
        if body.faction == taker.faction {
            continue;
        }
        let gap = near.downed_points[b].distance_to(here) as f64;
        if gap > tuning.reach || is_guarded(squads, near, b, guarded, tuning) {
            continue;
        }
        if body.playing_dead && fooled(taker, body, seed, tuning) {
            continue; // it takes the body for dead (Decision 127)
        }
        let key = (crate::maths::snapped(gap, 0.000001), contest::draw(seed, body.id));
        if best.is_none_or(|(_, k)| before(key, k)) {
            best = Some((b, key));
        }
    }
    let (body, _) = best?;
    let mut fighting = false;
    near.standing_grid.near(here, tuning.guard_reach + MARGIN, |i| {
        let (s, u) = near.standing[i as usize];
        let other = &squads[s].living[u];
        let close = near.standing_points[i as usize].distance_to(here) as f64 <= tuning.guard_reach;
        fighting = other.faction != taker.faction && close;
        !fighting
    });
    if fighting {
        None
    } else {
        Some(body)
    }
}

/// `a` before `b`, item by item (GDScript's `<` on two arrays).
fn before(a: (f64, i64), b: (f64, i64)) -> bool {
    if a.0 != b.0 {
        return a.0 < b.0;
    }
    a.1 < b.1
}

/// True if the taker believes a body playing dead is dead: the body's wits and cunning beat
/// the taker's wits and thoroughness in a seeded roll, once for the pair.
pub fn fooled(taker: &Taking, body: &Taking, seed: i64, tuning: &WoundsTuning) -> bool {
    let guile = body.wits + body.cunning as f64 * tuning.trait_wits;
    let sight = taker.wits + taker.thorough as f64 * tuning.trait_wits;
    let roll = ghash::uniform(seed, &[Key::Int(taker.id), Key::Int(body.id), Key::Str("believe")]);
    guile - sight + (roll - 0.5) * tuning.believe_die > 0.0
}

/// True if a friend of the `b`th downed body stands within `guard_reach` of it (once a body).
fn is_guarded(
    squads: &[Tended],
    near: &Near,
    b: usize,
    guarded: &mut [Option<bool>],
    tuning: &WoundsTuning,
) -> bool {
    if let Some(known) = guarded[b] {
        return known;
    }
    let (s, u) = near.downed[b];
    let faction = squads[s].downed[u].faction;
    let there = near.downed_points[b];
    let mut found = false;
    near.standing_grid.near(there, tuning.guard_reach + MARGIN, |i| {
        let (s, u) = near.standing[i as usize];
        found = squads[s].living[u].faction == faction
            && near.standing_points[i as usize].distance_to(there) as f64 <= tuning.guard_reach;
        !found
    });
    guarded[b] = Some(found);
    found
}

/// One body's takers act: a captor captures it; else a leader's messenger sends it home;
/// else each taker due strikes it. What it came to, if anything did.
fn take(
    columns: &mut Columns,
    squads: &[Tended],
    body_at: At,
    takers: &[At],
    messengers: &mut [Vec<i64>],
    interval: i64,
    fell: &mut Vec<i64>,
) -> Option<Taken> {
    if let Some(&by) = takers.iter().find(|t| squads[t.0].living[t.1].captor) {
        return Some(Taken::Captured { body: body_at, by });
    }
    for taker in takers {
        let squad = &squads[taker.0];
        let leader =
            (0..squad.living.len()).find(|&u| squad.living[u].leads && messengers[taker.0][u] > 0);
        if let Some(leader) = leader {
            messengers[taker.0][leader] -= 1;
            return Some(Taken::SentHome { body: body_at, leader: (taker.0, leader) });
        }
    }
    let body = &squads[body_at.0].downed[body_at.1];
    for taker in takers {
        let striker = &squads[taker.0].living[taker.1];
        let slot = striker.slot;
        columns.target_id[slot] = body.id;
        columns.attack_cooldown[slot] -= 1;
        if columns.attack_cooldown[slot] > 0 {
            continue;
        }
        let every = (interval as f64 * striker.melee_seconds).round() as i32;
        columns.attack_cooldown[slot] = every.max(1);
        hurt(columns, body.slot, body.id, striker.dmg, fell);
    }
    if columns.hp[body.slot] as i64 <= -depth(body.constitution) {
        return Some(Taken::Finished { body: body_at });
    }
    None
}

/// Takes `amount` from the slot's hp (UnitHealth.hurt); one taken from above 0 to 0 or below
/// falls.
pub fn hurt(columns: &mut Columns, slot: usize, id: i64, amount: i64, fell: &mut Vec<i64>) {
    let was = columns.hp[slot];
    columns.hp[slot] = (was as i64 - amount) as i32;
    if was > 0 && columns.hp[slot] <= 0 {
        fell.push(id);
    }
}

/// A routing unit struck, as surrender is given it: its id, store slot and courage.
#[derive(Clone, Debug)]
pub struct Caught {
    pub id: i64,
    pub slot: usize,
    pub courage: f64,
}

/// Which of the struck routers (still standing, none "never_surrenders") surrender: one still
/// above 0 hp, by a seeded chance from its want of courage.
pub fn surrenders(
    columns: &Columns,
    caught: &[Caught],
    tick: i64,
    seed: i64,
    chance: f64,
) -> Vec<bool> {
    caught
        .iter()
        .map(|unit| {
            if columns.hp[unit.slot] <= 0 {
                return false;
            }
            let want = chance * (1.0 - clampf(unit.courage / 100.0, 0.0, 1.0));
            let keys = [Key::Int(tick), Key::Int(unit.id), Key::Str("surrender")];
            ghash::uniform(seed, &keys) < want
        })
        .collect()
}

#[cfg(test)]
mod tests;

//! What the wounded come to (FormationRecovery, Decisions 121, 125, 126 and 127; spec 39 step
//! 6b). A unit's **condition** is 1 less `condition` for each wound past those it is hardened
//! to; below `drain_below` it loses HP, standing or downed, the faster the further below. A
//! **downed** unit lies a seeded while (`wake_seconds`, shorter the hardier it is) and then
//! comes to at 1 HP where it lies: it joins the nearest standing friendly formation it can
//! see, and otherwise makes for home on its own. With a foe within `danger_reach` when it
//! could get up, it may play dead instead (Decision 127). Regeneration is `regeneration.rs`'s,
//! run here. `pass.rs` reads the squads in the crate and lands what this decides: the
//! states, the joins and the events.

use crate::columns::Columns;
use crate::contest;
use crate::ghash::{self, Key};
use crate::grid::{Grid, MARGIN};
use crate::maths::{clampf, lerpf, maxf, snapped, V2};
use crate::regeneration::{self, Mender};
use crate::wounds::{self, At, WoundsTuning, AVERAGE};

/// A unit as recovery is given it: its id, store slot, faction (a code), its wounds, its
/// "hardened" and "cunning" traits, whether it leads, mends downed ("regenerates_downed") and
/// plays dead, its regeneration and most HP, and its wits, constitution and detection range.
#[derive(Clone, Debug)]
pub struct Recovering {
    pub id: i64,
    pub slot: usize,
    pub faction: u32,
    pub wounded: i64,
    pub hardened: i64,
    pub cunning: i64,
    pub leads: bool,
    pub regenerates_downed: bool,
    pub playing_dead: bool,
    pub regeneration: f64,
    pub max_hp: i64,
    pub wits: f64,
    pub constitution: f64,
    pub detection: f64,
}

impl Recovering {
    /// An unwounded unit of the first faction with 10 HP, average wits and constitution, that
    /// sees 10 cells.
    #[cfg(test)]
    pub fn plain(id: i64, slot: usize) -> Recovering {
        Recovering {
            id,
            slot,
            faction: 0,
            wounded: 0,
            hardened: 0,
            cunning: 0,
            leads: false,
            regenerates_downed: false,
            playing_dead: false,
            regeneration: 0.0,
            max_hp: 10,
            wits: AVERAGE,
            constitution: AVERAGE,
            detection: 10.0,
        }
    }

    fn mender(&self) -> Mender {
        Mender {
            slot: self.slot,
            regeneration: self.regeneration,
            max_hp: self.max_hp,
            regenerates_downed: self.regenerates_downed,
        }
    }
}

/// A squad as recovery is given it: its id, whether one coming to may join it (not destroyed
/// or routing), and its living units and its downed (borne or not), each in its order.
#[derive(Clone, Debug, Default)]
pub struct Ward {
    pub id: i64,
    pub joinable: bool,
    pub living: Vec<Recovering>,
    pub downed: Vec<Recovering>,
}

/// What befell a downed unit (by its place in its squad's downed list), in the pass's order.
#[derive(Clone, Debug, PartialEq)]
pub enum Lay {
    DiedOfWounds(At),
    /// It began to play dead.
    PlayingDead(At),
}

/// A downed unit coming to, and the squad (its index) it joins, or none: it makes for home.
#[derive(Clone, Debug, PartialEq)]
pub struct Rising {
    pub unit: At,
    pub joins: Option<usize>,
}

/// What the pass decided: the downed's events, those coming to (nearest first, ties by their
/// draws) and the ids an hp write took from above 0 to 0 or below, in the order they fell.
#[derive(Debug, Default)]
pub struct Recovered {
    pub lay: Vec<Lay>,
    pub rising: Vec<Rising>,
    pub fell: Vec<i64>,
}

/// Its condition: 1, less a step for each wound past those it is hardened to; 0 to the cap.
pub fn condition_of(unit: &Recovering, tuning: &WoundsTuning) -> f64 {
    clampf(unclamped(unit, tuning), 0.0, tuning.condition_cap)
}

fn unclamped(unit: &Recovering, tuning: &WoundsTuning) -> f64 {
    let wounds = (unit.wounded - unit.hardened).max(0);
    1.0 - wounds as f64 * tuning.condition
}

/// One tick of `seconds`. The standing touch only themselves, so each squad's go first; then
/// its downed, as the list began.
pub fn step(
    columns: &mut Columns,
    wards: &[Ward],
    seconds: f64,
    seed: i64,
    tuning: &WoundsTuning,
) -> Recovered {
    let mut out = Recovered::default();
    let mut waking: Vec<At> = vec![]; // decided first, so no squad's order moves another's
    let mut standing: Option<Standing> = None; // found once a tick, when first asked
    for (s, ward) in wards.iter().enumerate() {
        for unit in &ward.living {
            columns.condition[unit.slot] = condition_of(unit, tuning);
            regeneration::mend(columns, unit.mender(), false, seconds);
            drain(columns, unit, seconds, tuning, &mut out.fell);
        }
        for (u, unit) in ward.downed.iter().enumerate() {
            columns.condition[unit.slot] = condition_of(unit, tuning);
            regeneration::mend(columns, unit.mender(), true, seconds);
            let drained = drain(columns, unit, seconds, tuning, &mut out.fell);
            let depth = wounds::depth(unit.constitution);
            if drained && columns.hp[unit.slot] as i64 <= -depth {
                out.lay.push(Lay::DiedOfWounds((s, u)));
                continue;
            }
            if !wakes(columns, unit, seconds, seed, tuning) {
                continue;
            }
            let standing = standing.get_or_insert_with(|| Standing::of(columns, wards));
            if !lies_still(columns, unit, s, standing, seed, tuning) {
                waking.push((s, u));
            } else if !unit.playing_dead {
                out.lay.push(Lay::PlayingDead((s, u)));
            }
        }
    }
    out.rising = rise(columns, wards, &waking, seed);
    out
}

/// A poor condition drains its HP (the part of a point drained kept in `drain_carry`); true
/// if it lost any this tick.
fn drain(
    columns: &mut Columns,
    unit: &Recovering,
    seconds: f64,
    tuning: &WoundsTuning,
    fell: &mut Vec<i64>,
) -> bool {
    let below = tuning.drain_below - unclamped(unit, tuning);
    if below <= 0.0 {
        return false;
    }
    let slot = unit.slot;
    columns.drain_carry[slot] += tuning.drain * below / tuning.drain_below * seconds;
    let whole = (columns.drain_carry[slot] + 0.000001).floor();
    wounds::hurt(columns, slot, unit.id, whole as i64, fell);
    columns.drain_carry[slot] -= whole;
    whole > 0.0
}

/// Counts down its time downed, reckoning it first if it hasn't been; true once it is up.
fn wakes(
    columns: &mut Columns,
    unit: &Recovering,
    seconds: f64,
    seed: i64,
    tuning: &WoundsTuning,
) -> bool {
    let slot = unit.slot;
    if columns.wake_left[slot] < 0.0 {
        let keys = [Key::Int(unit.id), Key::Int(unit.wounded), Key::Str("wake")];
        let roll = ghash::uniform(seed, &keys);
        let hardiness = maxf(1.0, unit.constitution) / AVERAGE;
        let (low, high) = tuning.wake_spread;
        columns.wake_left[slot] = tuning.wake_seconds * lerpf(low, high, roll) / hardiness;
    }
    columns.wake_left[slot] -= seconds;
    columns.wake_left[slot] <= 0.0
}

/// Every squad's standing units, where they stand and their factions, and each squad's
/// leaders' greatest cunning (very low with none).
struct Standing {
    grid: Grid,
    points: Vec<V2>,
    factions: Vec<u32>,
    led_cunning: Vec<i64>,
}

impl Standing {
    fn of(columns: &Columns, wards: &[Ward]) -> Standing {
        let all = wards.iter().flat_map(|w| &w.living);
        let points: Vec<V2> = all.clone().map(|u| columns.position[u.slot]).collect();
        let led_cunning = wards
            .iter()
            .map(|w| {
                let leaders = w.living.iter().filter(|u| u.leads);
                leaders.map(|u| u.cunning).fold(-(1i64 << 62), i64::max)
            })
            .collect();
        Standing {
            grid: Grid::build(&points),
            factions: all.map(|u| u.faction).collect(),
            points,
            led_cunning,
        }
    }
}

/// Whether a unit due to come to lies still instead (Decision 127): with a standing foe
/// near, one already playing dead keeps at it, and one not yet decides by a seeded chance
/// from its wits and cunning (its own or its leaders'); either looks again a few seconds on.
fn lies_still(
    columns: &mut Columns,
    unit: &Recovering,
    squad: usize,
    standing: &Standing,
    seed: i64,
    tuning: &WoundsTuning,
) -> bool {
    let reach = tuning.danger_reach;
    let here = columns.position[unit.slot];
    let mut danger = false;
    standing.grid.near(here, reach + MARGIN, |i| {
        let i = i as usize;
        danger = standing.factions[i] != unit.faction
            && standing.points[i].distance_to(here) as f64 <= reach;
        !danger
    });
    if !danger {
        return false;
    }
    if !unit.playing_dead {
        let cunning = unit.cunning.max(standing.led_cunning[squad]);
        let chance = tuning.play_dead * unit.wits / AVERAGE + cunning as f64 * tuning.cunning;
        let keys = [Key::Int(unit.id), Key::Int(unit.wounded), Key::Str("play dead")];
        if ghash::uniform(seed, &keys) >= chance {
            return false;
        }
    }
    columns.wake_left[unit.slot] = tuning.play_dead_check;
    true
}

/// Those coming to this tick get up together (Decision 97): each finds the nearest standing
/// friendly formation it can see as all stood before any rose, then they are taken nearest
/// first, ties by their seeded draws - never the list. Each is put at least at 1 HP.
fn rise(columns: &mut Columns, wards: &[Ward], waking: &[At], seed: i64) -> Vec<Rising> {
    if waking.is_empty() {
        return vec![];
    }
    let friends = Friends::of(columns, wards, seed);
    let mut plans: Vec<((f64, i64), Rising)> = waking
        .iter()
        .map(|&at| {
            let unit = &wards[at.0].downed[at.1];
            let (joins, gap) = friends.nearest(columns.position[unit.slot], unit);
            ((gap, contest::draw(seed, unit.id)), Rising { unit: at, joins })
        })
        .collect();
    plans.sort_by(|a, b| a.0 .0.total_cmp(&b.0 .0).then(a.0 .1.cmp(&b.0 .1)));
    for (_, plan) in &plans {
        let slot = wards[plan.unit.0].downed[plan.unit.1].slot;
        columns.hp[slot] = columns.hp[slot].max(1);
    }
    plans.into_iter().map(|(_, plan)| plan).collect()
}

/// The standing of the formations one coming to may join: each with its squad's index and
/// seeded draw, where it stands and its faction, in the squads' order.
struct Friends {
    grid: Grid,
    points: Vec<V2>,
    factions: Vec<u32>,
    squads: Vec<(usize, i64)>,
}

impl Friends {
    fn of(columns: &Columns, wards: &[Ward], seed: i64) -> Friends {
        let mut f =
            Friends { grid: Grid::default(), points: vec![], factions: vec![], squads: vec![] };
        for (s, ward) in wards.iter().enumerate().filter(|(_, w)| w.joinable) {
            let draw = contest::squad_draw(seed, ward.id);
            for unit in &ward.living {
                f.points.push(columns.position[unit.slot]);
                f.factions.push(unit.faction);
                f.squads.push((s, draw));
            }
        }
        f.grid = Grid::build(&f.points);
        f
    }

    /// The nearest friendly formation the unit at `at` can see, and how far its nearest
    /// member is (INF if none): nearest first, then the squads' draws, then the squads' order.
    fn nearest(&self, at: V2, unit: &Recovering) -> (Option<usize>, f64) {
        let mut best: Option<(usize, (f64, i64, usize))> = None;
        self.grid.near(at, unit.detection + MARGIN, |i| {
            let i = i as usize;
            if self.factions[i] != unit.faction {
                return true;
            }
            let gap = self.points[i].distance_to(at) as f64;
            let key = (snapped(gap, 0.000001), self.squads[i].1, i);
            if gap <= unit.detection && best.is_none_or(|(_, k)| key < k) {
                best = Some((self.squads[i].0, key));
            }
            true
        });
        match best {
            Some((squad, key)) => (Some(squad), key.0),
            None => (None, f64::INFINITY),
        }
    }
}

pub mod pass;
#[cfg(test)]
mod tests;

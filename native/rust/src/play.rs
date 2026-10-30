//! A digest battle played end to end in the crate (spec 39 step 8c-vii), as
//! FormationSimulation plays it with nothing from GDScript: `Play::new` sets the battle up
//! (`battles::set_up`) with its ground made from its map, and each `step` is
//! FormationSimulation.step: the tick counted, the squads listed with every unit each holds as
//! CrateRoster.of lists them (`listed`), the whole tick (`tick::step`) on the rules the crate
//! builds from the battle's tuning (`tick::given`, `tick::rules`), the orders queued cleared,
//! the squads listed as GDScript's mirrors leave them, and the units no squad holds any more
//! let go (UnitStore.settle). `Watch` holds the digest's stop rules (formation_digest.gd's
//! `_run`): a limit, so many ticks past first contact, or TAIL ticks after a side no longer
//! stands.

#[cfg(test)]
mod tests;

use std::collections::HashSet;

use crate::battles::{self, Battle, Set};
use crate::commands::MERGES;
use crate::content::Content;
use crate::events::{Log, ENGAGED, KINDS, LISTS};
use crate::field::Field;
use crate::geometry::CELLS;
use crate::ground::{Ground, GroundTuning, Walker};
use crate::locks;
use crate::pathing::pass::walker;
use crate::paths::shares::Cache;
use crate::paths::Over;
use crate::roster::{Listed, Member, Roster};
use crate::squads::Squads;
use crate::tick::{self, given, Ticked};

/// FormationSimulation.TRAVEL_SCALE: tiles a second at speed 1.
const TRAVEL_SCALE: f64 = 0.125;
/// The digest's ticks a run plays on once a side has broken: its rout, the pursuit, the
/// downed taken (formation_digest.gd's TAIL).
pub const TAIL: i64 = 150;
/// The sides whose standing the digest watches (BattleTrials.FACTIONS).
const FACTIONS: [&str; 2] = ["player", "the_kingdom"];

/// A battle being played: what it was set up with, its squads in GDScript's list's order,
/// its tick, seed and whether blows roll, and its ground with the paths' cache it keeps.
pub struct Play {
    pub set: Set,
    pub listed: Vec<usize>,
    pub tick: i64,
    pub seed: i64,
    pub rolls: bool,
    pub threaded: bool,
    ground: Option<(Ground, Cache)>,
}

impl Play {
    /// `battle` set up on `squads` and `field` from `content`, seeded `seed`, its blows rolled
    /// if `rolls`; None, with nothing written, if it can't be.
    pub fn new(
        battle: &Battle,
        (squads, field): (&mut Squads, &mut Field),
        content: &Content,
        (seed, rolls, threaded): (i64, bool, bool),
    ) -> Option<Play> {
        let first = squads.count();
        let set = battles::set_up(battle, squads, field, content)?;
        let listed = (first..squads.count()).collect();
        let made = set.map.and_then(|name| crate::ground::map::of(name, &content.tuning));
        let ground =
            made.map(|map| (map.ground(GroundTuning::of(&content.tuning)), Cache::default()));
        Some(Play { set, listed, tick: 0, seed, rolls, threaded, ground })
    }

    /// One tick (FormationSimulation.step), its events in `log`; None, with nothing done but
    /// the tick counted, if the tuning gives no rules or no squad is listed.
    pub fn step(
        &mut self,
        squads: &mut Squads,
        field: &mut Field,
        content: &Content,
        log: &mut Log,
    ) -> Option<Ticked> {
        self.tick += 1;
        if self.listed.is_empty() {
            return None; // FormationTick.step: nothing to do
        }
        let seconds = self.set.tick_seconds;
        let at = given::At {
            tick: self.tick,
            seed: self.seed,
            ticks_a_second: (1.0 / seconds).round().max(1.0) as i64,
            rolls: self.rolls,
            threaded: self.threaded,
        };
        let paces = given::Paces {
            travel_scale: TRAVEL_SCALE,
            seconds,
            route_length: self.set.route_tiles,
            cells_a_second: TRAVEL_SCALE * CELLS,
        };
        let (climb, rules) = tick::rules::of(&given::of(&content.tuning, &at, &paces))?;
        let roster = listed(squads, field, &self.listed, self.ground.is_some());
        let ticked = match self.ground.as_mut() {
            Some((ground, cache)) => {
                let mut over = Over { ground, cache, tuning: climb };
                tick::step(squads, field, &roster, Some(&mut over), &rules, log)
            }
            None => tick::step(squads, field, &roster, None, &rules, log),
        };
        self.listed.clone_from(&ticked.listed);
        squads.pending.clear(); // taken, or nobody was left to take them
        settle(squads, field, &self.listed);
        Some(ticked)
    }

    /// Whether some squad of each side still stands: anyone standing, not routing.
    pub fn both_stand(&self, squads: &Squads, field: &Field) -> bool {
        FACTIONS.iter().all(|name| {
            let Some(code) = field.lexicon.find(name) else { return false };
            self.listed.iter().any(|&s| {
                squads.faction[s] == code
                    && squads.state[s] != locks::ROUTING
                    && !squads.living(s, &field.columns.state).is_empty()
            })
        })
    }
}

/// The squads in the slots `listed`, each with every unit it holds, living or not, as
/// CrateRoster.of(squads, terrain, true) lists them: what of each unit the roster carries read
/// from its sheet and objects (its kind its definition's code), its walker on ground (`on`)
/// and the open ground's otherwise. The factions are left to `tick::step`, which codes them
/// in the list's order.
pub fn listed(squads: &Squads, field: &Field, listed: &[usize], on: bool) -> Roster {
    let columns = &field.columns;
    let open = Walker { height: 0.0, swimmer: 0, climber: 0, swims: false, climbs: false };
    let member = |unit: i32| {
        let u = unit as usize;
        let (sheet, objects) = (&columns.sheet[u], &columns.objects[u]);
        Member {
            id: field.unit_at(unit as u32).unwrap_or(0),
            slot: u,
            foot: (sheet.footprint_width, sheet.footprint_depth),
            preferred: sheet.preferred_position,
            attack_range: sheet.attack_range,
            kind: objects.definition as u32,
            turn_rate: sheet.turn_rate,
            backward_pace: sheet.backward_pace,
            detection: sheet.detection,
            max_stamina: sheet.max_stamina,
            walker: if on { walker(objects.walker, sheet.height) } else { open },
        }
    };
    let listed = listed.iter().map(|&s| Listed {
        squad: s,
        id: squads.id[s],
        faction: 0,
        joined: !squads.joined[s].is_empty(),
        merges: squads.command[s].is_some_and(|c| squads.commands.gives(c, MERGES)),
        living: squads.members[s].iter().map(|&u| member(u)).collect(),
    });
    Roster { listed: listed.collect() }
}

/// UnitStore.settle: the units no listed squad holds any more give up their slots.
fn settle(squads: &Squads, field: &mut Field, listed: &[usize]) {
    let members = listed.iter().flat_map(|&s| squads.members[s].iter());
    let kept: HashSet<i64> = members.filter_map(|&u| field.unit_at(u as u32)).collect();
    if kept.len() == field.held_count() {
        return;
    }
    for id in field.held_units() {
        if !kept.contains(&id) {
            field.release(id);
        }
    }
}

/// Whether the flat log tells of an event of `kind`.
pub fn told(flat: &[i64], kind: usize) -> bool {
    let mut at = 0;
    while at < flat.len() {
        let found = flat[at] as usize;
        if found == kind {
            return true;
        }
        at += 4;
        for name in KINDS[found].1 {
            at += if LISTS.contains(name) { 1 + flat[at] as usize } else { 1 };
        }
    }
    false
}

/// The digest's stop rules (formation_digest.gd's `_run`): `limit` ticks; `after` ticks past
/// the first engagement, if given; TAIL ticks after a side stopped standing.
#[derive(Clone, Copy, Debug, Default)]
pub struct Watch {
    pub limit: i64,
    pub after: Option<i64>,
    contact: Option<i64>,
    broken: Option<i64>,
}

impl Watch {
    pub fn new(limit: i64, after: Option<i64>) -> Watch {
        Watch { limit, after, ..Watch::default() }
    }

    /// Whether the run stops after `ticks`, whose events were `engaged` or not, with both
    /// sides standing or not.
    pub fn stops(&mut self, ticks: i64, engaged: bool, both_stand: bool) -> bool {
        if self.contact.is_none() && engaged {
            self.contact = Some(ticks);
        }
        if let (Some(after), Some(contact)) = (self.after, self.contact) {
            if ticks - contact >= after {
                return true;
            }
        }
        if self.broken.is_none() && !both_stand {
            self.broken = Some(ticks);
        }
        self.broken.is_some_and(|broken| ticks - broken >= TAIL) || ticks >= self.limit
    }
}

/// `play` played on until `watch` stops it, each tick's events taken from `log` (and
/// dropped); the ticks played.
pub fn run(
    play: &mut Play,
    (squads, field): (&mut Squads, &mut Field),
    content: &Content,
    log: &mut Log,
    watch: &mut Watch,
) -> i64 {
    loop {
        play.step(squads, field, content, log);
        let engaged = told(&log.take(), ENGAGED);
        if watch.stops(play.tick, engaged, play.both_stand(squads, field)) {
            return play.tick;
        }
    }
}

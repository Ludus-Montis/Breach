//! The named scenarios the capture plays and the bench times (spec 44 step 2, Decision 169):
//! `content/scenarios/scenarios.json` names each one's map, its record (a FormationRecord,
//! Decision 115, whose header carries the seed) and the behaviours it must show. A scenario is
//! played on its map from its record hands-free (`Replay`), and the behaviours are read from
//! what the replay does (`Seen`). Nothing in the shipped game reads the list.

pub mod seen;
#[cfg(test)]
mod tests;
pub mod timed;

use std::fs;
use std::path::Path;

use serde::Deserialize;

use crate::battles::TICK_SECONDS;
use crate::content::Content;
use crate::digest::state::state;
use crate::events::Log;
use crate::field::Field;
use crate::field_play::FieldPlay;
use crate::ground::map;
use crate::ground::{Ground, GroundTuning};
use crate::play::TAIL;
use crate::record::{self, Record};
use crate::squads::Squads;
use crate::threads::Threads;

pub use seen::{Seen, BEHAVIOURS};

/// The list's folder, under `content/`, and its file.
pub const FOLDER: &str = "scenarios";
const LIST: &str = "scenarios.json";
/// The maps a scenario can be played on, for now.
const MAPS: [&str; 1] = ["field"];
/// The most a scenario plays, in seconds (Decision 169).
pub const MOST_SECONDS: f64 = 90.0;

/// A scenario as listed: its name, map, record file and the behaviours it must show.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
struct Listed {
    name: String,
    map: String,
    record: String,
    shows: Vec<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct List {
    scenarios: Vec<Listed>,
}

/// A scenario read: its name, map, record (its text as written, and parsed) and the behaviours
/// it must show.
#[derive(Clone, Debug, PartialEq)]
pub struct Scenario {
    pub name: String,
    pub map: String,
    pub text: String,
    pub record: Record,
    pub shows: Vec<String>,
}

/// The scenarios listed under `content` (the repo's `content/` folder), in the list's order;
/// an error naming the file and what is wrong with it: an unknown map or behaviour, a record
/// that doesn't read, a name used twice.
pub fn list(content: &Path) -> Result<Vec<Scenario>, String> {
    let folder = content.join(FOLDER);
    let path = folder.join(LIST);
    let text = fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    let list: List = serde_json::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))?;
    let mut read: Vec<Scenario> = Vec::new();
    for listed in list.scenarios {
        let name = listed.name.clone();
        let wrong = |what: String| format!("{}: scenario {name}: {what}", path.display());
        if read.iter().any(|s| s.name == name) {
            return Err(wrong("listed twice".into()));
        }
        if !MAPS.contains(&listed.map.as_str()) {
            return Err(wrong(format!("no map {}", listed.map)));
        }
        if let Some(unknown) = listed.shows.iter().find(|b| !BEHAVIOURS.contains(&b.as_str())) {
            return Err(wrong(format!("no behaviour {unknown}")));
        }
        let file = folder.join(&listed.record);
        let text =
            fs::read_to_string(&file).map_err(|e| wrong(format!("{}: {e}", file.display())))?;
        let record = record::parse(&text).ok_or_else(|| wrong("its record doesn't read".into()))?;
        let Listed { name, map, shows, .. } = listed;
        read.push(Scenario { name, map, text, record, shows });
    }
    Ok(read)
}

/// The scenario named `name` in `list`, else an error naming it.
pub fn named<'a>(list: &'a [Scenario], name: &str) -> Result<&'a Scenario, String> {
    list.iter().find(|s| s.name == name).ok_or_else(|| format!("no scenario named {name}"))
}

/// A scenario being played: the field from its record, its commands given on their ticks, and
/// what it has shown so far. It ends when a side no longer stands after first contact (TAIL
/// ticks on, so a rout plays out) or at MOST_SECONDS of play, whichever comes first.
pub struct Replay {
    pub play: FieldPlay,
    pub seen: Seen,
    record: Record,
    ground: Ground,
    most: i64,
    broken: Option<i64>,
}

impl Replay {
    /// `scenario` set up on `squads` and `field` from `content`, its blows rolled, `threaded`
    /// or not; None if the field can't be set up.
    pub fn new(
        scenario: &Scenario,
        (squads, field): (&mut Squads, &mut Field),
        content: &Content,
        threaded: Threads,
    ) -> Option<Replay> {
        let record = scenario.record.clone();
        let at = (record.seed, true, threaded);
        let play = FieldPlay::new(record.captained, (squads, field), content, at)?;
        let ground =
            map::of(&scenario.map, &content.tuning)?.ground(GroundTuning::of(&content.tuning));
        let most = (MOST_SECONDS / TICK_SECONDS).round() as i64;
        let seen = Seen::new(&scenario.shows);
        Some(Replay { play, seen, record, ground, most, broken: None })
    }

    /// The tick played next.
    pub fn tick(&self) -> i64 {
        self.play.play.tick
    }

    /// One tick: its commands given, the field stepped, what it shows read; its events (flat)
    /// and whether the scenario has ended.
    pub fn step(
        &mut self,
        squads: &mut Squads,
        field: &mut Field,
        content: &Content,
    ) -> (Vec<i64>, bool) {
        let tick = self.tick();
        for given in self.record.commands.iter().filter(|c| c.tick == tick) {
            self.play.apply(given, (squads, field), content);
        }
        let mut log = Log::default();
        self.play.step(squads, field, content, &mut log);
        let flat = log.take();
        self.seen.read(tick, &flat, (squads, field), &self.ground);
        (flat, self.ended(tick, squads, field))
    }

    /// The listed squads' state as the digest records it, as bytes.
    pub fn state(&self, squads: &Squads, field: &Field) -> Vec<u8> {
        state(squads, field, &self.play.play.listed).bytes()
    }

    /// Whether the scenario ends after `tick`.
    fn ended(&mut self, tick: i64, squads: &Squads, field: &Field) -> bool {
        let contacted = self.seen.contact.is_some();
        if contacted && self.broken.is_none() && !self.play.play.both_stand(squads, field) {
            self.broken = Some(tick);
        }
        self.broken.is_some_and(|broken| tick - broken >= TAIL) || tick + 1 >= self.most
    }
}

/// `scenario` played to its end on a fresh crate, one thread: what it showed and the ticks
/// played; None if the field can't be set up.
pub fn watch(scenario: &Scenario, content: &Content) -> Option<(Seen, i64)> {
    let (mut squads, mut field) = (Squads::default(), Field::default());
    let mut replay = Replay::new(scenario, (&mut squads, &mut field), content, Threads::One)?;
    loop {
        let (_, ended) = replay.step(&mut squads, &mut field, content);
        if ended {
            let ticks = replay.tick();
            return Some((replay.seen, ticks));
        }
    }
}

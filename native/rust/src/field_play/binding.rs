//! `FieldState`, the 2D feel test's field held by the crate (spec 39 step 9d-i), as its scene
//! and HUD read it. `open` sets the field up and starts it on a worker (step 10d-ii,
//! `FieldPlayer`): the frame owes the ticks its clock gives (`owe`), pauses, gives the feel
//! test's words or a record's commands (`act`, `give`, `replay`; applied at the next boundary
//! between ticks and recorded with that tick when taken) and takes what was played (`take`),
//! never waiting. The events are the dictionaries CrateEvents made, the builders' and waves'
//! first. The reads show the newest snapshot: the squads with their units (`seen.rs`), the
//! units' positions, facings and ranks, the waves' places and B's staging point. The routes
//! and the ground don't change, and are read at open. `run_ticks` and `step` wait, for tests
//! and headless play.

use std::path::Path;

use godot::prelude::*;

use super::player::FieldPlayer;
use super::sim::{FieldSim, FieldTold, FieldView};
use super::{A, B, C};
use crate::events::told::{Said, Told};
use crate::maths::{self, V2};
use crate::record::{self, Command};
use crate::scenarios::{self, Scenario};
use crate::squads::DEAD;
use crate::threads::Threads;
use crate::waves::Produced;
use crate::ContentStore;

/// SkirmishSquad.State's names, by code.
const SQUAD_STATES: [&str; 6] =
    ["MOVING", "HOLDING", "FIGHTING", "ARRIVED", "DESTROYED", "ROUTING"];
/// SkirmishUnit.State's names, by code.
const UNIT_STATES: [&str; 9] =
    ["MOVING", "HOLDING", "FIGHTING", "ARRIVED", "DEAD", "DOWNED", "TAKEN", "RELEASED", "CARRIED"];
/// The waves' and routes' keys, by index.
const KEYS: [&str; 3] = ["A", "B", "C"];

#[derive(GodotClass)]
#[class(base = RefCounted, init)]
struct FieldState {
    played: Option<Played>,
}

/// A field opened: played on a worker, and what doesn't change while it plays (the routes
/// and the ground, read at open).
struct Played {
    player: FieldPlayer,
    seed: i64,
    captained: bool,
    routes: VarDictionary,
    ground: VarArray,
    /// The named scenario it plays hands-free, if it plays one (`open_scenario`).
    scenario: Option<Scenario>,
}

#[godot_api]
impl FieldState {
    /// A fresh field from `content`, its line `captained` or not, its battle seeded
    /// `battle_seed`, its record's header stamped with `build`, started on a worker; false,
    /// with nothing set up, if the content isn't loaded or lacks a unit the field needs.
    #[func]
    fn open(
        &mut self,
        content: Gd<ContentStore>,
        battle_seed: i64,
        captained: bool,
        build: GString,
    ) -> bool {
        let Some(loaded) = content.bind().content.clone() else { return false };
        let opened =
            FieldSim::open(loaded, (battle_seed, captained), &build.to_string(), threads());
        let Some(sim) = opened else { return false };
        self.start(sim, None);
        true
    }

    /// The scenario `name` from the list under `content_root` (the repo's `content/` folder),
    /// set up from `content` to play hands-free on a worker (spec 44 step 2c): its record's
    /// commands given on their ticks, what it shows read as the bench's replay reads it. ""
    /// if it opened; else what is wrong, with nothing set up (a name not on the list is named).
    #[func]
    fn open_scenario(
        &mut self,
        content: Gd<ContentStore>,
        content_root: GString,
        name: GString,
    ) -> GString {
        let name = name.to_string();
        let Some(loaded) = content.bind().content.clone() else {
            return GString::from("the content isn't loaded");
        };
        let opened = scenarios::list(Path::new(&content_root.to_string()))
            .and_then(|list| scenarios::named(&list, &name).cloned())
            .and_then(|scenario| {
                let sim = FieldSim::scenario(loaded, &scenario, threads());
                sim.map(|sim| (sim, scenario))
                    .ok_or_else(|| format!("scenario {name}: the field can't be set up"))
            });
        match opened {
            Ok((sim, scenario)) => {
                self.start(sim, Some(scenario));
                GString::new()
            }
            Err(wrong) => GString::from(wrong.as_str()),
        }
    }

    /// True once the scenario the field plays has ended (false for free play).
    #[func]
    fn ended(&self) -> bool {
        self.view().and_then(|v| v.watched.as_ref()).is_some_and(|w| w.ended)
    }

    /// The scenario's `capture.json` as played so far, at commit `commit`
    /// (`scenarios::capture`); "" for free play.
    #[func]
    fn capture(&self, commit: GString) -> GString {
        let Some(p) = self.played.as_ref() else { return GString::new() };
        let (Some(scenario), Some(watched)) = (&p.scenario, &p.player.newest.watched) else {
            return GString::new();
        };
        let made = scenarios::capture::capture(
            scenario,
            &watched.seen,
            p.player.tick,
            &commit.to_string(),
        );
        let text = serde_json::to_string_pretty(&made).unwrap_or_default();
        GString::from(text.as_str())
    }

    /// The ticks played, as the newest snapshot shows them.
    #[func]
    fn tick_number(&self) -> i64 {
        self.played.as_ref().map_or(0, |p| p.player.tick)
    }

    #[func]
    fn battle_seed(&self) -> i64 {
        self.played.as_ref().map_or(0, |p| p.seed)
    }

    #[func]
    fn captained(&self) -> bool {
        self.played.as_ref().is_some_and(|p| p.captained)
    }

    /// `ticks` more owed by the frame's clock, played on the worker; true if time was dropped
    /// (more than two owed: the simulation is behind). Never waits.
    #[func]
    fn owe(&self, ticks: i64) -> bool {
        let ticks = u32::try_from(ticks).unwrap_or(0);
        self.played.as_ref().is_some_and(|p| p.player.owe(ticks))
    }

    /// No tick starts while paused; an order given then applies at once.
    #[func]
    fn pause(&self, paused: bool) {
        if let Some(p) = self.played.as_ref() {
            p.player.pause(paused);
        }
    }

    /// True if the newest snapshot is the tick the clock is drawing towards (else the frame
    /// holds the units at the newest).
    #[func]
    fn caught_up(&self) -> bool {
        self.played.as_ref().is_none_or(|p| p.player.caught_up())
    }

    /// The ticks the worker has played since last asked, without waiting: their events, in
    /// order (each tick's builders' and waves' first), as CrateEvents made them. The reads
    /// below show the newest.
    #[func]
    fn take(&mut self) -> Array<VarDictionary> {
        let Some(p) = self.played.as_mut() else { return Array::new() };
        events_of(&p.player.take())
    }

    /// `count` ticks played, and every order given before applied, waiting for them (for
    /// tests and headless play); their events, in order.
    #[func]
    fn run_ticks(&mut self, count: i64) -> Array<VarDictionary> {
        let Some(p) = self.played.as_mut() else { return Array::new() };
        events_of(&p.player.run_ticks(u32::try_from(count).unwrap_or(0)))
    }

    /// One tick (FormationField.step), waiting for it: the builders fill the waves, full waves
    /// announce (and depart if automatic), then the fight. Its events, the builders' and
    /// waves' first.
    #[func]
    fn step(&mut self) -> Array<VarDictionary> {
        self.run_ticks(1)
    }

    /// Gives the feel test's `words` ("send A", "via_c on", "pursues off", ...) at the next
    /// boundary between ticks, recorded with that tick if the field takes them; true if they
    /// are a command (whether it was taken shows in the record).
    #[func]
    fn act(&mut self, words: GString) -> bool {
        match record::from_words(self.tick_number(), &words.to_string()) {
            Some(given) => self.given(given),
            None => false,
        }
    }

    /// Gives a record's command (`read_record`'s: tick, who, order, target, value) at the
    /// next boundary between ticks, recorded with that tick if the field takes it.
    #[func]
    fn give(&mut self, command: VarDictionary) -> bool {
        let given = command_from(self.tick_number(), &command);
        self.given(given)
    }

    /// A replay's commands (`read_record`'s), each given when its tick comes.
    #[func]
    fn replay(&mut self, commands: Array<VarDictionary>) {
        let Some(p) = self.played.as_ref() else { return };
        let read = commands.iter_shared().map(|c| {
            let tick = c.get("tick").and_then(|t| t.try_to::<i64>().ok()).unwrap_or(0);
            command_from(tick, &c)
        });
        p.player.replay(read.collect());
    }

    /// The record of play so far, to copy out: a replay of it gives the same battle.
    #[func]
    fn record(&self) -> GString {
        let record = self.played.as_ref().map_or(String::new(), |p| p.player.record.join("\n"));
        GString::from(record.as_str())
    }

    /// The record `text` holds (FormationRecord.parse): its seed, whether its line is
    /// captained, the build that played it and its commands (tick, who, order, target,
    /// value) in order; empty if it isn't one.
    #[func]
    fn read_record(text: GString) -> VarDictionary {
        let mut read = VarDictionary::new();
        let Some(parsed) = record::parse(&text.to_string()) else { return read };
        let commands: Array<VarDictionary> = parsed.commands.iter().map(command_of).collect();
        read.set("seed", parsed.seed);
        read.set("captained", parsed.captained);
        read.set("build", parsed.build.as_str());
        read.set("commands", &commands);
        read
    }

    /// The listed squads as the scene draws them, in the listed order: each its `id`,
    /// `faction`, `state` (SkirmishSquad.State's name), `position` (its front, in cells),
    /// `morale`, `band` (0 steady to 3 routing), `blocked`, `staged`, `heading` (its stance's,
    /// or null), `pursuit` (`post_at`, `gone`, `leash`, or null if it isn't pursuing),
    /// `loose` (unit ids) and `units`: each its `id`, `slot`, `state`
    /// (SkirmishUnit.State's name), `alive`, `footprint_width`, `footprint_depth`,
    /// `leadership` and `detection`.
    #[func]
    fn squads(&self) -> Array<VarDictionary> {
        let Some(view) = self.view() else { return Array::new() };
        view.seen.iter().map(squad_of).collect()
    }

    /// Where each unit stands, in cells, by its slot.
    #[func]
    fn positions(&self) -> PackedVector2Array {
        let Some(view) = self.view() else { return PackedVector2Array::new() };
        view.positions.iter().map(|&p| vector(p)).collect()
    }

    /// The way each unit faces, a unit vector, by its slot.
    #[func]
    fn facings(&self) -> PackedVector2Array {
        let Some(view) = self.view() else { return PackedVector2Array::new() };
        view.bearings.iter().map(|&b| vector(maths::bearing_vector(b))).collect()
    }

    /// Each unit's rank (0: the front), by its slot.
    #[func]
    fn ranks(&self) -> PackedInt32Array {
        let Some(view) = self.view() else { return PackedInt32Array::new() };
        PackedInt32Array::from(view.ranks.as_slice())
    }

    /// The kingdom's line's and reserve's ids.
    #[func]
    fn kingdom(&self) -> PackedInt64Array {
        let Some(view) = self.view() else { return PackedInt64Array::new() };
        PackedInt64Array::from(view.kingdom.as_slice())
    }

    /// Each wave's places by its key ("A", "B"): [rank and file built, in all, leaders built,
    /// in all].
    #[func]
    fn waves(&self) -> VarDictionary {
        let mut waves = VarDictionary::new();
        let Some(view) = self.view() else { return waves };
        for w in [A, B] {
            waves.set(KEYS[w], &PackedInt64Array::from(&view.waves[w][..]));
        }
        waves
    }

    /// Where B's waves wait, in cells, or null if they don't.
    #[func]
    fn staging_point(&self) -> Variant {
        let point = self.view().and_then(|v| v.staging);
        point.map_or(Variant::nil(), |p| vector(p).to_variant())
    }

    /// The routes by key ("A", "B", "C"): [their points, in cells, their half width].
    #[func]
    fn routes(&self) -> VarDictionary {
        self.played.as_ref().map_or_else(VarDictionary::new, |p| p.routes.clone())
    }

    /// The field's size in cells, and its ground as painted, in order: each [its area, in
    /// cells, what it is ("wood", "depth" or "height") and how deep or high].
    #[func]
    fn ground(&self) -> VarArray {
        self.played.as_ref().map_or_else(VarArray::new, |p| p.ground.clone())
    }
}

impl FieldState {
    /// `sim` started on a worker, playing `scenario` if it is one.
    fn start(&mut self, sim: FieldSim, scenario: Option<Scenario>) {
        let (seed, captained) = (sim.seed, sim.captained);
        let (routes, ground) = (routes_of(&sim), ground_of(&sim));
        let player = FieldPlayer::start(sim);
        self.played = Some(Played { player, seed, captained, routes, ground, scenario });
    }

    /// `given` queued for the field; true if there is a field.
    fn given(&mut self, given: Command) -> bool {
        let Some(p) = self.played.as_ref() else { return false };
        p.player.order(given);
        true
    }

    /// The newest snapshot's view.
    fn view(&self) -> Option<&FieldView> {
        self.played.as_ref().map(|p| &p.player.newest)
    }
}

/// A record's command as a dictionary gives it (who, order, target, value), at `tick`.
/// The threads the field plays on, as BREACH_NATIVE_THREADS says.
fn threads() -> Threads {
    Threads::of_env(std::env::var("BREACH_NATIVE_THREADS").ok().as_deref())
}

fn command_from(tick: i64, command: &VarDictionary) -> Command {
    let text = |key: &str| command.get(key).map_or(String::new(), |v| v.to_string());
    record::command(tick, &text("who"), &text("order"), &text("target"), &text("value"))
}

/// Ticks' events as `take` gives them, each tick's builders' and waves' first.
fn events_of(ticks: &[FieldTold]) -> Array<VarDictionary> {
    let mut out = Array::new();
    for (made, told) in ticks {
        for event in made {
            out.push(&produced(event));
        }
        for event in told {
            out.push(&event_of(event));
        }
    }
    out
}

/// The routes by key, as `routes` gives them.
fn routes_of(sim: &FieldSim) -> VarDictionary {
    let mut routes = VarDictionary::new();
    for r in [A, B, C] {
        let route = &sim.play.routes[r].0;
        let points: PackedVector2Array = route.points.iter().map(|&p| vector(p)).collect();
        routes.set(KEYS[r], &varray![&points, route.half_width]);
    }
    routes
}

/// The field's size and ground, as `ground` gives them; empty if it has no map.
fn ground_of(sim: &FieldSim) -> VarArray {
    let mut areas = VarArray::new();
    let map = sim.play.play.set.map.and_then(|n| crate::ground::map::of(n, &sim.content.tuning));
    let Some(map) = map else { return areas };
    areas.push(&Vector2i::new(map.size.0, map.size.1).to_variant());
    for ((x, y, w, h), paint) in &map.areas {
        let (kind, value) = if paint.blocks_sight == Some(true) {
            ("wood", 0.0)
        } else if let Some(depth) = paint.depth {
            ("depth", depth)
        } else {
            ("height", f64::from(paint.height.unwrap_or(0)))
        };
        areas.push(&varray![Rect2i::from_components(*x, *y, *w, *h), kind, value].to_variant());
    }
    areas
}

fn vector(p: V2) -> Vector2 {
    Vector2::new(p.x, p.y)
}

/// A command as `read_record` gives it.
fn command_of(given: &Command) -> VarDictionary {
    let mut out = VarDictionary::new();
    out.set("tick", given.tick);
    for (key, value) in [("who", &given.who), ("order", &given.order)] {
        out.set(key, value.as_str());
    }
    out.set("target", given.target.as_str());
    out.set("value", given.value.as_str());
    out
}

/// What the builders or a wave did, as DomainProduction and FormationProduction told it.
fn produced(made: &Produced) -> VarDictionary {
    let mut out = VarDictionary::new();
    let (kind, tick) = match *made {
        Produced::FromReserve { tick, lane } => {
            out.set("lane", KEYS[lane]);
            ("from_reserve", tick)
        }
        Produced::Built { tick, lane } => {
            out.set("lane", KEYS[lane]);
            ("built", tick)
        }
        Produced::ToReserve { tick } => ("to_reserve", tick),
        Produced::WaveFull { tick, built } => {
            out.set("built", built);
            ("wave_full", tick)
        }
        Produced::Departed { tick, units } => {
            out.set("units", units);
            ("departed", tick)
        }
    };
    out.set("type", kind);
    out.set("tick", tick);
    out.set("faction", "player");
    out
}

/// An event as CrateEvents made it.
pub(crate) fn event_of(event: &Told) -> VarDictionary {
    let mut out = VarDictionary::new();
    out.set("type", event.kind);
    out.set("tick", event.tick);
    out.set(if event.of_unit { "unit" } else { "squad" }, event.who);
    out.set("faction", event.faction.as_str());
    for (name, said) in &event.fields {
        let value = match said {
            Said::Int(n) => n.to_variant(),
            Said::Bool(b) => b.to_variant(),
            Said::Real(r) => r.to_variant(),
            Said::Word(w) => GString::from(w.as_str()).to_variant(),
            Said::List(list) => {
                list.iter().map(|n| n.to_variant()).collect::<VarArray>().to_variant()
            }
        };
        out.set(*name, &value);
    }
    out
}

/// A squad as `squads` gives it.
fn squad_of(seen: &super::seen::SeenSquad) -> VarDictionary {
    let mut out = VarDictionary::new();
    out.set("id", seen.id);
    out.set("faction", seen.faction.as_str());
    out.set("state", name(&SQUAD_STATES, seen.state));
    out.set("position", vector(seen.position));
    out.set("morale", seen.morale);
    out.set("band", seen.band);
    out.set("blocked", seen.blocked);
    out.set("staged", seen.staged);
    out.set("heading", &seen.heading.map_or(Variant::nil(), |h| h.to_variant()));
    let pursuit = seen.pursuit.as_ref().map_or(Variant::nil(), |p| {
        let mut pursuing = VarDictionary::new();
        pursuing.set("post_at", vector(p.post_at));
        pursuing.set("gone", p.gone);
        pursuing.set("leash", p.leash);
        pursuing.to_variant()
    });
    out.set("pursuit", &pursuit);
    out.set("loose", &PackedInt64Array::from(seen.loose.as_slice()));
    let units: Array<VarDictionary> = seen
        .units
        .iter()
        .map(|unit| {
            let mut out = VarDictionary::new();
            out.set("id", unit.id);
            out.set("slot", unit.slot as i64);
            out.set("state", name(&UNIT_STATES, unit.state));
            out.set("alive", unit.state < DEAD);
            out.set("footprint_width", unit.foot.0);
            out.set("footprint_depth", unit.foot.1);
            out.set("leadership", unit.leadership);
            out.set("detection", unit.detection);
            out
        })
        .collect();
    out.set("units", &units);
    out
}

/// The name of `code` among `names`; "" if it is none of them.
fn name(names: &'static [&'static str], code: i32) -> &'static str {
    usize::try_from(code).ok().and_then(|n| names.get(n).copied()).unwrap_or("")
}

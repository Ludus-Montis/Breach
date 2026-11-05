//! `FieldState`, the 2D feel test's field held by the crate (spec 39 step 9d-i), as its scene
//! and HUD read it: `open` sets the field up, `step` plays a tick as FormationField.step does
//! and returns its events as the dictionaries CrateEvents made (the builders' and waves'
//! first), and `act` and `give` give a command now (the feel test's words, or a record's
//! command), recorded with its tick when the field takes it (FormationRecord). After each tick
//! the scene reads what it draws: the squads with their units (`seen.rs`), the units'
//! positions, facings and ranks, the waves' places, B's staging point, the routes and the
//! ground.

use godot::prelude::*;

use super::{FieldPlay, A, B, C};
use crate::events::told::{self, Said, Told};
use crate::events::Log;
use crate::field::Field;
use crate::maths::{self, V2};
use crate::record::{self, Command};
use crate::squads::{Squads, DEAD};
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
    squads: Squads,
    field: Field,
    play: Option<FieldPlay>,
    content: Option<Gd<ContentStore>>,
    events: Log,
    seed: i64,
    captained: bool,
    /// The record of play: its header, then each command taken, a line each.
    record: Vec<String>,
}

#[godot_api]
impl FieldState {
    /// A fresh field from `content`, its line `captained` or not, its battle seeded
    /// `battle_seed`, its record's header stamped with `build`; false, with nothing set up, if
    /// the content isn't loaded or lacks a unit the field needs.
    #[func]
    fn open(
        &mut self,
        content: Gd<ContentStore>,
        battle_seed: i64,
        captained: bool,
        build: GString,
    ) -> bool {
        let (mut squads, mut field) = (Squads::default(), Field::default());
        let threaded = std::env::var("BREACH_NATIVE_THREADS").is_ok_and(|v| v == "1");
        let at = (battle_seed, true, threaded);
        let made = {
            let store = content.bind();
            let Some(loaded) = store.content.as_ref() else { return false };
            FieldPlay::new(captained, (&mut squads, &mut field), loaded, at)
        };
        let Some(play) = made else { return false };
        *self = FieldState {
            squads,
            field,
            play: Some(play),
            content: Some(content),
            events: Log::default(),
            seed: battle_seed,
            captained,
            record: vec![record::header(battle_seed, captained, &build.to_string())],
        };
        true
    }

    /// The ticks played.
    #[func]
    fn tick_number(&self) -> i64 {
        self.play.as_ref().map_or(0, |p| p.play.tick)
    }

    #[func]
    fn battle_seed(&self) -> i64 {
        self.seed
    }

    #[func]
    fn captained(&self) -> bool {
        self.captained
    }

    /// One tick (FormationField.step): the builders fill the waves, full waves announce (and
    /// depart if automatic), then the fight. Its events, the builders' and waves' first.
    #[func]
    fn step(&mut self) -> Array<VarDictionary> {
        let Some(content) = self.content.clone() else { return Array::new() };
        let store = content.bind();
        let (Some(loaded), Some(play)) = (store.content.as_ref(), self.play.as_mut()) else {
            return Array::new();
        };
        let (made, _) = play.step(&mut self.squads, &mut self.field, loaded, &mut self.events);
        let mut out: Array<VarDictionary> = made.iter().map(produced).collect();
        let flat = self.events.take();
        let words = self.field.lexicon.words();
        for event in told::decoded(&flat, words, &play.play.codes.definitions) {
            out.push(&event_of(&event));
        }
        out
    }

    /// Gives the feel test's `words` ("send A", "via_c on", "pursues off", ...) now; true,
    /// and recorded at this tick, if the field takes them.
    #[func]
    fn act(&mut self, words: GString) -> bool {
        match record::from_words(self.tick_number(), &words.to_string()) {
            Some(given) => self.given(given),
            None => false,
        }
    }

    /// Gives a record's command (`read_record`'s: tick, who, order, target, value) now, at
    /// this tick; true, and recorded, if the field takes it.
    #[func]
    fn give(&mut self, command: VarDictionary) -> bool {
        let text = |key: &str| command.get(key).map_or(String::new(), |v| v.to_string());
        let given = record::command(
            self.tick_number(),
            &text("who"),
            &text("order"),
            &text("target"),
            &text("value"),
        );
        self.given(given)
    }

    /// The record of play so far, to copy out: a replay of it gives the same battle.
    #[func]
    fn record(&self) -> GString {
        GString::from(self.record.join("\n").as_str())
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
        let Some(content) = self.content.as_ref() else { return Array::new() };
        let store = content.bind();
        let (Some(loaded), Some(play)) = (store.content.as_ref(), self.play.as_ref()) else {
            return Array::new();
        };
        play.seen(&self.squads, &self.field, loaded).iter().map(squad_of).collect()
    }

    /// Where each unit stands, in cells, by its slot.
    #[func]
    fn positions(&self) -> PackedVector2Array {
        self.field.columns.position.iter().map(|&p| vector(p)).collect()
    }

    /// The way each unit faces, a unit vector, by its slot.
    #[func]
    fn facings(&self) -> PackedVector2Array {
        let bearings = self.field.columns.bearing.iter();
        bearings.map(|&b| vector(maths::bearing_vector(b))).collect()
    }

    /// Each unit's rank (0: the front), by its slot.
    #[func]
    fn ranks(&self) -> PackedInt32Array {
        PackedInt32Array::from(self.field.columns.rank.as_slice())
    }

    /// The kingdom's line's and reserve's ids.
    #[func]
    fn kingdom(&self) -> PackedInt64Array {
        let Some(play) = self.play.as_ref() else { return PackedInt64Array::new() };
        play.kingdom.iter().map(|&s| self.squads.id[s]).collect()
    }

    /// Each wave's places by its key ("A", "B"): [rank and file built, in all, leaders built,
    /// in all].
    #[func]
    fn waves(&self) -> VarDictionary {
        let mut waves = VarDictionary::new();
        let Some(content) = self.content.as_ref() else { return waves };
        let store = content.bind();
        let (Some(loaded), Some(play)) = (store.content.as_ref(), self.play.as_ref()) else {
            return waves;
        };
        for w in [A, B] {
            waves.set(KEYS[w], &PackedInt64Array::from(&play.counted(w, loaded)[..]));
        }
        waves
    }

    /// Where B's waves wait, in cells, or null if they don't.
    #[func]
    fn staging_point(&self) -> Variant {
        let point = self.play.as_ref().and_then(FieldPlay::staging_point);
        point.map_or(Variant::nil(), |p| vector(p).to_variant())
    }

    /// The routes by key ("A", "B", "C"): [their points, in cells, their half width].
    #[func]
    fn routes(&self) -> VarDictionary {
        let mut routes = VarDictionary::new();
        let Some(play) = self.play.as_ref() else { return routes };
        for r in [A, B, C] {
            let route = &play.routes[r].0;
            let points: PackedVector2Array = route.points.iter().map(|&p| vector(p)).collect();
            routes.set(KEYS[r], &varray![&points, route.half_width]);
        }
        routes
    }

    /// The field's size in cells, and its ground as painted, in order: each [its area, in
    /// cells, what it is ("wood", "depth" or "height") and how deep or high].
    #[func]
    fn ground(&self) -> VarArray {
        let mut areas = VarArray::new();
        let Some(content) = self.content.as_ref() else { return areas };
        let store = content.bind();
        let (Some(loaded), Some(play)) = (store.content.as_ref(), self.play.as_ref()) else {
            return areas;
        };
        let Some(map) = play.play.set.map.and_then(|n| crate::ground::map::of(n, &loaded.tuning))
        else {
            return areas;
        };
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
}

impl FieldState {
    /// `given` given to the field; recorded, and true, if it takes it.
    fn given(&mut self, given: Command) -> bool {
        let Some(content) = self.content.clone() else { return false };
        let store = content.bind();
        let (Some(loaded), Some(play)) = (store.content.as_ref(), self.play.as_mut()) else {
            return false;
        };
        let taken = play.apply(&given, (&mut self.squads, &mut self.field), loaded);
        if taken {
            self.record.push(record::line(&given));
        }
        taken
    }
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
fn event_of(event: &Told) -> VarDictionary {
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

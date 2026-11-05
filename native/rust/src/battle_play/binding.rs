//! `BattleState`, the 1D feel test's battle held by the crate (spec 39 step 9d-ii), as its
//! scene and HUD read it: `open` makes the battle and `add_lane` each lane, `step` plays a
//! tick as FormationBattle.step does and returns its events by lane (the domains' under ""),
//! the wave's, then the battle's as CrateEvents made them. The scene paints, applies presets,
//! sends, orders and sets the builders through it, and after each tick reads each lane's
//! squads (`seen.rs`), its wave's places, the pool and the builders.

use godot::prelude::*;

use super::presets::{self, Saved};
use super::{foot, BattlePlay, KINDS};
use crate::content::Content;
use crate::events::told;
use crate::field_play::binding::event_of;
use crate::waves::{Claim, Produced, Template};
use crate::ContentStore;

/// SkirmishSquad.State's names, by code.
const SQUAD_STATES: [&str; 6] =
    ["MOVING", "HOLDING", "FIGHTING", "ARRIVED", "DESTROYED", "ROUTING"];
/// SkirmishUnit.Order's names, by code.
const ORDERS: [&str; 3] = ["ADVANCE", "HOLD", "RETREAT"];

#[derive(GodotClass)]
#[class(base = RefCounted, init)]
struct BattleState {
    battle: Option<BattlePlay>,
    content: Option<Gd<ContentStore>>,
}

#[godot_api]
impl BattleState {
    /// FormationBattle.new from `content`: `pool_total` slots across the lanes `widths` names
    /// ({key: its most frontline width}, in order), ticks of `tick_seconds`, the kingdom's
    /// lines `line` militia long; false if the content isn't loaded or has no militia.
    #[func]
    fn open(
        &mut self,
        content: Gd<ContentStore>,
        pool_total: i64,
        widths: VarDictionary,
        tick_seconds: f64,
        line: i64,
    ) -> bool {
        let widths: Vec<(String, i32)> =
            widths.iter_shared().map(|(k, w)| (k.to_string(), w.to::<i64>() as i32)).collect();
        let made = {
            let store = content.bind();
            let Some(loaded) = store.content.as_ref() else { return false };
            BattlePlay::new(pool_total as i32, widths, tick_seconds, line as i32, loaded)
        };
        let Some(battle) = made else { return false };
        *self = BattleState { battle: Some(battle), content: Some(content) };
        true
    }

    /// FormationBattle.add_lane: lane `key`, `route_tiles` long, starting with a line of
    /// `count` of kind `kind` (KINDS' index) as wide as the lane; false if it can't be.
    #[func]
    fn add_lane(&mut self, key: GString, route_tiles: f64, kind: i64, count: i64) -> bool {
        self.with(|battle, content| {
            let (Some(&id), Some(&(_, width))) = (
                KINDS.get(kind as usize),
                battle.widths.iter().find(|(k, _)| *k == key.to_string()),
            ) else {
                return false;
            };
            let Some(feet) = foot(content, id) else { return false };
            let start = Template::line(id, feet, count as i32, width);
            battle.add_lane(&key.to_string(), route_tiles, start, content)
        })
        .unwrap_or(false)
    }

    #[func]
    fn lane_keys(&self) -> PackedStringArray {
        let keys = self.battle.iter().flat_map(|b| b.lanes.iter().map(|l| l.key.as_str()));
        keys.map(GString::from).collect()
    }

    /// The ticks lane `key` has played.
    #[func]
    fn tick_number(&self, key: GString) -> i64 {
        self.lane_of(&key).map_or(0, |(b, k)| b.lanes[k].play.tick)
    }

    /// Lane `key`'s length, in tiles.
    #[func]
    fn route_length(&self, key: GString) -> f64 {
        self.lane_of(&key).map_or(0.0, |(b, k)| b.lanes[k].play.set.route_tiles)
    }

    /// The width lane `key`'s squads may spread to.
    #[func]
    fn combat_width(&self, key: GString) -> i64 {
        self.lane_of(&key).map_or(0, |(b, k)| b.lanes[k].play.set.combat_width)
    }

    /// One tick of everything (FormationBattle.step), the kingdom building if `with_kingdom`:
    /// {lane key: its events}, the domains' under "".
    #[func]
    fn step(&mut self, with_kingdom: bool) -> VarDictionary {
        let mut out = VarDictionary::new();
        self.with(|battle, content| {
            let stepped = battle.step(with_kingdom, content);
            let keys: Vec<String> = battle.lanes.iter().map(|l| l.key.clone()).collect();
            let domains: Array<VarDictionary> =
                stepped.domains.iter().map(|(side, p)| produced(side, p, &keys)).collect();
            out.set("", &domains);
            for (k, told_now) in stepped.lanes.iter().enumerate() {
                let lane = &mut battle.lanes[k];
                let mut events: Array<VarDictionary> =
                    told_now.iter().map(|(side, p)| produced(side, p, &keys)).collect();
                let flat = lane.log.take();
                let words = lane.field.lexicon.words();
                for event in told::decoded(&flat, words, &lane.play.codes.definitions) {
                    events.push(&event_of(&event));
                }
                out.set(keys[k].as_str(), &events);
            }
        });
        out
    }

    /// The cells lane `key` may paint: its own plus whatever the pool has free.
    #[func]
    fn allowance(&self, key: GString) -> i64 {
        self.lane_of(&key).map_or(0, |(b, k)| i64::from(b.allowance(k)))
    }

    /// Every lane's brush: KINDS' index, or any other number to erase.
    #[func]
    fn set_brush(&mut self, kind: i64) {
        self.with(|battle, content| {
            let id = usize::try_from(kind).ok().and_then(|k| KINDS.get(k).copied());
            let brush = id.and_then(|id| foot(content, id).map(|f| (id, f)));
            for lane in &mut battle.lanes {
                lane.brush = brush;
            }
        });
    }

    /// Paints (or erases) `cell` (rank, column) of lane `key`'s wave; the units banked.
    #[func]
    fn paint(&mut self, key: GString, cell: Vector2i, erase: bool) -> i64 {
        let k = self.index(&key);
        let banked = self.with(|b, _| k.map_or(0, |k| b.paint(k, (cell.x, cell.y), erase)));
        banked.unwrap_or(0) as i64
    }

    /// WavePresets.to_dict: lane `key`'s shape saved as `name`: {"name", "units": [{"kind",
    /// "rank", "column"}]}.
    #[func]
    fn preset(&self, key: GString, name: GString) -> VarDictionary {
        let mut out = VarDictionary::new();
        out.set("name", &name);
        let saved = self.lane_of(&key).map(|(b, k)| presets::saved(&b.waves[k].template));
        let units: VarArray = saved
            .unwrap_or_default()
            .iter()
            .map(|unit| {
                let mut out = VarDictionary::new();
                out.set("kind", unit.kind.as_str());
                out.set("rank", unit.rank);
                out.set("column", unit.column);
                out.to_variant()
            })
            .collect();
        out.set("units", &units);
        out
    }

    /// WavePresets.from_dict, then FormationBattle.apply: `preset` fitted to lane `key`'s
    /// width and allowance and applied now; the units banked.
    #[func]
    fn apply_preset(&mut self, key: GString, preset: VarDictionary) -> i64 {
        let units: Vec<Saved> = preset
            .get("units")
            .and_then(|u| u.try_to::<VarArray>().ok())
            .map(|list| list.iter_shared().filter_map(saved_of).collect())
            .unwrap_or_default();
        let k = self.index(&key);
        let banked = self.with(|battle, content| {
            let k = k?;
            let (width, allowance) = (battle.lanes[k].width, battle.allowance(k));
            Some(battle.apply(k, presets::fitted(&units, width, allowance, content)))
        });
        banked.flatten().unwrap_or(0) as i64
    }

    /// Lane `key`'s wave: `places`, each [rank, column, depth, width, filled, the unit's
    /// preferred position (0 front, 1 mid, 2 back), its kind (KINDS' index, or -1)],
    /// front-first; `built`; `auto`,
    /// whether it departs on its own; `merges`.
    #[func]
    fn wave(&self, key: GString) -> VarDictionary {
        let mut out = VarDictionary::new();
        let Some((battle, k)) = self.lane_of(&key) else { return out };
        let Some(content) = self.content.as_ref() else { return out };
        let store = content.bind();
        let Some(loaded) = store.content.as_ref() else { return out };
        let wave = &battle.waves[k];
        let places: VarArray = wave
            .template
            .ordered()
            .iter()
            .zip(&wave.filled)
            .map(|(p, &filled)| {
                let preferred = loaded.units.get(p.id).map_or(0, |u| u.preferred_position as i64);
                let kind = KINDS.iter().position(|&k| k == p.id).map_or(-1, |k| k as i64);
                varray![p.at.0, p.at.1, p.foot.0, p.foot.1, filled, preferred, kind].to_variant()
            })
            .collect();
        out.set("places", &places);
        out.set("built", wave.built());
        out.set("auto", wave.auto);
        out.set("merges", battle.lanes[k].merges);
        out
    }

    /// Lane `key`'s wave departs on its own once full (true) or waits to be sent.
    #[func]
    fn set_auto_departure(&mut self, key: GString, automatic: bool) {
        let k = self.index(&key);
        self.with(|b, _| k.map(|k| b.waves[k].auto = automatic));
    }

    /// The squads lane `key` sends merge into friends they catch up with.
    #[func]
    fn set_auto_merge(&mut self, key: GString, merges: bool) {
        let k = self.index(&key);
        self.with(|b, _| k.map(|k| b.lanes[k].merges = merges));
    }

    /// Sends lane `key`'s wave now; its squad's id, or -1 if nothing is built.
    #[func]
    fn send(&mut self, key: GString) -> i64 {
        let k = self.index(&key);
        let sent = self.with(|b, content| {
            let k = k?;
            b.send(k, content).map(|s| b.lanes[k].squads.id[s])
        });
        sent.flatten().unwrap_or(-1)
    }

    /// A line of the kingdom's militia at the far end of lane `key`.
    #[func]
    fn spawn_kingdom_line(&mut self, key: GString) {
        let k = self.index(&key);
        self.with(|b, content| k.map(|k| b.spawn_kingdom_line(k, content)));
    }

    /// `order` (SkirmishUnit.Order) for squad `squad_id` in lane `key`, on the next tick.
    #[func]
    fn order(&mut self, key: GString, squad_id: i64, order: i64) {
        let k = self.index(&key);
        self.with(|b, _| k.map(|k| b.order(k, squad_id, order as i32)));
    }

    /// The player's builders of kind `kind` (KINDS' index) set to `count`.
    #[func]
    fn set_builders(&mut self, kind: i64, count: i64) {
        let Some(&id) = KINDS.get(kind as usize) else { return };
        self.with(|b, content| {
            let seconds = content.units.get(id).map_or(1.0, |u| u.build_seconds);
            b.player.set(id, seconds, count.max(0) as usize);
        });
    }

    #[func]
    fn builder_count(&self, kind: i64) -> i64 {
        let Some((battle, &id)) = self.battle.as_ref().zip(KINDS.get(kind as usize)) else {
            return 0;
        };
        let kinds = battle.player.kinds.iter().find(|k| k.0 == id);
        kinds.map_or(0, |k| k.2.len() as i64)
    }

    /// DomainProduction.prefer: lane `key` first, or "" to share in turn.
    #[func]
    fn prefer(&mut self, key: GString) {
        let k = self.index(&key);
        self.with(|b, _| b.player.prefer(k));
    }

    /// The player's sharing rule: {"round_robin", "first": the lane served first, or ""}.
    #[func]
    fn sharing(&self) -> VarDictionary {
        let mut out = VarDictionary::new();
        let Some(battle) = self.battle.as_ref() else { return out };
        out.set("round_robin", battle.player.round_robin);
        let first = battle.player.lane_order.first().map(|&k| battle.lanes[k].key.as_str());
        out.set("first", first.unwrap_or(""));
        out
    }

    /// The player's builds under way: [[KINDS' index, lane key or "" for the reserve,
    /// progress 0..1], ...].
    #[func]
    fn builds(&self) -> VarArray {
        let mut out = VarArray::new();
        let Some(battle) = self.battle.as_ref() else { return out };
        for (id, _, builders) in &battle.player.kinds {
            let kind = KINDS.iter().position(|k| k == id).map_or(-1, |k| k as i64);
            for builder in builders {
                let lane = match builder.claim {
                    Claim::Idle => continue,
                    Claim::Reserve => "",
                    Claim::Lane(k) => battle.lanes[k].key.as_str(),
                };
                out.push(&varray![kind, lane, builder.progress].to_variant());
            }
        }
        out
    }

    /// The pool and the reserve: {"total", "free", "assigned": {lane key: slots},
    /// "reserve", "reserve_cap"}.
    #[func]
    fn pool(&self) -> VarDictionary {
        let mut out = VarDictionary::new();
        let Some(battle) = self.battle.as_ref() else { return out };
        let mut assigned = VarDictionary::new();
        for (k, lane) in battle.lanes.iter().enumerate() {
            assigned.set(lane.key.as_str(), battle.pool.assigned(k));
        }
        out.set("total", battle.pool.total);
        out.set("free", battle.pool.free());
        out.set("assigned", &assigned);
        out.set("reserve", battle.player.reserve.len() as i64);
        out.set("reserve_cap", battle.player.reserve_cap as i64);
        out
    }

    /// Lane `key`'s squads as the scene draws them, in the listed order: each its `id`,
    /// `faction`, `state` (SkirmishSquad.State's name), `order` (SkirmishUnit.Order's name),
    /// `direction` and its living `units`: each its `id`, `distance` (tiles along the lane),
    /// `lateral` (the middle of its span across the lane, a swap's move added),
    /// `footprint_width`, `footprint_depth`, `hp`, `max_hp`, `ranged` and `target` (0: none).
    #[func]
    fn squads(&self, key: GString) -> Array<VarDictionary> {
        let Some((battle, k)) = self.lane_of(&key) else { return Array::new() };
        battle.lanes[k].seen().iter().map(squad_of).collect()
    }
}

impl BattleState {
    /// `act` on the battle and the loaded content; None if either is missing.
    fn with<T>(&mut self, act: impl FnOnce(&mut BattlePlay, &Content) -> T) -> Option<T> {
        let content = self.content.clone()?;
        let store = content.bind();
        Some(act(self.battle.as_mut()?, store.content.as_ref()?))
    }

    fn index(&self, key: &GString) -> Option<usize> {
        self.battle.as_ref()?.lane(&key.to_string())
    }

    fn lane_of(&self, key: &GString) -> Option<(&BattlePlay, usize)> {
        let battle = self.battle.as_ref()?;
        Some((battle, battle.lane(&key.to_string())?))
    }
}

/// A preset's unit read from its dictionary (rank and column 0 if left out).
fn saved_of(unit: Variant) -> Option<Saved> {
    let unit = unit.try_to::<VarDictionary>().ok()?;
    let number = |key: &str| {
        let value = unit.get(key)?;
        value.try_to::<i64>().ok().or_else(|| value.try_to::<f64>().ok().map(|n| n as i64))
    };
    let kind = unit.get("kind").map_or(String::new(), |v| v.to_string());
    let (rank, column) = (number("rank").unwrap_or(0), number("column").unwrap_or(0));
    Some(Saved { kind, rank: rank as i32, column: column as i32 })
}

/// What a side's builders or wave did, as DomainProduction and FormationProduction told it.
fn produced(side: &str, made: &Produced, keys: &[String]) -> VarDictionary {
    let mut out = VarDictionary::new();
    let (kind, tick) = match *made {
        Produced::FromReserve { tick, lane } => {
            out.set("lane", keys[lane].as_str());
            ("from_reserve", tick)
        }
        Produced::Built { tick, lane } => {
            out.set("lane", keys[lane].as_str());
            ("built", tick)
        }
        Produced::ToReserve { tick } => {
            out.set("lane", "");
            ("to_reserve", tick)
        }
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
    out.set("faction", side);
    out
}

/// A squad as `squads` gives it.
fn squad_of(seen: &super::seen::SeenSquad) -> VarDictionary {
    let mut out = VarDictionary::new();
    out.set("id", seen.id);
    out.set("faction", seen.faction.as_str());
    out.set("state", name(&SQUAD_STATES, seen.state));
    out.set("order", name(&ORDERS, seen.order));
    out.set("direction", seen.direction);
    let units: Array<VarDictionary> = seen
        .units
        .iter()
        .map(|unit| {
            let mut out = VarDictionary::new();
            out.set("id", unit.id);
            out.set("distance", unit.distance);
            out.set("lateral", unit.lateral);
            out.set("footprint_width", unit.foot.0);
            out.set("footprint_depth", unit.foot.1);
            out.set("hp", unit.hp);
            out.set("max_hp", unit.max_hp);
            out.set("ranged", unit.ranged);
            out.set("target", unit.target);
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

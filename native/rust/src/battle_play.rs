//! The 1D feel test's battle in the crate (spec 39 step 9d-ii), as FormationBattle and
//! FormationLane play it: its lanes, each a battle of its own (`Play`, on a straight route as
//! long as the lane) with the player's wave and the kingdom's militia wave; the player's and the
//! kingdom's builders (`Builders`, DomainProduction), and the shared slot pool (`SlotPool`). A
//! lane may paint into its own cells plus any free slot, and every reshape banks the units it
//! displaces in the player's reserve. Each `step` is FormationBattle.step: the player's builders
//! (and the kingdom's, when asked), then each lane in turn: its wave announced full (and sent,
//! if it departs on its own), the kingdom's likewise, then its battle's tick.

mod binding;
pub mod presets;
pub mod seen;
pub mod slots;
#[cfg(test)]
mod tests;

use crate::battles::{self, Set};
use crate::content::Content;
use crate::events::Log;
use crate::field::Field;
use crate::field_play::{sent, Sending};
use crate::orders;
use crate::play::Play;
use crate::route::Route;
use crate::spawn::{self, Placed, Spawn};
use crate::squads::Squads;
use crate::waves::{Builders, Produced, Template, Wave, MAX_RANKS};
use slots::SlotPool;

/// FormationBattle.KINGDOM_BUILDERS: the kingdom's militia builders, building lines in turn.
pub const KINGDOM_BUILDERS: usize = 2;
/// The kingdom's unit.
pub const MILITIA: &str = "kingdom_militia";
/// The player's units, in the HUD's order (its brushes and builder rows).
pub const KINDS: [&str; 3] = ["grem", "grem_brute", "grem_spitter"];

/// A side's name, and whether it starts at the player's end.
const PLAYER: (&str, bool) = ("player", true);
const KINGDOM: (&str, bool) = ("the_kingdom", false);

/// One lane (FormationLane): its key and width, its battle with the squads and units in it and
/// its events, its route, the unit painted on click (None erases) and whether the waves it sends
/// merge into friends they catch up with.
pub struct Lane {
    pub key: String,
    pub width: i32,
    pub play: Play,
    pub squads: Squads,
    pub field: Field,
    pub log: Log,
    pub route: Route,
    pub brush: Option<(&'static str, (i32, i32))>,
    pub merges: bool,
}

/// The battle (FormationBattle): its lanes, the player's wave and the kingdom's in each (by
/// lane), the slot pool, both sides' builders, the kingdom's line's length and the tick.
pub struct BattlePlay {
    pub tick_seconds: f64,
    pub lanes: Vec<Lane>,
    pub waves: Vec<Wave>,
    pub militia: Vec<Wave>,
    pub pool: SlotPool,
    pub player: Builders,
    pub kingdom: Builders,
    line: i32,
    widths: Vec<(String, i32)>,
    tick: i64,
}

/// What a step did: the builders' (with their side), then each lane's waves' (with theirs).
pub struct Stepped {
    pub domains: Vec<(&'static str, Produced)>,
    pub lanes: Vec<Vec<(&'static str, Produced)>>,
}

/// A unit's footprint as a template paints it: (depth, width).
pub fn foot(content: &Content, id: &str) -> Option<(i32, i32)> {
    let unit = content.units.get(id)?;
    Some((unit.footprint_depth as i32, unit.footprint_width as i32))
}

impl BattlePlay {
    /// FormationBattle.new: `pool_total` slots across the lanes `widths` names (each its most
    /// frontline width), ticks of `tick_seconds`, the kingdom's lines `line` militia long; None
    /// if the content has no militia.
    pub fn new(
        pool_total: i32,
        widths: Vec<(String, i32)>,
        tick_seconds: f64,
        line: i32,
        content: &Content,
    ) -> Option<BattlePlay> {
        let militia = content.units.get(MILITIA)?;
        let caps = widths.iter().map(|(_, w)| w * MAX_RANKS).collect();
        let mut kingdom = Builders::default();
        kingdom.set(MILITIA, militia.build_seconds, KINGDOM_BUILDERS);
        kingdom.round_robin = true;
        kingdom.reserve_cap = 0;
        Some(BattlePlay {
            tick_seconds,
            lanes: Vec::new(),
            waves: Vec::new(),
            militia: Vec::new(),
            pool: SlotPool::new(pool_total, caps),
            player: Builders::default(),
            kingdom,
            line,
            widths,
            tick: 0,
        })
    }

    /// FormationBattle.add_lane: the lane `key` (one `widths` named), `route_tiles` long,
    /// starting with `start` painted (its cells from the pool), its combat width its own; false
    /// if `widths` didn't name it.
    pub fn add_lane(
        &mut self,
        key: &str,
        route_tiles: f64,
        start: Template,
        content: &Content,
    ) -> bool {
        let Some(&(_, width)) = self.widths.iter().find(|(k, _)| k == key) else { return false };
        let Some(militia) = foot(content, MILITIA) else { return false };
        let set = Set {
            route_tiles,
            tick_seconds: self.tick_seconds,
            combat_width: i64::from(width),
            map: None,
        };
        let play = Play::open(set, (0, false, false), content);
        let route = battles::lane(route_tiles);
        let (squads, field, log) = (Squads::default(), Field::default(), Log::default());
        let lane = Lane {
            key: key.to_string(),
            width,
            play,
            squads,
            field,
            log,
            route,
            brush: None,
            merges: false,
        };
        self.lanes.push(lane);
        self.waves.push(Wave::new(Template::new(width, 0), 0));
        let mut kingdom = Wave::new(Template::line(MILITIA, militia, self.line, self.line), 0);
        kingdom.auto = true;
        self.militia.push(kingdom);
        let k = self.lanes.len() - 1;
        self.reshaped(k, start);
        true
    }

    /// The lane `key`'s index.
    pub fn lane(&self, key: &str) -> Option<usize> {
        self.lanes.iter().position(|l| l.key == key)
    }

    /// FormationBattle.step: the builders, then each lane's waves and its battle's tick.
    pub fn step(&mut self, with_kingdom: bool, content: &Content) -> Stepped {
        let mut produced = Vec::new();
        self.player.step(&mut self.waves, self.tick_seconds, self.tick, &mut produced);
        let mut domains: Vec<(&'static str, Produced)> =
            produced.drain(..).map(|p| (PLAYER.0, p)).collect();
        if with_kingdom {
            self.kingdom.step(&mut self.militia, self.tick_seconds, self.tick, &mut produced);
            domains.extend(produced.drain(..).map(|p| (KINGDOM.0, p)));
        }
        let mut lanes = Vec::new();
        for k in 0..self.lanes.len() {
            let lane = &mut self.lanes[k];
            let mut told = announced(&mut self.waves[k], lane, (PLAYER, lane.merges), content);
            if with_kingdom {
                told.extend(announced(&mut self.militia[k], lane, (KINGDOM, false), content));
            }
            lane.play.step(&mut lane.squads, &mut lane.field, content, &mut lane.log);
            lanes.push(told);
        }
        self.tick += 1;
        Stepped { domains, lanes }
    }

    /// The cells lane `k` may paint: its own plus whatever the pool has free.
    pub fn allowance(&self, k: usize) -> i32 {
        self.pool.assigned(k) + self.pool.free()
    }

    /// FormationLane.paint: lane `k`'s brush painted at `cell` (or the cell erased, with
    /// `erase` or no brush), within its allowance; the units banked.
    pub fn paint(&mut self, k: usize, cell: (i32, i32), erase: bool) -> usize {
        let mut edited = self.waves[k].template.clone();
        edited.slot_limit = self.allowance(k);
        let changed = match self.lanes[k].brush.filter(|_| !erase) {
            Some((id, foot)) => edited.paint(id, foot, cell),
            None => edited.erase(cell),
        };
        if changed {
            self.reshaped(k, edited)
        } else {
            0
        }
    }

    /// A whole template (a preset, already fitted) applied to lane `k`; the units banked.
    pub fn apply(&mut self, k: usize, template: Template) -> usize {
        self.reshaped(k, template)
    }

    fn reshaped(&mut self, k: usize, template: Template) -> usize {
        let left = self.waves[k].reshape(template);
        self.pool.assign(k, self.waves[k].template.cells());
        for &id in &left {
            self.player.bank(id);
        }
        left.len()
    }

    /// FormationProduction.send: lane `k`'s wave sent now as one squad; its slot, or None if
    /// nothing is built.
    pub fn send(&mut self, k: usize, content: &Content) -> Option<usize> {
        let merges = self.lanes[k].merges;
        send(&mut self.waves[k], &mut self.lanes[k], PLAYER, merges, content)
    }

    /// FormationLane.spawn_kingdom_line: a line of the kingdom's militia, `line` wide, at the
    /// far end of lane `k`.
    pub fn spawn_kingdom_line(&mut self, k: usize, content: &Content) {
        let Some(unit) = content.units.get(MILITIA) else { return };
        let lane = &mut self.lanes[k];
        let code = lane.play.codes.definition(MILITIA);
        let placed: Vec<Placed> =
            (0..self.line).map(|column| Placed { unit, code, place: (0, column) }).collect();
        let route_code = Some(lane.play.codes.route(&mut lane.squads, 0));
        let spawn = Spawn {
            width: i64::from(self.line),
            placed: &placed,
            faction: KINGDOM.0,
            at_player_end: KINGDOM.1,
            wait_ticks: 0,
            route: &lane.route,
            route_code,
            combat_width: lane.play.set.combat_width,
        };
        let s = spawn::squad(&mut lane.squads, &mut lane.field, content, &spawn);
        lane.play.listed.push(s);
    }

    /// FormationSimulation.order: `order` for squad `squad_id` in lane `k`, on the next tick.
    pub fn order(&mut self, k: usize, squad_id: i64, order: i32) {
        orders::queue(&mut self.lanes[k].squads, squad_id, order);
    }
}

/// FormationProduction.step: `wave` announced once full, and sent on `lane` as `side`'s
/// squad (merging if `merges`) if it departs on its own; what it told, with its side.
fn announced(
    wave: &mut Wave,
    lane: &mut Lane,
    (side, merges): ((&'static str, bool), bool),
    content: &Content,
) -> Vec<(&'static str, Produced)> {
    if !wave.full() || wave.announced {
        return Vec::new();
    }
    wave.announced = true;
    let (tick, built) = (lane.play.tick, wave.built());
    let mut told = vec![(side.0, Produced::WaveFull { tick, built })];
    if wave.auto {
        send(wave, lane, side, merges, content);
        told.push((side.0, Produced::Departed { tick, units: built }));
    }
    told
}

/// `wave` sent as `side`'s squad on `lane`'s route, merging if `merges`.
fn send(
    wave: &mut Wave,
    lane: &mut Lane,
    side: (&str, bool),
    merges: bool,
    content: &Content,
) -> Option<usize> {
    let route_code = Some(lane.play.codes.route(&mut lane.squads, 0));
    let how = Sending {
        faction: side.0,
        at_player_end: side.1,
        wait_ticks: 0,
        route: &lane.route,
        route_code,
        merges,
    };
    sent(wave, &mut lane.play, &how, (&mut lane.squads, &mut lane.field), content)
}

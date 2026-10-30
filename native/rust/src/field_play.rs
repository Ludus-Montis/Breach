//! The field's waves played in the crate (spec 39 step 8c-viii), as FormationField plays them:
//! the kingdom's line and reserve set up (`Play`, `Battle::Field`), the player's waves on routes
//! A and B (eight grems in a line; B's led from its second rank by a chieftain), four grem
//! builders and one chieftain's sharing in turn, and each `step` FormationField.step: the
//! hurried and tending routes set on the player's squads, the builders and the waves, the
//! battle's tick, and the player's routers home banked. `send` is FormationProduction.send: the
//! filled places spawned as one squad, a partial wave closed up and centred on its route, and
//! its units stood on their places. `send_together` is FormationField.send_together: the waves
//! sent at once, each waiting so that all reach the line together, their marches timed by
//! rehearsal.

pub mod actions;
#[cfg(test)]
mod tests;

use crate::battles::Battle;
use crate::commands::{HURRY, MERGES};
use crate::content::Content;
use crate::events::{Log, FLED_HOME};
use crate::field::Field;
use crate::maths::V2;
use crate::play::{self, Play};
use crate::ranks::{self, Ranked};
use crate::rehearsal::{self, March};
use crate::rendezvous;
use crate::route::Route;
use crate::spawn::{self, Placed, Spawn};
use crate::squads::Squads;
use crate::tick::Ticked;
use crate::waves::{Builders, Painted, Produced, Template, Wave};

/// FormationFieldActions.BUILDERS.
pub const BUILDERS: usize = 4;
/// FormationField's measures, in cells.
const WAVE_WIDTH: i32 = 8;
const B_Y: f32 = 21.0;
const FLANK_X: f32 = 81.0;
const C_Y: f32 = 62.0;
/// The field's routes, in FormationField.route_points' order.
pub const A: usize = 0;
pub const B: usize = 1;
pub const C: usize = 2;
/// FormationField.contact_cells, by wave: how far along its route A's front meets the line's
/// front, and B's the line's north side.
pub(crate) const CONTACT: [f64; 2] = [79.0, 88.0];
/// The waves' units (FormationFieldActions.field).
const GREM: &str = "grem";
const CHIEFTAIN: &str = "grem_chieftain";

/// FormationField.route_points: A straight at the line, B north through the wood and onto
/// the line's side, C along the south edge and slanting onto A's lane; each 4 cells either
/// side.
pub fn routes() -> Vec<Route> {
    let half = f64::from(WAVE_WIDTH) / 2.0;
    let route = |points: &[(f32, f32)]| {
        Route::new(points.iter().map(|&(x, y)| V2::new(x, y)).collect(), half)
    };
    vec![
        route(&[(0.0, 32.0), (128.0, 32.0)]),
        route(&[(0.0, B_Y), (FLANK_X, B_Y), (FLANK_X, 64.0)]),
        route(&[(0.0, C_Y), (40.0, C_Y), (60.0, 32.0), (128.0, 32.0)]),
    ]
}

/// The field being played: the battle, the kingdom's line and reserve, its routes and the
/// codes they were given as first used, the waves (A's, B's), the builders, the routes whose
/// waves hurry and what each route's waves do with their downed.
pub struct FieldPlay {
    pub play: Play,
    pub kingdom: Vec<usize>,
    pub routes: Vec<(Route, Option<i64>)>,
    pub waves: Vec<Wave>,
    pub builders: Builders,
    pub hurried: Vec<usize>,
    pub tending: Vec<(usize, String)>,
}

impl FieldPlay {
    /// The field (its line `captained` or not) set up on `squads` and `field` from `content`,
    /// seeded `seed`, its blows rolled if `rolls`; None, with nothing written, if a
    /// definition it needs isn't there.
    pub fn new(
        captained: bool,
        (squads, field): (&mut Squads, &mut Field),
        content: &Content,
        (seed, rolls, threaded): (i64, bool, bool),
    ) -> Option<FieldPlay> {
        let (grem, chieftain) = (content.units.get(GREM)?, content.units.get(CHIEFTAIN)?);
        let battle = Battle::Field { captained };
        let play = Play::new(&battle, (squads, field), content, (seed, rolls, threaded))?;
        let line = play.listed.first().map(|&s| squads.route_code[s]);
        let mut routes: Vec<(Route, Option<i64>)> =
            routes().into_iter().map(|r| (r, None)).collect();
        routes[A].1 = line; // the kingdom holds on route A
        let foot = |unit: &crate::content::Unit| {
            (unit.footprint_depth as i32, unit.footprint_width as i32)
        };
        let a = Template::line(GREM, foot(grem), WAVE_WIDTH, WAVE_WIDTH);
        let mut b = a.clone();
        b.slot_limit += 1;
        b.paint(CHIEFTAIN, foot(chieftain), (1, WAVE_WIDTH / 2));
        let mut builders = Builders::default();
        builders.set(GREM, grem.build_seconds, BUILDERS);
        builders.set(CHIEFTAIN, chieftain.build_seconds, 1);
        builders.prefer(None); // round robin (Decision 51)
        let waves = vec![Wave::new(a, A), Wave::new(b, B)];
        let kingdom = play.listed.clone(); // the line, then the reserve
        let (hurried, tending) = (Vec::new(), Vec::new());
        Some(FieldPlay { play, kingdom, routes, waves, builders, hurried, tending })
    }

    /// FormationField.step: the player's squads hurried and tending as their routes' waves
    /// are, the builders, the waves (announcing, departing if automatic), the battle's tick
    /// (its events in `log`) and the player's routers home banked; what the builders and
    /// waves did, and the tick.
    pub fn step(
        &mut self,
        squads: &mut Squads,
        field: &mut Field,
        content: &Content,
        log: &mut Log,
    ) -> (Vec<Produced>, Option<Ticked>) {
        self.orders(squads, field);
        let tick = self.play.tick;
        let mut produced = Vec::new();
        let seconds = self.play.set.tick_seconds;
        self.builders.step(&mut self.waves, seconds, tick, &mut produced);
        for w in 0..self.waves.len() {
            let wave = &mut self.waves[w];
            if wave.full() && !wave.announced {
                wave.announced = true;
                let built = wave.built();
                produced.push(Produced::WaveFull { tick, built });
                if wave.auto {
                    self.send(w, 0, (squads, field), content);
                    produced.push(Produced::Departed { tick, units: built });
                }
            }
        }
        let from = log.mark();
        let ticked = self.play.step(squads, field, content, log);
        self.bank(field, log.since(from));
        (produced, ticked)
    }

    /// The hurry and the tending set on each of the player's squads by its route.
    fn orders(&self, squads: &mut Squads, field: &Field) {
        let Some(player) = field.lexicon.find("player") else { return };
        for &s in &self.play.listed {
            let Some(command) = squads.command[s].filter(|_| squads.faction[s] == player) else {
                continue;
            };
            let code = Some(squads.route_code[s]);
            let on = |key: usize| self.routes[self.waves[key].route].1 == code;
            let hurries = self.hurried.iter().any(|&key| on(key));
            squads.commands.give(command, HURRY, hurries);
            for (key, tends) in &self.tending {
                if on(*key) {
                    squads.commands.tends[command].clone_from(tends);
                }
            }
        }
    }

    /// The player's routers home this tick (in the flat log `flat`) back to the reserve.
    fn bank(&mut self, field: &Field, flat: &[i64]) {
        let Some(player) = field.lexicon.find("player") else { return };
        for at in play::starts(flat) {
            if flat[at] as usize == FLED_HOME && flat[at + 3] == i64::from(player) {
                let code = flat[at + 5] as usize;
                if let Some(&id) = self.play.codes.definitions.get(code) {
                    self.builders.bank(id);
                }
            }
        }
    }

    /// FormationProduction.send: wave `w`'s filled places spawned as one squad setting out
    /// after `wait_ticks`, closed up and centred if the wave isn't full, its units stood on
    /// their places, and the wave emptied; the squad's slot, or None if nothing is built.
    pub fn send(
        &mut self,
        w: usize,
        wait_ticks: i64,
        (squads, field): (&mut Squads, &mut Field),
        content: &Content,
    ) -> Option<usize> {
        let wave = &self.waves[w];
        if wave.built() == 0 {
            return None;
        }
        let full = wave.full();
        let (width, painted) = wave.template.layout();
        let codes = &mut self.play.codes;
        let placed: Vec<Placed> = painted
            .iter()
            .zip(&wave.filled)
            .filter(|(_, &filled)| filled)
            .map(|(p, _)| Placed {
                unit: &content.units[p.id],
                code: codes.definition(p.id),
                place: p.at,
            })
            .collect();
        let (route, code) = &self.routes[wave.route];
        let spawn = Spawn {
            width: i64::from(width),
            placed: &placed,
            faction: "player",
            at_player_end: true,
            wait_ticks,
            route,
            route_code: *code,
            combat_width: self.play.set.combat_width,
        };
        let s = spawn::squad(squads, field, content, &spawn);
        let route = wave.route;
        self.routes[route].1 = Some(squads.route_code[s]);
        if let Some(command) = squads.command[s] {
            squads.commands.give(command, MERGES, false); // the field's waves don't merge
        }
        squads.staging[s].clone_from(&self.waves[w].staging);
        if !full {
            close_up(squads, field, s);
        }
        centre(squads, field, s);
        spawn::stand(squads, field, content, s);
        let wave = &mut self.waves[w];
        wave.filled.fill(false);
        wave.announced = false;
        self.play.listed.push(s);
        Some(s)
    }

    /// FormationField.send_together: the waves `ws` sent now, each waiting so that all reach
    /// the line together (planned rendezvous, Decision 87), their marches timed by rehearsal
    /// (FormationRehearsal) on the field's ground; the squads sent.
    pub fn send_together(
        &mut self,
        ws: &[usize],
        (squads, field): (&mut Squads, &mut Field),
        content: &Content,
    ) -> Vec<usize> {
        let predicted: Vec<i64> = ws.iter().map(|&w| self.rehearsed(w, content)).collect();
        let waits = rendezvous::waits(&predicted);
        let sent = ws.iter().zip(waits);
        sent.filter_map(|(&w, wait)| self.send(w, wait, (squads, field), content)).collect()
    }

    /// FormationRehearsal.ticks_to over wave `w`'s preview: the ticks its filled places,
    /// eight wide on its route on the field's ground, take to bring their front to the line.
    pub(crate) fn rehearsed(&self, w: usize, content: &Content) -> i64 {
        let wave = &self.waves[w];
        let ordered = wave.template.ordered().into_iter().zip(&wave.filled);
        let painted: Vec<Painted> = ordered.filter(|(_, &filled)| filled).map(|(p, _)| p).collect();
        let route = &self.routes[wave.route].0;
        let march = March { painted: &painted, width: WAVE_WIDTH.into(), route, cells: CONTACT[w] };
        let set = &self.play.set;
        rehearsal::ticks_to(&march, (set.tick_seconds, set.map, self.play.threaded), content)
    }
}

/// A partial wave's unbuilt places are holes: the squad in `s` as wide as its front band,
/// closed up (SquadRanks.close), its swaps cleared.
fn close_up(squads: &mut Squads, field: &mut Field, s: usize) {
    let sheet = |u: &i32| &field.columns.sheet[*u as usize];
    let front = squads.members[s].iter().filter(|u| sheet(u).preferred_position == 0);
    let span: i64 = front.map(|u| sheet(u).footprint_width).sum();
    squads.width[s] = span.clamp(1, i64::from(squads.width[s].max(1))) as i32;
    let living = squads.living(s, &field.columns.state);
    let units: Vec<Ranked> = living
        .iter()
        .map(|&u| {
            let sheet = &field.columns.sheet[u as usize];
            Ranked {
                slot: u as usize,
                preferred: sheet.preferred_position,
                foot: (sheet.footprint_width as i32, sheet.footprint_depth as i32),
            }
        })
        .collect();
    ranks::close(&mut field.columns, &units, squads.width[s]);
    squads.swaps[s].clear();
}

/// FormationProduction._centre: the squad in `s` shifted so its units stand centred on its
/// route.
fn centre(squads: &mut Squads, field: &Field, s: usize) {
    let (mut low, mut high) = (f64::INFINITY, f64::NEG_INFINITY);
    for u in squads.living(s, &field.columns.state) {
        let column = f64::from(field.columns.column[u as usize]);
        low = low.min(column);
        high = high.max(column + field.columns.sheet[u as usize].footprint_width as f64);
    }
    if low < f64::INFINITY {
        squads.centre_shift[s] = f64::from(squads.width[s]) / 2.0 - (low + high) / 2.0;
    }
}

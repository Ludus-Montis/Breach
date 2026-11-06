//! tools/rout_bench.gd in the crate (spec 39 step 8c-xix), for `breach_rout_bench`: how long a
//! tick of routs takes with many friendly formations (spec 30 round 3). The player's side is
//! split into `squads` formations along one long lane, the front half at no morale (they break
//! on the first tick and rout back through the rear half, crushing, steering for refuge, caught,
//! rallying), a kingdom formation beyond them (`set_up`, rout_bench.gd's `_field`); then the
//! rout's pass alone (`rout::pass`, as FormationRout.step runs it, with its gate and a roster
//! of the living read afresh each tick) is timed over `ticks` ticks.

#[cfg(test)]
mod tests;

use crate::bands::BandTuning;
use crate::battles::Codes;
use crate::content::Content;
use crate::digest::state::state;
use crate::events::Log;
use crate::field::Field;
use crate::geometry::CELLS;
use crate::locks::{HOLDING, ROUTING};
use crate::march;
use crate::maths::V2;
use crate::play::{self, starts, TRAVEL_SCALE};
use crate::rout::pass::{self as rout, RoutShocks};
use crate::rout::RoutTuning;
use crate::route::Route;
use crate::spawn::{self, Placed, Spawn};
use crate::squads::Squads;
use crate::threads::Threads;
use crate::tick::{self, given, rules};

/// FormationSimulation.new's tick seconds, as rout_bench.gd makes it.
const SECONDS: f64 = 0.1;
/// FormationRout.step's `fight_seed`, as rout_bench.gd gives it.
const SEED: i64 = 1;

/// rout_bench.gd's arguments, with its defaults.
#[derive(Clone, Copy, Debug)]
pub struct Options {
    pub side: usize,
    pub squads: usize,
    pub width: usize,
    pub ticks: usize,
}

impl Default for Options {
    fn default() -> Options {
        Options { side: 1024, squads: 16, width: 16, ticks: 20 }
    }
}

/// The battle set up, on its own crate, with the rout's rules.
pub struct Routs {
    squads: Squads,
    field: Field,
    listed: Vec<usize>,
    log: Log,
    rules: (RoutTuning, RoutShocks, BandTuning),
}

impl Routs {
    /// FormationRout.step on `tick`: nothing if nothing routs or breaks, else the rout's pass
    /// over a roster of the listed squads' living. How many events it told of.
    pub fn step(&mut self, tick: i64) -> usize {
        let (squads, field) = (&mut self.squads, &mut self.field);
        let routs =
            self.listed.iter().any(|&s| squads.state[s] == ROUTING || squads.morale[s] == 0);
        if !routs {
            return 0; // nothing routs and nothing breaks
        }
        let every = play::listed(squads, field, &self.listed, false);
        let known = tick::known(&every);
        let roster = tick::relisted(every, squads, field, &known, &[], &[]);
        let (t, hurt, bands) = &self.rules;
        rout::step(squads, field, &roster, (t, hurt, bands), (tick, SEED), None, &mut self.log);
        starts(&self.log.take()).count()
    }

    /// How many of the listed squads are routing.
    pub fn routing(&self) -> usize {
        self.listed.iter().filter(|&&s| self.squads.state[s] == ROUTING).count()
    }

    /// The listed squads' state as the digest records it, as bytes.
    pub fn state(&self) -> Vec<u8> {
        state(&self.squads, &self.field, &self.listed).bytes()
    }
}

/// rout_bench.gd's `_field`: the player's `side` grems in `squads` formations `width` wide a
/// few cells apart along a lane long enough for them, holding, the front half at no morale;
/// `width * 2` militia beyond them; all stood on their places. None if a definition is
/// missing or the tuning gives no rules.
pub fn set_up(options: &Options, content: &Content) -> Option<Routs> {
    let (count, width) = (options.squads.max(1), options.width.max(1));
    let per = (options.side / count).max(1);
    let gap = per.div_ceil(width) as f64 + 4.0;
    let length = ((count as f64 * gap + 80.0) / CELLS).ceil();
    let route = Route::new(vec![V2::new(0.0, 0.0), V2::new((length * CELLS) as f32, 0.0)], 0.0);
    let (grem, militia) = (content.units.get("grem")?, content.units.get("kingdom_militia")?);
    let rules = rules_of(content, length)?;
    let (mut squads, mut field) = (Squads::default(), Field::default());
    let mut codes = Codes::default();
    let code = squads.issue_route();
    let mut spawned = |squads: &mut Squads, field: &mut Field, id, count, faction, home| {
        let unit = if id == "grem" { grem } else { militia };
        let code_of = codes.definition(id);
        let placed: Vec<Placed> = (0..count)
            .map(|k| Placed {
                unit,
                code: code_of,
                place: ((k / width) as i32, (k % width) as i32),
            })
            .collect();
        let spawn = Spawn {
            width: width as i64,
            placed: &placed,
            faction,
            at_player_end: home,
            wait_ticks: 0,
            route: &route,
            route_code: Some(code),
            combat_width: 0,
        };
        spawn::squad(squads, field, content, &spawn)
    };
    let mut listed = Vec::new();
    for index in 0..count {
        let s = spawned(&mut squads, &mut field, "grem", per, "player", true);
        place(&mut squads, &route, s, (20.0 + (index + 1) as f64 * gap) / CELLS);
        squads.state[s] = HOLDING;
        if index >= count / 2 {
            squads.morale[s] = 0;
        }
        listed.push(s);
    }
    let theirs =
        spawned(&mut squads, &mut field, "kingdom_militia", width * 2, "the_kingdom", false);
    place(&mut squads, &route, theirs, (20.0 + (count + 3) as f64 * gap) / CELLS);
    listed.push(theirs);
    let walk = given::walk(&content.tuning);
    let sync = march::sync::Given {
        ground: None,
        timing: None,
        seed: 0,
        wounds_reach: content.tuning.wounds_reach,
        tuning: rules::walk_tuning_of(&walk),
    };
    march::sync::step(&squads, &mut field, &listed, &sync); // FormationMarch.sync_units: placed
    Some(Routs { squads, field, listed, log: Log::default(), rules })
}

/// StoredSquad's front_distance setter: the front `front` tiles along, the squad placed there.
fn place(squads: &mut Squads, route: &Route, s: usize, front: f64) {
    squads.front_distance[s] = front;
    squads.position[s] = route.point_at(front * CELLS);
}

/// FormationRout.rules on rout_bench.gd's pace, with the bands: the mustering's rules as the
/// tick's are read (`tick::given`, then `tick::rules`).
fn rules_of(content: &Content, length: f64) -> Option<(RoutTuning, RoutShocks, BandTuning)> {
    let at = given::At {
        tick: 1,
        seed: SEED,
        ticks_a_second: 10,
        rolls: false,
        threaded: Threads::Pool,
    };
    let paces = given::Paces {
        travel_scale: TRAVEL_SCALE,
        seconds: SECONDS,
        route_length: length,
        cells_a_second: TRAVEL_SCALE * CELLS,
    };
    let (_, rules) = rules::of(&given::of(&content.tuning, &at, &paces))?;
    let m = rules.mustering;
    Some((m.rout, m.shocks, m.bands))
}

/// The line rout_bench.gd prints, `ms` a tick.
pub fn line(options: &Options, ms: f64, events: usize, routing: usize) -> String {
    format!(
        "rout: {} a side in {} formations, {} wide: {:.2} ms a tick ({} events, {} routing)",
        options.side, options.squads, options.width, ms, events, routing
    )
}

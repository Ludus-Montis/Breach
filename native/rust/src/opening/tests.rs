//! The opening as one pass, on a player squad (id 1) fighting a kingdom squad (id 2) front
//! to front, with a second kingdom squad (id 3) beside them, each of two grems: ordered to
//! retreat, the one pass leaves the squads, the units and the log as the passes one by one,
//! and keeps the routes' codes the crate holds as GDScript's mirrors would.

use super::*;
use crate::geometry::CELLS;
use crate::ground::Walker;
use crate::maths::V2;
use crate::moving::share::ShareTuning;
use crate::pathing::LookTuning;
use crate::roster::{Listed, Member};
use crate::route::Route;

const GREM: Walker = Walker { height: 1.0, swimmer: 0, climber: 0, swims: false, climbs: false };
const RETREAT: i32 = 2;

fn grem(id: i64, slot: usize) -> Member {
    Member {
        id,
        slot,
        foot: (1, 1),
        preferred: 0,
        attack_range: 0,
        kind: 0,
        turn_rate: 90.0,
        backward_pace: 0.5,
        detection: 8.0,
        max_stamina: 100.0,
        walker: GREM,
    }
}

fn rules() -> Rules<'static> {
    let share = ShareTuning { seed: 3, wounds_reach: 1.0, lost: 4.0, slack: [5.0, 1.0, 0.5] };
    Rules {
        tick: 5,
        seed: 3,
        within: 4.0,
        ragged_shock: 20,
        discipline: DisciplineTuning {
            per_leadership: 5,
            meets_threats: 50,
            expected_max: 100.0,
            leash_shares: vec![0.8, 0.5],
            leash_steps: vec![2.0, 4.0, 8.0, 16.0],
        },
        bands: BandTuning {
            per_leadership: 5,
            steady_at: 70,
            shaken_at: 40,
            loss: 2,
            leader_loss: 3,
        },
        impacts: Impacts { side: 10, rear: 20 },
        looks: Looks {
            relook: 4.0,
            look: LookTuning { look: 8.0, detour_gain: 1.5, sight_budget: 64.0 },
        },
        moving: Given {
            routes: &[],
            tick: 5,
            travel_scale: 1.0,
            seconds: 0.1,
            route_length: 64.0,
            share,
            drag: [0.5, 0.9],
        },
    }
}

/// The road along y = 0 that squads 2 and 3 march, and squad 1's own, which passes by
/// squad 2 a cell off it.
fn road() -> Route {
    Route::new(vec![V2::new(0.0, 0.0), V2::new(64.0, 0.0)], 0.0)
}

fn theirs() -> Route {
    Route::new(vec![V2::new(20.0, 1.0), V2::new(0.0, 1.0)], 0.0)
}

struct Built {
    squads: Squads,
    field: Field,
    roster: Roster,
    log: Log,
}

/// Squads 1 (player, front at x = 10, on its own route), 2 and 3 (kingdom, fronts at x = 11
/// and at x = 11 y = 6, on the road), 1 and 2 locked front to front, 2 under a command that
/// pursues; squad 1 ordered to retreat.
fn built() -> Built {
    let (mut squads, mut field, mut roster) =
        (Squads::default(), Field::default(), Roster::default());
    let fronts = [V2::new(10.0, 0.0), V2::new(11.0, 0.0), V2::new(11.0, 6.0)];
    for (s, (id, faction)) in [(1i64, 0u32), (2, 1), (3, 1)].into_iter().enumerate() {
        squads.open(id);
        squads.faction[s] = faction;
        squads.width[s] = 2;
        squads.position[s] = fronts[s];
        squads.front_distance[s] = fronts[s].x as f64 / CELLS;
        squads.direction[s] = if faction == 0 { 1 } else { -1 };
        squads.heading[s] = if faction == 0 { 90.0 } else { 270.0 };
        squads.route[s] = Some(if s == 0 { theirs() } else { road() });
        squads.route_code[s] = codes()[s];
        roster.listed.push(Listed { squad: s, id, faction, ..Listed::default() });
        for column in 0..2 {
            let unit = 100 * id + column as i64;
            let slot = field.hold(unit) as usize;
            field.columns.column[slot] = column;
            field.columns.speed[slot] = 1.0;
            field.columns.bearing[slot] = squads.heading[s];
            field.columns.position[slot] = fronts[s].add(V2::new(0.0, column as f32));
            squads.members[s].push(slot as i32);
            roster.listed[s].living.push(grem(unit, slot));
        }
    }
    for (s, foe) in [(0, 2), (1, 1)] {
        squads.engaged_with[s] = foe;
        squads.state[s] = locks::FIGHTING;
    }
    let command = squads.commands.open(0.0);
    squads.command[1] = Some(command);
    squads.pending = vec![(1, RETREAT)];
    Built { squads, field, roster, log: Log::default() }
}

/// The codes the squads' routes hold: squad 1's 7, the road 0.
fn codes() -> Vec<i64> {
    vec![7, 0, 0]
}

impl Built {
    /// Each pass, one call each, as FormationSimulation made them, GDScript's mirrors
    /// between: the pursuer takes its quarry's route (and code) after the orders.
    fn passes(&mut self, rules: &Rules) -> Vec<(usize, usize)> {
        let (squads, field, roster, log) =
            (&mut self.squads, &mut self.field, &self.roster, &mut self.log);
        let (tick, seed) = (rules.tick, rules.seed);
        let given = ordering::Given {
            routes: coded(squads, roster),
            within: rules.within,
            ragged_shock: rules.ragged_shock,
            discipline: &rules.discipline,
            bands: &rules.bands,
            tick,
            seed,
        };
        for a in ordering::pass(squads, &mut field.columns, roster, given, log) {
            for p in a.retreat.unwrap_or_default().into_iter().filter(|p| p.took) {
                squads.route[p.listed] = squads.route[a.listed].clone();
                squads.route_code[p.listed] = squads.route_code[a.listed];
            }
        }
        let seen = perception::see(squads, &field.columns, roster, None);
        perception::keep(squads, roster, &seen);
        contact::engage_fronts(squads, &mut field.columns, roster, (tick, seed), log);
        let shocks = (&rules.impacts, &rules.bands);
        contact::engage_edges(squads, &mut field.columns, roster, (tick, seed), shocks, log);
        let routes = routes(squads, roster);
        let given = Given { routes: &routes, ..rules.moving };
        let joiners = moving::step(squads, field, roster, None, &given, log);
        joins::step(squads, field, roster, &joiners, seed, tick, log)
    }

    fn whole(&mut self, rules: &Rules) -> Opened {
        let (squads, field, log) = (&mut self.squads, &mut self.field, &mut self.log);
        step(squads, field, &self.roster, None, rules, log)
    }

    fn places(&self) -> Vec<(V2, f64)> {
        let c = &self.field.columns;
        (0..c.position.len()).map(|n| (c.position[n], c.bearing[n])).collect()
    }
}

#[test]
fn the_one_pass_leaves_the_squads_their_units_and_the_log_as_the_passes_one_by_one() {
    let rules = rules();
    let (mut passes, mut whole) = (built(), built());
    let joins = passes.passes(&rules);
    let opened = whole.whole(&rules);
    let logs = (passes.log.take(), whole.log.take());
    assert!(!logs.0.is_empty(), "the retreat was told of");
    assert_eq!(logs.1, logs.0);
    assert_eq!(opened.joins, joins);
    assert_eq!(whole.squads, passes.squads);
    assert_eq!(whole.places(), passes.places());
}

#[test]
fn a_pursuer_that_took_its_quarrys_route_holds_it_and_its_code() {
    let rules = rules();
    let mut b = built();
    let (squads, field, log) = (&mut b.squads, &mut b.field, &mut b.log);
    let pursuers = order(squads, field, &b.roster, &rules, log);
    assert_eq!(pursuers, vec![[0, 1, 1, 1]], "squad 2 pursues and takes squad 1's route");
    assert_eq!(b.squads.route[1], Some(theirs()));
    assert_eq!(b.squads.route_code, vec![7, 7, 0]);
}

#[test]
fn what_each_squad_sees_is_handed_back() {
    let rules = rules();
    let mut b = built();
    let opened = b.whole(&rules);
    assert_eq!(opened.seen, vec![vec![1, 2], vec![0], vec![0]]);
    assert_eq!(opened.pursuers, vec![[0, 1, 1, 1]]);
}

#[test]
fn each_squad_s_threat_is_kept_as_it_saw() {
    let rules = rules();
    let mut b = built();
    b.whole(&rules);
    assert_eq!(b.squads.threatened, vec![true, true, true], "each saw a foe");
}

#[test]
fn the_move_reads_the_codes_the_routes_hold_now() {
    let mut b = built();
    b.squads.route_code[2] = 4;
    assert_eq!(routes(&b.squads, &b.roster), vec![7, 0, 4]);
    let pursuers = [[0, 2, 1, 1]];
    took(&mut b.squads, &b.roster, &pursuers);
    assert_eq!(routes(&b.squads, &b.roster), vec![7, 0, 7], "squad 3 took squad 1's");
}

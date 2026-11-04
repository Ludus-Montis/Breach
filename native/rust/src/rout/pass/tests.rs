//! The rout's landing (Decision 82, spec 27 round 3), from test_formation_rout.gd, on squads
//! built here: the player's routers fleeing towards home at 0 along their route (none: the
//! line y = 0) a cell a tick, and friends standing where they are put. A crush deals 2 a cell
//! of the router's footprint, panics by 5, and a break shocks each friend that sees it by 7.

use super::*;
use crate::entries::{Key, Value, LOOSE};
use crate::maths::V2;
use crate::roster::Listed;
use crate::rout::flight;
use crate::rout::tests::{east, member, SEED};
use crate::{det_math, route::Route, staging};

const TICK: i64 = 12;
const TUNING: RoutTuning = RoutTuning {
    pace: 1.0,
    tick_seconds: 0.1,
    fan_degrees: 45.0,
    fan_cells: 6.0,
    rally_reach: 1.5,
    steady_rally_seconds: 3.0,
    rally_seconds: 5.0,
    enemy_near: 6.0,
    reformed_morale: 30,
    steady_at: 50,
};
const HURT: RoutShocks = RoutShocks { crush: 2, panic: 5, seen: 7 };
const BANDS: BandTuning =
    BandTuning { per_leadership: 5, steady_at: 70, shaken_at: 40, loss: 2, leader_loss: 3 };

#[derive(Default)]
struct Built {
    squads: Squads,
    field: Field,
    roster: Roster,
}

impl Built {
    /// A player's formation in `state` (`locks`' values), its morale `morale`, on `route`.
    fn formation(&mut self, state: i32, morale: i32, route: Option<Route>) -> usize {
        let n = self.roster.listed.len();
        let squad = self.squads.open(n as i64 + 1) as usize;
        let player = self.field.lexicon.code("player");
        self.squads.faction[squad] = player;
        self.squads.state[squad] = state;
        self.squads.morale[squad] = morale;
        self.squads.route[squad] = route;
        let listed = Listed { squad, id: n as i64 + 1, faction: player, ..Listed::default() };
        self.roster.listed.push(listed);
        n
    }

    /// A unit of the listed squad `n` standing at `at`, at 10 hp.
    fn unit(&mut self, n: usize, id: i64, at: V2) -> usize {
        let slot = self.field.hold(id) as usize;
        let s = self.roster.listed[n].squad;
        let columns = &mut self.field.columns;
        columns.position[slot] = at;
        columns.speed[slot] = 1.0;
        columns.hp[slot] = 10;
        columns.sheet[slot].squad_id = self.squads.id[s];
        columns.words[slot].faction_id = self.squads.faction[s];
        self.roster.listed[n].living.push(member(id, slot));
        self.squads.members[s].push(slot as i32);
        slot
    }

    /// A router of `n`, `along` cells along its route and `offset` off it.
    fn router(&mut self, n: usize, id: i64, along: f64, offset: V2) -> usize {
        let slot = self.unit(n, id, V2::ZERO);
        let s = self.roster.listed[n].squad;
        entries::flee(&mut self.squads, &mut self.field.columns, s, (id, slot), along, offset);
        slot
    }

    /// The pass: the routers that left and what it logged.
    fn rout(&mut self) -> (Vec<Moved>, Vec<i64>) {
        let mut log = Log::default();
        let (squads, field) = (&mut self.squads, &mut self.field);
        let tuning = (&TUNING, &HURT, &BANDS);
        let moved = step(squads, field, &self.roster, tuning, (TICK, SEED), None, &mut log);
        (moved, log.take())
    }

    fn event(&self, kind: usize, n: usize, values: &[i64]) -> Vec<i64> {
        let s = self.roster.listed[n].squad;
        let head = [kind as i64, TICK, self.squads.id[s], i64::from(self.squads.faction[s])];
        [&head[..], values].concat()
    }

    fn squad(&self, n: usize) -> usize {
        self.roster.listed[n].squad
    }
}

#[test]
fn a_crush_hurts_both_by_the_routers_size_and_panics_the_friend() {
    let mut b = Built::default();
    let mine = b.formation(locks::ROUTING, 0, None);
    let router = b.router(mine, 1, 10.0, V2::ZERO);
    let friend = b.formation(locks::HOLDING, 50, None);
    // the router's step: back along y = 0 and out by its own angle
    let aside = det_math::sin(flight::fan(1, SEED, 45.0));
    let landing = V2::new((10.0 - (1.0 - aside * aside).sqrt()) as f32, -aside as f32);
    let hit = b.unit(friend, 3, landing.add(V2::new(0.2, 0.0)));
    let (moved, log) = b.rout();
    assert_eq!(moved, vec![]);
    assert_eq!(log, b.event(events::CRUSHED, friend, &[1, 3, 2]));
    let columns = &b.field.columns;
    assert_eq!((columns.hp[router], columns.hp[hit]), (8, 8));
    assert_eq!(b.squads.morale[b.squad(friend)], 45);
}

#[test]
fn a_router_home_leaves_its_squad_and_a_squad_left_with_none_scatters() {
    let mut b = Built::default();
    let mine = b.formation(locks::ROUTING, 0, None);
    let router = b.router(mine, 1, 0.5, V2::ZERO);
    b.field.columns.objects[router].definition = 4;
    let (moved, log) = b.rout();
    assert_eq!(moved, vec![Moved { listed: mine, id: 1, into: None }]);
    let home = b.event(events::FLED_HOME, mine, &[1, 4]);
    assert_eq!(log, [home, b.event(events::SCATTERED, mine, &[])].concat());
    assert!(b.squads.members[b.squad(mine)].is_empty());
    assert_eq!(b.squads.state[b.squad(mine)], locks::DESTROYED);
}

#[test]
fn a_router_near_a_led_friend_joins_its_rear_loose_where_it_stood() {
    let mut b = Built::default();
    let mine = b.formation(locks::ROUTING, 0, east(100.0));
    let router = b.router(mine, 1, 20.0, V2::ZERO);
    // held where it stands, so it doesn't flee this tick
    entries::keep(&mut b.field.columns, router, Key::Caught, Value::Int(1));
    let led = b.formation(locks::HOLDING, 20, east(100.0));
    let leader = b.unit(led, 3, V2::new(21.0, 0.0));
    b.field.columns.sheet[leader].leadership = 1;
    b.unit(led, 4, V2::new(31.0, 0.0));
    let (moved, log) = b.rout();
    assert_eq!(moved, vec![Moved { listed: mine, id: 1, into: Some(led) }]);
    assert_eq!(log, b.event(events::RALLIED, mine, &[1, 2]));
    let s = b.squad(led);
    assert_eq!(b.squads.members[s].last(), Some(&(router as i32)));
    assert_eq!(b.squads.joined[s], vec![1]);
    assert_eq!(b.squads.reforming[s], 1);
    assert!(b.squads.loose[s].contains(&1));
    assert_ne!(b.field.columns.flags[router] & LOOSE, 0);
    assert_eq!(b.field.columns.sheet[router].squad_id, 2);
    assert!(b.squads.fleeing[b.squad(mine)].is_empty());
    assert_eq!(b.squads.state[b.squad(mine)], locks::DESTROYED);
}

#[test]
fn a_break_routs_drops_its_staging_and_shocks_the_friends_that_see_it() {
    let mut b = Built::default();
    let breaking = b.formation(locks::HOLDING, 0, None);
    b.unit(breaking, 1, V2::ZERO);
    let stage = staging::Stage {
        at: 0.0,
        sees_fight: false,
        partner: 0,
        fallback: 0,
        then: None,
        meet: None,
    };
    let s = b.squad(breaking);
    b.squads.staging[s] = Some(stage);
    let friend = b.formation(locks::HOLDING, 50, None);
    b.unit(friend, 2, V2::new(3.0, 0.0));
    let far = b.formation(locks::HOLDING, 50, None);
    b.unit(far, 3, V2::new(300.0, 0.0));
    let (moved, log) = b.rout();
    assert_eq!(moved, vec![]);
    assert_eq!(log, b.event(events::ROUTED, breaking, &[]));
    assert_eq!(b.squads.state[b.squad(breaking)], locks::ROUTING);
    assert_eq!(b.squads.staging[b.squad(breaking)], None);
    let morale = |n: usize| b.squads.morale[b.squad(n)];
    assert_eq!((morale(friend), morale(far)), (43, 50));
}

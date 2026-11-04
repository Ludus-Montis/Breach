//! The rout's scenarios (Decision 82, spec 27 round 3), on formations built here: the
//! player's (faction 0) routers fleeing towards home at 0 along their route (none: the line
//! y = 0), and the formations standing where they are put. A tick is a tenth of a second and
//! a router moves a cell a tick at full pace.

use super::*;
use crate::content::tests::repo_content;
use crate::content::Content;
use crate::det_math;
use crate::ground::{GroundTuning, Paint, Walker};
use crate::roster::{Listed, Member};
use crate::route::Route;

pub(crate) const SEED: i64 = 7;
const GREM: Walker = Walker { height: 1.0, swimmer: 0, climber: 0, swims: false, climbs: false };
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

pub(crate) fn member(id: i64, slot: usize) -> Member {
    Member {
        id,
        slot,
        foot: (1, 1),
        preferred: 0,
        attack_range: 0,
        kind: 0,
        turn_rate: 360.0,
        backward_pace: 0.5,
        detection: 8.0,
        max_stamina: 100.0,
        walker: GREM,
    }
}

fn v(x: f32, y: f32) -> V2 {
    V2::new(x, y)
}

#[derive(Default)]
struct Built {
    squads: Squads,
    columns: Columns,
    roster: Roster,
    routing: Vec<Routing>,
    /// Each listed squad's units' leadership, by member.
    leaders: Vec<Vec<i64>>,
}

impl Built {
    /// A formation of `faction` in `state` (`locks`' values), its morale `morale`.
    fn formation(&mut self, faction: u32, state: i32, morale: i32) -> usize {
        let n = self.roster.listed.len();
        let squad = self.squads.open(n as i64 + 1) as usize;
        self.squads.state[squad] = state;
        self.squads.morale[squad] = morale;
        self.roster.listed.push(Listed { squad, id: n as i64 + 1, faction, ..Listed::default() });
        self.leaders.push(Vec::new());
        n
    }

    /// A unit of the listed squad `n` standing at `at`, with this leadership.
    fn unit(&mut self, n: usize, id: i64, at: V2, leadership: i64) -> usize {
        let slot = self.columns.position.len();
        self.columns.grow();
        self.columns.position[slot] = at;
        self.columns.speed[slot] = 1.0;
        self.roster.listed[n].living.push(member(id, slot));
        self.squads.members[self.roster.listed[n].squad].push(slot as i32);
        self.leaders[n].push(leadership);
        slot
    }

    /// A routing formation of `faction` on `route` (None: none), home `home` tiles along.
    fn routing(&mut self, faction: u32, route: Option<Route>, home: f64) -> usize {
        let n = self.formation(faction, locks::ROUTING, 0);
        self.squads.home_distance[self.roster.listed[n].squad] = home;
        self.routing.push(Routing { listed: n, route });
        n
    }

    /// A router of `n`, `along` cells along its route and `offset` off it.
    fn router(&mut self, n: usize, id: i64, along: f64, offset: V2, leadership: i64) -> usize {
        let slot = self.unit(n, id, V2::ZERO, leadership);
        let squad = self.roster.listed[n].squad;
        entries::flee(&mut self.squads, &mut self.columns, squad, (id, slot), along, offset);
        slot
    }

    fn leaders(&self) -> Vec<i64> {
        self.leaders.iter().flatten().copied().collect()
    }

    fn flee(&mut self, ground: Option<&Ground>) -> Vec<Fled> {
        let leaders = self.leaders();
        let (squads, columns) = (&mut self.squads, &mut self.columns);
        flee(squads, columns, &self.roster, &self.routing, &leaders, &TUNING, SEED, ground)
    }

    /// The rallies, every squad's morale already read.
    fn rallies(&mut self) -> Rallied {
        let leaders = self.leaders();
        let morale: Vec<(i64, i64)> = (0..self.roster.listed.len())
            .map(|n| (self.squads.morale[self.roster.listed[n].squad] as i64, 0))
            .collect();
        let (squads, columns) = (&self.squads, &mut self.columns);
        rallies(squads, columns, &self.roster, &self.routing, &leaders, &morale, &TUNING, SEED)
    }

    fn regroup(&mut self) -> Vec<usize> {
        let leaders = self.leaders();
        let slots: Vec<(i64, usize)> = self
            .roster
            .listed
            .iter()
            .flat_map(|l| l.living.iter().map(|m| (m.id, m.slot)))
            .collect();
        let slot_of = |id: i64| slots.iter().find(|s| s.0 == id).map(|s| s.1);
        let (squads, columns) = (&mut self.squads, &mut self.columns);
        regroup(squads, columns, &self.roster, &self.routing, &leaders, &TUNING, SEED, slot_of)
    }

    fn squad(&self, n: usize) -> usize {
        self.roster.listed[n].squad
    }
}

pub(crate) fn east(length: f32) -> Option<Route> {
    Some(Route::new(vec![V2::ZERO, v(length, 0.0)], 0.0))
}

// Breaking.

#[test]
fn a_formation_at_no_morale_breaks_and_only_it() {
    let mut b = Built::default();
    let broken = b.formation(0, locks::FIGHTING, 0);
    b.unit(broken, 1, V2::ZERO, 0);
    let steady = b.formation(0, locks::MOVING, 40);
    b.unit(steady, 2, V2::ZERO, 0);
    let empty = b.formation(0, locks::MOVING, 0);
    let arrived = b.formation(0, locks::ARRIVED, 0);
    b.unit(arrived, 3, V2::ZERO, 0);
    let routing = b.formation(0, locks::ROUTING, 0);
    b.unit(routing, 4, V2::ZERO, 0);
    let unread = b.formation(0, locks::HOLDING, -1);
    b.unit(unread, 5, V2::ZERO, 0);
    let _ = empty;
    assert_eq!(breaking(&b.squads, &b.roster), vec![broken]);
}

#[test]
fn a_break_ends_its_fight_and_its_units_flee_from_where_they_stand() {
    let mut b = Built::default();
    let mine = b.formation(0, locks::FIGHTING, 0);
    let s = b.squad(mine);
    b.squads.front_distance[s] = 0.5;
    b.unit(mine, 1, v(30.0, 2.0), 0);
    b.unit(mine, 2, v(31.0, -1.0), 0);
    let theirs = b.formation(1, locks::FIGHTING, 50);
    b.unit(theirs, 3, v(33.0, 0.0), 0);
    let t = b.squad(theirs);
    b.squads.engaged_with[s] = 2;
    b.squads.engaged_with[t] = 1;
    b.squads.flank(s, 1, 2, 0);
    let breaking = [Routing { listed: mine, route: east(100.0) }];
    breaks(&mut b.squads, &mut b.columns, &b.roster, &breaking, None);
    assert_eq!(b.squads.state[s], locks::ROUTING);
    assert_eq!((b.squads.engaged_with[s], b.squads.engaged_with[t]), (0, 0));
    assert_eq!(b.squads.state[t], locks::MOVING);
    assert!(b.squads.flanks[s].is_empty());
    assert_eq!(b.squads.fleeing[s], vec![1, 2]);
    let slot = b.roster.listed[mine].living[1].slot;
    assert_eq!(b.columns.along[slot], 32.0);
    assert_eq!(b.columns.flee_offset[slot], v(-1.0, -1.0));
}

#[test]
fn friends_that_see_a_break_are_named_and_no_others() {
    let mut b = Built::default();
    let mine = b.formation(0, locks::MOVING, 0);
    b.unit(mine, 1, v(10.0, 0.0), 0);
    let near = b.formation(0, locks::MOVING, 50);
    b.unit(near, 2, v(14.0, 0.0), 0);
    let far = b.formation(0, locks::MOVING, 50);
    b.unit(far, 3, v(40.0, 0.0), 0);
    let foe = b.formation(1, locks::MOVING, 50);
    b.unit(foe, 4, v(12.0, 0.0), 0);
    let _gone = b.formation(0, locks::DESTROYED, 50);
    let breaking = [Routing { listed: mine, route: None }];
    let seen = breaks(&mut b.squads, &mut b.columns, &b.roster, &breaking, None);
    assert_eq!(seen, vec![vec![near]]);
}

// The flight.

#[test]
fn a_lone_router_flees_home_at_full_pace_fanning_out_by_its_own_angle() {
    let mut b = Built::default();
    let mine = b.routing(0, east(256.0), 0.0);
    let slot = b.router(mine, 1, 100.0, V2::ZERO, 0);
    assert!(b.flee(None).is_empty());
    let aside = det_math::sin(flight::fan(1, SEED, 45.0));
    let ahead = (1.0 - aside * aside).sqrt();
    assert_eq!(b.columns.along[slot], 100.0 - ahead);
    assert_eq!(b.columns.fanned[slot], aside);
    assert_eq!(b.columns.flee_offset[slot], v(0.0, -1.0).scale(aside));
}

#[test]
fn a_router_fanned_out_as_far_as_it_may_keeps_to_its_line() {
    let mut b = Built::default();
    let mine = b.routing(0, east(256.0), 0.0);
    let slot = b.router(mine, 1, 100.0, V2::ZERO, 0);
    let aside = det_math::sin(flight::fan(1, SEED, 45.0));
    b.columns.fanned[slot] = 6.0 * aside.signum();
    b.flee(None);
    assert_eq!(b.columns.along[slot], 99.0);
    assert_eq!(b.columns.flee_offset[slot], V2::ZERO);
}

#[test]
fn a_router_in_a_wood_flees_at_half_pace() {
    let content = Content::load(&repo_content()).unwrap_or_else(|error| panic!("{error}"));
    let mut wood = Ground::new(256, 32, GroundTuning::of(&content.tuning));
    wood.paint((0, 0, 256, 32), &Paint { cost: Some(0.5), ..Paint::default() });
    let mut fled = [0.0; 2];
    for (i, ground) in [None, Some(&wood)].into_iter().enumerate() {
        let mut b = Built::default();
        let route = Some(Route::new(vec![v(0.0, 16.0), v(256.0, 16.0)], 0.0));
        let mine = b.routing(0, route, 0.0);
        let slot = b.router(mine, 1, 100.0, V2::ZERO, 0);
        b.flee(ground);
        fled[i] = 100.0 - b.columns.along[slot];
    }
    assert!(fled[0] > 0.1);
    assert!((fled[1] / fled[0] - 0.5).abs() < 0.01);
}

#[test]
fn a_router_makes_for_a_standing_friend_between_it_and_home() {
    let mut b = Built::default();
    let mine = b.routing(0, east(256.0), 0.0);
    let slot = b.router(mine, 1, 100.0, V2::ZERO, 0);
    let friend = b.formation(0, locks::HOLDING, 50);
    b.unit(friend, 2, v(90.0, 3.0), 0);
    b.unit(friend, 3, v(90.0, 4.0), 0);
    assert!(b.flee(None).is_empty());
    // it turns aside towards the friend's ranks, 2.5 cells across at least, by at most the
    // sine of the fan's angle
    let most = det_math::sin(flight::deg_to_rad(45.0));
    assert_eq!(b.columns.flee_offset[slot], v(0.0, most as f32));
    assert_eq!(b.columns.fanned[slot], -most);
}

#[test]
fn a_held_router_stands() {
    let mut b = Built::default();
    let mine = b.routing(0, east(256.0), 0.0);
    let slot = b.router(mine, 1, 100.0, V2::ZERO, 0);
    b.columns.caught[slot] = 2;
    let friend = b.formation(0, locks::HOLDING, 50);
    b.unit(friend, 2, v(100.5, 0.0), 0);
    assert!(b.flee(None).is_empty());
    assert_eq!(b.columns.along[slot], 100.0);
}

#[test]
fn a_router_crushes_the_nearest_friend_it_runs_into_and_panics_it_once() {
    let mut b = Built::default();
    let mine = b.routing(0, None, 0.0);
    b.router(mine, 1, 10.0, V2::ZERO, 0);
    b.router(mine, 2, 30.0, V2::ZERO, 0);
    let friend = b.formation(0, locks::HOLDING, 50);
    // each router's step: back along y = 0 and out by its own angle
    let landing = |id: i64, along: f64| {
        let aside = det_math::sin(flight::fan(id, SEED, 45.0));
        v((along - (1.0 - aside * aside).sqrt()) as f32, -aside as f32)
    };
    b.unit(friend, 3, landing(1, 10.0).add(v(0.2, 0.0)), 0);
    b.unit(friend, 4, landing(2, 30.0).add(v(0.0, 0.3)), 0);
    b.unit(friend, 5, landing(2, 30.0).add(v(0.0, 2.0)), 0);
    let fled = b.flee(None);
    assert_eq!(
        fled,
        vec![
            Fled::Crushed { routing: 0, member: 0, hit: (friend, 0), panic: true },
            Fled::Crushed { routing: 0, member: 1, hit: (friend, 1), panic: false },
        ]
    );
}

#[test]
fn no_router_crushes_a_foe_or_a_routing_friend() {
    let mut b = Built::default();
    let mine = b.routing(0, None, 0.0);
    b.router(mine, 1, 10.0, V2::ZERO, 0);
    let other = b.routing(0, None, 0.0);
    let theirs = b.router(other, 2, 30.0, V2::ZERO, 0);
    b.columns.position[theirs] = v(9.3, 0.0);
    let far = b.formation(0, locks::HOLDING, 50);
    b.unit(far, 5, v(200.0, 0.0), 0);
    let foe = b.formation(1, locks::HOLDING, 50);
    b.unit(foe, 3, v(9.2, 0.0), 0);
    b.unit(foe, 4, v(9.0, 0.5), 0);
    let crushes = b.flee(None).iter().filter(|f| matches!(f, Fled::Crushed { .. })).count();
    assert_eq!(crushes, 0);
}

#[test]
fn routers_reaching_home_leave_and_a_formation_left_with_none_scatters() {
    let mut b = Built::default();
    let mine = b.routing(0, None, 0.0);
    b.router(mine, 1, 0.5, V2::ZERO, 0);
    b.router(mine, 2, 0.2, V2::ZERO, 0);
    let held = b.routing(0, None, 0.0);
    b.router(held, 3, 0.5, V2::ZERO, 0);
    let stays = b.router(held, 4, 50.0, V2::ZERO, 0);
    let _ = stays;
    let fled = b.flee(None);
    assert_eq!(
        fled,
        vec![
            Fled::Home { routing: 0, member: 0 },
            Fled::Home { routing: 0, member: 1 },
            Fled::Scattered { routing: 0 },
            Fled::Home { routing: 1, member: 0 },
        ]
    );
    assert!(b.squads.fleeing[b.squad(mine)].is_empty());
    assert_eq!(b.squads.state[b.squad(mine)], locks::DESTROYED);
    assert_eq!(b.squads.fleeing[b.squad(held)], vec![4]);
    assert_eq!(b.squads.state[b.squad(held)], locks::ROUTING);
}

// The rallies.

#[test]
fn a_router_near_a_led_friend_it_has_got_behind_joins_it() {
    let mut b = Built::default();
    let mine = b.routing(0, east(100.0), 0.0);
    b.router(mine, 1, 20.0, V2::ZERO, 0);
    b.router(mine, 2, 32.0, V2::ZERO, 0);
    let led = b.formation(0, locks::HOLDING, 20);
    b.unit(led, 3, v(21.0, 0.0), 1);
    b.unit(led, 4, v(31.0, 0.0), 0);
    let rallied = b.rallies();
    // the first has got behind the friend's unit at 21 (between it and home), the second
    // is short of the one at 31
    assert_eq!(rallied.joins, vec![(0, 0, led)]);
}

#[test]
fn a_router_held_by_a_steady_friend_counts_its_time_and_then_joins() {
    let mut b = Built::default();
    let mine = b.routing(0, east(100.0), 0.0);
    let slot = b.router(mine, 1, 20.0, V2::ZERO, 0);
    let friend = b.formation(0, locks::HOLDING, 60);
    b.unit(friend, 2, v(20.8, 0.0), 0);
    let rallied = b.rallies();
    assert!(rallied.joins.is_empty());
    assert_eq!(b.columns.caught[slot], 1);
    assert_eq!(b.columns.held_by[slot], 2);
    assert!(entries::holds_value(&b.columns, slot, Key::HeldBy));
    b.columns.caught[slot] = 29;
    assert_eq!(b.rallies().joins, vec![(0, 0, friend)]);
    assert_eq!(b.columns.caught[slot], 30);
}

#[test]
fn a_shaken_friend_holds_its_routers_without_counting() {
    let mut b = Built::default();
    let mine = b.routing(0, east(100.0), 0.0);
    let slot = b.router(mine, 1, 20.0, V2::ZERO, 0);
    let friend = b.formation(0, locks::HOLDING, 30);
    b.unit(friend, 2, v(20.8, 0.0), 0);
    b.rallies();
    assert_eq!(b.columns.caught[slot], 1);
    b.columns.caught[slot] = 40;
    assert!(b.rallies().joins.is_empty());
    assert_eq!(b.columns.caught[slot], 40);
}

#[test]
fn a_held_router_stays_with_its_friend_wherever_it_steps_while_that_friend_stands() {
    let mut b = Built::default();
    let mine = b.routing(0, east(100.0), 0.0);
    let slot = b.router(mine, 1, 20.0, V2::ZERO, 0);
    let first = b.formation(0, locks::HOLDING, 60);
    b.unit(first, 2, v(60.0, 0.0), 0);
    let near = b.formation(0, locks::HOLDING, 60);
    b.unit(near, 3, v(20.5, 0.0), 0);
    entries::keep(&mut b.columns, slot, Key::HeldBy, Value::Int(2));
    b.rallies();
    assert_eq!(b.columns.held_by[slot], 2);
    let f = b.squad(first);
    b.squads.state[f] = locks::ROUTING;
    b.rallies();
    assert_eq!(b.columns.held_by[slot], 3);
}

#[test]
fn a_band_first_read_is_named_and_counted_on_the_ceiling() {
    let mut b = Built::default();
    let mine = b.routing(0, east(100.0), 0.0);
    let slot = b.router(mine, 1, 20.0, V2::ZERO, 0);
    let friend = b.formation(0, locks::HOLDING, -1);
    b.unit(friend, 2, v(20.8, 0.0), 0);
    let leaders = b.leaders();
    let morale = vec![(0, 0), (-1, 80)];
    let rallied =
        rallies(&b.squads, &mut b.columns, &b.roster, &b.routing, &leaders, &morale, &TUNING, SEED);
    assert_eq!(rallied.first_read, vec![friend]);
    assert_eq!(b.columns.caught[slot], 1);
}

#[test]
fn routers_join_nearest_first_whatever_the_lists_order() {
    let mut joins = Vec::new();
    for reversed in [false, true] {
        let mut b = Built::default();
        let order: Vec<(i64, f64)> = vec![(1, 20.2), (2, 20.6), (3, 20.6), (4, 20.4)];
        let order: Vec<(i64, f64)> =
            if reversed { order.into_iter().rev().collect() } else { order };
        let mine = b.routing(0, east(100.0), 0.0);
        for (id, along) in order {
            b.router(mine, id, along, V2::ZERO, 0);
        }
        let led = b.formation(0, locks::HOLDING, 60);
        b.unit(led, 9, v(21.0, 0.0), 1);
        let rallied = b.rallies();
        let living = &b.roster.listed[mine].living;
        joins.push(rallied.joins.iter().map(|j| living[j.1].id).collect::<Vec<_>>());
    }
    assert_eq!(joins[0], joins[1]);
    // 2 and 3 equally near, by their draws, then 4 and 1
    assert_eq!(joins[0][2..], [4, 1]);
}

// Re-forming.

#[test]
fn a_led_rout_with_no_enemy_near_re_forms_round_its_best_leader_and_holds() {
    let mut b = Built::default();
    let route = Some(Route::new(vec![V2::ZERO, v(30.0, 40.0)], 0.0));
    let mine = b.routing(0, route, 0.0);
    b.router(mine, 1, 10.0, V2::ZERO, 0);
    let leader = b.router(mine, 2, 20.0, V2::ZERO, 2);
    let s = b.squad(mine);
    b.squads.rally_ticks[s] = 49;
    assert_eq!(b.regroup(), vec![0]);
    assert_eq!(b.squads.front_distance[s], b.columns.along[leader] / CELLS);
    assert_eq!(b.squads.state[s], locks::HOLDING);
    assert_eq!(b.squads.order[s], HOLD);
    assert_eq!(b.squads.morale[s], 30);
    assert_eq!(b.squads.reforming[s], 1);
    assert_eq!(b.squads.direction[s], 1);
    assert!(b.squads.fleeing[s].is_empty());
    let facing = (30.0f64).atan2(-40.0).to_degrees();
    assert!((b.squads.heading[s] - facing).abs() < 0.01);
}

#[test]
fn an_enemy_near_keeps_a_rout_from_re_forming_and_a_leaderless_one_never_does() {
    let mut b = Built::default();
    let mine = b.routing(0, None, 0.0);
    b.router(mine, 1, 10.0, V2::ZERO, 1);
    let leaderless = b.routing(0, None, 0.0);
    b.router(leaderless, 2, 50.0, V2::ZERO, 0);
    let foe = b.formation(1, locks::MOVING, 50);
    b.unit(foe, 3, v(15.0, 0.0), 0);
    let (s, l) = (b.squad(mine), b.squad(leaderless));
    b.squads.rally_ticks[s] = 49;
    b.squads.rally_ticks[l] = 49;
    assert!(b.regroup().is_empty());
    assert_eq!((b.squads.rally_ticks[s], b.squads.rally_ticks[l]), (0, 49));
    b.columns.position[b.roster.listed[foe].living[0].slot] = v(17.0, 0.0);
    b.squads.rally_ticks[s] = 48;
    assert!(b.regroup().is_empty());
    assert_eq!(b.squads.rally_ticks[s], 49);
    assert_eq!(b.regroup(), vec![0]);
}

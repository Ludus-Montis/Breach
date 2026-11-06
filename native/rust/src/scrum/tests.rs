//! The scrum's seek (spec 39 step 4a), from the gdUnit scenarios it replaces
//! (test_scrum_slots, test_formation_mirror, test_body_field's list order): a phase run alone
//! on a field the builders make.

use std::collections::BTreeMap;

use super::*;
use crate::build::{reversed, scrum, Fighter, Side};
use crate::seeking::{self, SquadSay};
use crate::threads::Threads;

const SEED: i64 = 3;
/// BattleTuning's reach_contact and scrum_leash in the scenarios.
const REACH: f64 = 0.25;
const LEASH: f64 = 12.0;

fn v(x: f32, y: f32) -> V2 {
    V2::new(x, y)
}

fn params(leash: f64) -> Params {
    Params { tick: 7, seed: SEED, die: 0, reach: REACH, leash, threaded: Threads::One }
}

/// A fighting squad as the rules say of it: its id, its foes, whether it may seek, its
/// frame (heading, width) and its frame's anchor.
struct Fights {
    id: i64,
    foes: Vec<i64>,
    seeking: bool,
    heading: f64,
    anchor: V2,
}

impl Fights {
    fn against(id: i64, foes: &[i64], heading: f64, anchor: V2) -> Fights {
        Fights { id, foes: foes.to_vec(), seeking: true, heading, anchor }
    }
}

/// Unit id -> [what it does, its goal slot, its next point], after one seek of the
/// fighting squads (`fights`, in that order) on `field`, every unit at speed 1.
fn seek(field: &mut Field, fights: &[Fights], leash: f64) -> BTreeMap<i64, (u8, i32, V2)> {
    seek_on(field, fights, Params { leash, ..params(leash) })
}

/// `seek`, with `params` (how threaded, say).
fn seek_on(field: &mut Field, fights: &[Fights], params: Params) -> BTreeMap<i64, (u8, i32, V2)> {
    let ids: Vec<i64> = fights.iter().map(|f| f.id).collect();
    let foe_counts: Vec<i32> = fights.iter().map(|f| f.foes.len() as i32).collect();
    let foe_ids: Vec<i64> = fights.iter().flat_map(|f| f.foes.clone()).collect();
    let seeking: Vec<u8> = fights.iter().map(|f| f.seeking as u8).collect();
    let frames: Vec<f64> = fights.iter().flat_map(|f| [f.heading, 1.0, 0.0]).collect();
    let anchors: Vec<V2> = fights.iter().map(|f| f.anchor).collect();
    let speeds = vec![1.0; field.flat.len()];
    let say = SquadSay {
        ids: &ids,
        foe_counts: &foe_counts,
        foe_ids: &foe_ids,
        seeking: &seeking,
        frames: &frames,
        anchors: &anchors,
        chaser_counts: &vec![0; fights.len()],
        chaser_ids: &[],
        speeds: &speeds,
    };
    let open = |_: i32, _: V2| true;
    let (gathered, out, _) = seeking::seek(field, &say, &params, &open, [1, 2]);
    let mut by_id = BTreeMap::new();
    for (m, &flat) in gathered.members.iter().enumerate() {
        let id = field.unit_id[field.flat[flat as usize] as usize];
        by_id.insert(id, (out.codes[m], out.goal_slots[m], out.nexts[m]));
    }
    by_id
}

fn near(a: V2, b: V2) -> bool {
    (a.distance_to(b) as f64) < 0.0001
}

/// A lone kingdom grem (id 1, squad 9) at (10, 10) facing east, and the player's seeker
/// (id 5, squad 1) east of it at (13, 10) facing it, its place where it stands.
fn duel() -> Vec<Side> {
    vec![
        Side {
            id: 9,
            faction: "kingdom",
            flags: 0,
            fighters: vec![Fighter::grem(1, v(10.0, 10.0), 90.0)],
        },
        Side {
            id: 1,
            faction: "player",
            flags: 0,
            fighters: vec![Fighter::grem(5, v(13.0, 10.0), 270.0)],
        },
    ]
}

/// The seeker's squad fighting the foe's, its frame putting its place where it stands.
fn seeker_fights() -> Fights {
    // heading west: its place is half a cell behind the anchor, so the anchor is ahead
    Fights::against(1, &[9], 270.0, v(12.5, 10.0))
}

#[test]
fn six_grems_fit_round_a_grem_and_nine_round_a_brute() {
    assert_eq!(slot_count(0.5 + 0.5, 0.5), 6);
    assert_eq!(slot_count(1.0 + 0.5, 0.5), 9);
}

#[test]
fn a_seeker_takes_the_nearest_open_slot_laid_out_from_the_foe_s_front() {
    let mut field = scrum(&duel(), SEED);
    let got = seek(&mut field, &[seeker_fights()], LEASH)[&5];
    assert_eq!((got.0, got.1), (AIM, 0)); // slot 0: the foe's front, facing east
    assert!(near(got.2, v(11.0, 10.0)));
    let c = &field.columns;
    let b = field.slot_of(5).unwrap() as usize;
    assert_eq!((c.goal_foe[b], c.goal_slot[b], c.next[b]), (1, 0, got.2)); // in the store
}

#[test]
fn a_slot_under_a_body_is_not_open() {
    let mut sides = duel();
    let stander = Fighter::grem(7, v(11.4, 10.0), 0.0); // on slot 0, in a squad not fighting
    sides.push(Side { id: 2, faction: "player", flags: 0, fighters: vec![stander] });
    let mut field = scrum(&sides, SEED);
    let got = seek(&mut field, &[seeker_fights()], LEASH)[&5];
    assert_eq!(got.0, AIM);
    assert!(!near(got.2, v(11.0, 10.0)));
    assert!((got.2.distance_to(v(10.0, 10.0)) - 1.0).abs() < 0.0001); // another slot round it
    assert!(got.2.x > 10.0); // on its east side, nearest the seeker
}

#[test]
fn a_slot_claimed_by_a_nearer_seeker_is_not_open() {
    let mut sides = duel();
    sides[1].fighters.push(Fighter::grem(6, v(12.0, 10.0), 270.0)); // nearer: it arrives first
    let mut field = scrum(&sides, SEED);
    let got = seek(&mut field, &[seeker_fights()], LEASH);
    assert!(near(got[&6].2, v(11.0, 10.0)));
    assert_eq!(got[&5].0, AIM);
    assert!(got[&5].2.distance_to(v(11.0, 10.0)) as f64 >= 1.0 - 0.001); // a breadth away
}

#[test]
fn a_slot_past_the_leash_is_not_taken_and_nothing_is_pressed_for() {
    let mut field = scrum(&duel(), SEED);
    let got = seek(&mut field, &[seeker_fights()], 1.0)[&5]; // slots are 2 cells from its place
    assert_eq!(got.0, IDLE);
    assert!(near(got.2, v(13.0, 10.0)));
}

#[test]
fn with_no_slot_open_it_waits_a_breadth_short_of_the_nearest_taken_one() {
    let mut sides = duel();
    let ring: Vec<Fighter> = (0..6)
        .map(|n| {
            let way = bearing_vector(90.0 + n as f64 * 60.0);
            Fighter::grem(20 + n, v(10.0 + way.x, 10.0 + way.y), 0.0)
        })
        .collect();
    sides.push(Side { id: 2, faction: "player", flags: 0, fighters: ring }); // on every slot
    let mut field = scrum(&sides, SEED);
    let got = seek(&mut field, &[seeker_fights()], LEASH)[&5];
    assert_eq!((got.0, got.1), (PRESS, 0));
    assert!(near(got.2, v(12.0, 10.0)));
}

#[test]
fn a_unit_touching_a_foe_stands_and_one_not_free_to_seek_keeps_its_place() {
    let mut sides = duel();
    sides[1].fighters[0].at = v(11.1, 10.0); // within reach of the foe's body
    let mut field = scrum(&sides, SEED);
    let got = seek(&mut field, &[seeker_fights()], LEASH)[&5];
    assert_eq!(got.0, TOUCH);
    assert!(near(got.2, v(11.1, 10.0)));
    let mut field = scrum(&duel(), SEED);
    let mut retreating = seeker_fights();
    retreating.seeking = false;
    assert_eq!(seek(&mut field, &[retreating], LEASH)[&5].0, STAND);
}

/// Which side of the foe's front a seeker coming from the north takes, its front slot
/// under a body: 1 for the foe's right (east), -1 for its left.
fn side_taken(seed: i64) -> i32 {
    let sides = vec![
        Side {
            id: 9,
            faction: "kingdom",
            flags: 0,
            fighters: vec![Fighter::grem(1, v(10.0, 12.0), 0.0)],
        },
        Side {
            id: 1,
            faction: "player",
            flags: 0,
            fighters: vec![Fighter::grem(5, v(10.0, 9.0), 180.0)],
        },
        Side {
            id: 2,
            faction: "player",
            flags: 0,
            fighters: vec![Fighter::grem(7, v(10.0, 11.0), 0.0)],
        },
    ];
    let mut field = scrum(&sides, seed);
    let fights = Fights::against(1, &[9], 180.0, v(10.0, 9.5));
    let ids: Vec<i64> = vec![1];
    let say = SquadSay {
        ids: &ids,
        foe_counts: &[1],
        foe_ids: &fights.foes,
        seeking: &[1],
        frames: &[180.0, 1.0, 0.0],
        anchors: &[fights.anchor],
        chaser_counts: &[0],
        chaser_ids: &[],
        speeds: &vec![1.0; field.flat.len()],
    };
    let p = Params { seed, ..params(LEASH) };
    let (_, out, _) = seeking::seek(&mut field, &say, &p, &|_, _| true, [1, 2]);
    if out.nexts[0].x > 10.0 {
        1
    } else {
        -1
    }
}

#[test]
fn two_mirror_image_slots_are_taken_about_as_often() {
    let right = (1..61).filter(|&seed| side_taken(seed) > 0).count();
    assert!((20..=40).contains(&right), "{right} of 60 to the right"); // 30 expected; sd ~4
}

#[test]
fn the_seek_is_the_same_whatever_the_list_order() {
    let player: Vec<Fighter> =
        (0..6).map(|n| Fighter::grem(100 + n, v(14.0, 7.0 + n as f32 * 1.2), 270.0)).collect();
    let kingdom: Vec<Fighter> =
        (0..3).map(|n| Fighter::grem(200 + n, v(10.0, 8.0 + n as f32 * 1.6), 90.0)).collect();
    let sides = vec![
        Side { id: 1, faction: "player", flags: 0, fighters: player },
        Side { id: 2, faction: "kingdom", flags: 0, fighters: kingdom },
    ];
    let fights = |s: &[Side]| -> Vec<Fights> {
        s.iter()
            .map(|side| {
                let (foe, heading, x) =
                    if side.id == 1 { (2, 270.0, 13.5) } else { (1, 90.0, 10.5) };
                Fights::against(side.id, &[foe], heading, v(x, 10.0))
            })
            .collect()
    };
    let forward = seek(&mut scrum(&sides, SEED), &fights(&sides), LEASH);
    let back = reversed(&sides);
    let backward = seek(&mut scrum(&back, SEED), &fights(&back), LEASH);
    assert_eq!(forward, backward);
    assert!(forward.values().filter(|o| o.0 == AIM).count() >= 3); // the scrum is under way
}

#[test]
fn the_seek_is_the_same_on_the_pool() {
    let side = |id: i64, x: f32, bearing: f64, units: i64| -> Vec<Fighter> {
        (0..units)
            .map(|n| {
                Fighter::grem(
                    id * 100 + n,
                    v(x + (n % 4) as f32 * 1.1, (n / 4) as f32 * 1.2),
                    bearing,
                )
            })
            .collect()
    };
    let sides = vec![
        Side { id: 1, faction: "player", flags: 0, fighters: side(1, 14.0, 270.0, 24) },
        Side { id: 2, faction: "kingdom", flags: 0, fighters: side(2, 6.0, 90.0, 12) },
        Side { id: 3, faction: "kingdom", flags: 0, fighters: side(3, 22.0, 270.0, 8) },
    ];
    let fights = || {
        vec![
            Fights::against(1, &[2, 3], 270.0, v(15.5, 4.0)),
            Fights::against(2, &[1], 90.0, v(7.5, 2.0)),
            Fights::against(3, &[1], 270.0, v(23.0, 1.0)),
        ]
    };
    let one = seek_on(&mut scrum(&sides, SEED), &fights(), params(LEASH));
    let pool = Params { threaded: Threads::Always, ..params(LEASH) };
    let pooled = seek_on(&mut scrum(&sides, SEED), &fights(), pool);
    assert_eq!(one, pooled);
    assert!(one.values().filter(|o| o.0 == AIM).count() >= 10); // the scrum is under way
}

/// Six seekers of the player's in a line east of the lone foe, a little south of its front:
/// every one of them finds the same slot nearest (the front, then the next round it), so
/// each round settles one of them.
fn line_of_seekers() -> Vec<Side> {
    let way = bearing_vector(95.0);
    let mut sides = duel();
    sides[1].fighters = (0..6)
        .map(|n| {
            let d = 3.0 + n as f32 * 1.01;
            Fighter::grem(30 + n as i64, v(10.0 + way.x * d, 10.0 + way.y * d), 270.0)
        })
        .collect();
    sides
}

#[test]
fn a_seeker_still_unplaced_after_the_rounds_presses_toward_the_nearest_slot() {
    const { assert!(ROUNDS < 6, "the line settles one seeker a round, round six slots") };
    let got = seek(&mut scrum(&line_of_seekers(), SEED), &[seeker_fights()], LEASH);
    let aiming: Vec<i64> = got.iter().filter(|(_, o)| o.0 == AIM).map(|(&id, _)| id).collect();
    assert_eq!(aiming, (30..30 + ROUNDS as i64).collect::<Vec<_>>()); // the nearest first
    for id in 30 + ROUNDS as i64..36 {
        assert_eq!((got[&id].0, got[&id].1), (PRESS, 0), "{id}"); // the front, the nearest
    }
}

#[test]
fn of_two_seekers_wanting_one_slot_the_higher_key_claims_and_the_other_takes_the_next() {
    let mut sides = line_of_seekers();
    sides[1].fighters.truncate(2);
    let got = seek(&mut scrum(&sides, SEED), &[seeker_fights()], LEASH);
    assert_eq!((got[&30].0, got[&30].1), (AIM, 0)); // the nearer arrives first
    assert_eq!((got[&31].0, got[&31].1), (AIM, 1)); // the next nearest, a round later
}

#[test]
fn seekers_not_contending_aim_where_each_alone_would() {
    let north = Fighter::grem(5, v(10.0, 7.0), 180.0);
    let south = Fighter::grem(6, v(10.0, 13.0), 0.0);
    let alone = |f: &Fighter| {
        let mut sides = duel();
        sides[1].fighters = vec![f.clone()];
        let got = seek(&mut scrum(&sides, SEED), &[seeker_fights()], LEASH);
        got[&f.id]
    };
    let mut sides = duel();
    sides[1].fighters = vec![north.clone(), south.clone()];
    let got = seek(&mut scrum(&sides, SEED), &[seeker_fights()], LEASH);
    assert_eq!(got[&5], alone(&north));
    assert_eq!(got[&6], alone(&south));
    assert_eq!((got[&5].0, got[&6].0), (AIM, AIM));
    assert_ne!(got[&5].1, got[&6].1);
}

#[test]
fn a_dense_seek_is_the_same_whatever_the_list_order_and_on_the_pool() {
    let side = |id: i64, x: f32, bearing: f64| -> Vec<Fighter> {
        (0..30)
            .map(|n| {
                let at = v(x + (n % 5) as f32 * 1.1, (n / 5) as f32 * 1.1);
                Fighter::grem(id * 100 + n, at, bearing)
            })
            .collect()
    };
    let sides = vec![
        Side { id: 1, faction: "player", flags: 0, fighters: side(1, 16.0, 270.0) },
        Side { id: 2, faction: "kingdom", flags: 0, fighters: side(2, 8.0, 90.0) },
    ];
    let fights = |s: &[Side]| -> Vec<Fights> {
        s.iter()
            .map(|side| {
                let (foe, heading, x) =
                    if side.id == 1 { (2, 270.0, 16.5) } else { (1, 90.0, 12.5) };
                Fights::against(side.id, &[foe], heading, v(x, 3.0))
            })
            .collect()
    };
    let forward = seek(&mut scrum(&sides, SEED), &fights(&sides), LEASH);
    let back = reversed(&sides);
    assert_eq!(forward, seek(&mut scrum(&back, SEED), &fights(&back), LEASH));
    let pool = Params { threaded: Threads::Always, ..params(LEASH) };
    assert_eq!(forward, seek_on(&mut scrum(&sides, SEED), &fights(&sides), pool));
    let aims = forward.values().filter(|o| o.0 == AIM).count();
    let presses = forward.values().filter(|o| o.0 == PRESS).count();
    assert!(aims >= 10 && presses >= 10, "{aims} aim, {presses} press"); // contended
}

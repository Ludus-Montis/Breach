//! The march (spec 39 step 3d), from the gdUnit tests it replaces: test_walk_modes',
//! test_formation_pour's and test_place_order's, and test_formation_walk's and
//! test_formation_sweep's that call the classes directly. Their scenarios through a battle
//! stay gdUnit until the tick ports.

use super::front::Cell;
use super::modes::{self, Climber, Footing, ModeTuning};
use super::pour::{self, Frame, Pourer};
use super::shuffle::{self, Ranked, Squad};
use super::walk::{self, Lag, Marcher, Rule, Standing, Traits, WalkTuning, Walking};
use super::*;
use crate::content::tests::repo_content;
use crate::content::Content;
use crate::ground::{GroundTuning, Paint};
use crate::maths::V2;
use crate::motion;
use crate::route::Route;

/// FormationSimulation.TRAVEL_SCALE: cells a second at speed 1.
const PACE: f64 = 8.0;

fn v(x: f32, y: f32) -> V2 {
    V2::new(x, y)
}

fn content() -> Content {
    Content::load(&repo_content()).unwrap_or_else(|error| panic!("{error}"))
}

fn ground(cols: i32, rows: i32) -> Ground {
    Ground::new(cols, rows, GroundTuning::of(&content().tuning))
}

fn depth(g: &mut Ground, area: (i32, i32, i32, i32), depth: f64) {
    g.paint(area, &Paint { depth: Some(depth), ..Paint::default() });
}

fn mode_tuning() -> ModeTuning {
    let t = content().tuning;
    ModeTuning {
        climb_rate: t.stamina_climb,
        swim_rate: t.stamina_swim,
        spent: t.stamina_spent,
        cliff_quarters: t.ground_cliff_quarters,
        fall_per_cell: t.fall_damage_per_cell,
        fall_mass_power: t.fall_mass_power,
    }
}

pub(super) fn walk_tuning() -> WalkTuning {
    let t = content().tuning;
    WalkTuning {
        face_travel: t.walk_face_travel,
        drag: t.bodies_ground_drag,
        block: t.bodies_ground_block,
        modes: mode_tuning(),
    }
}

/// A grounded walker a cell tall (TerrainWalker.grounded).
fn grounded() -> Walker {
    Walker { height: 1.0, swimmer: 0, climber: 0, swims: false, climbs: false }
}

const SWIMMER: Walker = Walker { height: 1.0, swimmer: 0, climber: 0, swims: true, climbs: false };
const CLIMBER: Walker = Walker { height: 1.0, swimmer: 0, climber: 1, swims: false, climbs: true };

fn climber(walker: Walker) -> Climber {
    Climber { walker, most_stamina: 100.0, area: 1.0 }
}

fn footing(at: V2, foothold: V2, stamina: f64) -> Footing {
    Footing { at, foothold, stamina, on_face: false, climb_left: 0.0 }
}

/// Water three cells deep, columns 10 to 14.
fn pool() -> Ground {
    let mut g = ground(20, 20);
    depth(&mut g, (10, 0, 5, 20), 3.0);
    g
}

/// Four cells up at x 10.
fn cliff() -> Ground {
    let mut g = ground(20, 20);
    g.paint((10, 0, 10, 20), &Paint { height: Some(16), ..Paint::default() });
    g
}

// WalkModes (test_walk_modes).

#[test]
fn a_swimmer_crosses_deep_water_and_a_grounded_unit_does_not() {
    let g = pool();
    assert!(g.crossing(&SWIMMER, v(9.5, 5.5), v(10.5, 5.5)) > 0.0);
    assert_eq!(g.crossing(&grounded(), v(9.5, 5.5), v(10.5, 5.5)), 0.0);
}

#[test]
fn a_second_swimming_costs_the_swim_rate() {
    let g = pool();
    let tuning = mode_tuning();
    let mut at = footing(v(10.5, 5.5), v(10.5, 5.5), 50.0);
    modes::after_step(&g, &climber(SWIMMER), &mut at, v(10.2, 5.5), 1.0, &tuning);
    assert!((at.stamina - (50.0 - tuning.swim_rate)).abs() < 0.0001);
    assert!(at.on_face); // no breather in deep water
}

#[test]
fn a_spent_unit_starts_no_swim_but_carries_on_one_it_is_in() {
    let g = pool();
    let tuning = mode_tuning();
    let spent = climber(SWIMMER);
    assert!(!modes::may_step(&g, &spent, 0.0, v(9.5, 5.5), v(10.5, 5.5), &tuning));
    assert!(modes::may_step(&g, &spent, 0.0, v(11.5, 5.5), v(12.5, 5.5), &tuning));
    assert!(modes::may_step(&g, &spent, 0.0, v(5.5, 5.5), v(6.5, 5.5), &tuning));
    // walking
}

#[test]
fn stepping_onto_a_cliff_begins_a_climb_of_its_height() {
    let g = cliff();
    let mut at = footing(v(10.1, 5.5), v(9.5, 5.5), 100.0);
    modes::after_step(&g, &climber(CLIMBER), &mut at, v(9.5, 5.5), 0.1, &mode_tuning());
    assert!((at.climb_left - 4.0).abs() < 0.0001);
    assert!(at.on_face);
}

#[test]
fn a_climb_takes_time_and_stamina_at_the_climb_pace() {
    let g = cliff();
    let t = content().tuning;
    let mut at = Footing { climb_left: 4.0, ..footing(v(10.1, 5.5), v(9.5, 5.5), 30.0) };
    let mut hurt = 0;
    let still = modes::climb(&g, &climber(CLIMBER), &mut at, 1.0, 1.0, &mode_tuning(), &mut hurt);
    assert!(still);
    assert!((at.climb_left - (4.0 - t.ground_climb_pace)).abs() < 0.0001);
    assert!((at.stamina - (30.0 - t.stamina_climb)).abs() < 0.0001);
    assert_eq!(at.at, v(10.1, 5.5));
    assert_eq!(hurt, 0);
}

#[test]
fn a_unit_spent_on_a_climb_face_falls_hurt_by_what_it_climbed() {
    let g = cliff();
    let t = content().tuning;
    // three cells up already
    let mut at = Footing { climb_left: 1.0, ..footing(v(10.1, 5.5), v(9.5, 5.5), 0.01) };
    let mut hurt = 0;
    let still = modes::climb(&g, &climber(CLIMBER), &mut at, 0.1, 1.0, &mode_tuning(), &mut hurt);
    let climbed = 4.0 - (1.0 - t.ground_climb_pace * 0.1);
    assert!(!still);
    assert_eq!(hurt, (t.fall_damage_per_cell * climbed).round() as i64);
    assert_eq!(at.at, v(9.5, 5.5));
    assert!(!at.on_face);
}

// FormationWalk (test_formation_walk).

/// A grem: turn rate 720, backward pace 0.6, a cell's body.
fn grem() -> Marcher {
    Marcher { climber: climber(grounded()), turn_rate: 720.0, backward_pace: 0.6, radius: 0.5 }
}

fn standing(at: V2, bearing: f64) -> Standing {
    Standing { footing: footing(at, at, 100.0), bearing, speed: 1.0 }
}

fn walking(g: Option<&Ground>, heading: f64) -> Walking<'_> {
    Walking { ground: g, lying: None, seconds: 0.1, pace: PACE, heading, tuning: walk_tuning() }
}

#[test]
fn a_unit_pushed_off_its_place_walks_back_at_its_pace() {
    let place = v(20.0, 16.0);
    let mut unit = standing(place.sub(bearing_vector(90.0).scale(3.0)), 90.0); // pushed 3 cells back
    walk::walk(&walking(None, 90.0), &grem(), &mut unit, place);
    let gone = unit.footing.at.distance_to(place) as f64;
    assert!(gone < 3.0);
    assert!(3.0 - gone <= PACE * 0.1 + 0.0001);
}

#[test]
fn far_from_its_place_it_faces_its_way_and_near_it_its_formations() {
    let place = v(20.0, 16.0);
    let across = bearing_vector(90.0).orthogonal();
    let mut unit = standing(place.add(across.scale(6.0)), 90.0);
    let on = walking(None, 90.0);
    for _ in 0..10 {
        walk::walk(&on, &grem(), &mut unit, place);
    }
    let way = motion::bearing_to(unit.footing.at, place, unit.bearing);
    let off = (motion::fposmod(way - unit.bearing + 180.0, 360.0) - 180.0).abs();
    assert!(off < 0.01_f64.to_degrees());
    for _ in 0..40 {
        walk::walk(&on, &grem(), &mut unit, place);
    }
    assert!(unit.footing.at.distance_to(place) < 0.001);
    assert!((unit.bearing - 90.0).abs() < 0.01);
}

#[test]
fn a_unit_barred_straight_ahead_slides_along_the_bank() {
    let mut g = ground(64, 32);
    depth(&mut g, (10, 0, 4, 10), 3.0); // a pool, open to its south
    let before = v(9.5, 9.5);
    let mut unit = standing(before, 90.0);
    walk::walk(&walking(Some(&g), 90.0), &grem(), &mut unit, v(11.5, 10.5));
    assert_ne!(unit.footing.at, before);
    assert!(g.crossing(&grounded(), unit.footing.at, unit.footing.at) > 0.0);
}

#[test]
fn a_walk_over_a_climb_face_climbs_before_it_moves_on() {
    let g = cliff();
    let unit = Marcher { climber: climber(CLIMBER), ..grem() };
    let mut state = Standing {
        footing: footing(v(9.7, 5.5), v(9.5, 5.5), 100.0),
        ..standing(v(9.7, 5.5), 90.0)
    };
    let on = walking(Some(&g), 90.0);
    for _ in 0..10 {
        if state.footing.on_face {
            break;
        }
        walk::walk(&on, &unit, &mut state, v(14.5, 5.5));
    }
    assert!(state.footing.on_face, "stepped onto the face");
    let face = state.footing.at;
    walk::walk(&on, &unit, &mut state, v(14.5, 5.5));
    assert_eq!(state.footing.at, face, "it climbs before it moves on");
}

/// A frame 3 wide facing east at (20, 16), and the walker of its units.
fn frame(width: i32) -> Frame {
    Frame { anchor: v(20.0, 16.0), heading: 90.0, width, centre_shift: 0.0 }
}

fn pourer(id: i64, column: i32, walker: Walker) -> Pourer {
    Pourer {
        id,
        at: V2::ZERO,
        rank: 0,
        column,
        footprint_width: 1,
        footprint_depth: 1,
        offset: V2::ZERO,
        walker,
        taking: None,
    }
}

/// A line `width` wide, its units at their places.
fn line(frame: &Frame, walker: Walker) -> Vec<Pourer> {
    (0..frame.width)
        .map(|column| {
            let unit = pourer(column as i64 + 1, column, walker);
            Pourer { at: pour::place(frame, &unit), ..unit }
        })
        .collect()
}

fn lags(frame: &Frame, units: &[Pourer]) -> Vec<Lag> {
    let aims = pour::targets(None, frame, units, 0, 1.0);
    units
        .iter()
        .zip(aims)
        .map(|(u, target)| Lag { target, place: pour::place(frame, u), at: u.at })
        .collect()
}

#[test]
fn the_frame_waits_while_a_unit_lags_beyond_its_slack() {
    let f = frame(3);
    let t = content().tuning;
    let slack = walk::slack_of(0.0, t.walk_slack_loose, t.walk_slack_drilled);
    let back = bearing_vector(90.0).scale(-1.0);
    let mut units = line(&f, grounded());
    units[1].at = pour::place(&f, &units[1]).add(back.scale(slack * 0.9));
    assert!(walk::share(Rule::Lagging, slack, t.walk_lost, &lags(&f, &units)) > 0.000001);
    units[1].at = pour::place(&f, &units[1]).add(back.scale(slack + 0.5));
    assert!(walk::share(Rule::Lagging, slack, t.walk_lost, &lags(&f, &units)) <= 0.000001);
}

#[test]
fn a_drilled_formation_keeps_a_tighter_slack() {
    let t = content().tuning;
    let loose = walk::slack_of(0.0, t.walk_slack_loose, t.walk_slack_drilled);
    assert!((loose - t.walk_slack_loose).abs() < 0.001);
    assert!(walk::slack_of(100.0, t.walk_slack_loose, t.walk_slack_drilled) < loose);
}

#[test]
fn a_unit_fallen_far_behind_is_waited_for_only_if_no_man_is_left_behind() {
    let f = frame(2);
    let t = content().tuning;
    let slack = walk::slack_of(0.0, t.walk_slack_loose, t.walk_slack_drilled);
    let mut units = line(&f, grounded());
    let lost = t.walk_lost + 1.0;
    units[0].at = pour::place(&f, &units[0]).sub(bearing_vector(90.0).scale(lost));
    let lag = lags(&f, &units);
    assert!(walk::share(Rule::Lagging, slack, t.walk_lost, &lag) > 0.000001);
    let all = [Traits { leads: false, left_behind: false, no_man_left: true }; 2];
    assert_eq!(walk::rule_of(&all), Rule::NoManLeft);
    assert!(walk::share(Rule::NoManLeft, slack, t.walk_lost, &lag) <= 0.000001);
}

#[test]
fn fall_behind_left_behind_never_waits() {
    let led = [
        Traits { leads: true, left_behind: true, no_man_left: false },
        Traits { leads: false, left_behind: false, no_man_left: true },
    ];
    assert_eq!(walk::rule_of(&led), Rule::LeftBehind); // a leader's trait, and the never-wait wins
    let lone = [Traits { leads: false, left_behind: true, no_man_left: false }; 1];
    assert_eq!(walk::rule_of(&lone), Rule::LeftBehind); // one every unit has
    assert_eq!(walk::rule_of(&[]), Rule::Lagging);
    let far = [Lag { target: v(0.0, 0.0), place: v(0.0, 0.0), at: v(9.0, 0.0) }];
    assert_eq!(walk::share(Rule::LeftBehind, 1.0, 1.0, &far), 1.0);
}

#[test]
fn a_unit_whose_place_is_barred_squeezes_in_towards_the_centre_line() {
    let mut g = ground(64, 32);
    depth(&mut g, (0, 0, 64, 32), 3.0); // deep water everywhere
    depth(&mut g, (0, 0, 64, 2), 0.0); // but a 2-cell strip along the route
    let f = Frame { anchor: v(20.0, 1.0), heading: 90.0, width: 4, centre_shift: 0.0 };
    let units = line(&f, grounded());
    let aims = pour::targets(Some(&g), &f, &units, 0, 1.0);
    for (unit, target) in units.iter().zip(aims) {
        assert!(g.crossing(&grounded(), target, target) > 0.0);
        assert!(target.x <= pour::place(&f, unit).x + 0.0001); // across it, or filed in behind
    }
}

// FormationPour (test_formation_pour).

/// A line `count` wide that can't swim, its front in a stream 4 cells across (x 20 to 24)
/// with a 4-cell ford (y 14 to 18) on its route along y 16.
fn at_the_ford(count: i32) -> (Ground, Frame, Vec<Pourer>) {
    let mut g = ground(64, 32);
    depth(&mut g, (20, 0, 4, 32), 3.0);
    depth(&mut g, (20, 14, 4, 4), 0.0);
    let f = Frame { anchor: v(22.5, 16.0), heading: 90.0, width: count, centre_shift: 0.0 };
    let units = line(&f, grounded());
    (g, f, units)
}

const SEED: i64 = 841592;

#[test]
fn units_squeezed_at_a_ford_make_for_points_a_body_apart() {
    let (g, f, units) = at_the_ford(8);
    let aims = pour::targets(Some(&g), &f, &units, SEED, 1.0);
    for (i, &aim) in aims.iter().enumerate() {
        assert!(g.crossing(&grounded(), aim, aim) > 0.0);
        for other in &aims[i + 1..] {
            assert!(aim.distance_to(*other) as f64 >= 1.0 - 0.0001);
        }
    }
}

#[test]
fn the_squeezed_file_in_behind_front_first_then_nearest_the_centre_line() {
    let (g, f, units) = at_the_ford(8);
    let aims = pour::targets(Some(&g), &f, &units, SEED, 1.0);
    let ahead = bearing_vector(90.0);
    let across = ahead.orthogonal();
    let by_offset = |half_cells: i64| {
        let index = units
            .iter()
            .position(|u| {
                ((pour::place(&f, u).sub(f.anchor).dot(across) * 2.0) as f64).round() as i64
                    == half_cells
            })
            .unwrap();
        aims[index]
    };
    for side in [-1, 1] {
        // 2.5 and 3.5 cells out squeeze onto one point: the nearer leads
        let (nearer, further) = (by_offset(5 * side), by_offset(7 * side));
        assert!(nearer.sub(further).dot(ahead) as f64 >= 1.0 - 0.0001);
    }
}

#[test]
fn a_unit_whose_place_it_can_walk_keeps_to_its_place() {
    let (g, f, units) = at_the_ford(4); // 4 wide: every place lies in the ford
    let aims = pour::targets(Some(&g), &f, &units, SEED, 1.0);
    for (unit, aim) in units.iter().zip(aims) {
        assert_eq!(aim, pour::place(&f, unit));
    }
}

#[test]
fn the_targets_are_the_same_whatever_the_order_of_the_units() {
    let (g, f, units) = at_the_ford(9);
    let aims = pour::targets(Some(&g), &f, &units, SEED, 1.0);
    let reversed: Vec<Pourer> = units.iter().rev().copied().collect();
    let mut back = pour::targets(Some(&g), &f, &reversed, SEED, 1.0);
    back.reverse();
    assert_eq!(aims, back);
}

#[test]
fn one_out_taking_a_downed_foe_makes_for_beside_it() {
    let f = frame(2);
    let mut units = line(&f, grounded());
    units[0].at = v(10.0, 10.0);
    units[0].taking = Some(v(14.0, 10.0));
    let aims = pour::targets(None, &f, &units, 0, 2.0);
    assert_eq!(aims[0], v(13.0, 10.0)); // half its reach from the body, on its own side
    assert_eq!(aims[1], pour::place(&f, &units[1]));
}

#[test]
fn a_unit_part_way_through_a_swap_belongs_part_way_between_its_places() {
    let f = frame(4);
    let unit = pourer(1, 2, grounded());
    let half = Pourer { offset: v(0.5, 1.0), ..unit };
    let ahead = bearing_vector(90.0);
    let expected = pour::place(&f, &unit).add(ahead.scale(0.5)).sub(ahead.orthogonal().scale(1.0));
    assert_eq!(pour::place(&f, &half), expected);
}

// FormationMarch and FormationSweep.

fn along(front: f64, home: f64, order: i32) -> Along {
    Along { front, home, order, direction: if home == 0.0 { 1 } else { -1 } }
}

#[test]
fn the_front_stops_at_its_target_and_never_passes_either_end() {
    assert_eq!(toward(&along(9.5, 0.0, ADVANCE), 1.0, 10.0), 10.0);
    assert_eq!(toward(&along(2.0, 0.0, ADVANCE), 1.0, 10.0), 3.0);
    assert_eq!(toward(&along(0.5, 0.0, RETREAT), -1.0, 10.0), 0.0);
    assert_eq!(toward(&along(0.5, 10.0, ADVANCE), -1.0, 10.0), 0.0); // from the far end
    assert_eq!(toward(&along(9.0, 10.0, RETREAT), 2.0, 10.0), 10.0);
}

#[test]
fn advancing_heads_away_from_home_and_anything_else_for_home() {
    assert_eq!(travel_sign(&along(2.0, 0.0, ADVANCE), 10.0), 1);
    assert_eq!(travel_sign(&along(2.0, 0.0, RETREAT), 10.0), -1);
    assert_eq!(travel_sign(&along(2.0, 10.0, ADVANCE), 10.0), -1);
    assert_eq!(travel_sign(&along(2.0, 10.0, 1), 10.0), 1); // holding: home
    assert_eq!(travel_sign(&along(10.0, 0.0, ADVANCE), 10.0), 1); // there: as it was going
}

#[test]
fn the_pace_is_the_worst_ground_its_front_steps_into_and_none_where_all_are_barred() {
    let mut g = ground(64, 32);
    g.paint((21, 0, 4, 8), &Paint { cost: Some(0.5), ..Paint::default() });
    depth(&mut g, (21, 8, 4, 8), 3.0);
    let walkers = [grounded(); 3];
    let points = [v(20.5, 4.5), v(20.5, 12.5), v(10.5, 20.5)];
    assert_eq!(pace(&g, &walkers, &points, 90.0), g.crossing(&grounded(), points[0], v(21.5, 4.5)));
    assert_eq!(pace(&g, &walkers[1..2], &points[1..2], 90.0), 0.0);
    let mut reversed = points;
    reversed.reverse();
    assert_eq!(pace(&g, &walkers, &reversed, 90.0), pace(&g, &walkers, &points, 90.0));
}

#[test]
fn a_frame_sweeps_no_faster_than_its_outer_file_marches_the_arc() {
    let cells_per_second = 8.0;
    let rate = sweep::rate(8, cells_per_second);
    let quarter = std::f64::consts::PI * 8.0 / 4.0 / cells_per_second;
    assert!((90.0 / rate - quarter).abs() < 0.0001);
    let bend = Route::new(vec![v(0.0, 0.0), v(32.0, 0.0), v(32.0, 64.0)], 0.0);
    let wanted = sweep::wanted(&bend, 40.0, 1, 90.0);
    assert_eq!(wanted, 180.0);
    let turned = sweep::step(90.0, wanted, 8, cells_per_second, 0.1);
    assert!((turned - (90.0 + rate * 0.1)).abs() < 1e-9);
    assert_eq!(sweep::step(170.0, 180.0, 8, cells_per_second, 1.0), 180.0); // no further than it wants
    assert_eq!(sweep::wanted(&bend, 40.0, -1, 90.0), 0.0); // going back up the route
}

/// A squad of `width` columns, its front 40 cells along a route bending south at 32 cells.
fn on_the_bend(width: i32) -> crate::squads::Squads {
    let mut squads = crate::squads::Squads::default();
    squads.open(1);
    squads.width[0] = width;
    squads.direction[0] = 1;
    squads.heading[0] = 90.0;
    squads.front_distance[0] = 40.0 / 64.0;
    squads.route[0] = Some(Route::new(vec![v(0.0, 0.0), v(32.0, 0.0), v(32.0, 64.0)], 0.0));
    squads
}

#[test]
fn a_squad_faces_its_held_route_where_its_front_stands() {
    let mut squads = on_the_bend(8);
    sweep::face_held(&mut squads, 0);
    assert_eq!(squads.heading[0], 180.0);
    squads.route[0] = None;
    squads.heading[0] = 90.0;
    sweep::face_held(&mut squads, 0);
    assert_eq!(squads.heading[0], 90.0, "no route: as it is");
}

#[test]
fn a_squad_sweeps_towards_its_held_route_no_faster_than_its_rate() {
    let mut squads = on_the_bend(8);
    sweep::sweep_held(&mut squads, 0, 8.0, 0.1);
    assert_eq!(squads.heading[0], sweep::step(90.0, 180.0, 8, 8.0, 0.1));
    squads.route[0] = None;
    sweep::sweep_held(&mut squads, 0, 8.0, 0.1);
    assert_eq!(squads.heading[0], sweep::step(90.0, 180.0, 8, 8.0, 0.1), "no route: as it is");
}

// PlaceOrder (test_place_order) and FormationShuffle.

#[test]
fn front_rank_first_and_each_rank_left_to_right() {
    let order = shuffle::front_first(&[(2, 0), (0, 3), (1, -2), (0, -1)]);
    assert_eq!(order, vec![3, 1, 2, 0]);
}

#[test]
fn a_place_held_twice_keeps_its_order() {
    let order = shuffle::front_first(&[(1, 1), (0, 0), (1, 1), (0, 2)]);
    assert_eq!(order, vec![1, 3, 0, 2]);
}

fn ranked(place: Cell, preferred_position: i32, position_priority: i32) -> Ranked {
    Ranked {
        place,
        footprint_width: 1,
        footprint_depth: 1,
        preferred_position,
        position_priority,
        speed: 1.0,
        joined: false,
    }
}

const MARCHING: Squad = Squad { width: 3, combat_width: 0, centre_shift: 0.0, fighting: false };

#[test]
fn a_front_preferring_unit_behind_takes_a_free_front_place() {
    let units = [ranked((0, 0), 0, 1), ranked((0, 2), 0, 1), ranked((1, 1), 0, 1)];
    let moves = shuffle::start_moves(&MARCHING, &units, &[], &[], [0.1, PACE, 1.0]);
    assert_eq!(moves.len(), 1);
    assert_eq!(moves[0].plan.to, vec![(2, (0, 1))]);
    let ticks = (1.0 / 64.0 / PACE / 0.1 - 0.000001_f64).ceil() as i64;
    assert_eq!(moves[0].ticks, ticks.max(1));
}

#[test]
fn a_stronger_unit_passes_the_weaker_ahead_into_its_back_row() {
    // a full front of back-preferring units, a front-preferring one behind the middle
    let units =
        [ranked((0, 0), 2, 1), ranked((0, 1), 2, 1), ranked((0, 2), 2, 1), ranked((1, 1), 0, 1)];
    let moves = shuffle::start_moves(&MARCHING, &units, &[], &[], [0.1, PACE, 1.0]);
    assert_eq!(moves.len(), 1);
    assert_eq!(moves[0].plan.mover, 3);
    assert_eq!(moves[0].plan.passed, vec![1]);
    assert_eq!(moves[0].plan.to, vec![(3, (0, 1)), (1, (1, 1))]);
    // under way, the same move isn't planned again
    let again = shuffle::start_moves(
        &MARCHING,
        &units,
        &[((0, 1), 1, 1), ((1, 1), 1, 1)],
        &[3, 1],
        [0.1, PACE, 1.0],
    );
    assert!(again.is_empty());
}

#[test]
fn the_moves_planned_are_the_same_whatever_the_order_of_the_units() {
    let units = [
        ranked((0, 0), 2, 1),
        ranked((0, 1), 2, 1),
        ranked((0, 2), 2, 1),
        ranked((1, 1), 0, 1),
        ranked((1, 0), 0, 2),
    ];
    let moves = shuffle::start_moves(&MARCHING, &units, &[], &[], [0.1, PACE, 1.0]);
    let reversed: Vec<Ranked> = units.iter().rev().copied().collect();
    let back = shuffle::start_moves(&MARCHING, &reversed, &[], &[], [0.1, PACE, 1.0]);
    let last = units.len() - 1;
    let unreversed: Vec<Vec<(usize, Cell)>> =
        back.iter().map(|m| m.plan.to.iter().map(|&(u, at)| (last - u, at)).collect()).collect();
    let planned: Vec<Vec<(usize, Cell)>> = moves.iter().map(|m| m.plan.to.clone()).collect();
    assert_eq!(planned, unreversed);
}

#[test]
fn a_reinforcement_in_a_fight_spreads_the_line_within_the_combat_width() {
    let fighting = Squad { width: 2, combat_width: 4, centre_shift: 0.0, fighting: true };
    let joined = Ranked { joined: true, ..ranked((1, 0), 0, 1) };
    let units = [ranked((0, 0), 0, 1), ranked((0, 1), 0, 1), joined];
    let moves = shuffle::start_moves(&fighting, &units, &[], &[], [0.1, PACE, 0.5]);
    assert_eq!(moves.len(), 1);
    let (_, (rank, column)) = moves[0].plan.to[0];
    assert_eq!(rank, 0);
    assert!(column == -1 || column == 2, "beside the line: {column}");
}

// The march's ends (FormationMarch.check_ends, spec 39 step 8a2-v), from test_retreat_home.gd's
// rule and the arrivals the battles reach.

fn ended(front: f64, home: f64, order: i32) -> Along {
    Along { front, home, order, direction: if home == 0.0 { 1 } else { -1 } }
}

const HOLD: i32 = 1;

#[test]
fn an_advance_whose_front_reaches_the_enemy_end_arrives_there() {
    assert_eq!(end(&ended(12.0, 0.0, ADVANCE), 12.0, true, || false), End::Arrived(12.0));
    assert_eq!(end(&ended(12.0 + 1e-9, 0.0, ADVANCE), 12.0, true, || false), End::Arrived(12.0));
    assert_eq!(end(&ended(0.0, 12.0, ADVANCE), 12.0, true, || false), End::Arrived(0.0));
}

#[test]
fn an_advance_short_of_the_enemy_end_marches_on() {
    assert_eq!(end(&ended(11.5, 0.0, ADVANCE), 12.0, true, || true), End::Under);
}

#[test]
fn a_retreat_whose_front_is_home_returns_without_asking_where_its_units_stand() {
    let asked = |answer: bool| move || -> bool { panic!("asked whether all home: {answer}") };
    assert_eq!(end(&ended(0.0, 0.0, RETREAT), 12.0, true, asked(true)), End::Returned(0.0));
    assert_eq!(end(&ended(12.0, 12.0, RETREAT), 12.0, false, asked(false)), End::Returned(12.0));
}

#[test]
fn a_retreat_whose_units_are_all_home_returns_wherever_its_front_stands() {
    assert_eq!(end(&ended(0.4, 0.0, RETREAT), 12.0, true, || true), End::Returned(0.0));
    assert_eq!(end(&ended(0.4, 0.0, RETREAT), 12.0, true, || false), End::Under);
}

#[test]
fn off_a_route_only_its_front_brings_a_retreat_home() {
    let never = || -> bool { panic!("asked off a route") };
    assert_eq!(end(&ended(0.4, 0.0, RETREAT), 12.0, false, never), End::Under);
}

#[test]
fn a_holding_squad_neither_arrives_nor_returns() {
    assert_eq!(end(&ended(12.0, 0.0, HOLD), 12.0, true, || true), End::Under);
    assert_eq!(end(&ended(0.0, 0.0, HOLD), 12.0, true, || true), End::Under);
}

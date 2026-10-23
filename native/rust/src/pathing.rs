//! Groups find their way (FormationPathing, spec 30 round 3, part 6b; spec 39 step 3c). A
//! marching group looks along its route ahead (path_look cells) as its pathfinder sees it -
//! the unit with the best wits (`pathfinder`), its sight shaped and obscured - for the
//! whole group as one walker (`group_walker`): as tall as its tallest, swimming or climbing
//! only if all of them can, at the least of their abilities. Where that stretch is barred,
//! or much slower than a way round (path_detour_gain), it plans the way round
//! (`paths::plan`) and gives it back for the command to keep as a patch to the route.
//!
//! Which groups look, and the patches their commands keep, are FormationPathing's in
//! GDScript until the tick ports (step 8): a squad's route and its command's patches are
//! not in the crate's store yet.

use crate::contest;
use crate::ground::Walker;
use crate::maths::V2;
use crate::paths::sight::Sight;
use crate::paths::{self, Eyes, Over, PathWalker};
use crate::route::Route;

/// A living unit of the group, as a look reads it: how it crosses the ground, its wits,
/// its id (for the battle's seeded draw), where it stands and, with a definition, its sight
/// ([detection range, sight_ahead, sight_side, sight_behind]).
#[derive(Clone, Copy, Debug)]
pub struct Member {
    pub walker: PathWalker,
    pub wits: i64,
    pub id: i64,
    pub at: V2,
    pub eyes: Option<[f64; 4]>,
}

/// The tuning a look reads: BattleTuning's path_look, path_detour_gain and sight_budget
/// (a climb's is the ground's, `Over`).
#[derive(Clone, Copy, Debug)]
pub struct LookTuning {
    pub look: f64,
    pub detour_gain: f64,
    pub sight_budget: f64,
}

/// The group as one walker: its tallest height, swimming or climbing only if every unit
/// can, at the least of their abilities, stamina and speed. A group with none standing
/// walks as a grounded walker a cell tall.
pub fn group_walker(members: &[Member]) -> PathWalker {
    let Some(first) = members.first() else {
        let grounded = Walker { height: 1.0, swimmer: 0, climber: 0, swims: false, climbs: false };
        return PathWalker::of(grounded);
    };
    let mut group = first.walker;
    for member in &members[1..] {
        let (w, g) = (&member.walker, &mut group);
        g.walker.height = g.walker.height.max(w.walker.height);
        g.walker.swimmer = g.walker.swimmer.min(w.walker.swimmer);
        g.walker.climber = g.walker.climber.min(w.walker.climber);
        g.walker.swims &= w.walker.swims;
        g.walker.climbs &= w.walker.climbs;
        g.stamina = g.stamina.min(w.stamina);
        g.speed = g.speed.min(w.speed);
    }
    group
}

/// Who finds the way for the group (SquadPathfinder): the living unit with the best wits,
/// whoever leads it. Equal wits go to the battle's seeded draw, never to ids or the order
/// the units are listed in (Decision 97). None with none standing.
pub fn pathfinder(members: &[Member], seed: i64) -> Option<usize> {
    let mut best: Option<(usize, (i64, i64))> = None;
    for (index, member) in members.iter().enumerate() {
        let key = (member.wits, contest::draw(seed, member.id));
        if best.is_none_or(|(_, best_key)| best_key < key) {
            best = Some((index, key));
        }
    }
    best.map(|(index, _)| index)
}

/// The way round a group facing `direction` along `route`, its front `front` cells along
/// it, should keep - reversed for a group walking it backwards - or None where its route
/// ahead is open enough, or no way round is better.
pub fn look_ahead(
    over: &mut Over,
    route: &Route,
    (front, direction): (f64, f64),
    members: &[Member],
    seed: i64,
    tuning: &LookTuning,
) -> Option<Vec<V2>> {
    let length = route.length();
    let ahead = (front + direction * tuning.look).clamp(0.0, length);
    let from = route.point_at(front);
    let to = route.point_at(ahead);
    let gap = from.distance_to(to) as f64;
    if gap < 1.0 {
        return None;
    }
    let walker = group_walker(members);
    let straight = paths::cost(over.ground, &walker.walker, &[from, to]);
    if straight <= gap * (1.0 + tuning.detour_gain) {
        return None; // open enough: it keeps to its route
    }
    let sight = pathfinder(members, seed).and_then(|index| {
        let finder = &members[index];
        finder.eyes.map(|[range, ahead, side, behind]| Sight {
            origin: finder.at,
            facing: to.sub(from).normalized(),
            ahead: range * ahead,
            side: range * side,
            behind: range * behind,
            hides: true,
            budget: tuning.sight_budget,
        })
    });
    let eyes = Eyes { sight: sight.as_ref(), memory: None };
    let plan = paths::plan(over, &walker, from, to, eyes);
    let mut way = plan.waypoints;
    if !plan.reaches || way.len() < 3 {
        return None;
    }
    let round = paths::cost(over.ground, &walker.walker, &way);
    if round >= straight * (1.0 - tuning.detour_gain) {
        return None; // no better than going straight on
    }
    if direction < 0.0 {
        way.reverse();
    }
    Some(way)
}

#[cfg(test)]
mod tests {
    //! From test_formation_pathing's group walker and test_squad_pathfinder (spec 39 step
    //! 3c); the looks through a battle stay gdUnit until the tick ports.

    use super::*;
    use crate::content::tests::repo_content;
    use crate::content::Content;
    use crate::ground::{Ground, GroundTuning, Paint};
    use crate::paths::shares::Cache;
    use crate::paths::PathTuning;

    fn member(walker: Walker, wits: i64, id: i64) -> Member {
        let eyes = Some([40.0, 1.0, 0.5, 0.25]);
        Member { walker: PathWalker::of(walker), wits, id, at: V2::new(10.0, 20.0), eyes }
    }

    fn grounded() -> Walker {
        Walker { height: 1.0, swimmer: 0, climber: 0, swims: false, climbs: false }
    }

    fn squad(wits: &[i64]) -> Vec<Member> {
        wits.iter().enumerate().map(|(i, &w)| member(grounded(), w, 100 + i as i64)).collect()
    }

    #[test]
    fn the_group_walks_as_its_most_restricted_unit() {
        let walkers = [
            Walker { height: 1.0, swimmer: 2, climber: 1, swims: true, climbs: true },
            Walker { height: 2.0, swimmer: 1, climber: 3, swims: true, climbs: false },
            Walker { height: 1.5, swimmer: 3, climber: 0, swims: true, climbs: true },
        ];
        let mut members: Vec<Member> = walkers.iter().map(|&w| member(w, 10, 1)).collect();
        members[1].walker.stamina = 50.0;
        let group = group_walker(&members);

        assert_eq!(group.walker.height, 2.0);
        assert_eq!((group.walker.swimmer, group.walker.climber), (1, 0));
        assert!(group.walker.swims && !group.walker.climbs);
        assert_eq!((group.stamina, group.speed), (50.0, 1.0));
        assert_eq!(group_walker(&[]).walker.height, 1.0);
    }

    #[test]
    fn the_wittiest_living_unit_finds_the_way() {
        assert_eq!(pathfinder(&squad(&[8, 14, 12]), 1), Some(1));
        assert_eq!(pathfinder(&squad(&[8, 12]), 1), Some(1));
        assert_eq!(pathfinder(&[], 1), None);
    }

    #[test]
    fn equal_wits_go_to_the_seeded_draw_not_the_list_order() {
        let mut picked = std::collections::BTreeSet::new();
        for seed in 1..21 {
            let members = squad(&[12, 12, 9]);
            let chosen = members[pathfinder(&members, seed).unwrap()].id;
            assert!(chosen == 100 || chosen == 101);
            let reversed: Vec<Member> = members.iter().rev().copied().collect();
            assert_eq!(reversed[pathfinder(&reversed, seed).unwrap()].id, chosen);
            picked.insert(chosen);
        }
        assert_eq!(picked.len(), 2);
    }

    /// Deep water across an east-west lane at x 30-33, open round its north end.
    fn pond(painted: bool) -> (Ground, LookTuning, PathTuning) {
        let content = Content::load(&repo_content()).unwrap_or_else(|error| panic!("{error}"));
        let t = &content.tuning;
        let mut ground = Ground::new(80, 40, GroundTuning::of(t));
        if painted {
            ground.paint((30, 8, 4, 32), &Paint { depth: Some(3.0), ..Paint::default() });
        }
        let tuning = LookTuning {
            look: t.path_look,
            detour_gain: t.path_detour_gain,
            sight_budget: t.sight_budget,
        };
        let climb = PathTuning {
            cliff_quarters: t.ground_cliff_quarters,
            stamina_climb: t.stamina_climb,
            stamina_swim: t.stamina_swim,
        };
        (ground, tuning, climb)
    }

    fn look(
        field: &(Ground, LookTuning, PathTuning),
        direction: f64,
        members: &[Member],
    ) -> Option<Vec<V2>> {
        let (ground, tuning, climb) = field;
        let lane = Route::new(vec![V2::new(2.0, 20.0), V2::new(78.0, 20.0)], 0.0);
        let mut cache = Cache::default();
        let mut over = Over { ground, cache: &mut cache, tuning: *climb };
        let front = if direction > 0.0 { 20.0 } else { 20.0 + tuning.look }; // the same stretch
        let standing: Vec<Member> = // at the group's front
            members.iter().map(|m| Member { at: lane.point_at(front), ..*m }).collect();
        look_ahead(&mut over, &lane, (front, direction), &standing, 7, tuning)
    }

    #[test]
    fn a_barred_route_ahead_has_a_way_round_and_an_open_one_none() {
        let field = pond(true);
        let members = squad(&[10, 12]);
        let way = look(&field, 1.0, &members).expect("a way round the pond");

        assert!(way.len() >= 3);
        assert!(way.iter().all(|at| field.0.crossing(&grounded(), *at, *at) > 0.0));
        assert_eq!(look(&pond(false), 1.0, &members), None);
    }

    #[test]
    fn the_way_round_is_the_same_whatever_the_order_of_the_units() {
        let field = pond(true);
        let members = squad(&[12, 12, 9]);
        let reversed: Vec<Member> = members.iter().rev().copied().collect();

        assert_eq!(look(&field, 1.0, &members), look(&field, 1.0, &reversed));
    }

    #[test]
    fn a_group_walking_its_route_backwards_keeps_the_way_round_reversed() {
        let way = look(&pond(true), -1.0, &squad(&[10])).expect("a way round the pond");

        assert!(way.first().unwrap().x < way.last().unwrap().x); // along the route, as patches run
    }
}

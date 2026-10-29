//! The march's sync (FormationMarch.sync_units) on a squad built here: squad 1, one column
//! wide, at front 5.0 on no route, heading 90, anchored at (10, 4); its units one cell each,
//! unit 100 in its front rank, 101 behind it, both at speed 1.

use super::*;
use crate::entries;
use crate::maths::bearing_vector;
use crate::squads::DOWNED;

const HEADING: f64 = 90.0;
/// [seconds, cells a second at speed 1], as FormationSimulation's.
const TIMING: [f64; 2] = [0.1, 1.0];

struct Built {
    squads: Squads,
    field: Field,
}

fn built() -> Built {
    let (mut squads, mut field) = (Squads::default(), Field::default());
    squads.open(1);
    squads.width[0] = 1;
    squads.direction[0] = 1;
    squads.front_distance[0] = 5.0;
    squads.heading[0] = HEADING;
    squads.position[0] = V2::new(10.0, 4.0);
    for (rank, id) in [100, 101].into_iter().enumerate() {
        let slot = field.hold(id) as usize;
        field.columns.rank[slot] = rank as i32;
        field.columns.speed[slot] = 1.0;
        field.columns.stamina[slot] = 100.0;
        field.columns.sheet[slot].footprint_width = 1;
        field.columns.sheet[slot].footprint_depth = 1;
        squads.members[0].push(slot as i32);
    }
    Built { squads, field }
}

impl Built {
    fn slot(&mut self, id: i64) -> usize {
        self.field.hold(id) as usize
    }

    /// One sync: placing if `timing` is `None`, else a tick's walk on open ground.
    fn sync(&mut self, timing: Option<[f64; 2]>) {
        let tuning = crate::march::tests::walk_tuning();
        let given = Given { ground: None, timing, seed: 3, wounds_reach: 1.0, tuning };
        step(&self.squads, &mut self.field, &[0], &given);
    }

    /// Where the unit in the squad's frame at `rank` belongs: half a cell behind its rank's
    /// line through the anchor.
    fn place(&self, rank: f64) -> V2 {
        self.squads.position[0].sub(bearing_vector(HEADING).scale(rank + 0.5))
    }
}

#[test]
fn placing_sets_each_unit_on_its_place_facing_the_heading_its_distance_its_rank_s() {
    let mut b = built();
    let (front, back) = (b.slot(100), b.slot(101));

    b.sync(None);

    let c = &b.field.columns;
    assert_eq!((c.position[front], c.position[back]), (b.place(0.0), b.place(1.0)));
    assert_eq!((c.foothold[front], c.foothold[back]), (b.place(0.0), b.place(1.0)));
    assert_eq!((c.bearing[front], c.bearing[back]), (HEADING, HEADING));
    assert_eq!((c.distance[front], c.distance[back]), (5.0, 5.0 - RANK_DEPTH));
}

#[test]
fn a_router_stands_on_its_point_on_the_route_facing_the_heading() {
    let mut b = built();
    let slot = b.slot(101);
    entries::flee(&mut b.squads, &mut b.field.columns, 0, (101, slot), 3.0, V2::new(0.0, 1.0));

    b.sync(Some(TIMING));

    let c = &b.field.columns;
    assert_eq!(c.position[slot], V2::new(3.0, 1.0), "no route: along x, then its offset");
    assert_eq!(c.bearing[slot], HEADING);
}

#[test]
fn a_loose_unit_stands_where_its_own_moves_took_it() {
    let mut b = built();
    let slot = b.slot(100);
    b.field.columns.bearing[slot] = 45.0;
    entries::loosen(&mut b.squads, &mut b.field.columns, 0, (100, slot), V2::new(7.0, 7.0));

    b.sync(Some(TIMING));

    let c = &b.field.columns;
    assert_eq!((c.position[slot], c.bearing[slot]), (V2::new(7.0, 7.0), 45.0));
}

#[test]
fn a_unit_in_its_place_walks_towards_it() {
    let mut b = built();
    let slot = b.slot(100);
    let from = V2::new(10.0, 0.0);
    b.field.columns.position[slot] = from;
    b.field.columns.foothold[slot] = from;
    let place = b.place(0.0);
    let gap = |at: V2| at.sub(place).length();

    b.sync(Some(TIMING));

    let at = b.field.columns.position[slot];
    assert!(gap(at) < gap(from), "it walked nearer its place: {at:?}");
    assert!(gap(at) > 0.0, "a tick's walk, not placed");
}

#[test]
fn the_fallen_lie_where_they_fell() {
    let mut b = built();
    let slot = b.slot(101);
    b.field.columns.state[slot] = DOWNED;
    b.field.columns.position[slot] = V2::new(1.0, 1.0);

    b.sync(None);

    let c = &b.field.columns;
    assert_eq!((c.position[slot], c.distance[slot]), (V2::new(1.0, 1.0), 0.0));
}

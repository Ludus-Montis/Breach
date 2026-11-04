//! The pass's scenarios (Decision 106, spec 30 round 1), from test_body_parting.gd, on
//! squads built here: each squad n (slot n, id n + 1) of one faction, its grems placed by
//! hand; the bodies' rules are `parting`'s own (`parting/tests.rs`).

use super::*;
use crate::content::tests::repo_content;
use crate::content::Content;

const SEED: i64 = 9;

fn tuning() -> Tuning {
    let content = Content::load(&repo_content()).unwrap_or_else(|error| panic!("{error}"));
    let t = &content.tuning;
    Tuning { passes: t.bodies_passes, resist: t.bodies_resist, brush: t.bodies_brush }
}

#[derive(Default)]
struct Built {
    squads: Squads,
    field: Field,
}

impl Built {
    /// A squad with nobody yet: its slot.
    fn squad(&mut self) -> usize {
        let s = self.squads.id.len();
        self.squads.open(s as i64 + 1);
        s
    }

    /// Unit `id` of the squad in slot `s`, in its frame at `at`: its store slot.
    fn unit(&mut self, s: usize, id: i64, at: V2) -> usize {
        let slot = self.field.hold(id) as usize;
        self.field.columns.position[slot] = at;
        self.squads.members[s].push(slot as i32);
        slot
    }

    fn loose(&mut self, s: usize, id: i64, at: V2) -> usize {
        let slot = self.unit(s, id, at);
        entries::loosen(&mut self.squads, &mut self.field.columns, s, (id, slot), at);
        slot
    }

    fn part(&mut self) -> FieldOutcome {
        let listed: Vec<usize> = (0..self.squads.id.len()).collect();
        step(&mut self.squads, &mut self.field, &listed, SEED, (&tuning(), false))
    }
}

#[test]
fn a_unit_its_own_loose_one_pushes_from_its_place_goes_loose_where_it_stood() {
    let mut b = Built::default();
    let s = b.squad();
    let framed = b.unit(s, 100, V2::new(5.0, 5.0));
    b.loose(s, 101, V2::new(5.5, 5.0));

    b.part();

    assert_eq!(b.squads.loose[s], vec![101, 100]);
    assert_eq!(b.field.columns.position[framed], V2::new(5.0, 5.0)); // its place, to walk back to
    assert!(b.field.columns.loose_at[framed].x < 5.0, "pushed off it");
}

#[test]
fn a_router_pushes_through_from_its_point_on_its_squads_route() {
    let mut b = Built::default();
    let (fleeing, other) = (b.squad(), b.squad());
    b.squads.state[fleeing] = ROUTING;
    let router = b.unit(fleeing, 100, V2::ZERO);
    let columns = &mut b.field.columns;
    entries::flee(&mut b.squads, columns, fleeing, (100, router), 4.0, V2::ZERO);
    let friend = b.loose(other, 200, V2::new(4.4, 0.0));

    b.part();

    assert!(b.field.columns.loose_at[friend].x > 4.4, "pushed by a router at (4, 0)");
}

#[test]
fn a_destroyed_squads_bodies_are_left() {
    let mut b = Built::default();
    let (gone, other) = (b.squad(), b.squad());
    b.squads.state[gone] = DESTROYED;
    b.loose(gone, 100, V2::new(4.0, 0.0));
    let friend = b.loose(other, 200, V2::new(4.4, 0.0));

    let out = b.part();

    assert_eq!((out.moved.len(), b.field.columns.loose_at[friend]), (0, V2::new(4.4, 0.0)));
}

#[test]
fn bodies_on_one_spot_part_along_the_seeded_way() {
    let mut b = Built::default();
    let s = b.squad();
    let one = b.loose(s, 100, V2::new(4.0, 4.0));
    let two = b.loose(s, 101, V2::new(4.0, 4.0));

    b.part();

    let (at, other) = (b.field.columns.loose_at[one], b.field.columns.loose_at[two]);
    let apart = V2::new(at.x - other.x, at.y - other.y);
    let ways = [
        way(SEED, contest::draw(SEED, 100), contest::draw(SEED, 101)),
        way(SEED, contest::draw(SEED, 101), contest::draw(SEED, 100)),
    ];
    let along = |w: V2| ((apart.x * w.y - apart.y * w.x) as f64).abs() < 0.001;
    assert!(apart.x.abs() + apart.y.abs() > 0.5, "parted");
    assert!(ways.iter().any(|&w| along(w)), "along a seeded way, not {apart:?}");
}

//! Recovery's landing (Decisions 121, 126 and 127), from test_formation_recovery.gd, on
//! squads built here: squad 1 (listed 0) and squad 2 (listed 1), both "player"; each unit at
//! 10 hp of 10, seeing 40 cells, a downed one once wounded at 0 hp and due to wake this tick.
//! Ticks of 0.1 s under seed 3.

use super::*;
use crate::content::tests::repo_content;
use crate::content::Content;
use crate::entries::LOOSE;
use crate::maths::V2;
use crate::squads::DOWNED;

const TICK: i64 = 10;

struct Built {
    squads: Squads,
    field: Field,
    wounds: WoundsTuning,
}

fn built() -> Built {
    let (mut squads, mut field) = (Squads::default(), Field::default());
    let player = field.lexicon.code("player");
    for s in 0..2 {
        squads.open(s as i64 + 1);
        squads.faction[s] = player;
    }
    let content = Content::load(&repo_content()).unwrap_or_else(|error| panic!("{error}"));
    Built { squads, field, wounds: WoundsTuning::of(&content.tuning) }
}

impl Built {
    fn slot(&self, id: i64) -> usize {
        self.field.slot_of(id).unwrap() as usize
    }

    /// Unit `id` standing in the squad in slot `s` at `at`.
    fn standing(&mut self, s: usize, id: i64, at: (f32, f32)) -> usize {
        let slot = self.field.hold(id) as usize;
        let faction = self.squads.faction[s];
        let columns = &mut self.field.columns;
        columns.position[slot] = V2::new(at.0, at.1);
        columns.hp[slot] = 10;
        columns.sheet[slot].max_hp = 10;
        columns.sheet[slot].squad_id = self.squads.id[s];
        columns.words[slot].faction_id = faction;
        self.squads.members[s].push(slot as i32);
        slot
    }

    /// Unit `id` downed in the squad in slot `s` at `at`.
    fn downed(&mut self, s: usize, id: i64, at: (f32, f32)) {
        let slot = self.standing(s, id, at);
        let columns = &mut self.field.columns;
        columns.hp[slot] = 0;
        columns.state[slot] = DOWNED;
        columns.sheet[slot].wounded = 1;
        columns.wake_left[slot] = 0.0;
    }

    /// The pass over both squads: what landed and what it logged.
    fn recover(&mut self) -> (Vec<Landed>, Vec<i64>) {
        let mut log = Log::default();
        let at = (TICK, 0.1, 3);
        let landed = step(&mut self.squads, &mut self.field, &[0, 1], at, &self.wounds, &mut log);
        (landed, log.take())
    }

    fn event(&self, kind: usize, s: usize, values: &[i64]) -> Vec<i64> {
        let head = [kind as i64, TICK, self.squads.id[s], i64::from(self.squads.faction[s])];
        [&head[..], values].concat()
    }
}

#[test]
fn one_coming_to_rejoins_its_squad_behind_its_rearmost_loose_where_it_lies() {
    let mut b = built();
    b.standing(0, 1, (0.0, 0.0));
    b.downed(0, 2, (0.0, 2.0));

    let (landed, logged) = b.recover();

    let slot = b.slot(2);
    let columns = &b.field.columns;
    assert_eq!((columns.state[slot], columns.hp[slot]), (MOVING, 1));
    assert_eq!((columns.rank[slot], columns.column[slot]), (1, 0));
    assert_eq!((b.squads.loose[0].clone(), columns.flags[slot] & LOOSE), (vec![2], LOOSE));
    assert_eq!(columns.loose_at[slot], V2::new(0.0, 2.0));
    assert_eq!(b.squads.reforming[0], 1);
    assert_eq!(landed, vec![Landed { listed: 0, id: 2, came: Came::Joined(0) }]);
    assert_eq!(logged, b.event(events::CAME_TO, 0, &[2, 0, 1]));
}

#[test]
fn one_coming_to_with_its_own_gone_joins_a_friendly_squad_in_sight() {
    let mut b = built();
    b.squads.command[0] = Some(5);
    let leader = b.standing(0, 1, (0.0, 0.0));
    b.field.columns.sheet[leader].footprint_depth = 2;
    b.downed(1, 2, (0.0, 2.0));
    let slot = b.slot(2);
    b.squads.loose[1].push(2);
    b.field.columns.flags[slot] |= LOOSE;

    let (landed, logged) = b.recover();

    assert_eq!((b.squads.members[0].clone(), b.squads.members[1].clone()), (vec![0, 1], vec![]));
    assert_eq!((b.squads.loose[0].clone(), b.squads.loose[1].clone()), (vec![2], vec![]));
    let objects = &b.field.columns.objects[slot];
    assert_eq!((b.field.columns.sheet[slot].squad_id, objects.command, objects.origin), (1, 5, 5));
    assert_eq!(b.field.columns.rank[slot], 2);
    assert!(b.squads.joined[0].is_empty());
    assert_eq!(landed, vec![Landed { listed: 1, id: 2, came: Came::Joined(0) }]);
    assert_eq!(logged, b.event(events::CAME_TO, 1, &[2, 0, 1]));
}

#[test]
fn with_its_only_friends_routing_it_comes_to_alone() {
    let mut b = built();
    b.standing(0, 1, (0.0, 0.0));
    b.squads.state[0] = ROUTING;
    b.downed(1, 2, (0.0, 2.0));

    let (landed, logged) = b.recover();

    assert_eq!(b.field.columns.state[b.slot(2)], MOVING);
    assert_eq!(b.squads.members[1], vec![1]);
    assert!(b.squads.loose[1].is_empty() && b.squads.reforming[1] == 0);
    assert_eq!(landed, vec![Landed { listed: 1, id: 2, came: Came::Alone }]);
    assert_eq!(logged, b.event(events::CAME_TO, 1, &[2, 1, -1]));
}

#[test]
fn one_bled_out_dies_of_its_wounds() {
    let mut b = built();
    b.downed(0, 1, (0.0, 0.0));
    let slot = b.slot(1);
    b.field.columns.sheet[slot].wounded = 10;
    b.field.columns.hp[slot] = -100;
    b.field.columns.drain_carry[slot] = 0.99; // a whole point drains this tick

    let (landed, logged) = b.recover();

    assert_eq!(b.field.columns.state[slot], DEAD);
    assert_eq!(landed, vec![Landed { listed: 0, id: 1, came: Came::DiedOfWounds }]);
    assert_eq!(logged, b.event(events::DIED_OF_WOUNDS, 0, &[1]));
}

#[test]
fn a_cunning_one_with_a_foe_near_plays_dead() {
    let mut b = built();
    b.downed(0, 1, (0.0, 0.0));
    let foe = b.standing(1, 2, (1.0, 0.0));
    b.field.columns.words[foe].faction_id = b.field.lexicon.code("the_kingdom");
    let cunning = b.field.lexicon.code("cunning");
    let slot = b.slot(1);
    b.field.columns.words[slot].traits.push((cunning, 1000.0));

    let (landed, logged) = b.recover();

    assert!(b.field.columns.sheet[slot].playing_dead);
    assert_eq!(b.field.columns.state[slot], DOWNED);
    assert_eq!(landed, vec![Landed { listed: 0, id: 1, came: Came::PlayingDead }]);
    assert_eq!(logged, b.event(events::PLAYING_DEAD, 0, &[1]));
}

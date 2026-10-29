//! The tending's scenarios (Decision 121), from test_formation_wounds.gd, on squads built
//! here: squad 1 (listed 0, "the_kingdom"), unit 1 downed at the origin; squad 2 (listed 1,
//! "player"), unit 2 a cell off, striking for 4 a second; squad 3 (listed 2, "the_kingdom"),
//! unit 3 standing far off, at morale 75. Each at 10 hp, constitution 6; ten ticks a second.

use super::*;
use crate::content::tests::repo_content;
use crate::content::Content;
use crate::maths::V2;

const TICK: i64 = 10;
const SECOND: i64 = 10;
const SEED: i64 = 1;
const SHOCK: i64 = 40;

fn bands() -> BandTuning {
    BandTuning { per_leadership: 5, steady_at: 70, shaken_at: 40, loss: 2, leader_loss: 3 }
}

struct Built {
    squads: Squads,
    field: Field,
    wounds: WoundsTuning,
}

fn built() -> Built {
    let (mut squads, mut field) = (Squads::default(), Field::default());
    let kingdom = field.lexicon.code("the_kingdom");
    let factions = [kingdom, field.lexicon.code("player"), kingdom];
    let constitution = field.lexicon.code("constitution");
    let at = [(0.0, 0.0), (1.0, 0.0), (200.0, 0.0)];
    for s in 0..3 {
        squads.open(s as i64 + 1);
        squads.faction[s] = factions[s];
        let slot = field.hold(s as i64 + 1) as usize;
        field.columns.position[slot] = V2::new(at[s].0, at[s].1);
        field.columns.hp[slot] = 10;
        field.columns.words[slot].faction_id = factions[s];
        field.columns.words[slot].attributes.push((constitution, 6.0));
        field.columns.sheet[slot].dmg = 4;
        squads.members[s].push(slot as i32);
    }
    squads.morale[2] = 75;
    let body = field.slot_of(1).unwrap() as usize;
    field.columns.hp[body] = 0;
    field.columns.state[body] = DOWNED;
    let content = Content::load(&repo_content()).unwrap_or_else(|error| panic!("{error}"));
    Built { squads, field, wounds: WoundsTuning::of(&content.tuning) }
}

impl Built {
    fn slot(&self, id: i64) -> usize {
        self.field.slot_of(id).unwrap() as usize
    }

    /// The pass over the three squads on `tick`: what landed and what it logged.
    fn tend(&mut self, tick: i64) -> (Vec<Landed>, Vec<i64>) {
        let mut log = Log::default();
        let bands = bands();
        let t = Tuning { wounds: &self.wounds, shock: SHOCK, bands: &bands };
        let listed = [0, 1, 2];
        let at = (tick, SECOND, SEED);
        let landed = step(&mut self.squads, &mut self.field, &listed, at, &t, &mut log);
        (landed, log.take())
    }

    fn said(&mut self, id: i64, name: &str, level: f64) {
        let (code, slot) = (self.field.lexicon.code(name), self.slot(id));
        self.field.columns.words[slot].traits.push((code, level));
    }

    /// Unit 4 of squad 2, a leader 30 cells off with a messenger.
    fn leader(&mut self) {
        let slot = self.field.hold(4) as usize;
        self.field.columns.position[slot] = V2::new(30.0, 0.0);
        self.field.columns.hp[slot] = 10;
        self.field.columns.words[slot].faction_id = self.squads.faction[1];
        self.field.columns.sheet[slot].leadership = 2;
        self.squads.members[1].push(slot as i32);
        self.said(4, "messenger", 1.0);
    }

    fn event(&self, kind: usize, s: usize, values: &[i64]) -> Vec<i64> {
        let head = [kind as i64, TICK, self.squads.id[s], i64::from(self.squads.faction[s])];
        [&head[..], values].concat()
    }
}

#[test]
fn a_captor_captures_the_body_and_tells_of_it() {
    let mut b = built();
    b.said(2, "captor", 1.0);

    let (landed, logged) = b.tend(TICK);

    assert_eq!(b.field.columns.state[b.slot(1)], TAKEN);
    assert_eq!(landed, vec![Landed { listed: 0, id: 1, state: TAKEN, leader: None }]);
    assert_eq!(logged, b.event(events::CAPTURED, 0, &[1, 2]));
}

#[test]
fn a_leader_sends_it_home_spends_a_messenger_and_shakes_its_side_standing() {
    let mut b = built();
    b.leader();

    let (landed, logged) = b.tend(TICK);

    assert_eq!(b.field.columns.state[b.slot(1)], RELEASED);
    assert_eq!(b.field.columns.words[b.slot(4)].traits[0].1, 0.0);
    assert_eq!(landed, vec![Landed { listed: 0, id: 1, state: RELEASED, leader: Some((1, 4)) }]);
    assert_eq!(b.squads.morale[2], 35);
    let shaken = b.event(events::MORALE_BAND, 2, &[2, 35]);
    assert_eq!(logged, [b.event(events::SENT_HOME, 0, &[1]), shaken].concat());
}

#[test]
fn an_unguarded_body_is_struck_till_it_is_finished() {
    let mut b = built();
    let mut finished = vec![];
    for _ in 0..4 * SECOND {
        let (landed, logged) = b.tend(TICK);
        if !landed.is_empty() {
            finished = vec![(landed, logged)];
            break;
        }
    }

    assert_eq!(b.field.columns.state[b.slot(1)], DEAD);
    let landed = vec![Landed { listed: 0, id: 1, state: DEAD, leader: None }];
    assert_eq!(finished, vec![(landed, b.event(events::FINISHED, 0, &[1]))]);
}

#[test]
fn a_borne_body_is_not_taken() {
    let mut b = built();
    b.said(2, "captor", 1.0);
    let body = b.slot(1);
    b.field.columns.state[body] = 8; // CARRIED

    let (landed, logged) = b.tend(TICK);

    assert!(landed.is_empty() && logged.is_empty());
}

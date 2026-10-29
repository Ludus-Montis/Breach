//! The burial's scenarios (Decisions 47, 82, 121 and 145), from test_unit_health.gd,
//! test_formation_wounds.gd and test_formation_morale.gd, on squads built here: squad 1
//! (listed 0) two abreast, units 100 and 101 in its front rank and 102 and 103 behind; squad
//! 2 (listed 1), units 200 and 201 abreast; each unit at 10 hp, constitution 6, courage 50.

use super::*;
use crate::locks::FIGHTING;

const TICK: i64 = 5;

fn tuning() -> BandTuning {
    BandTuning { per_leadership: 5, steady_at: 70, shaken_at: 40, loss: 2, leader_loss: 3 }
}

struct Built {
    squads: Squads,
    field: Field,
}

fn built() -> Built {
    let (mut squads, mut field) = (Squads::default(), Field::default());
    let constitution = field.lexicon.code("constitution");
    for (s, ids) in [&[100, 101, 102, 103][..], &[200, 201][..]].into_iter().enumerate() {
        squads.open(s as i64 + 1);
        squads.width[s] = 2;
        squads.direction[s] = 1;
        for (n, &id) in ids.iter().enumerate() {
            let slot = field.hold(id) as usize;
            field.columns.rank[slot] = n as i32 / 2;
            field.columns.column[slot] = n as i32 % 2;
            field.columns.hp[slot] = 10;
            field.columns.sheet[slot].squad_id = s as i64 + 1;
            field.columns.sheet[slot].courage = 50;
            field.columns.words[slot].attributes = vec![(constitution, 6.0)];
            squads.members[s].push(slot as i32);
        }
    }
    Built { squads, field }
}

impl Built {
    fn slot(&self, id: i64) -> usize {
        self.field.slot_of(id).unwrap() as usize
    }

    /// The unit `id` brought to `hp` by a write that felled it.
    fn falls(&mut self, id: i64, hp: i32) {
        let slot = self.slot(id);
        self.field.columns.hp[slot] = hp;
        self.field.fell(id);
    }

    /// The burial over both squads; what it buried and logged.
    fn bury(&mut self) -> (Vec<Buried>, Vec<i64>) {
        let mut log = Log::default();
        let buried = bury(&mut self.squads, &mut self.field, &[0, 1], (&tuning(), TICK), &mut log);
        (buried, log.take())
    }

    /// The events logged of `kind`, each its squad's id, then its fields.
    fn told(logged: &[i64], kind: usize) -> Vec<Vec<i64>> {
        let mut out = vec![];
        let mut at = 0;
        while at < logged.len() {
            let fields = events::KINDS[logged[at] as usize].1.len();
            if logged[at] == kind as i64 {
                let mut told = vec![logged[at + 2]];
                told.extend_from_slice(&logged[at + 4..at + 4 + fields]);
                out.push(told);
            }
            at += 4 + fields;
        }
        out
    }
}

#[test]
fn the_fallen_are_buried_in_the_lists_order_once_each_past_constitution_dying() {
    let mut b = built();
    b.falls(201, -1); // falls first, but squad 2 is listed second
    b.falls(103, -6); // past its constitution of 6
    b.falls(102, 0);
    b.falls(201, -1); // fell twice: buried once
    b.falls(200, 0);
    let healed = b.slot(200);
    b.field.columns.hp[healed] = 1; // healed above 0 again
    b.falls(101, 0);
    let taken = b.slot(101);
    b.field.columns.state[taken] = 6; // TAKEN: no longer living

    let (buried, logged) = b.bury();

    let said: Vec<(usize, i64, Option<i64>)> =
        buried.iter().map(|x| (x.listed, x.id, x.wounded)).collect();
    assert_eq!(said, vec![(0, 102, Some(1)), (0, 103, None), (1, 201, Some(1))]);
    let died = Built::told(&logged, events::DIED);
    let downed = Built::told(&logged, events::DOWNED);
    assert_eq!((died, downed), (vec![vec![1, 103]], vec![vec![1, 102], vec![2, 201]]));
    let (s102, s103, s201) = (b.slot(102), b.slot(103), b.slot(201));
    let states = [s102, s103, s201].map(|s| b.field.columns.state[s]);
    assert_eq!(states, [DOWNED, DEAD, DOWNED]);
    assert_eq!(b.field.columns.hp[s201], 0); // the excess lost
    assert_eq!(b.field.columns.sheet[s102].wounded, 1);
    assert!(b.field.take_fallen().is_empty());
}

#[test]
fn a_unit_moved_since_it_fell_is_buried_with_the_squad_it_is_in_now() {
    let mut b = built();
    b.falls(101, 0);
    let slot = b.slot(101) as i32;
    b.squads.members[0].retain(|&u| u != slot);
    b.squads.members[1].push(slot); // its sheet still says squad 1

    let (buried, _) = b.bury();

    assert_eq!(buried.iter().map(|x| (x.listed, x.id)).collect::<Vec<_>>(), vec![(1, 101)]);
}

#[test]
fn a_fallen_leader_is_reported_and_the_losses_shock_the_squad_from_its_ceiling() {
    let mut b = built();
    let leader = b.slot(100);
    b.field.columns.sheet[leader].leadership = 3;
    b.falls(100, 0);
    b.falls(101, 0);

    let (_, logged) = b.bury();

    assert_eq!(Built::told(&logged, events::LEADER_FELL), vec![vec![1, 100, 3]]);
    // the ceiling of those left (courage 50, no leader): 50, less 2 losses and 3 per point
    assert_eq!(b.squads.morale[0], 50 - 2 * 2 - 3 * 3);
    let band = vec![vec![1, crate::bands::WAVERING, 37]]; // from shaken
    assert_eq!(Built::told(&logged, events::MORALE_BAND), band);
}

#[test]
fn the_ranks_behind_step_up_and_the_squad_reforms() {
    let mut b = built();
    b.falls(100, 0);

    let (_, logged) = b.bury();

    assert_eq!(Built::told(&logged, events::STEPPED_UP), vec![vec![1, 102, 0]]);
    assert_eq!(b.field.columns.rank[b.slot(102)], 0);
    assert_eq!((b.squads.reforming[0], b.squads.reforming[1]), (1, 0));
}

#[test]
fn a_squad_with_no_one_left_is_destroyed_and_frees_whoever_fought_it() {
    let mut b = built();
    b.squads.state[0] = FIGHTING;
    b.squads.engaged_with[0] = 2;
    b.squads.state[1] = FIGHTING;
    b.squads.engaged_with[1] = 1;
    b.falls(200, -9);
    b.falls(201, -9);

    let (_, logged) = b.bury();

    assert_eq!(b.squads.state[1], DESTROYED);
    assert_eq!((b.squads.engaged_with[0], b.squads.engaged_with[1]), (0, 0));
    assert_eq!(Built::told(&logged, events::DESTROYED), vec![vec![2]]);
    assert_eq!(Built::kinds(&logged).last(), Some(&(events::DESTROYED as i64)));
    // after the deaths
}

impl Built {
    /// The kinds logged, in order.
    fn kinds(logged: &[i64]) -> Vec<i64> {
        let mut out = vec![];
        let mut at = 0;
        while at < logged.len() {
            out.push(logged[at]);
            at += 4 + events::KINDS[logged[at] as usize].1.len();
        }
        out
    }
}

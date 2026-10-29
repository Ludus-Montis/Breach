//! Bearing the wounded as one pass (Decision 126), from test_formation_carry.gd, on squads
//! built here: the player's, holding and told to tend their downed, each unit in the next
//! store slot of definition 0 (speed 2), strength 10, no gear, one cell across.

use super::*;
use crate::content::tests::repo_content;
use crate::content::Content;
use crate::maths::V2;

const TICK: i64 = 4;
const SEED: i64 = 7;

fn tuning() -> (CarryTuning, LoadTuning) {
    let content = Content::load(&repo_content()).unwrap_or_else(|error| panic!("{error}"));
    let t = &content.tuning;
    let loads = LoadTuning {
        pace: t.load_pace.clone(),
        tiring: t.load_tiring.clone(),
        dodge: t.load_dodge.clone(),
    };
    (CarryTuning::of(t), loads)
}

#[derive(Default)]
struct Built {
    squads: Squads,
    field: Field,
    listed: Vec<usize>,
}

impl Built {
    /// A player's formation holding, that tends its downed by `tends`.
    fn formation(&mut self, tends: &str) -> usize {
        let n = self.listed.len();
        let s = self.squads.open(n as i64 + 1) as usize;
        self.squads.faction[s] = self.field.lexicon.code("player");
        self.squads.state[s] = locks::HOLDING;
        let command = self.squads.commands.open(0.0);
        self.squads.commands.tends[command] = tends.to_string();
        self.squads.command[s] = Some(command);
        self.listed.push(s);
        self.field.put_speed(0, 2.0);
        n
    }

    /// A unit of the listed squad `n` at `at`, in `state`.
    fn unit(&mut self, n: usize, id: i64, at: (f32, f32), state: i32) -> usize {
        let slot = self.field.hold(id) as usize;
        let s = self.listed[n];
        let columns = &mut self.field.columns;
        columns.position[slot] = V2::new(at.0, at.1);
        columns.state[slot] = state;
        columns.words[slot].faction_id = self.squads.faction[s];
        columns.objects[slot].definition = 0;
        self.squads.members[s].push(slot as i32);
        slot
    }

    /// The pass, given those that fled home: what it did and what it logged.
    fn carry(&mut self, homes: &[i64]) -> (Carried, Vec<i64>) {
        let (t, loads) = tuning();
        let mut log = Log::default();
        let (squads, field) = (&mut self.squads, &mut self.field);
        let out = step(squads, field, (&self.listed, homes), (TICK, SEED), (&t, &loads), &mut log);
        (out, log.take())
    }

    fn event(&self, kind: usize, n: usize, values: &[i64]) -> Vec<i64> {
        let s = self.listed[n];
        let head = [kind as i64, TICK, self.squads.id[s], i64::from(self.squads.faction[s])];
        [&head[..], values].concat()
    }
}

/// A bearer of one cell's body: its stage, pace, tiring and dodge on the repo's tuning.
fn laden() -> (i64, f64, f64, f64) {
    let (t, loads) = tuning();
    let bearer = Bearer {
        id: 1,
        slot: 0,
        faction: 0,
        carrying: false,
        gear_weight: 0.0,
        strength: 10,
        hauler: 0,
    };
    let at = stage(&bearer, Some(&Body { id: 2, slot: 1, faction: 0, cells: 1 }), &t);
    (at as i64, 2.0 * loads.pace[at], loads.tiring[at], loads.dodge[at])
}

#[test]
fn a_free_unit_beside_a_friendly_body_bears_it_and_is_slowed_by_its_load() {
    let mut b = Built::default();
    let mine = b.formation("carry");
    let bearer = b.unit(mine, 1, (0.0, 0.0), locks::HOLDING);
    let body = b.unit(mine, 2, (0.5, 0.0), DOWNED);
    let (out, log) = b.carry(&[]);
    assert_eq!(log, b.event(events::BORNE, mine, &[1, 2]));
    assert_eq!(out, Carried { changed: vec![(0, 2), (0, 1)], ..Carried::default() });
    let columns = &b.field.columns;
    assert_eq!(columns.state[body], CARRIED);
    let (sheet, borne) = (&columns.sheet[bearer], &columns.sheet[body]);
    assert_eq!((sheet.carrying, borne.carried_by), (2, 1));
    let load = (sheet.load_stage, sheet.fresh_speed, sheet.tiring, sheet.dodging);
    assert_eq!(load, laden());
}

#[test]
fn a_bearer_told_to_recover_sets_off_home_alone() {
    let mut b = Built::default();
    let mine = b.formation("recover");
    b.unit(mine, 1, (0.0, 0.0), locks::HOLDING);
    b.unit(mine, 2, (0.5, 0.0), DOWNED);
    let (out, _) = b.carry(&[]);
    assert_eq!(out.strays, vec![(0, 1)]);
}

#[test]
fn a_borne_body_follows_its_bearer_and_is_dropped_when_it_falls() {
    let mut b = Built::default();
    let mine = b.formation("");
    let bearer = b.unit(mine, 1, (3.0, 1.0), locks::HOLDING);
    let body = b.unit(mine, 2, (0.0, 0.0), CARRIED);
    b.field.columns.sheet[bearer].carrying = 2;
    b.field.columns.sheet[body].carried_by = 1;
    let (out, _) = b.carry(&[]);
    assert_eq!(out.changed, vec![]);
    assert_eq!(b.field.columns.position[body], V2::new(3.0, 1.0));
    b.field.columns.state[bearer] = DOWNED;
    let (out, _) = b.carry(&[]);
    assert_eq!(out.changed, vec![(0, 2), (0, 1)]);
    let columns = &b.field.columns;
    assert_eq!(columns.state[body], DOWNED);
    let sheets = (&columns.sheet[bearer], &columns.sheet[body]);
    assert_eq!((sheets.0.carrying, sheets.1.carried_by), (0, 0));
    let (_, loads) = tuning();
    assert_eq!(columns.sheet[bearer].fresh_speed, 2.0 * loads.pace[0]);
}

#[test]
fn a_body_whose_bearer_fled_home_comes_home_too() {
    let mut b = Built::default();
    let mine = b.formation("");
    let body = b.unit(mine, 2, (0.0, 0.0), CARRIED);
    b.field.columns.sheet[body].carried_by = 1;
    b.field.columns.objects[body].definition = 3;
    let (out, log) = b.carry(&[1]);
    assert_eq!(log, b.event(events::FLED_HOME, mine, &[2, 3]));
    assert_eq!(out.home, vec![(0, 2)]);
    assert_eq!(out.changed, vec![(0, 2)]);
    assert_eq!(b.squads.members[b.listed[mine]], Vec::<i32>::new());
}

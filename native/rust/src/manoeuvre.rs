//! A formation's current manoeuvre (FormationManoeuvre, Decisions 94 and 99; spec 39 step
//! 4b): it always has one, and the highest-priority one that applies wins - tactical
//! decisions, its own, before the player's order: combat (locked in melee, front or flank,
//! or skirmishing), withdrawing (combat's equal), re-forming (units out of their places, or
//! a line re-faced), else the player's order, on which alone it marches. And the stall
//! (FormationScrum._stall): a fight where nobody on either side has touched or sought a foe
//! for `scrum_stall_seconds` is released.

use crate::columns::Columns;
use crate::entries;
use crate::locks::{self, FIGHTING};
use crate::squads::Squads;

/// FormationManoeuvre.Kind.
pub const COMBAT: i32 = 0;
pub const WITHDRAW: i32 = 1;
pub const RE_FORM: i32 = 3;
pub const ORDER: i32 = 4;

/// What of a squad the manoeuvre reads that the squad store doesn't hold: whether it is
/// withdrawing (FormationWithdraw) and skirmishing (FormationContact; read only where it
/// can decide), and, if it narrowed through a gap, its living units as (id, store slot).
pub struct Case {
    pub squad: usize,
    pub withdrawing: bool,
    pub skirmishing: bool,
    pub living: Vec<(i64, usize)>,
}

/// True if the squad in `squad` is locked in melee, front or flank.
pub fn fighting(squads: &Squads, squad: usize) -> bool {
    squads.state[squad] == FIGHTING
        || squads.engaged_with[squad] != 0
        || !squads.flanks[squad].is_empty()
}

/// Settles each squad's manoeuvre this tick, in `cases`' order. A gap's narrowing becomes a
/// re-form: its units walk from where they stand to their new places.
pub fn step(squads: &mut Squads, columns: &mut Columns, cases: &[Case]) {
    for case in cases {
        let squad = case.squad;
        if squads.narrow_ticks[squad] > 0 {
            squads.narrow_ticks[squad] = 0;
            for &(id, slot) in &case.living {
                if !squads.loose[squad].contains(&id) {
                    let at = columns.position[slot];
                    entries::loosen(squads, columns, squad, (id, slot), at);
                }
            }
        }
        squads.manoeuvre[squad] = current(squads, case);
    }
}

/// The highest-priority manoeuvre that applies to the squad now.
pub fn current(squads: &Squads, case: &Case) -> i32 {
    let squad = case.squad;
    if case.withdrawing {
        return WITHDRAW;
    }
    if fighting(squads, squad) || case.skirmishing {
        return COMBAT;
    }
    if !squads.loose[squad].is_empty() || squads.stance[squad].is_some() {
        return RE_FORM;
    }
    ORDER
}

/// A fight's stall this tick: the squads listed (store slots, in the lists' order), each
/// one's foes (`foes`, by its place in `listed`: locks::foe_ids), the ids of the squads
/// whose units touched or sought a foe (`active`), and the ticks a fight may stall.
pub struct Stall<'a> {
    pub listed: &'a [usize],
    pub foes: &'a [Vec<i64>],
    pub active: &'a [i64],
    pub ticks: i64,
}

/// Counts each fighting squad's quiet ticks, and releases the fights stalled `stall.ticks`
/// (locks::release), in the lists' order once all are counted. Returns the released
/// squads' slots.
pub fn stall(squads: &mut Squads, stall: &Stall) -> Vec<usize> {
    let mut stalled = Vec::new();
    for (n, &squad) in stall.listed.iter().enumerate() {
        if squads.state[squad] != FIGHTING {
            continue;
        }
        let quiet = !stall.active.contains(&squads.id[squad])
            && stall.foes[n].iter().all(|foe| !stall.active.contains(foe));
        squads.stall_ticks[squad] = if quiet { squads.stall_ticks[squad] + 1 } else { 0 };
        if squads.stall_ticks[squad] as i64 >= stall.ticks {
            stalled.push(squad);
        }
    }
    for &squad in &stalled {
        squads.stall_ticks[squad] = 0;
        locks::release(squads, squad, stall.listed);
    }
    stalled
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::maths::V2;
    use crate::squads::{Flank, Stance};

    fn two() -> (Squads, Columns) {
        let mut squads = Squads::default();
        squads.open(1);
        squads.open(2);
        let mut columns = Columns::default();
        for _ in 0..3 {
            columns.grow();
        }
        (squads, columns)
    }

    fn case(squad: usize) -> Case {
        Case { squad, withdrawing: false, skirmishing: false, living: Vec::new() }
    }

    #[test]
    fn the_highest_priority_that_applies_wins() {
        let (mut squads, _) = two();
        let mut it = case(0);
        assert_eq!(current(&squads, &it), ORDER);
        squads.stance[0] = Some(Stance { anchor: V2::ZERO, heading: 0.0, foe: 2 });
        assert_eq!(current(&squads, &it), RE_FORM);
        squads.flanks[0].push(Flank { edge: 1, foe: 2, since: 0 });
        assert_eq!(current(&squads, &it), COMBAT);
        it.withdrawing = true;
        assert_eq!(current(&squads, &it), WITHDRAW);
        squads.flanks[0].clear();
        squads.stance[0] = None;
        it.withdrawing = false;
        it.skirmishing = true;
        assert_eq!(current(&squads, &it), COMBAT);
    }

    #[test]
    fn a_squad_that_narrowed_walks_from_where_it_stands_to_its_new_places() {
        let (mut squads, mut columns) = two();
        squads.narrow_ticks[0] = 5;
        columns.position[1] = V2::new(3.0, 4.0);
        let cases = [Case { living: vec![(10, 0), (11, 1)], ..case(0) }, case(1)];
        step(&mut squads, &mut columns, &cases);
        assert_eq!(squads.narrow_ticks[0], 0);
        assert_eq!(squads.loose[0], vec![10, 11]);
        assert_eq!(columns.loose_at[1], V2::new(3.0, 4.0));
        assert_eq!((squads.manoeuvre[0], squads.manoeuvre[1]), (RE_FORM, ORDER));
    }

    #[test]
    fn a_fight_quiet_on_both_sides_for_its_ticks_is_released() {
        let (mut squads, _) = two();
        for (squad, foe) in [(0, 2), (1, 1)] {
            squads.state[squad] = FIGHTING;
            squads.engaged_with[squad] = foe;
        }
        let foes = [vec![2], vec![1]];
        let mut quiet = Stall { listed: &[0, 1], foes: &foes, active: &[], ticks: 2 };
        assert!(stall(&mut squads, &quiet).is_empty());
        assert_eq!(squads.stall_ticks, vec![1, 1]);
        quiet.active = &[2]; // the foe sought: neither side is quiet
        assert!(stall(&mut squads, &quiet).is_empty());
        assert_eq!(squads.stall_ticks, vec![0, 0]);
        quiet.active = &[];
        stall(&mut squads, &quiet);
        assert_eq!(stall(&mut squads, &quiet), vec![0, 1]);
        assert_eq!((squads.state[0], squads.state[1]), (locks::MOVING, locks::MOVING));
        assert_eq!(squads.stall_ticks, vec![0, 0]);
    }
}

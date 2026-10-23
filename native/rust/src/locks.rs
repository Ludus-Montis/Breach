//! Which squads are locked in a fight (FormationLocks, Decisions 40 and 47; spec 39 step
//! 4b): a lock makes a squad fight its foe from this tick; a release frees it and everyone
//! fighting it, front or flank. A turning squad keeps turning through either and takes up
//! the fight when the turn ends (Decision 74). Also which squads a squad fights
//! (ScrumSeek.foe_ids), read from the same locks, and who may be engaged at all
//! (FormationContact.can_engage).

use crate::columns::Columns;
use crate::squads::Squads;

/// SkirmishSquad.State's values the locks read and write.
pub const MOVING: i32 = 0;
pub const FIGHTING: i32 = 2;
pub const ARRIVED: i32 = 3;
pub const DESTROYED: i32 = 4;
pub const ROUTING: i32 = 5;
/// SkirmishUnit.Order's retreat.
pub const RETREAT: i32 = 2;

/// True if the squad in `squad` may be engaged: standing, not arrived, routing or ordered
/// to retreat, and not waiting to set out.
pub fn can_engage(squads: &Squads, squad: usize) -> bool {
    !matches!(squads.state[squad], DESTROYED | ARRIVED | ROUTING)
        && squads.order[squad] != RETREAT
        && squads.wait_ticks[squad] == 0
}

/// A squad to lock: its store slot, its living units' store slots (whose first blows land
/// this tick), and whether units joined it on the march (they may now spread).
pub struct Locking<'a> {
    pub squad: usize,
    pub living: &'a [i32],
    pub joined: bool,
}

/// Locks the squad onto the squad `foe` (its id).
pub fn lock(squads: &mut Squads, columns: &mut Columns, locking: &Locking, foe: i64) {
    let squad = locking.squad;
    squads.engaged_with[squad] = foe;
    squads.state[squad] = FIGHTING;
    if locking.joined {
        squads.reforming[squad] = 1;
    }
    for &slot in locking.living {
        columns.attack_cooldown[slot as usize] = 1;
    }
}

/// Ends the fight of the squad in `released` and frees every squad in `listed` (store
/// slots, in the lists' order) that was fighting it, front or flank.
pub fn release(squads: &mut Squads, released: usize, listed: &[usize]) {
    free(squads, released);
    let id = squads.id[released];
    for &other in listed {
        if squads.engaged_with[other] == id {
            free(squads, other);
        }
        squads.flanks[other].retain(|f| f.foe != id);
        let unlocked = squads.flanks[other].is_empty() && squads.engaged_with[other] == 0;
        if unlocked && squads.state[other] == FIGHTING {
            squads.state[other] = MOVING;
        }
    }
}

fn free(squads: &mut Squads, squad: usize) {
    squads.engaged_with[squad] = 0;
    if squads.state[squad] == FIGHTING && squads.flanks[squad].is_empty() {
        squads.state[squad] = MOVING;
    }
}

/// Ids of the squads in `listed` (store slots, in the lists' order) the squad in `squad`
/// fights: on its front, its edges, or that fight it. `factions`: each listed squad's
/// faction, by its place in `listed`, and the squad's own.
pub fn foe_ids(
    squads: &Squads,
    squad: usize,
    listed: &[usize],
    factions: &[u32],
    own: u32,
) -> Vec<i64> {
    let id = squads.id[squad];
    let mut out = Vec::new();
    for (n, &other) in listed.iter().enumerate() {
        if factions[n] == own {
            continue;
        }
        let other_id = squads.id[other];
        let theirs = squads.flanked_by(other, id);
        let mine = squads.flanked_by(squad, other_id);
        if other_id == squads.engaged_with[squad]
            || squads.engaged_with[other] == id
            || theirs
            || mine
        {
            out.push(other_id);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn three() -> (Squads, Columns) {
        let mut squads = Squads::default();
        for id in [1, 2, 3] {
            squads.open(id);
        }
        let mut columns = Columns::default();
        for _ in 0..4 {
            columns.grow();
        }
        (squads, columns)
    }

    fn free_of(squad: usize) -> Locking<'static> {
        Locking { squad, living: &[], joined: false }
    }

    #[test]
    fn a_lock_fights_its_foe_this_tick_its_living_striking_at_once() {
        let (mut squads, mut columns) = three();
        lock(&mut squads, &mut columns, &Locking { squad: 0, living: &[0, 2], joined: true }, 2);
        assert_eq!(
            (squads.engaged_with[0], squads.state[0], squads.reforming[0]),
            (2, FIGHTING, 1)
        );
        assert_eq!(columns.attack_cooldown, vec![1, 0, 1, 0]);
        lock(&mut squads, &mut columns, &Locking { squad: 1, living: &[1], joined: false }, 1);
        assert_eq!(squads.reforming[1], 0);
    }

    #[test]
    fn a_release_frees_the_squad_and_all_fighting_it_front_or_flank() {
        let (mut squads, mut columns) = three();
        lock(&mut squads, &mut columns, &free_of(0), 2);
        lock(&mut squads, &mut columns, &free_of(1), 1);
        lock(&mut squads, &mut columns, &free_of(2), 2);
        squads.flank(2, 1, 1, 7); // squad 3 has squad 1 on its right edge
        release(&mut squads, 0, &[0, 1, 2]);
        assert_eq!((squads.engaged_with[0], squads.state[0]), (0, MOVING));
        assert_eq!((squads.engaged_with[1], squads.state[1]), (0, MOVING)); // it fought squad 1
        assert!(squads.flanks[2].is_empty());
        assert_eq!((squads.engaged_with[2], squads.state[2]), (2, FIGHTING)); // its own foe
    }

    #[test]
    fn a_squad_still_flanked_keeps_fighting_when_its_front_is_freed() {
        let (mut squads, mut columns) = three();
        lock(&mut squads, &mut columns, &free_of(0), 2);
        squads.flank(0, 2, 3, 1);
        release(&mut squads, 1, &[0, 1, 2]);
        assert_eq!((squads.engaged_with[0], squads.state[0]), (0, FIGHTING));
    }

    #[test]
    fn foes_are_those_on_its_front_its_edges_or_fighting_it_in_the_lists_order() {
        let (mut squads, mut columns) = three();
        lock(&mut squads, &mut columns, &free_of(0), 2);
        squads.flank(2, 3, 1, 4);
        assert_eq!(foe_ids(&squads, 0, &[2, 1, 0], &[1, 1, 0], 0), vec![3, 2]);
        assert_eq!(foe_ids(&squads, 0, &[2, 1, 0], &[0, 1, 0], 0), vec![2]); // a friend
        assert_eq!(foe_ids(&squads, 1, &[2, 1, 0], &[1, 1, 0], 1), vec![1]);
    }
}

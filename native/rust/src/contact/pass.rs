//! Engagement's events and shocks (FormationFronts.engage and FormationEdges.engage; spec 39
//! step 8b5-vi): the locks land as `fronts` and `edges` decide them, then each is reported
//! here, in the order it landed. A front lock is the attacker's "engaged" event, with its
//! foe's id; a flank lock the victim's "flanked" event, by the attacker's id on its edge,
//! then the victim's shock (`shocks`): the rear's impact for its rear, the side's for a side.

use super::{edges, fronts};
use crate::bands::BandTuning;
use crate::columns::Columns;
use crate::events::{self, Log};
use crate::roster::Roster;
use crate::shocks;
use crate::squads::Squads;

/// The rear edge's number (SquadEdges.REAR).
const REAR: i32 = 2;

/// What a flank lock shocks its victim by: `morale_side_impact`, `morale_rear_impact`.
pub struct Impacts {
    pub side: i64,
    pub rear: i64,
}

/// FormationFronts.engage on `tick`: free squads lock onto the hostile fronts they reach;
/// each pick's "engaged" event goes to `log`, in the lists' order.
pub fn engage_fronts(
    squads: &mut Squads,
    columns: &mut Columns,
    roster: &Roster,
    (tick, seed): (i64, i64),
    log: &mut Log,
) {
    for (attacker, foe) in fronts(squads, columns, roster, seed) {
        let s = roster.listed[attacker].squad;
        log.squad(events::ENGAGED, tick, squads, s, &[roster.listed[foe].id]);
    }
}

/// FormationEdges.engage on `tick`: free attackers whose fronts reach a hostile's side or
/// rear lock on; each lock's "flanked" event and its victim's shock go to `log`, in the
/// order the locks landed.
pub fn engage_edges(
    squads: &mut Squads,
    columns: &mut Columns,
    roster: &Roster,
    (tick, seed): (i64, i64),
    (impacts, bands): (&Impacts, &BandTuning),
    log: &mut Log,
) {
    for f in edges(squads, columns, roster, tick, seed) {
        let victim = &roster.listed[f.victim];
        let by = roster.listed[f.attacker].id;
        log.squad(events::FLANKED, tick, squads, victim.squad, &[by, i64::from(f.edge)]);
        let impact = if f.edge == REAR { impacts.rear } else { impacts.side };
        shocks::shock((squads, &*columns), victim, impact, (bands, tick), log);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bands;
    use crate::contact::tests::{built, EAST, NORTH, WEST};
    use crate::maths::V2;

    fn tuning() -> BandTuning {
        BandTuning { per_leadership: 5, steady_at: 70, shaken_at: 40, loss: 2, leader_loss: 3 }
    }

    const IMPACTS: Impacts = Impacts { side: 10, rear: 40 };

    #[test]
    fn each_front_lock_picked_is_its_attackers_engaged_event_in_the_lists_order() {
        let mut b = built(&[(V2::new(0.0, 0.0), EAST, 0), (V2::new(1.0, 0.0), WEST, 1)]);
        b.squads.faction[1] = 5;
        let mut log = Log::default();
        engage_fronts(&mut b.squads, &mut b.columns, &b.roster, (3, 7), &mut log);
        let engaged = events::ENGAGED as i64;
        assert_eq!(log.take(), vec![engaged, 3, 1, 0, 2, engaged, 3, 2, 5, 1]);
        assert_eq!(b.squads.engaged_with, vec![2, 1]);
    }

    #[test]
    fn a_flank_lock_is_the_victims_event_then_the_shock_of_its_edge() {
        // an east-facing line at (0, 0) spans y -1 to 1; an attacker heading north from y 2
        let mut b = built(&[(V2::new(0.0, 0.0), EAST, 0), (V2::new(-0.5, 2.0), NORTH, 1)]);
        b.squads.morale[0] = 75;
        let mut log = Log::default();
        let given = (&IMPACTS, &tuning());
        engage_edges(&mut b.squads, &mut b.columns, &b.roster, (9, 7), given, &mut log);
        let flanked = events::FLANKED as i64;
        let band = vec![events::MORALE_BAND as i64, 9, 1, 0, bands::SHAKEN, 65];
        assert_eq!(log.take(), [vec![flanked, 9, 1, 0, 2, 1], band].concat());
        assert_eq!(b.squads.morale[0], 65);
    }

    #[test]
    fn a_lock_on_the_rear_shocks_by_the_rears_impact() {
        // an attacker heading east from behind the east-facing line
        let mut b = built(&[(V2::new(0.0, 0.0), EAST, 0), (V2::new(-2.0, 0.0), EAST, 1)]);
        b.squads.morale[0] = 75;
        let mut log = Log::default();
        let given = (&IMPACTS, &tuning());
        engage_edges(&mut b.squads, &mut b.columns, &b.roster, (9, 7), given, &mut log);
        assert_eq!(log.take()[..6], [events::FLANKED as i64, 9, 1, 0, 2, i64::from(REAR)]);
        assert_eq!(b.squads.morale[0], 35);
    }
}

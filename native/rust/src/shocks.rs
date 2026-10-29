//! A morale shock landed (FormationMorale.shock, Decision 82; spec 39 step 8b5-v): it drains
//! the squad's morale, held to 0..100, first read as its ceiling (from its living units'
//! sheets) if it has none yet; a change of band is a "morale_band" event, its band and the
//! morale it came to.

use crate::bands::{self, BandTuning};
use crate::columns::Columns;
use crate::events::{self, Log};
use crate::roster::Listed;
use crate::squads::Squads;

/// The squad's morale, its ceiling if it has none yet (FormationMorale._morale).
pub fn morale(squads: &mut Squads, columns: &Columns, listed: &Listed, t: &BandTuning) -> i64 {
    let s = listed.squad;
    if squads.morale[s] < 0 {
        let sheets = listed.living.iter().map(|m| &columns.sheet[m.slot]);
        let courage: Vec<i64> = sheets.clone().map(|sheet| sheet.courage).collect();
        let leaderships: Vec<i64> = sheets.map(|sheet| sheet.leadership).collect();
        squads.morale[s] = bands::ceiling(&courage, &leaderships, t) as i32;
    }
    i64::from(squads.morale[s])
}

/// Drains the listed squad's morale by `amount` on `tick`.
pub fn shock(
    (squads, columns): (&mut Squads, &Columns),
    listed: &Listed,
    amount: i64,
    (t, tick): (&BandTuning, i64),
    log: &mut Log,
) {
    let s = listed.squad;
    let before = morale(squads, columns, listed, t);
    let after = bands::held(before - amount);
    squads.morale[s] = after as i32;
    let band = bands::band(after, t);
    if band != bands::band(before, t) {
        log.squad(events::MORALE_BAND, tick, squads, s, &[band, after]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tuning() -> BandTuning {
        BandTuning { per_leadership: 5, steady_at: 70, shaken_at: 40, loss: 2, leader_loss: 3 }
    }

    #[test]
    fn a_shock_drains_morale_and_a_change_of_band_is_an_event() {
        let mut squads = Squads::default();
        squads.open(4);
        squads.morale[0] = 75;
        let listed = Listed::default();
        let mut log = Log::default();
        let columns = Columns::default();
        shock((&mut squads, &columns), &listed, 3, (&tuning(), 9), &mut log);
        assert_eq!((squads.morale[0], log.take()), (72, vec![]));
        shock((&mut squads, &columns), &listed, 90, (&tuning(), 9), &mut log);
        let routing = vec![events::MORALE_BAND as i64, 9, 4, 0, bands::ROUTING, 0];
        assert_eq!((squads.morale[0], log.take()), (0, routing));
    }

    #[test]
    fn a_squad_with_no_morale_yet_is_shocked_from_its_ceiling() {
        let mut squads = Squads::default();
        squads.open(4);
        let mut columns = Columns::default();
        columns.grow();
        columns.sheet[0].courage = 50;
        let members = crate::discipline::tests::members(&[0]);
        let listed = Listed { living: members, ..Listed::default() };
        let mut log = Log::default();
        shock((&mut squads, &columns), &listed, 0, (&tuning(), 1), &mut log);
        assert_eq!((squads.morale[0], log.take()), (50, vec![]));
    }
}

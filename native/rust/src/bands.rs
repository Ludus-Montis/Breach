//! Morale's ceiling, bands and shocks (FormationMorale, Decisions 81 and 82; spec 39 step
//! 8a2-iv). A formation's ceiling is its living units' mean courage, rounded, plus a step for
//! each point of its best living leader's leadership, at most 100. Its band is steady, shaken,
//! wavering or routing at 0, by its morale; a wavering formation strikes half again more
//! slowly. A shock drains it, held to 0..100; the tick's dead shock it by each loss, and more
//! per point of each fallen leader's leadership. GDScript hands in the living's courage and
//! leadership, the morale and the tuning, keeps each formation's morale, and makes the events
//! ("morale_band", "leader_fell"). Each answer reads one formation alone (Decision 97).

/// The bands, in GDScript's `FormationMorale.Band` order.
pub const STEADY: i64 = 0;
pub const SHAKEN: i64 = 1;
pub const WAVERING: i64 = 2;
pub const ROUTING: i64 = 3;

/// The tuning the rules read (BattleTuning's `morale_*`).
#[derive(Clone, Copy, Debug)]
pub struct BandTuning {
    pub per_leadership: i64,
    pub steady_at: i64,
    pub shaken_at: i64,
    pub loss: i64,
    pub leader_loss: i64,
}

impl BandTuning {
    /// The tuning as the binding is given it: [per leadership, steady at, shaken at, loss,
    /// leader loss].
    pub fn of_slice(t: &[i64]) -> BandTuning {
        BandTuning {
            per_leadership: t[0],
            steady_at: t[1],
            shaken_at: t[2],
            loss: t[3],
            leader_loss: t[4],
        }
    }
}

/// The best living leader's leadership: 0 with none.
pub fn leadership(leaderships: &[i64]) -> i64 {
    leaderships.iter().fold(0, |best, &each| best.max(each))
}

/// The ceiling of a formation whose living units have `courage` and `leaderships`: 0 with
/// none living.
pub fn ceiling(courage: &[i64], leaderships: &[i64], t: &BandTuning) -> i64 {
    if courage.is_empty() {
        return 0;
    }
    let sum: i64 = courage.iter().sum();
    let mean = (sum as f64 / courage.len() as f64).round() as i64;
    100.min(mean + t.per_leadership * leadership(leaderships))
}

/// The band at `morale`.
pub fn band(morale: i64, t: &BandTuning) -> i64 {
    if morale >= t.steady_at {
        STEADY
    } else if morale >= t.shaken_at {
        SHAKEN
    } else if morale > 0 {
        WAVERING
    } else {
        ROUTING
    }
}

/// The attack interval, `usual` ticks, for a unit of a formation in `band`.
pub fn interval(band: i64, usual: i64) -> i64 {
    if band == WAVERING {
        (usual as f64 * 1.5).round() as i64
    } else {
        usual
    }
}

/// The morale `value` comes to, held to 0..100.
pub fn held(value: i64) -> i64 {
    value.clamp(0, 100)
}

/// The shock of the tick's dead: each loss, and per point of each fallen leader's
/// leadership.
pub fn losses(fallen_leaderships: &[i64], t: &BandTuning) -> i64 {
    let mut amount = t.loss * fallen_leaderships.len() as i64;
    for &each in fallen_leaderships {
        if each > 0 {
            amount += t.leader_loss * each;
        }
    }
    amount
}

#[cfg(test)]
mod tests;

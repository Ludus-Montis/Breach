//! The movement modes as units walk (WalkModes, spec 30 round 3, part 6). A unit swims and
//! climbs as its own walker allows at the pace the ground gives (`Ground::crossing`), and:
//! - a climb takes time: stepping into a cell more than a cliff above where it last stood,
//!   it is on the face for that height, climbing at its climb pace before it moves on;
//! - it spends stamina for each second climbing or swimming (ModeStamina's rates);
//! - it can't start a climb or a swim spent (FormationStamina's last stage) - it carries on
//!   one it is in;
//! - spent on a face, it falls to the last place it stood, hurt by the height it had
//!   climbed: fall_damage_per_cell a cell, times its footprint area to fall_mass_power.

use crate::det_pow;
use crate::ground::{Ground, Mode, Walker};
use crate::maths::{maxf, V2};
use crate::paths::climb;

/// WalkModes.EPSILON.
const EPSILON: f64 = 0.000001;
/// ClimbStamina.QUARTERS: quarter-cells in a cell of height.
const QUARTERS: f64 = 4.0;

/// BattleTuning's numbers the modes read.
#[derive(Clone, Copy)]
pub struct ModeTuning {
    /// stamina_climb and stamina_swim: stamina a second in the mode.
    pub climb_rate: f64,
    pub swim_rate: f64,
    /// stamina_spent: the share of its most stamina below which a unit is spent.
    pub spent: f64,
    /// ground_cliff_quarters, for where it can stand (ClimbStamina.standable).
    pub cliff_quarters: i64,
    /// fall_damage_per_cell and fall_mass_power.
    pub fall_per_cell: f64,
    pub fall_mass_power: f64,
}

/// What of a unit the modes read: its walker, its most stamina and its footprint's area.
#[derive(Clone, Copy)]
pub struct Climber {
    pub walker: Walker,
    pub most_stamina: f64,
    pub area: f64,
}

/// A unit's columns the modes read and write (UnitColumns' POSITION, FOOTHOLD, STAMINA,
/// ON_FACE, CLIMB_LEFT).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Footing {
    pub at: V2,
    pub foothold: V2,
    pub stamina: f64,
    pub on_face: bool,
    pub climb_left: f64,
}

/// FormationStamina.stage_at's last stage: spent, with `stamina` of `most`.
fn spent(stamina: f64, most: f64, tuning: &ModeTuning) -> bool {
    stamina / maxf(most, 0.000001) < tuning.spent
}

/// ClimbStamina.drop: how far (cells) a unit at `at` falls to `foot`.
fn drop(ground: &Ground, foot: V2, at: V2) -> f64 {
    maxf(0.0, (ground.height_at(at) - ground.height_at(foot)) as f64) / QUARTERS
}

/// WalkModes.may_step: whether the unit at `from` may step towards `to`: walking always;
/// into a climb or a swim unless it is spent and not already in that mode.
pub fn may_step(
    ground: &Ground,
    unit: &Climber,
    stamina: f64,
    from: V2,
    to: V2,
    tuning: &ModeTuning,
) -> bool {
    let mode = ground.mode(&unit.walker, from, to);
    if mode == Mode::Walking || ground.mode(&unit.walker, from, from) == mode {
        return true;
    }
    !spent(stamina, unit.most_stamina, tuning)
}

/// WalkModes.climb: a tick of `seconds` on a climb face, `cells_per_second` its pace on open
/// ground: true while it is climbing (it moves no further this tick). False once it is up,
/// or if it has fallen - then the hurt it took (`hurt`, else 0).
pub fn climb(
    ground: &Ground,
    unit: &Climber,
    footing: &mut Footing,
    seconds: f64,
    cells_per_second: f64,
    tuning: &ModeTuning,
    hurt: &mut i64,
) -> bool {
    let mut left = footing.climb_left;
    if left <= EPSILON {
        return false;
    }
    let share = ground.crossing(&unit.walker, footing.foothold, footing.at);
    left -= share * cells_per_second * seconds;
    footing.climb_left = left;
    footing.stamina = maxf(0.0, footing.stamina - tuning.climb_rate * seconds);
    let on_face = left > EPSILON;
    footing.on_face = on_face;
    if on_face && footing.stamina <= 0.0 {
        *hurt = fall(ground, unit, footing, tuning);
        return false;
    }
    on_face
}

/// WalkModes.after_step: after the unit stepped from `from` for `seconds`, begins a climb it
/// stepped onto, charges a swim's stamina, and notes where it can stand.
pub fn after_step(
    ground: &Ground,
    unit: &Climber,
    footing: &mut Footing,
    from: V2,
    seconds: f64,
    tuning: &ModeTuning,
) {
    let (at, foothold) = (footing.at, footing.foothold);
    if ground.mode(&unit.walker, foothold, at) == Mode::Climbing {
        footing.climb_left = drop(ground, foothold, at);
        footing.on_face = true;
        return;
    }
    let here = ground.mode(&unit.walker, at, at);
    if here == Mode::Swimming && (from.distance_to(at) as f64) > EPSILON {
        footing.stamina = maxf(0.0, footing.stamina - tuning.swim_rate * seconds);
    }
    footing.on_face = here == Mode::Swimming;
    if climb::standable(ground, &unit.walker, at, tuning.cliff_quarters) {
        footing.foothold = at;
    }
}

/// WalkModes._fall: drops the unit from its climb face to where it last stood; the hurt
/// the height it climbed does it.
fn fall(ground: &Ground, unit: &Climber, footing: &mut Footing, tuning: &ModeTuning) -> i64 {
    let climbed = maxf(0.0, drop(ground, footing.foothold, footing.at) - footing.climb_left);
    let mass = det_pow::pow(unit.area, tuning.fall_mass_power);
    let hurt = (tuning.fall_per_cell * climbed * mass).round() as i64;
    footing.at = footing.foothold; // placement: back at the foot of its climb (Decision 139)
    footing.climb_left = 0.0;
    footing.on_face = false;
    hurt
}

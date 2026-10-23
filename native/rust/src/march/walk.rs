//! Units walk to their places (FormationWalk, spec 30 round 3, part 2): a formation's frame
//! is a set of places, not positions. Every unit in its place - not loose in a fight, nor
//! fleeing - walks to where it makes for (`pour`) each tick at its own pace on the ground
//! it crosses, as loose units do. Far from its place it faces where it walks; near it, its
//! formation's way, stepping sideways or back. The frame keeps within a slack of its units
//! (`share`): it slows as a unit lags towards what its formation's discipline allows
//! (walk_slack_drilled to walk_slack_loose) and waits beyond it, unless that unit has
//! fallen behind (walk_lost) - then only "no man left behind" waits for it; under "fall
//! behind, left behind" it never waits.

use super::modes::{self, Climber, Footing, ModeTuning};
use crate::ground::Ground;
use crate::lying::Lying;
use crate::maths::{bearing_vector, clampf, lerpf, maxf, V2};
use crate::motion::Mover;
use crate::mover;

/// FormationWalk.EPSILON.
const EPSILON: f64 = 0.000001;

/// BattleTuning's numbers the walk reads.
#[derive(Clone, Copy)]
pub struct WalkTuning {
    /// walk_face_travel: how far from its place a unit faces where it walks.
    pub face_travel: f64,
    /// bodies_ground_drag and bodies_ground_block.
    pub drag: f64,
    pub block: f64,
    pub modes: ModeTuning,
}

/// The tick a squad's units walk in: the ground (`None`: open), the bodies lying on it
/// (`None`: none looked at), the seconds, cells a second at speed 1 on open ground, and
/// the squad's heading.
pub struct Walking<'a> {
    pub ground: Option<&'a Ground>,
    pub lying: Option<&'a Lying<'a>>,
    pub seconds: f64,
    pub pace: f64,
    pub heading: f64,
    pub tuning: WalkTuning,
}

/// What of a unit its walk reads: its walker and stamina (`Climber`), how it turns
/// (`Mover`, its bearing aside) and its body's radius.
#[derive(Clone, Copy)]
pub struct Marcher {
    pub climber: Climber,
    pub turn_rate: f64,
    pub backward_pace: f64,
    pub radius: f64,
}

/// A unit's columns its walk reads and writes: its footing, its bearing and its pace.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Standing {
    pub footing: Footing,
    pub bearing: f64,
    pub speed: f64,
}

/// FormationWalk.walk: walks the unit a tick towards `place`; the hurt a fall did it (0
/// for none).
pub fn walk(on: &Walking, unit: &Marcher, state: &mut Standing, place: V2) -> i64 {
    let walker = &unit.climber.walker;
    let modes_tuning = &on.tuning.modes;
    let mut hurt = 0;
    if let Some(ground) = on.ground {
        let cells_per_second = state.speed * on.pace;
        let footing = &mut state.footing;
        if modes::climb(
            ground,
            &unit.climber,
            footing,
            on.seconds,
            cells_per_second,
            modes_tuning,
            &mut hurt,
        ) {
            return hurt; // on a climb face: it climbs before it moves on
        }
    }
    let from = state.footing.at;
    let (toward, _) = mover::way(on.ground, walker, from, place, 1.0); // its way round a barrier
    let way = toward.sub(from);
    let mut full = state.speed * on.pace * on.seconds;
    if let Some(ground) = on.ground {
        if !modes::may_step(
            ground,
            &unit.climber,
            state.footing.stamina,
            from,
            toward,
            modes_tuning,
        ) {
            full = 0.0; // spent: it won't start a climb or a swim
        }
    }
    if let Some(lying) = on.lying {
        if (way.length() as f64) > EPSILON {
            let step = from.add(way.normalized().scale(0.5));
            full *= lying.underfoot(
                unit.radius,
                unit.climber.area,
                step,
                on.tuning.drag,
                on.tuning.block,
            );
        }
    }
    let facing = bearing_vector(on.heading);
    let far = (place.distance_to(from) as f64) > on.tuning.face_travel;
    let look = if far { toward } else { place.add(facing) };
    let turning = Mover {
        bearing: state.bearing,
        turn_rate: unit.turn_rate,
        backward_pace: unit.backward_pace,
    };
    let (stepped, bearing) =
        mover::step(on.ground, walker, &turning, from, toward, full, on.seconds, Some(look));
    state.footing.at = stepped;
    state.bearing = bearing;
    if let Some(ground) = on.ground {
        modes::after_step(
            ground,
            &unit.climber,
            &mut state.footing,
            from,
            on.seconds,
            modes_tuning,
        );
    }
    hurt
}

/// Who a squad's frame waits for (FormationWalk.rule_of): everyone ("no man left behind"),
/// no one ("fall behind, left behind"), or those within walk_lost of their places.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Rule {
    Lagging,
    LeftBehind,
    NoManLeft,
}

/// What of a living unit the rule reads: whether it leads, and has each trait.
#[derive(Clone, Copy)]
pub struct Traits {
    pub leads: bool,
    pub left_behind: bool,
    pub no_man_left: bool,
}

/// FormationWalk.rule_of: a leader's trait, or one every unit has; the never-wait wins if
/// both are found.
pub fn rule_of(living: &[Traits]) -> Rule {
    let found = |has: fn(&Traits) -> bool| {
        let led = living.iter().any(|u| u.leads && has(u));
        led || (!living.is_empty() && living.iter().all(has))
    };
    if found(|u| u.left_behind) {
        Rule::LeftBehind
    } else if found(|u| u.no_man_left) {
        Rule::NoManLeft
    } else {
        Rule::Lagging
    }
}

/// FormationWalk.slack_of: how far (cells) a unit may lag behind its place before the frame
/// waits, by its formation's `discipline` (0 to 100): walk_slack_loose to walk_slack_drilled.
pub fn slack_of(discipline: f64, loose: f64, drilled: f64) -> f64 {
    lerpf(loose, drilled, clampf(discipline / 100.0, 0.0, 1.0))
}

/// A unit in its frame as the share weighs it: where it makes for, its place, where it
/// stands.
#[derive(Clone, Copy)]
pub struct Lag {
    pub target: V2,
    pub place: V2,
    pub at: V2,
}

/// FormationWalk.share: the share (0 to 1) of its step the frame takes for its units
/// lagging behind their places - all of it while every unit is within half its `slack`,
/// easing to none as the furthest nears the slack itself; one fallen behind (`lost`) is
/// waited for only if no man is left behind. One whose place is barred pours through the
/// gap behind the frame and isn't waited for.
pub fn share(rule: Rule, slack: f64, lost: f64, units: &[Lag]) -> f64 {
    if rule == Rule::LeftBehind {
        return 1.0;
    }
    let mut furthest = 0.0;
    for unit in units {
        if !unit.target.is_equal_approx(unit.place) {
            continue;
        }
        let lag = unit.at.distance_to(unit.target) as f64;
        if lag <= lost || rule == Rule::NoManLeft {
            furthest = maxf(furthest, lag);
        }
    }
    clampf((slack - furthest) / (slack / 2.0), 0.0, 1.0)
}

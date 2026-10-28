//! How a tick's blows land (BlowLanding, BlowRoll and BlowDamage, Decisions 118 and 119;
//! spec 39 step 8a1). With rolls on, each blow is rolled, seeded by the battle, the tick and
//! the striker (Decision 93; never the list, Decision 97): the striker's skill plus a roll
//! about it, shifted by its formation's morale band, by high ground, by each foe beyond the
//! first that the target is fighting off, by skill past the damage floor and by how tired it
//! is. That margin is read against the target's parry (melee only, from its front), its
//! dodge (not from its rear) and its defence, which a blow short of grazes; then a hit, and
//! past the hit band a critical. What it deals is rolled within its weapons' range, from a
//! floor skill lifts, each weapon's part taken by its damage type against the target's
//! weakness, resistance or immunity, armour off the mundane parts and ward off the magical.
//! With rolls off every blow is a plain hit at its weapons' full damage. A blow that deals
//! damage of a type the target fears stops its regeneration a while (FormationRegeneration's
//! scorch), and marks the wounds its striker's weapons add.
//!
//! GDScript hands in each unit's arms (what of it the crate doesn't hold), who presses each
//! target and the striker's morale band, and turns what this returns into events.

use crate::columns::Columns;
#[cfg(test)]
use crate::content::BattleTuning;
use crate::det_math;
use crate::ghash::{self, Key};
use crate::ground::Ground;
use crate::maths::{bearing_vector, clampf, maxf, V2};
use crate::reach;
use crate::stamina;

/// How a blow landed (BlowRoll's outcomes, in its layers' order).
pub const PARRIED: u8 = 0;
pub const DODGED: u8 = 1;
pub const GRAZED: u8 = 2;
pub const HIT: u8 = 3;
pub const CRITICAL: u8 = 4;

/// The side of its target a blow falls on (BlowRoll.Side).
pub const FRONT: u8 = 0;
pub const FLANK: u8 = 1;
pub const REAR: u8 = 2;

/// One weapon's part of a blow: its damage, its damage type (a code) and whether it is
/// magical.
#[derive(Clone, Copy, Debug)]
pub struct Part {
    pub amount: f64,
    pub kind: u32,
    pub magical: bool,
}

/// What of a unit its blows and the blows on it read, the crate not holding it
/// (SkirmishUnit's arms). The damage types it is weak to, resists, is immune to and that
/// stop its regeneration are sets of type codes, one bit a code.
#[derive(Clone, Debug, Default)]
pub struct Arms {
    pub id: i64,
    pub slot: usize,
    pub melee_skill: f64,
    pub ranged_skill: f64,
    pub critical: f64,
    pub melee_floor: f64,
    pub ranged_floor: f64,
    pub max_stamina: f64,
    pub agility: f64,
    pub dodging: f64,
    pub defence: i64,
    pub armour: i64,
    pub ward: i64,
    pub wounding: i32,
    pub parries: bool,
    pub weak: u64,
    pub resists: u64,
    pub immune: u64,
    pub stops: u64,
    pub melee_parts: Vec<Part>,
    pub ranged_parts: Vec<Part>,
}

impl Arms {
    fn skill(&self, ranged: bool) -> f64 {
        if ranged {
            self.ranged_skill
        } else {
            self.melee_skill
        }
    }

    fn floor(&self, ranged: bool) -> f64 {
        if ranged {
            self.ranged_floor
        } else {
            self.melee_floor
        }
    }

    fn parts(&self, ranged: bool) -> &[Part] {
        if ranged {
            &self.ranged_parts
        } else {
            &self.melee_parts
        }
    }
}

/// A blow to land: its striker's and its target's index in the arms, the damage it would
/// deal, whether it is a shot, whether it strikes from outside the target's front (a shot's
/// is worked out here), its striker's formation's morale band (None: its formation not
/// found) and how many foes press its target.
#[derive(Clone, Copy, Debug)]
pub struct Blow {
    pub striker: usize,
    pub target: usize,
    pub nominal: i64,
    pub ranged: bool,
    pub flank: bool,
    pub band: Option<usize>,
    pub pressed: i64,
}

/// What landing reads of BattleTuning.
#[derive(Clone, Debug, Default)]
pub struct LandingTuning {
    pub die: f64,
    pub parry_share: f64,
    pub dodge_per_agility: f64,
    pub hit_band: f64,
    pub graze_share: f64,
    pub high_ground: f64,
    pub surrounded: f64,
    pub rear_arc_degrees: f64,
    pub weak: f64,
    pub resist: f64,
    pub immune: f64,
    pub floor_skill: f64,
    pub spill: f64,
    pub regeneration_halt: f64,
    pub reach_front_arc_degrees: f64,
    pub stamina_tired: f64,
    pub stamina_spent: f64,
    /// By morale band (steady, shaken, wavering, routing).
    pub morale_shift: Vec<f64>,
    /// By stamina stage (fresh, tired, spent).
    pub stamina_skill: Vec<f64>,
}

/// How many of LandingTuning's numbers come before its two lists, as `of_slice` reads them.
pub const SCALARS: usize = 17;

impl LandingTuning {
    /// From `t`: the SCALARS numbers in the fields' order, then the number of morale shifts
    /// and the shifts, then the stamina skill shifts.
    pub fn of_slice(t: &[f64]) -> LandingTuning {
        let bands = t[SCALARS] as usize;
        let shifts = SCALARS + 1;
        LandingTuning {
            die: t[0],
            parry_share: t[1],
            dodge_per_agility: t[2],
            hit_band: t[3],
            graze_share: t[4],
            high_ground: t[5],
            surrounded: t[6],
            rear_arc_degrees: t[7],
            weak: t[8],
            resist: t[9],
            immune: t[10],
            floor_skill: t[11],
            spill: t[12],
            regeneration_halt: t[13],
            reach_front_arc_degrees: t[14],
            stamina_tired: t[15],
            stamina_spent: t[16],
            morale_shift: t[shifts..shifts + bands].to_vec(),
            stamina_skill: t[shifts + bands..].to_vec(),
        }
    }

    /// The battle tuning's.
    #[cfg(test)]
    pub fn of(t: &BattleTuning) -> LandingTuning {
        let mut all = vec![
            t.blow_die,
            t.blow_parry_share,
            t.blow_dodge_per_agility,
            t.blow_hit_band,
            t.blow_graze_share,
            t.blow_high_ground,
            t.blow_surrounded,
            t.blow_rear_arc_degrees,
            t.damage_weak,
            t.damage_resist,
            t.damage_immune,
            t.damage_floor_skill,
            t.damage_spill,
            t.wounds_regeneration_halt,
            t.reach_front_arc_degrees,
            t.stamina_tired,
            t.stamina_spent,
            t.blow_morale_shift.len() as f64,
        ];
        all.extend(&t.blow_morale_shift);
        all.extend(&t.stamina_skill);
        LandingTuning::of_slice(&all)
    }
}

/// Lands `blows` in order: [(the damage each deals, how it landed), ...]. `rolls` = (battle
/// seed, tick), or None for no rolls. Writes the targets' regeneration halts and the wounds
/// the last blow that struck each would add.
pub fn land(
    columns: &mut Columns,
    arms: &[Arms],
    blows: &[Blow],
    tuning: &LandingTuning,
    rolls: Option<(i64, i64)>,
    ground: Option<&Ground>,
) -> Vec<(i64, u8)> {
    let mut out = Vec::with_capacity(blows.len());
    for blow in blows {
        let (striker, target) = (&arms[blow.striker], &arms[blow.target]);
        let Some((seed, tick)) = rolls else {
            let dealt = dealt(striker, target, blow.nominal, HIT, blow.ranged, 1.0, tuning);
            scorch(columns, target, striker.parts(blow.ranged), dealt, tuning);
            if dealt > 0 && !blow.ranged {
                columns.last_wounding[target.slot] = striker.wounding;
            }
            out.push((dealt, HIT));
            continue;
        };
        let from = columns.position[striker.slot];
        let (at, bearing) = (columns.position[target.slot], columns.bearing[target.slot]);
        let flank = if blow.ranged { from_flank(bearing, at, from, tuning) } else { blow.flank };
        let mut shift = 0.0;
        if let Some(band) = blow.band {
            shift += tuning.morale_shift[band];
        }
        if ground.is_some_and(|g| g.high_ground(from, at)) {
            shift += tuning.high_ground;
        }
        shift += (blow.pressed - 1).max(0) as f64 * tuning.surrounded;
        shift += spill(striker, blow.ranged, tuning);
        let stage = stamina::stage_of(
            columns.stamina[striker.slot],
            striker.max_stamina,
            tuning.stamina_tired,
            tuning.stamina_spent,
        );
        shift += tuning.stamina_skill[stage];
        let roll = draw(seed, tick, striker.id, "blow", blow.ranged);
        let side = side(bearing, at, from, flank, tuning);
        let landed = outcome(
            margin(striker, blow.ranged, shift, roll, tuning),
            target,
            side,
            blow.ranged,
            tuning,
        );
        let spread = draw(seed, tick, striker.id, "damage", blow.ranged);
        let dealt = dealt(striker, target, blow.nominal, landed, blow.ranged, spread, tuning);
        scorch(columns, target, striker.parts(blow.ranged), dealt, tuning);
        if dealt > 0 {
            columns.last_wounding[target.slot] = striker.wounding;
        }
        out.push((dealt, landed));
    }
    out
}

/// BattleRolls.uniform(seed, [tick, striker, what, ranged]).
fn draw(seed: i64, tick: i64, striker: i64, what: &str, ranged: bool) -> f64 {
    ghash::uniform(seed, &[Key::Int(tick), Key::Int(striker), Key::Str(what), Key::Bool(ranged)])
}

/// How a blow with this margin lands on `target`, struck from `side` (at range: no parry).
pub fn outcome(margin: f64, target: &Arms, side: u8, ranged: bool, tuning: &LandingTuning) -> u8 {
    let mut parry = 0.0;
    if target.parries && !ranged && side == FRONT {
        parry = target.melee_skill * tuning.parry_share;
    }
    let mut dodge = 0.0;
    if side != REAR {
        dodge = target.agility * tuning.dodge_per_agility * target.dodging;
    }
    let layers = [
        (PARRIED, parry),
        (DODGED, dodge),
        (GRAZED, target.defence as f64),
        (HIT, tuning.hit_band),
    ];
    let mut reached = 0.0;
    for (landed, depth) in layers {
        reached += depth;
        if depth > 0.0 && margin < reached {
            return landed;
        }
    }
    CRITICAL
}

/// The blow's margin: the striker's skill, plus `shift`, plus `roll` (0 to 1) across the die
/// about it.
pub fn margin(striker: &Arms, ranged: bool, shift: f64, roll: f64, tuning: &LandingTuning) -> f64 {
    striker.skill(ranged) + shift + (roll - 0.5) * tuning.die
}

/// The side of the target standing at `at` facing `bearing` a blow from `from` falls on: its
/// front unless `flank`; then its rear if within the rear arc of straight behind it.
pub fn side(bearing: f64, at: V2, from: V2, flank: bool, tuning: &LandingTuning) -> u8 {
    if !flank {
        return FRONT;
    }
    let away = from.sub(at);
    if (away.length() as f64) < 0.000001 {
        return FLANK;
    }
    let behind = det_math::cos(tuning.rear_arc_degrees * (std::f64::consts::PI / 180.0));
    let facing = away.normalized().dot(bearing_vector(bearing)) as f64;
    if facing <= -behind + 0.000001 {
        REAR
    } else {
        FLANK
    }
}

/// Whether a shot from `from` comes from outside the front of the target standing at `at`
/// facing `bearing`.
pub fn from_flank(bearing: f64, at: V2, from: V2, tuning: &LandingTuning) -> bool {
    let front = reach::front(tuning.reach_front_arc_degrees);
    !reach::in_front(bearing_vector(bearing), at, from, front)
}

/// The damage a blow of `nominal` that landed as `landed` deals to the target; `roll` (0 to
/// 1) places it in its weapons' range (1: the full).
pub fn dealt(
    striker: &Arms,
    target: &Arms,
    nominal: i64,
    landed: u8,
    ranged: bool,
    roll: f64,
    tuning: &LandingTuning,
) -> i64 {
    if landed == PARRIED || landed == DODGED || nominal <= 0 {
        return 0;
    }
    let bare = [Part { amount: nominal as f64, kind: u32::MAX, magical: false }];
    let mut parts = striker.parts(ranged);
    let mut whole = 0.0;
    for part in parts {
        whole += part.amount;
    }
    if whole <= 0.0 {
        parts = &bare;
        whole = nominal as f64;
    }
    let share =
        range_share(striker, ranged, roll, tuning) * landed_share(landed, striker.critical, tuning);
    let mut mundane = 0.0;
    let mut magical = 0.0;
    for part in parts {
        let amount =
            part.amount * nominal as f64 / whole * share * kind_factor(target, part.kind, tuning);
        if part.magical {
            magical += amount;
        } else {
            mundane += amount;
        }
    }
    let total = maxf(0.0, mundane - target.armour as f64) + maxf(0.0, magical - target.ward as f64);
    if total > 0.0 {
        (total.round() as i64).max(1)
    } else {
        0
    }
}

/// A damage type's worth against the target: its immunity, else resistance, else weakness
/// to it, else 1.
pub fn kind_factor(target: &Arms, kind: u32, tuning: &LandingTuning) -> f64 {
    if has(target.immune, kind) {
        return tuning.immune;
    }
    if has(target.resists, kind) {
        return tuning.resist;
    }
    if has(target.weak, kind) {
        tuning.weak
    } else {
        1.0
    }
}

/// The margin a striker's skill past the damage floor skill adds (crits, not more damage).
pub fn spill(striker: &Arms, ranged: bool, tuning: &LandingTuning) -> f64 {
    maxf(0.0, striker.skill(ranged) - tuning.floor_skill) * tuning.spill
}

/// A blow that landed of a type that stops the target's regeneration stops it a while.
fn scorch(
    columns: &mut Columns,
    target: &Arms,
    parts: &[Part],
    dealt: i64,
    tuning: &LandingTuning,
) {
    if dealt <= 0 || target.stops == 0 {
        return;
    }
    if parts.iter().any(|part| has(target.stops, part.kind)) {
        columns.regeneration_halt[target.slot] = tuning.regeneration_halt;
    }
}

#[inline]
fn has(set: u64, kind: u32) -> bool {
    kind < 64 && set & (1 << kind) != 0
}

/// Where in its weapons' range the roll falls: from its floor, lifted by skill, to 1.
fn range_share(striker: &Arms, ranged: bool, roll: f64, tuning: &LandingTuning) -> f64 {
    let lift = clampf(striker.skill(ranged) / maxf(1.0, tuning.floor_skill), 0.0, 1.0);
    let low = lerpf(striker.floor(ranged), 1.0, lift);
    lerpf(low, 1.0, roll)
}

fn landed_share(landed: u8, critical: f64, tuning: &LandingTuning) -> f64 {
    if landed == GRAZED {
        return tuning.graze_share;
    }
    if landed == CRITICAL {
        critical
    } else {
        1.0
    }
}

/// Godot's `lerpf`: `from + (to - from) * weight`.
#[inline]
fn lerpf(from: f64, to: f64, weight: f64) -> f64 {
    from + (to - from) * weight
}

#[cfg(test)]
mod tests;

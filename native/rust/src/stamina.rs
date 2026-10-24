//! Gaits and stamina (FormationStamina, spec 28 part 7, Decisions 125 and 126; spec 39 step
//! 6a). A unit marches at its speed or runs at its speed times its run pace, building up to
//! it over a moment; a pursuer first reacts, giving a retreat its head start. A formation
//! runs while it flees, pursues or retreats - unless its leader, or all its units, keep to
//! a march then - or when the player hurries it; a unit chasing on its own runs; a spent unit
//! can't run, so a formation of them walks. Marching costs nothing unless its load makes it
//! (`load_march_tiring`); running costs `stamina_run` a second and each blow `stamina_blow`,
//! faster the heavier the load. Each cost restarts a breather: once `stamina_delay` seconds
//! pass with none, stamina is regained at `stamina_recovery` a second - the delay shorter
//! and the rate faster the hardier the unit (its constitution), the rate times its
//! condition. Below `stamina_tired` a unit is tired, below `stamina_spent` spent: slower and
//! less skilled. GDScript keeps each squad's pursuit record and whether the player hurries
//! it, and hands them in with what of each unit the crate doesn't hold.

use crate::columns::Columns;
#[cfg(test)]
use crate::content::BattleTuning;
use crate::locks;
use crate::squads::Squads;

/// An attribute's ordinary value (UnitDef.AVERAGE).
const AVERAGE: f64 = 10.0;

/// The tuning the passes read (BattleTuning's).
#[derive(Clone, Debug)]
pub struct StaminaTuning {
    pub run: f64,
    pub delay: f64,
    pub recovery: f64,
    pub run_build_seconds: f64,
    pub run_reaction: f64,
    pub condition_cap: f64,
    pub tired: f64,
    pub spent: f64,
    /// Its pace by stage (fresh, tired, spent).
    pub pace: Vec<f64>,
    /// What marching costs a second, by load stage.
    pub march_tiring: Vec<f64>,
}

#[cfg(test)]
impl StaminaTuning {
    /// The battle tuning's.
    pub fn of(t: &BattleTuning) -> StaminaTuning {
        StaminaTuning {
            run: t.stamina_run,
            delay: t.stamina_delay,
            recovery: t.stamina_recovery,
            run_build_seconds: t.run_build_seconds,
            run_reaction: t.run_reaction,
            condition_cap: t.condition_cap,
            tired: t.stamina_tired,
            spent: t.stamina_spent,
            pace: t.stamina_pace.clone(),
            march_tiring: t.load_march_tiring.clone(),
        }
    }
}

/// A living unit as the pass is given it: its id, store slot, its fresh march speed, most
/// stamina, how fast its load tires it, its run pace, its wits and constitution, its load
/// stage, whether it leads, and its "pursues_at_march" and "retreats_at_march" traits.
#[derive(Clone, Debug)]
pub struct Gaited {
    pub id: i64,
    pub slot: usize,
    pub fresh_speed: f64,
    pub max_stamina: f64,
    pub tiring: f64,
    pub run_pace: f64,
    pub wits: f64,
    pub constitution: f64,
    pub load_stage: i64,
    pub leads: bool,
    pub pursues_at_march: bool,
    pub retreats_at_march: bool,
}

impl Gaited {
    /// A unit of average wits and constitution, unladen, at pace 1 with 100 stamina.
    #[cfg(test)]
    pub fn plain(id: i64, slot: usize) -> Gaited {
        Gaited {
            id,
            slot,
            fresh_speed: 1.0,
            max_stamina: 100.0,
            tiring: 1.0,
            run_pace: 1.5,
            wits: AVERAGE,
            constitution: AVERAGE,
            load_stage: 0,
            leads: false,
            pursues_at_march: false,
            retreats_at_march: false,
        }
    }
}

/// A squad as the pass is given it: its squad-store slot, whether it pursues (its pursuit
/// record is set) and is on its way back, whether the player hurries it, and its living
/// units in its order.
#[derive(Clone, Debug, Default)]
pub struct Paced {
    pub squad: usize,
    pub pursuing: bool,
    pub returning: bool,
    pub hurry: bool,
    pub living: Vec<Gaited>,
}

/// 0 fresh, 1 tired, 2 spent: the stage of a unit with `stamina` of `most`.
pub fn stage_at(tuning: &StaminaTuning, stamina: f64, most: f64) -> usize {
    stage_of(stamina, most, tuning.tired, tuning.spent)
}

/// `stage_at` on the shares below which a unit is `tired` and `spent`.
pub fn stage_of(stamina: f64, most: f64, tired: f64, spent: f64) -> usize {
    let share = stamina / most.max(0.000001);
    if share < spent {
        return 2;
    }
    if share < tired {
        1
    } else {
        0
    }
}

/// True if the formation runs: fleeing; pursuing or retreating, unless its leader - or
/// every one of its units - keeps to a march then; or hurried by the player - while it
/// isn't fighting.
pub fn runs(squads: &Squads, paced: &Paced) -> bool {
    let state = squads.state[paced.squad];
    if state == locks::ROUTING {
        return true;
    }
    if paced.pursuing && !paced.returning {
        return !marches(&paced.living, |u| u.pursues_at_march);
    }
    if state == locks::FIGHTING {
        return false;
    }
    if squads.order[paced.squad] == locks::RETREAT
        && !marches(&paced.living, |u| u.retreats_at_march)
    {
        return true;
    }
    paced.hurry
}

/// True if the squad keeps to a march for the trait: its leader has it, or all its units.
fn marches(living: &[Gaited], has: impl Fn(&Gaited) -> bool) -> bool {
    if living.iter().any(|u| u.leads && has(u)) {
        return true;
    }
    !living.is_empty() && living.iter().all(has)
}

/// One tick of `seconds` for each squad in the lists' order: runners and laden marchers
/// tire, the rest recover after their breather, and each unit's pace follows its gait and
/// how tired it is.
pub fn step(
    squads: &Squads,
    columns: &mut Columns,
    listed: &[Paced],
    seconds: f64,
    tuning: &StaminaTuning,
) {
    for paced in listed {
        step_squad(squads, columns, paced, seconds, tuning);
    }
}

fn step_squad(
    squads: &Squads,
    columns: &mut Columns,
    paced: &Paced,
    seconds: f64,
    tuning: &StaminaTuning,
) {
    let running = runs(squads, paced);
    let marching = squads.state[paced.squad] == locks::MOVING;
    let chasers = &squads.chasers[paced.squad];
    for unit in &paced.living {
        let slot = unit.slot;
        let chasing = chasers.iter().any(|c| c.unit == unit.id);
        let mut gait = 1.0;
        let runner =
            (running || chasing) && stage_at(tuning, columns.stamina[slot], unit.max_stamina) < 2;
        if runner {
            let pursuing = paced.pursuing || chasing;
            gait = build(columns, columns.was_running[slot] != 0, unit, pursuing, seconds, tuning);
            if columns.run_build[slot] > 0.0 {
                spend(columns, slot, tuning.run * unit.tiring * seconds);
            }
        } else if marching && march_cost(unit, tuning) > 0.0 {
            spend(columns, slot, march_cost(unit, tuning) * seconds);
        }
        columns.was_running[slot] = runner as i32;
        if columns.exerted[slot] == 0 && columns.on_face[slot] == 0 {
            breathe(columns, unit, seconds, tuning); // no breather on a face (the walk modes)
        }
        columns.exerted[slot] = 0;
        let stage = stage_at(tuning, columns.stamina[slot], unit.max_stamina);
        columns.speed[slot] = unit.fresh_speed * gait * tuning.pace[stage];
    }
}

/// Each striker's stamina spent on this tick's blows, `blow` (`stamina_blow`) each, in their
/// order: [(its store slot, how fast its load tires it), ...].
pub fn strike(columns: &mut Columns, strikers: &[(usize, f64)], blow: f64) {
    for &(slot, tiring) in strikers {
        spend(columns, slot, blow * tiring);
    }
}

/// True if most of the squad's living units ([(store slot, most stamina), ...]) are too
/// tired to keep up a chase: below `give_up` of their most.
pub fn squad_gives_up(columns: &Columns, living: &[(usize, f64)], give_up: f64) -> bool {
    if living.is_empty() {
        return false;
    }
    let tired = living.iter().filter(|&&(slot, most)| columns.stamina[slot] < most * give_up);
    tired.count() * 2 > living.len()
}

/// Its gait this tick of a run: building from its march to its run pace over
/// `run_build_seconds`, a pursuer first reacting (the sooner, the quicker its wits).
fn build(
    columns: &mut Columns,
    was_running: bool,
    unit: &Gaited,
    pursuing: bool,
    seconds: f64,
    tuning: &StaminaTuning,
) -> f64 {
    let slot = unit.slot;
    if !was_running {
        let wits = unit.wits.max(1.0);
        columns.run_build[slot] =
            if pursuing { -tuning.run_reaction * AVERAGE / wits } else { 0.0 };
    }
    columns.run_build[slot] += seconds;
    let built = (columns.run_build[slot] / tuning.run_build_seconds.max(0.000001)).clamp(0.0, 1.0);
    1.0 + (unit.run_pace - 1.0) * built
}

/// Spends `amount` of the slot's stamina: it restarts its breather.
fn spend(columns: &mut Columns, slot: usize, amount: f64) {
    columns.stamina[slot] = (columns.stamina[slot] - amount).max(0.0);
    columns.exerted[slot] = 1;
    columns.breather[slot] = 0.0;
}

/// A tick without cost: once its breather is long enough, it recovers, at its condition.
fn breathe(columns: &mut Columns, unit: &Gaited, seconds: f64, tuning: &StaminaTuning) {
    let slot = unit.slot;
    let hardiness = unit.constitution.max(1.0) / AVERAGE;
    columns.breather[slot] += seconds;
    if columns.breather[slot] < tuning.delay / hardiness {
        return;
    }
    let condition = columns.condition[slot].clamp(0.0, tuning.condition_cap);
    let gain = tuning.recovery * hardiness * condition * seconds;
    columns.stamina[slot] = unit.max_stamina.min(columns.stamina[slot] + gain);
}

fn march_cost(unit: &Gaited, tuning: &StaminaTuning) -> f64 {
    let costs = &tuning.march_tiring;
    costs[unit.load_stage.clamp(0, costs.len() as i64 - 1) as usize]
}

#[cfg(test)]
mod tests;

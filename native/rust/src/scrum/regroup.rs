//! The scrum's regroup and its chasers over the roster (ScrumRegroup.step and
//! ScrumPursuit.step, spec 39 step 8b5-xiv). Each squad's say is read where the crate holds
//! it: whether it is out of the fight (its state), its units out loose, whether it is a
//! drilled retreat (its order, and its discipline from its units' sheets) and withdrawing
//! (its record), its re-form pace (FormationDiscipline.reform_pace), its chasers, and on
//! terrain where its units walk back to (TerrainStep.returns: the pour's targets in its
//! frame, `march::pour::targets`, any swap under way and any downed foe taken read from the
//! squad). Then the regroup (`regroup::step`) and the chase (`pursuit::chase`), in that
//! order, as the GDScript phases ran them.

use crate::discipline::{self, DisciplineTuning};
use crate::field::Field;
use crate::ground::Ground;
use crate::locks::{FIGHTING, RETREAT, ROUTING};
use crate::march::pour::{self, Frame};
use crate::maths::V2;
use crate::moving::share;
use crate::pursuit;
use crate::regroup::{self, Regrouper};
use crate::roster::{Listed, Roster};
use crate::squads::Squads;

/// The tuning the two phases read: each one's own, the discipline's
/// (`discipline_per_leadership`, `discipline_meets_threats`), the re-form pace's
/// (`discipline_march_pace_at`, `discipline_slowest`, `discipline_fastest`), the fight's
/// seed and `wounds_reach` (the pour's).
pub struct Rules {
    pub regroup: regroup::Rules,
    pub chase: pursuit::Rules,
    pub discipline: DisciplineTuning,
    pub reform: [f64; 3],
    pub seed: i64,
    pub wounds_reach: f64,
}

/// FormationScrum's regroup and chasers over `roster`'s squads on `ground` (none: open).
pub fn step(
    squads: &mut Squads,
    field: &mut Field,
    roster: &Roster,
    ground: Option<&Ground>,
    rules: &Rules,
) {
    let regroupers: Vec<Regrouper> = (0..roster.listed.len())
        .filter(|&n| ![FIGHTING, ROUTING].contains(&squads.state[roster.listed[n].squad]))
        .map(|n| regrouper(squads, field, roster, n, ground, rules))
        .collect();
    if !regroupers.is_empty() {
        let columns = &mut field.columns;
        regroup::step(squads, columns, roster, &regroupers, ground, &rules.regroup);
    }
    let chasing: Vec<pursuit::Chasing> = (0..roster.listed.len())
        .filter(|&n| !squads.chasers[roster.listed[n].squad].is_empty())
        .map(|n| pursuit::Chasing {
            listed: n,
            homes: returns(squads, field, &roster.listed[n], ground, rules),
        })
        .collect();
    if !chasing.is_empty() {
        let columns = &mut field.columns;
        pursuit::chase(squads, columns, roster, &chasing, ground, &rules.chase);
    }
}

/// ScrumRegroup's say of the listed squad `n`: only one with units out walks.
fn regrouper(
    squads: &Squads,
    field: &Field,
    roster: &Roster,
    n: usize,
    ground: Option<&Ground>,
    rules: &Rules,
) -> Regrouper {
    let listed = &roster.listed[n];
    let s = listed.squad;
    let loose = !squads.loose[s].is_empty();
    let held = discipline::of(&field.columns, listed, &rules.discipline);
    let [at, slowest, fastest] = rules.reform;
    Regrouper {
        listed: n,
        drilled: loose && squads.order[s] == RETREAT && held >= rules.discipline.meets_threats,
        reform: if loose { (held as f64 / at).clamp(slowest, fastest) } else { 0.0 },
        withdrawing: squads.withdraw[s].is_some(),
        homes: if loose { returns(squads, field, listed, ground, rules) } else { Vec::new() },
    }
}

/// TerrainStep.returns: where the listed squad's living units, none fleeing, walk back to on
/// `ground` - none on open ground or in a stance (their places).
fn returns(
    squads: &Squads,
    field: &Field,
    listed: &Listed,
    ground: Option<&Ground>,
    rules: &Rules,
) -> Vec<(i64, V2)> {
    let s = listed.squad;
    if ground.is_none() || squads.stance[s].is_some() {
        return Vec::new();
    }
    let framed: Vec<(i32, i64)> = listed
        .living
        .iter()
        .filter(|m| !squads.fleeing[s].contains(&m.id))
        .map(|m| (m.slot as i32, m.id))
        .collect();
    if framed.is_empty() {
        return Vec::new();
    }
    let frame = Frame {
        anchor: squads.position[s],
        heading: squads.heading[s],
        width: squads.width[s],
        centre_shift: squads.centre_shift[s],
    };
    let units = share::pourers(squads, field, s, &framed, true);
    let aims = pour::targets(ground, &frame, &units, rules.seed, rules.wounds_reach);
    framed.iter().map(|&(_, id)| id).zip(aims).collect()
}

#[cfg(test)]
mod tests;

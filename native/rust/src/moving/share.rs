//! The walk's share (FormationWalk.share, spec 39 step 8b5-ix): the share (0 to 1) of its
//! step a squad's frame takes this tick for its units lagging behind their places, each unit
//! read where it lives: its place and where it stands from the store, its footprint from its
//! sheet, its walker from its objects, how far it is through a swap from the squad's swaps
//! under way, and the body it is out taking from the squad's taking. Who the frame waits for
//! is the rule its living units' traits and leadership make (`march::walk::rule_of`); how far
//! they may lag, its discipline's slack.

use crate::discipline::{self, DisciplineTuning};
use crate::field::Field;
use crate::ground::Ground;
use crate::march::pour::{self, Frame, Pourer};
use crate::march::walk::{self, Lag, Rule, Traits};
use crate::maths::V2;
use crate::roster::Listed;
use crate::squads::Squads;
use crate::swaps;

/// What the share reads besides the squad: the fight's seed, wounds_reach, walk_lost, and
/// discipline_per_leadership, walk_slack_loose and walk_slack_drilled for its slack.
pub struct ShareTuning {
    pub seed: i64,
    pub wounds_reach: f64,
    pub lost: f64,
    pub slack: [f64; 3],
}

/// The traits that make the rule (`march::walk::rule_of`), by name.
const LEFT_BEHIND: &str = "fall_behind_left_behind";
const NO_MAN_LEFT: &str = "no_man_left_behind";

/// FormationWalk.share for the listed squad on `ground` (none: open ground).
pub fn of(
    squads: &Squads,
    field: &Field,
    listed: &Listed,
    ground: Option<&Ground>,
    t: &ShareTuning,
) -> f64 {
    let s = listed.squad;
    if !squads.taking[s].is_empty() {
        return 0.0; // it holds while its units are out taking the downed (FormationTaking)
    }
    let columns = &field.columns;
    let living = squads.living(s, &columns.state);
    let rule = rule(field, &living);
    if rule == Rule::LeftBehind {
        return 1.0;
    }
    let ids: Vec<i64> = living.iter().map(|&u| field.unit_at(u as u32).unwrap_or(0)).collect();
    let framed: Vec<(i32, i64)> = living
        .iter()
        .zip(ids)
        .filter(|(_, id)| !squads.loose[s].contains(id) && !squads.fleeing[s].contains(id))
        .map(|(&u, id)| (u, id))
        .collect();
    let frame = Frame {
        anchor: squads.position[s],
        heading: squads.heading[s],
        width: squads.width[s],
        centre_shift: squads.centre_shift[s],
    };
    let units = pourers(squads, field, s, &framed);
    let aims = pour::targets(ground, &frame, &units, t.seed, t.wounds_reach);
    let lags: Vec<Lag> = units
        .iter()
        .zip(&aims)
        .map(|(unit, &target)| Lag { target, place: pour::place(&frame, unit), at: unit.at })
        .collect();
    walk::share(rule, slack(field, listed, t.slack), t.lost, &lags)
}

/// FormationWalk.slack_of for the listed squad, by its discipline (FormationDiscipline.of):
/// [discipline_per_leadership, walk_slack_loose, walk_slack_drilled].
pub fn slack(field: &Field, listed: &Listed, [per_leadership, loose, drilled]: [f64; 3]) -> f64 {
    let tuning = DisciplineTuning {
        per_leadership: per_leadership as i64,
        meets_threats: 0,
        expected_max: 0.0,
        leash_shares: Vec::new(),
        leash_steps: Vec::new(),
    };
    let discipline = discipline::of(&field.columns, listed, &tuning);
    walk::slack_of(discipline as f64, loose, drilled)
}

/// FormationWalk.rule_of over the `living` units' store slots.
fn rule(field: &Field, living: &[i32]) -> Rule {
    let columns = &field.columns;
    let (left, none_left) = (field.lexicon.find(LEFT_BEHIND), field.lexicon.find(NO_MAN_LEFT));
    let has = |u: usize, code: Option<u32>| {
        code.is_some_and(|code| columns.words[u].traits.iter().any(|&(c, _)| c == code))
    };
    let traits: Vec<Traits> = living
        .iter()
        .map(|&unit| {
            let u = unit as usize;
            Traits {
                leads: columns.sheet[u].leadership > 0,
                left_behind: has(u, left),
                no_man_left: has(u, none_left),
            }
        })
        .collect();
    walk::rule_of(&traits)
}

/// The `framed` units ([store slot, id]) of the squad in `s` as a pour reads them
/// (CrateMarch.pourers), none of them out taking.
fn pourers(squads: &Squads, field: &Field, s: usize, framed: &[(i32, i64)]) -> Vec<Pourer> {
    let columns = &field.columns;
    let moving = swaps::offsets(squads, s);
    framed
        .iter()
        .map(|&(unit, id)| {
            let u = unit as usize;
            let sheet = &columns.sheet[u];
            let offset = moving
                .iter()
                .find(|&&(mover, _, _)| mover == id)
                .map_or(V2::ZERO, |&(_, ranks, across)| V2::new(ranks, across));
            Pourer {
                id,
                at: columns.position[u],
                rank: columns.rank[u],
                column: columns.column[u],
                footprint_width: sheet.footprint_width as i32,
                footprint_depth: sheet.footprint_depth as i32,
                offset,
                walker: crate::pathing::pass::walker(columns.objects[u].walker, sheet.height),
                taking: None, // the share is 0 while any of them is out taking
            }
        })
        .collect()
}

#[cfg(test)]
mod tests;

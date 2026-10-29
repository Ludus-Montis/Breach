//! FormationStamina.step as one pass (spec 39 step 8b5-xxxi): each listed squad's pursuit
//! and its command's hurry, and each living unit's fresh speed, most stamina, tiring, run
//! pace, load stage, leadership, wits and constitution and its "pursues_at_march" and
//! "retreats_at_march" traits, read here, then `stamina::step`.

#[cfg(test)]
mod tests;

use super::{Gaited, Paced, StaminaTuning, AVERAGE};
use crate::commands::HURRY;
use crate::field::Field;
use crate::squads::Squads;

/// One tick of `seconds` for the listed squads (their slots, in the lists' order). A unit
/// made by hand, with no fresh speed on its sheet, first takes its speed column's: returns
/// the ids of those, for GDScript's copies.
pub fn step(
    squads: &Squads,
    field: &mut Field,
    listed: &[usize],
    seconds: f64,
    tuning: &StaminaTuning,
) -> Vec<i64> {
    let mut made = vec![];
    let paced: Vec<Paced> = listed.iter().map(|&s| paced(squads, field, s, &mut made)).collect();
    super::step(squads, &mut field.columns, &paced, seconds, tuning);
    made
}

/// The squad in slot `s` as the pass takes it; a unit made by hand is given its fresh speed
/// and its id pushed onto `made`.
fn paced(squads: &Squads, field: &mut Field, s: usize, made: &mut Vec<i64>) -> Paced {
    let pursuit = squads.pursuit[s];
    let hurry = squads.command[s].is_some_and(|c| squads.commands.gives(c, HURRY));
    let living = squads.living(s, &field.columns.state);
    let living = living.into_iter().map(|u| gaited(field, u as usize, made)).collect();
    Paced {
        squad: s,
        pursuing: pursuit.is_some(),
        returning: pursuit.is_some_and(|p| p.returning),
        hurry,
        living,
    }
}

fn gaited(field: &mut Field, slot: usize, made: &mut Vec<i64>) -> Gaited {
    let id = field.unit_at(slot as u32).unwrap_or(0);
    let columns = &mut field.columns;
    if columns.sheet[slot].fresh_speed <= 0.0 {
        columns.sheet[slot].fresh_speed = columns.speed[slot];
        made.push(id);
    }
    let (sheet, words) = (&columns.sheet[slot], &columns.words[slot]);
    let level = |said: &[(u32, f64)], name: &str| field.lexicon.level(said, name);
    let has = |name: &str| level(&words.traits, name).unwrap_or(0.0) > 0.0;
    Gaited {
        id,
        slot,
        fresh_speed: sheet.fresh_speed,
        max_stamina: sheet.max_stamina,
        tiring: sheet.tiring,
        run_pace: sheet.run_pace,
        wits: level(&words.attributes, "wits").unwrap_or(AVERAGE),
        constitution: level(&words.attributes, "constitution").unwrap_or(AVERAGE),
        load_stage: sheet.load_stage,
        leads: sheet.leadership > 0,
        pursues_at_march: has("pursues_at_march"),
        retreats_at_march: has("retreats_at_march"),
    }
}

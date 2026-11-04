//! FatigueTrials played in the crate (spec 39 step 8c-xv), for `breach_trials scenario=fatigue`:
//! does fighting tire a line so it pursues less (spec 28 part 7, Decision 125)? A line of 8
//! grems holds its ground; a wave of 8 grems engages it, fights it for a set time, then
//! retreats at a run. Both sides are too stout to fall, so the contact lasts as long as it is
//! told to. Measured once per battle seed: how far (cells from its post) and how long (ticks)
//! the line pursued, its stamina (share of its most) when the wave turned, and whether the
//! chase ended with most of it tired. Printed as tools/formation_trials.gd's `_fatigue` prints
//! it, so the two outputs can be compared line for line.

#[cfg(test)]
mod tests;

use crate::battles::{Set, TICK_SECONDS};
use crate::commands::HURRY;
use crate::content::Content;
use crate::events::{Log, ENGAGED, PURSUIT_ENDED};
use crate::field::Field;
use crate::geometry::CELLS;
use crate::locks::RETREAT;
use crate::maths::V2;
use crate::orders;
use crate::play::{starts, told, Play};
use crate::route::Route;
use crate::spawn::{self, Placed, Spawn};
use crate::squads::Squads;
use crate::stamina;

/// FatigueTrials.LIMIT_TICKS: the most ticks a run plays.
pub const LIMIT_TICKS: i64 = 3000;
/// FatigueTrials.STOUT: the hp each grem is given, so no one falls.
const STOUT: i64 = 100_000;
/// The contacts formation_trials.gd's `_fatigue` tries, in seconds.
pub const CONTACTS: [f64; 4] = [3.0, 30.0, 60.0, 90.0];
/// SkirmishUnit.Order.HOLD.
const HOLD: i32 = 1;
/// Each side's grems, in a row.
const WIDTH: usize = 8;

/// FatigueTrials.run's result: the mean cells and ticks pursued, the mean stamina share at the
/// turn, the runs whose chase ended with most of the line tired, and the runs.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Fatigue {
    pub pursued: f64,
    pub ticks: f64,
    pub stamina: f64,
    pub tired: i64,
    pub runs: i64,
}

/// One battle's measures (FatigueTrials._battle's).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Battle {
    pub pursued: f64,
    pub ticks: i64,
    pub stamina: f64,
    pub tired: bool,
}

/// FatigueTrials.run: `runs` battles of `contact_seconds`' contact, seeded from `first_seed`
/// on, their measures summed as it sums them; None if the grem isn't in `content`.
pub fn run(
    contact_seconds: f64,
    runs: i64,
    first_seed: i64,
    threaded: bool,
    content: &Content,
) -> Option<Fatigue> {
    let mut out = Fatigue { pursued: 0.0, ticks: 0.0, stamina: 0.0, tired: 0, runs };
    for index in 0..runs.max(0) {
        let one = battle(contact_seconds, first_seed + index, threaded, content)?;
        out.pursued += one.pursued / runs as f64;
        out.ticks += one.ticks as f64 / runs as f64;
        out.stamina += one.stamina / runs as f64;
        out.tired += i64::from(one.tired);
    }
    Some(out)
}

/// FatigueTrials._battle on a fresh crate: the wave at the player's end, the line holding at
/// the other, on the simulation's own 2-tile lane, blows rolled.
pub fn battle(
    contact_seconds: f64,
    seed: i64,
    threaded: bool,
    content: &Content,
) -> Option<Battle> {
    let (mut squads, mut field, mut log) = (Squads::default(), Field::default(), Log::default());
    let mut grem = content.units.get("grem")?.clone(); // CrateContent.unit("grem").duplicate()
    grem.hp = STOUT;
    let set = Set { route_tiles: 2.0, tick_seconds: TICK_SECONDS, combat_width: 0, map: None };
    let mut play = Play::open(set, (seed, true, threaded), content);
    let lane = Route::new(vec![V2::new(0.0, 0.0), V2::new((2.0 * CELLS) as f32, 0.0)], 0.0);
    let route_code = Some(squads.issue_route()); // the simulation's own route, coded once
    let placed: Vec<Placed> = (0..WIDTH)
        .map(|column| Placed { unit: &grem, code: 0, place: (0, column as i32) })
        .collect();
    let spawned = [("player", true), ("the_kingdom", false)].map(|(faction, at_player_end)| {
        let spawn = Spawn {
            width: WIDTH as i64,
            placed: &placed,
            faction,
            at_player_end,
            wait_ticks: 0,
            route: &lane,
            route_code,
            combat_width: 0,
        };
        spawn::squad(&mut squads, &mut field, content, &spawn)
    });
    play.listed = spawned.to_vec();
    let [wave, line] = spawned;
    let (wave_id, line_id) = (squads.id[wave], squads.id[line]);
    orders::queue(&mut squads, line_id, HOLD);
    let give_up = content.tuning.stamina_give_up;
    let mut result = Battle { pursued: 0.0, ticks: 0, stamina: 1.0, tired: false };
    let (mut contact, mut turned) = (-1, -1);
    let turn_after = (contact_seconds / TICK_SECONDS).round() as i64;
    for tick in 0..LIMIT_TICKS {
        play.step(&mut squads, &mut field, content, &mut log);
        let flat = log.take();
        if contact < 0 && told(&flat, ENGAGED) {
            contact = tick;
        }
        if turned < 0 && contact >= 0 && tick - contact >= turn_after {
            turned = tick;
            result.stamina = share(&squads, &field, line);
            if let Some(command) = squads.command[wave] {
                squads.commands.give(command, HURRY, true);
            }
            orders::queue(&mut squads, wave_id, RETREAT);
        }
        if let Some(pursuit) = squads.pursuit[line].filter(|p| !p.returning) {
            result.ticks += 1;
            let gone = squads.position[line].distance_to(pursuit.post_at) as f64;
            result.pursued = result.pursued.max(gone);
        }
        let ended =
            starts(&flat).any(|at| flat[at] as usize == PURSUIT_ENDED && flat[at + 2] == line_id);
        if ended {
            let living = most(&squads, &field, line);
            result.tired = stamina::squad_gives_up(&field.columns, &living, give_up);
            break;
        }
    }
    Some(result)
}

/// The squad in `s`'s living units' slots and most stamina, in its members' order.
fn most(squads: &Squads, field: &Field, s: usize) -> Vec<(usize, f64)> {
    let living = squads.living(s, &field.columns.state);
    living
        .iter()
        .map(|&slot| (slot as usize, field.columns.sheet[slot as usize].max_stamina))
        .collect()
}

/// FatigueTrials._stamina: the living's mean share of their most stamina.
fn share(squads: &Squads, field: &Field, s: usize) -> f64 {
    let living = most(squads, field, s);
    let total =
        living.iter().fold(0.0, |sum, &(slot, most)| sum + field.columns.stamina[slot] / most);
    total / living.len().max(1) as f64
}

/// formation_trials.gd's `_fatigue` column heads.
pub fn columns() -> String {
    format!(
        "{:<10} {:<10} {:<16} {:<12} {}",
        "contact", "stamina", "pursued (cells)", "ticks", "tired"
    )
}

/// formation_trials.gd's `_fatigue` row: `"%-10s %-10s %-16s %-12s %d / %d"`.
pub fn row(contact_seconds: f64, result: &Fatigue) -> String {
    let contact = format!("{} s", contact_seconds as i64);
    let stamina = format!("{}%", (result.stamina * 100.0).round() as i64);
    let pursued = format!("{:.1}", result.pursued);
    let ticks = format!("{:.1}", result.ticks);
    format!(
        "{contact:<10} {stamina:<10} {pursued:<16} {ticks:<12} {} / {}",
        result.tired, result.runs
    )
}

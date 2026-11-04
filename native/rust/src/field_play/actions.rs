//! The record's commands given to the field in the crate (spec 39 step 8c-ix), as
//! FormationFieldActions.apply gives them: the player sends a route's wave, retreats every
//! squad on a wave's route, makes a wave depart when full or not, makes B's waves wait in the
//! wood, sets what a route's waves do with their downed, hurries them and puts a wave on
//! another route; the kingdom has its line and reserve pursue or not. `replay` plays a record
//! as the digest plays a field log, the waves sent together (`send A+B`) included.

#[cfg(test)]
mod tests;

use super::{FieldPlay, A, B, C, CONTACT};
use crate::commands::PURSUES;
use crate::content::Content;
use crate::events::Log;
use crate::field::Field;
use crate::locks::RETREAT;
use crate::maths::V2;
use crate::orders;
use crate::record::{Command, Record, KINGDOM, PLAYER};
use crate::squads::Squads;
use crate::staging::Stage;

/// Where B's waiting wave holds, along its route (FormationField.STAGING's x), in cells.
const STAGING_AT: f64 = 56.0;
/// How long a waiting wave holds before going anyway, in seconds.
const WAIT_SECONDS: f64 = 60.0;
/// Both waves sent at once (FormationField.send_together).
const TOGETHER: &str = "A+B";
/// What a route's waves may do with their downed (FormationFieldActions.apply's "tend").
const TENDS: [&str; 3] = ["leave", "recover", "carry"];

/// A wave's key ("A", "B") as its index.
fn wave_of(key: &str) -> Option<usize> {
    match key {
        "A" => Some(A),
        "B" => Some(B),
        _ => None,
    }
}

/// A route's key ("A", "B", "C") as its index.
fn route_of(key: &str) -> Option<usize> {
    wave_of(key).or((key == "C").then_some(C))
}

impl FieldPlay {
    /// FormationFieldActions.apply: `given` given to the field; false if it isn't one the
    /// field takes from that side.
    pub fn apply(
        &mut self,
        given: &Command,
        (squads, field): (&mut Squads, &mut Field),
        content: &Content,
    ) -> bool {
        let on = given.value == "on";
        if given.who == KINGDOM {
            if given.order != "pursue" {
                return false;
            }
            for &s in &self.kingdom {
                if let Some(command) = squads.command[s] {
                    squads.commands.give(command, PURSUES, on);
                }
            }
            return true;
        }
        if given.who != PLAYER {
            return false;
        }
        let wave = wave_of(&given.target);
        match (given.order.as_str(), wave) {
            ("send", Some(w)) => {
                self.send(w, 0, (squads, field), content);
            }
            ("send", None) if given.target == TOGETHER => {
                self.send_together(&[A, B], (squads, field), content);
            }
            ("retreat", Some(w)) => self.retreat(w, squads, field),
            ("auto", Some(w)) => self.waves[w].auto = on,
            ("wait", _) => self.set_wait(on),
            ("tend", Some(w)) if TENDS.contains(&given.value.as_str()) => {
                let tends = if given.value == "leave" { "" } else { given.value.as_str() };
                match self.tending.iter_mut().find(|(key, _)| *key == w) {
                    Some(tending) => tending.1 = tends.to_string(),
                    None => self.tending.push((w, tends.to_string())),
                }
            }
            ("hurry", Some(w)) if on && !self.hurried.contains(&w) => self.hurried.push(w),
            ("hurry", Some(w)) => self.hurried.retain(|&key| key != w || on),
            ("route", Some(w)) => match route_of(&given.value) {
                Some(route) => self.waves[w].route = route,
                None => return false,
            },
            _ => return false,
        }
        true
    }

    /// Every player squad on wave `w`'s route ordered to retreat, in the listed order.
    fn retreat(&self, w: usize, squads: &mut Squads, field: &Field) {
        let Some(player) = field.lexicon.find(PLAYER) else { return };
        let route = self.routes[self.waves[w].route].1;
        for &s in &self.play.listed {
            if squads.faction[s] == player && Some(squads.route_code[s]) == route {
                orders::queue(squads, squads.id[s], RETREAT);
            }
        }
    }

    /// FormationField.set_wait: B's waves wait in the wood until they see A's (a chieftain
    /// timing the flank to meet A at the line's front), going anyway after a minute; or not.
    pub fn set_wait(&mut self, waiting: bool) {
        let fallback = (WAIT_SECONDS / self.play.set.tick_seconds).round() as i64;
        self.waves[B].staging = waiting.then(|| Stage {
            at: STAGING_AT,
            sees_fight: false,
            partner: 0,
            fallback,
            then: Some("go".to_string()),
            meet: Some((V2::new(CONTACT[A] as f32, 32.0), CONTACT[B])),
        });
    }
}

/// The digest's field log played (formation_digest.gd's `_field`): `record`'s field set up
/// on `squads` and `field`, its blows rolled if `rolls`, each tick's commands given and the
/// tick played (its events taken from `log` and dropped) till `extra` ticks past the last
/// command. The ticks played and the builders' and waves' events in all; None, with nothing
/// played, if the field can't be set up.
pub fn replay(
    record: &Record,
    extra: i64,
    (squads, field): (&mut Squads, &mut Field),
    content: &Content,
    (rolls, threaded): (bool, bool),
    log: &mut Log,
) -> Option<(i64, i64)> {
    let at = (record.seed, rolls, threaded);
    let mut played = FieldPlay::new(record.captained, (squads, field), content, at)?;
    let last = record.commands.last().map_or(0, |c| c.tick);
    let mut produced = 0;
    for tick in 0..last + extra {
        for given in record.commands.iter().filter(|c| c.tick == tick) {
            played.apply(given, (squads, field), content);
        }
        produced += played.step(squads, field, content, log).0.len() as i64;
        log.take();
    }
    Some((last + extra, produced))
}

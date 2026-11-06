//! What a field being played keeps beside the crate's state (spec 39 step 8d-ii), so a cut of
//! a field log can be read back: the squads listed, the definitions coded as first met, each
//! route's code, each wave's places filled and orders, the builders' work and the reserve,
//! the routes whose waves hurry and what each route's waves do with their downed. What the
//! field's set-up fixes (its battle, its routes' points, the waves' templates, the builders'
//! kinds, the kingdom's squads) isn't carried: `FieldPlay::carrying` sets the field up again
//! from its record and lays this over it.

#[cfg(test)]
mod tests;

use serde::{Deserialize, Serialize};

use super::FieldPlay;
use crate::content::Content;
use crate::field::Field;
use crate::record::Record;
use crate::squads::Squads;
use crate::staging::Stage;
use crate::threads::Threads;
use crate::waves::Builder;

/// A wave's state: its places filled, whether it was announced full and departs when full,
/// the route it goes by and how it stages.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WaveCarried {
    pub filled: Vec<bool>,
    pub announced: bool,
    pub auto: bool,
    pub route: usize,
    pub staging: Option<Stage>,
}

/// FieldPlay's own state, by name where it holds a definition.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Carried {
    pub listed: Vec<usize>,
    pub definitions: Vec<String>,
    pub route_codes: Vec<Option<i64>>,
    pub waves: Vec<WaveCarried>,
    pub builders: Vec<Vec<Builder>>,
    pub reserve: Vec<String>,
    pub turn: usize,
    pub hurried: Vec<usize>,
    pub tending: Vec<(usize, String)>,
}

impl Carried {
    /// Written whole, as the snapshot writes the crate's state (bincode).
    pub fn write(&self) -> Vec<u8> {
        bincode::serialize(self).expect("a field's carried state serialises")
    }

    /// Read back; None if the bytes aren't one.
    pub fn read(bytes: &[u8]) -> Option<Carried> {
        bincode::deserialize(bytes).ok()
    }
}

impl FieldPlay {
    /// What this field keeps beside the crate's state.
    pub fn carried(&self) -> Carried {
        let named = |ids: &[&'static str]| ids.iter().map(|id| id.to_string()).collect();
        let waves = self.waves.iter().map(|wave| WaveCarried {
            filled: wave.filled.clone(),
            announced: wave.announced,
            auto: wave.auto,
            route: wave.route,
            staging: wave.staging.clone(),
        });
        Carried {
            listed: self.play.listed.clone(),
            definitions: named(&self.play.codes.definitions),
            route_codes: self.routes.iter().map(|(_, code)| *code).collect(),
            waves: waves.collect(),
            builders: self.builders.kinds.iter().map(|(_, _, list)| list.clone()).collect(),
            reserve: named(&self.builders.reserve),
            turn: self.builders.turn,
            hurried: self.hurried.clone(),
            tending: self.tending.clone(),
        }
    }

    /// `record`'s field set up afresh (on a scratch crate, which is dropped), then `carried`
    /// laid over it at `tick`, its blows rolled if `rolls`, on one thread; None if it can't be
    /// set up or `carried` doesn't fit it (a definition it doesn't know, or the waves, routes
    /// or builders' kinds counted otherwise).
    pub fn carrying(
        record: &Record,
        carried: &Carried,
        (tick, rolls): (i64, bool),
        content: &Content,
    ) -> Option<FieldPlay> {
        let (mut squads, mut field) = (Squads::default(), Field::default());
        let at = (record.seed, rolls, Threads::One);
        let mut played = FieldPlay::new(record.captained, (&mut squads, &mut field), content, at)?;
        let fits = carried.waves.len() == played.waves.len()
            && carried.route_codes.len() == played.routes.len()
            && carried.builders.len() == played.builders.kinds.len();
        if !fits {
            return None;
        }
        let definitions: Option<Vec<&'static str>> =
            carried.definitions.iter().map(|name| played.known(name)).collect();
        let reserve: Option<Vec<&'static str>> =
            carried.reserve.iter().map(|name| played.known(name)).collect();
        played.play.codes.definitions = definitions?;
        played.builders.reserve = reserve?;
        for (route, code) in played.routes.iter_mut().zip(&carried.route_codes) {
            route.1 = *code;
        }
        for (wave, kept) in played.waves.iter_mut().zip(&carried.waves) {
            wave.filled.clone_from(&kept.filled);
            wave.announced = kept.announced;
            wave.auto = kept.auto;
            wave.route = kept.route;
            wave.staging.clone_from(&kept.staging);
        }
        for (kind, list) in played.builders.kinds.iter_mut().zip(&carried.builders) {
            kind.2.clone_from(list);
        }
        played.builders.turn = carried.turn;
        played.play.listed.clone_from(&carried.listed);
        played.play.tick = tick;
        played.hurried.clone_from(&carried.hurried);
        played.tending.clone_from(&carried.tending);
        Some(played)
    }

    /// The definition `name` as the field set up holds it: one its set-up coded, a builder
    /// builds or a wave's template paints.
    fn known(&self, name: &str) -> Option<&'static str> {
        let coded = self.play.codes.definitions.iter().copied();
        let built = self.builders.kinds.iter().map(|kind| kind.0);
        let painted = self.waves.iter().flat_map(|wave| wave.template.ordered()).map(|p| p.id);
        coded.chain(built).chain(painted).find(|&id| id == name)
    }
}
